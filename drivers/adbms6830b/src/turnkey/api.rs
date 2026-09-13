//! Manages `N` ADBMS6830B chips reachable from two isoSPI lines.
//! 
//! This driver uses the term "logical order" a lot because that's what claude kept using. For these
//! purposes, "logical order" is the order of chips from the POV of Line A.

use embedded_hal_async::spi::SpiDevice;

use crate::chip::commands::{self, Command, CommandFrame};
use crate::chip::registers::{ReadableGroup, WritableGroup};
use crate::docs;
use crate::line::{conversion_times, ChipResponse, Error, Line};
use crate::chip::registers::{
    config_a::ConfigA,
    config_b::ConfigB,
};

/// Highest value `CCNT[5:0]` reaches before rolling over.
const CCNT_MAX: u8 = 63;

/// Which isoSPI line a chip is reached from.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LineId {
    /// Reaches chips from logical index `0` upwards.
    A,
    /// Reaches chips from logical index `N - 1` downwards.
    B,
}

/// Helper struct for `ChipState` that stores command count data.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct CommandCount {
    /// Expected command count.
    expected: u8,
    /// Reported command count (reported via reads).
    reported: u8,
}
impl CommandCount {
    /// New CommandCount with initial state.
    pub(crate) const fn new() -> Self {
        CommandCount {
            expected: 0,
            reported: 0,
        }
    }

    /// What this chip's counter "should" be. This is tracked from the commands sent to it.
    pub const fn expected(&self) -> u8 {
        self.expected
    }

    /// What this chip reported on the last read of it that passed its PEC.
    pub const fn reported(&self) -> u8 {
        self.reported
    }

    /// Whether the reported counter matches the expected one.
    /// 
    /// This can be expected to be false in some cases, like after isoSPI recovers from a break.
    pub const fn in_sync(&self) -> bool {
        self.expected == self.reported
    }
}

/// Command counter state for one chip.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChipState {
    /// Command Count metadata.
    pub(crate) command_count: CommandCount,
    /// Number of times the command count for this chip has been reset due to a sleep.
    pub(crate) command_count_resets: usize,
    /// Last time we heard from this chip with a good PEC.
    /// 
    /// `None` means we haven't heard from this chip yet.
    pub(crate) last_contacted: Option<embassy_time::Instant>,
    /// Number of times this chip has read in a successful PEC. 
    pub(crate) pec_success_count: usize,
    /// Number of times this chip has read in a failed PEC.
    pub(crate) pec_failed_count: usize,
}

impl ChipState {
    /// Command Count metadata.
    pub const fn command_count(&self) -> CommandCount {
        self.command_count
    }

    /// Resets an already-initialized command counter.
    /// 
    /// ### Parameters
    /// - `count`: The `count` you want to initialize both `expected` and `reported` to.
    pub(crate) const fn reset_command_count(&mut self, count: u8) {
        self.command_count = CommandCount {
            expected: count,
            reported: count,
        };
        self.command_count_resets += 1;
    }

    /// Last time we heard from this chip with a good PEC.
    /// 
    /// `None` means we haven't heard from this chip yet.
    pub const fn last_contacted(&self) -> Option<embassy_time::Instant> {
        self.last_contacted
    }

    /// Returns the number of times the command counter for this chip has
    /// been reset.
    pub const fn command_count_resets(&self) -> usize {
        self.command_count_resets
    }

    /// Number of times this chip has read in a successful PEC. 
    pub const fn pec_success_count(&self) -> usize {
        self.pec_success_count
    }

    /// Number of times this chip has read in a failed PEC.
    pub const fn pec_failed_count(&self) -> usize {
        self.pec_failed_count
    }
}

/// Module governing the register groups the application is allowed to write directly.
pub mod writeables {
    use super::super::super::{
        chip::registers::{
            WritableGroup,
            clear,
            pwm,
            comm
        }
    };
    
    /// A writable group the application is allowed to write directly.
    /// 
    /// Not all register groups are in here on purpose (since some need to be gaurded)
    #[diagnostic::on_unimplemented(
        message = "`{Self}` cannot be written through `Api::write()`",
        label = "this register group is owned by the Service",
        note = "use the dedicated methods instead"
    )]
    pub trait AppWritableGroup: WritableGroup {}
    impl AppWritableGroup for clear::ClearFlags {}
    impl AppWritableGroup for clear::ClearOvervoltageUndervoltage {}
    impl AppWritableGroup for pwm::PwmA {}
    impl AppWritableGroup for pwm::PwmB {}
    impl AppWritableGroup for comm::WriteCommI2c {}
    impl AppWritableGroup for comm::WriteCommSpi {}
}

/// Per-chip results of a read.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Responses<G, E, const N: usize> {
    chips: [Option<ChipResponse<G>>; N],
    errors: [Option<Error<E>>; 2],
}

impl<G: Copy, E, const N: usize> Responses<G, E, N> {
    /// The response from one chip.
    /// 
    /// If the chip's line transaction failed, this will return `None` (because that entire line would not be
    /// carrying any data). Upon getting a `None` response, you may look up the culprit SPI error via the `line_error()` function.
    pub fn chip(&self, chip: usize) -> Option<ChipResponse<G>> {
        self.chips[chip]
    }

    /// Every chip's response in logical order.
    pub fn iter(&self) -> impl Iterator<Item = Option<ChipResponse<G>>> + '_ {
        self.chips.iter().copied()
    }

    /// Returns the error that failed a line's transaction, if there was one.
    /// If the chip's line had a successful read, this will return `None` (since there's no error to report).
    pub fn line_error(&self, line: LineId) -> Option<&Error<E>> {
        self.errors[line as usize].as_ref()
    }

    /// Whether every chip answered and passed its PEC check.
    pub fn all_ok(&self) -> bool {
        self.chips
            .iter()
            .all(|chip| chip.is_some_and(|chip| chip.pec().is_success()))
    }
}

#[cfg(feature = "defmt")]
impl<G: Copy, E, const N: usize> defmt::Format for Responses<G, E, N> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "Responses {{ chips: {=usize}, all_ok: {=bool}, line_a_failed: {=bool}, line_b_failed: {=bool} }}",
            N,
            self.all_ok(),
            self.errors[0].is_some(),
            self.errors[1].is_some()
        )
    }
}

/// Newtype representing how many chips are on Line A.
/// This newtype is kinda pointless but it is useful for docs since it
/// is somewhat hard to remember what this value actually means.
/// 
/// TLDR: The inner represents the number of chips that are on Line A. In other words, chips `0..self.0` are reached from Line A, with `self.0` not being inclusive.
/// This defaults to `N`, where `N` is the total number of chips there are across both lines, no matter what the split is. So if `N = 10`, `OnLineA(10)` would mean that
/// all 10 chips are on Line A. "All 10 chips" corresponds to Chip 0 through Chip 9 (since the chips are 0-indexed).
/// Likewise, `OnLineA(0)` would mean that all 10 chips are on Line B.
/// 
/// Examples:
/// - `OnLineA(10)`: All 10 chips are on Line A
/// - `OnLineA(0)`: All 10 chips are on Line B
/// - `OnLineA(4)`: Chips `0..4` are on Line A, and chips `4..10` are on Line B. Aka: Chip 0 through Chip 3 are on Line A, and Chip 4 through Chip 9 are on Line B.
/// - `OnLineA(6)`: Chips `0..6` are on Line A, and chips `6..10` are on Line B. Aka: Chip 0 through Chip 5 are on Line A, and Chip 6 through Chip 9 are on Line B.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct OnLineA(pub usize);
impl From<usize> for OnLineA {
    fn from(num: usize) -> Self {
        Self(num)
    }
}
impl From<OnLineA> for usize {
    fn from(num: OnLineA) -> Self {
        num.0
    }
}

/// Two isoSPI lines reaching `N` chips, plus the state tracked for those chips.
///
#[doc = docs::isospi_indexing_example!()]
pub struct Api<SPI, const N: usize> {
    line_a: Line<SPI, N>,
    line_b: Line<SPI, N>,
    on_line_a: OnLineA,
    /// State metadata for each chip
    pub(crate) chips: [ChipState; N],

    /// cached ConfigA setting for the use of isoSPI recovery
    /// should not be public to the rest of the crate!
    config_a: [ConfigA; N],

    /// Count of times a SPI error has occured for Line A. For diagnostics.
    pub(crate) line_a_error_count: usize,
    /// Count of times a SPI error has occured for Line B. For diagnostics.
    pub(crate) line_b_error_count: usize,
    /// Most recent `Error` that has occured on Line A. `None` if no errors have occured yet.
    pub(crate) most_recent_line_a_error: Option<Error<embedded_hal_async::spi::ErrorKind>>,
    /// Most recent `Error` that has occured on Line B. `None` if no errors have occured yet.
    pub(crate) most_recent_line_b_error: Option<Error<embedded_hal_async::spi::ErrorKind>>,
}

/// isoSPI recovery!
impl<SPI: SpiDevice, const N: usize> Api<SPI, N> {
    /// PRIVATE! Helper that overwrites the provided ConfigAs with the current split's COMM_BK setting.
    fn overwrite_configa_with_split(&mut self, configs: &mut [ConfigA; N]) {
        use crate::chip::registers::config_a::types::CommunicationBreak;

        let boundary: usize = self.split().into();
        let split_active = boundary > 0 && boundary < N;
        for (i, config) in configs.iter_mut().enumerate() {
            let bk = if split_active && (i == boundary || i == boundary - 1) {
                CommunicationBreak::Enable
            } else {
                CommunicationBreak::Disable
            };
            *config = config.with_comm_bk(bk);
        }
    }

    /// Splits the chain at `chip`. Chips `0..chip` will be on Line A, and chips `chip..N` will be on Line B.
    ///
    /// This sets the `COMM_BK` bit for the two chips on either side of the new boundary point. See
    /// the "COMMUNICATION BREAK" section on page 53 of the datasheet.
    ///
    /// Note: `OnLineA(0)` puts all chips on Line B and `OnLineA(N)` puts all chips on Line A, so for those
    /// specific inputs, no boundary is set and no COMM_BK is set.
    /// 
    /// Other probably more important note: If this function returns an error, the split state (both in software and in hardware)
    /// are not guaranteed to be valid. If the SPI transactions carrying the COMM_BK commands fail, it isn't
    /// really possible for this function to tell if the COMM_BK commands were successfully recieved by the chips or not. So, the software state
    /// will kind of be in limbo. As such, it is the caller's responsibility to have some kind of recovery routine for when this
    /// function returns an error. Said recovery routine should probably not end until this function returns Ok(()), since only then is
    /// the state guaranteed as valid.
    pub(crate) async fn split_at(&mut self, chip: OnLineA) -> Result<(), Error<SPI::Error>> {
        self.set_split(chip)?;

        // DO A READ-MODIFY-WRITE OF CONFIGA:

        // for the "read" part of the read-modify-write, we are using the cached config_a we track in self
        // technically it probably would be more fullproof to just read the config_a states directly from what they
        // are on the chips right now, but that would use an additional SPI transaction that itself could be another point
        // of failure, especially since we are attempting to recover rn
        let mut configs: [ConfigA; N] = core::array::from_fn(|i| self.config_a[i]);
        self.overwrite_configa_with_split(&mut configs);
        self.set_configa(&configs).await?;

        Ok(())
    }

    /// Resets all chips to their default states and wakes them up.
    pub async fn reset(&mut self) -> Result<(), Error<SPI::Error>> {
        use crate::chip::registers::clear::{
            ClearFlags,
            ClearOvervoltageUndervoltage,
        };

        // make sure they're awake to recieve the srst command first
        self.wakeup().await?;

        // issue SRST
        self.command(commands::misc::srst()).await?;

        // wake them up again since we just used SRST on them
        self.wakeup().await?;

        // clear all flags so we start from normal
        let clear = ClearFlags::clear_all();
        let clears = [clear; N];
        self.write::<ClearFlags>(&clears).await?;

        // also clear all the OV/UV flags
        let clear = ClearOvervoltageUndervoltage::clear_all();
        let clears = [clear; N];
        self.write::<ClearOvervoltageUndervoltage>(&clears).await?;

        let boundary: usize = self.split().into();
        let split_active = boundary > 0 && boundary < N;

        // reset cached configA, and if a split is active, write the reset configA but with the COMM_BK bit set
        // this returns what we should reset the command count to
        let expected_command_count: u8 = if split_active {
            // if a split is active we need to re-write configA with COMM_BK. set_configa() will write a blank config, but with the COMM_BK bit set as necessary
            self.set_configa(&[ConfigA::new(); N]).await?;

            // ClearFlags + ClearOvervoltageUndervoltage + ConfigA
            3
        } else {
            // no split is active so cached configA should be blank, plus we don't need to write anything
            self.config_a = [ConfigA::new(); N];

            // just ClearFlags and ClearOvervoltageUndervoltage
            2
        };

        // reset command counts for every chip
        for chip in self.chips.iter_mut() {
            // we should know the exact expected command count by now based on the `expected_command_count` block
            // there can't be any writes in-between this because this is &mut self, so even if ClearFlags gets .awaited another task can't do anything that raises command count
            chip.reset_command_count(expected_command_count);
        }
        // the whole point of this reset() function is to reset to a known state, so any prior command count mismatches are basically thrown away here (which is on purpose). this is supposed to be a blank slate

        Ok(())
    }
}

#[cfg(feature = "defmt")]
impl<SPI, const N: usize> defmt::Format for Api<SPI, N> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "Api {{ chips: {=usize}, on_line_a: {=usize} }}",
            N,
            usize::from(self.on_line_a)
        )
    }
}

impl<SPI: SpiDevice, const N: usize> Api<SPI, N> {
    /// Builds a Api. This defaults to every chip routed to line A.
    pub(crate) const fn new(line_a: Line<SPI, N>, line_b: Line<SPI, N>) -> Self {
        Self {
            line_a,
            line_b,
            on_line_a: OnLineA(N),
            chips: [ChipState {
                command_count: CommandCount::new(),
                command_count_resets: 0,
                last_contacted: None,
                pec_success_count: 0,
                pec_failed_count: 0,
            }; N],
            config_a: [ConfigA::new(); N],
            line_a_error_count: 0,
            most_recent_line_a_error: None,
            line_b_error_count: 0,
            most_recent_line_b_error: None,
        }
    }

    /// Releases both lines' SPI devices.
    #[allow(dead_code)]
    pub(crate) fn release(self) -> (SPI, SPI) {
        (self.line_a.release(), self.line_b.release())
    }

    /// Per-chip metadata.
    pub const fn chips(&mut self) -> &[ChipState; N] {
        &mut self.chips
    }

    /// CRATE PRIVATE! How many chips are currently routed to line A.
    pub(crate) const fn split(&self) -> OnLineA {
        self.on_line_a
    }

    /// CRATE PRIVATE! Sets chips `0..on_line_a` to Line A, and `on_line_a..N` to Line B.
    ///
    /// This only changes the tracked software state! Callers must set `COMM_BK` as appropriate or
    /// commands will not adhere to the tracked software state.
    ///
    /// Note: It is up to the caller to call this function before setting COMM_BK on the hardware, or after. 
    /// Which order to do those in depends on why you are splitting:
    /// - If you are splitting a healthy chain (just to use both halves at once for whatever reason), you should assert `COMM_BK` first, since
    ///   without it commands cross the COMM_BK boundary while you are in the middle of making the change.
    /// - If you are trying to recover from a real break, you should call this function first before making the actual hardware `COMM_BK` asserts.
    ///   This is because an actual physical break makes it impossible for both ends to be reached from a single line (so configuring `COMM_K` on both sides
    ///   would not be possible). The physical break itself would already be keeping the halves apart in the meantime so you wouldnt need to worry about
    ///   commands crossing the COMM_BK boundary while you are in the middle of making the change.
    ///
    /// Also: See the `split_at()` (it does the second case for you).
    pub(crate) fn set_split(&mut self, on_line_a: OnLineA) -> Result<(), Error<SPI::Error>> {
        if usize::from(on_line_a) > N {
            return Err(Error::TooManyDevices);
        }
        self.on_line_a = on_line_a;
        Ok(())
    }

    /// Which line a chip is currently reached from.
    pub(crate) fn line_of(&self, chip: usize) -> LineId {
        if chip < self.on_line_a.into() {
            LineId::A
        } else {
            LineId::B
        }
    }

    /// PRIVATE! A chip's index from the point of view of its line.
    fn index_of(&self, chip: usize) -> usize {
        if chip < self.on_line_a.into() {
            chip
        } else {
            N - 1 - chip
        }
    }

    /// PRIVATE! How many chips are on a line.
    fn count(&self, line: LineId) -> usize {
        match line {
            LineId::A => self.on_line_a.into(),
            LineId::B => N - usize::from(self.on_line_a),
        }
    }

    /// PRIVATE! Gets a mut reference to a Line
    fn line_mut(&mut self, line: LineId) -> &mut Line<SPI, N> {
        match line {
            LineId::A => &mut self.line_a,
            LineId::B => &mut self.line_b,
        }
    }

    /// PRIVATE! Applies a dispatched frame to the tracked counters of every chip on `line`.
    fn note(&mut self, line: LineId, frame: CommandFrame) {
        if !frame.increments() && !frame.resets_counter() {
            return;
        }

        let chips = match line {
            LineId::A => 0..self.on_line_a.into(),
            LineId::B => self.on_line_a.into()..N,
        };

        for chip in chips {
            let expected = &mut self.chips[chip].command_count.expected;
            // 0 is reserved for resets, so the counter rolls over to 1. See page 53 of the datasheet.
            *expected = match () {
                _ if frame.resets_counter() => 0,
                _ if *expected >= CCNT_MAX => 1,
                _ => *expected + 1,
            };
        }
    }

    /// PRIVATE! Reads a register group from every chip on one line.
    async fn read_line<G: ReadableGroup>(
        &mut self,
        line: LineId,
    ) -> Result<crate::line::Responses<G, N>, Error<SPI::Error>> {
        let count = self.count(line);
        if count == 0 {
            return Ok(crate::line::Responses::empty());
        }

        self.note(line, G::READ_COMMAND);
        let result = self.line_mut(line).read::<G>(count).await;

        if let Err(err) = &result {
            match line {
                LineId::A => {
                    self.line_a_error_count += 1;
                    self.most_recent_line_a_error = Some(err.to_kind());
                    #[cfg(feature = "defmt")]
                    defmt::error!("ADBMS6830B: API: In `read_line()`: SPI transaction to Line A failed: {}", err.to_kind())
                },
                LineId::B => {
                    self.line_b_error_count += 1;
                    self.most_recent_line_b_error = Some(err.to_kind());
                    #[cfg(feature = "defmt")]
                    defmt::error!("ADBMS6830B: API: In `read_line()`: SPI transaction to Line B failed: {}", err.to_kind())
                }

            }
        }

        result
    }

    /// Reads a register group from every chip.
    pub async fn read<G: ReadableGroup>(&mut self) -> Responses<G, SPI::Error, N> {
        let line_a = self.read_line::<G>(LineId::A).await;
        let line_b = self.read_line::<G>(LineId::B).await;

        let mut chips = [None; N];

        for (chip, slot) in chips.iter_mut().enumerate() {
            let responses = match self.line_of(chip) {
                LineId::A => &line_a,
                LineId::B => &line_b,
            };

            let Ok(responses) = responses else { continue };
            let Some(response) = responses.get(self.index_of(chip)) else {
                continue;
            };

            // A failed PEC corrupts the counter bits too, so only believe a counter that checked out.
            if response.pec().is_success() {
                self.chips[chip].command_count.reported = response.command_counter();
                self.chips[chip].last_contacted = Some(embassy_time::Instant::now());
                self.chips[chip].pec_success_count += 1;
            } else {
                self.chips[chip].pec_failed_count += 1;
            }
            *slot = Some(*response);
        }

        Responses {
            chips,
            errors: [line_a.err(), line_b.err()],
        }
    }

    /// Read-only access to the cached ConfigA.
    #[allow(dead_code)]
    pub(crate) fn configa(&self) -> &[ConfigA; N] {
        &self.config_a
    }

    /// Sets ConfigA.
    /// 
    /// This will also update the cached ConfigA value inside Api. Also, it will overwrite whatever
    /// COMM_BK value you have provied with whatever is determined by the current Line split.
    pub async fn set_configa(&mut self, configs: &[ConfigA; N]) -> Result<(), Error<SPI::Error>> {
        let mut configs = *configs;
        self.overwrite_configa_with_split(&mut configs);
        self.config_a = configs;
        self.private_write(&configs).await
    }

    /// Sets ConfigB.
    pub async fn set_configb(&mut self, configs: &[ConfigB; N]) -> Result<(), Error<SPI::Error>> {
        // this function does nothing special right now. but probably keep it here in case we need to cache configb in the future
        self.private_write(configs).await
    }
    
    /// Writes one register group per chip. `groups` is indexed in logical chip order.
    pub async fn write<G: writeables::AppWritableGroup>(&mut self, groups: &[G; N]) -> Result<(), Error<SPI::Error>> {
        self.private_write(groups).await
    }

    /// Writes one register group per chip. `groups` is indexed in logical chip order.
    /// 
    /// PRIVATE! This lets you pass in any `WriteableGroup` so it's not meant to be used outside of this mod.
    async fn private_write<G: WritableGroup>(&mut self, groups: &[G; N]) -> Result<(), Error<SPI::Error>> {
        let (count_a, count_b) = (self.count(LineId::A), self.count(LineId::B));

        let line_a = if count_a > 0 {
            self.note(LineId::A, G::WRITE_COMMAND);
            self.line_a.write(&groups[..count_a]).await
        } else {
            Ok(())
        };

        let line_b = if count_b > 0 {
            // Line B reaches its chips in reverse logical order.
            let reversed: [G; N] = core::array::from_fn(|i| groups[N - 1 - i]);
            self.note(LineId::B, G::WRITE_COMMAND);
            self.line_b.write(&reversed[..count_b]).await
        } else {
            Ok(())
        };

        if let Err(err) = &line_a {
            self.line_a_error_count += 1;
            self.most_recent_line_a_error = Some(err.to_kind());
            #[cfg(feature = "defmt")]
            defmt::error!("ADBMS6830B: API: In `write()`: SPI transaction to Line A failed: {}", err.to_kind())
        }

        if let Err(err) = &line_b {
            self.line_b_error_count += 1;
            self.most_recent_line_b_error = Some(err.to_kind());
            #[cfg(feature = "defmt")]
            defmt::error!("ADBMS6830B: API: In `write()`: SPI transaction to Line B failed: {}", err.to_kind())
        }

        line_a.and(line_b)
    }

    /// Sends a command to every chip on both lines.
    pub async fn command(&mut self, command: Command) -> Result<(), Error<SPI::Error>> {
        let line_a = self.command_line(LineId::A, command).await;
        let line_b = self.command_line(LineId::B, command).await;

        line_a.and(line_b)
    }

    /// CRATE PRIVATE! Sends a command to the chips on one line.
    pub(crate) async fn command_line(
        &mut self,
        line: LineId,
        command: Command,
    ) -> Result<(), Error<SPI::Error>> {
        let count = self.count(line);

        if count == 0 {
            return Ok(());
        }

        self.note(line, command.frame());
        let result = self.line_mut(line).command(count, command).await;
        if let Err(err) = &result {
            match line {
                LineId::A => {
                    self.line_a_error_count += 1;
                    self.most_recent_line_a_error = Some(err.to_kind());
                    #[cfg(feature = "defmt")]
                    defmt::error!("ADBMS6830B: API: In `command_line()`: SPI transaction to Line A failed: {}", err.to_kind())
                },
                LineId::B => {
                    self.line_b_error_count += 1;
                    self.most_recent_line_b_error = Some(err.to_kind());
                    #[cfg(feature = "defmt")]
                    defmt::error!("ADBMS6830B: API: In `command_line()`: SPI transaction to Line B failed: {}", err.to_kind())
                }

            }
        }

        result
    }

    /// Sends a poll command to both lines and reports whether every chip has finished.
    ///
    /// This does not wait! It just returns a bool, so you need to keep polling it until it returns true (indicating that every chip has finished).
    /// If you want that done automatically for you, see the `*_autoconvert()` helpers.
    pub async fn poll(&mut self, command: Command) -> Result<bool, Error<SPI::Error>> {
        let line_a = self.poll_line(LineId::A, command).await;
        let line_b = self.poll_line(LineId::B, command).await;
        Ok(line_a? && line_b?)
    }

    /// PRIVATE! Sends a poll command to one line. A line with no chips reports `true`.
    async fn poll_line(
        &mut self,
        line: LineId,
        command: Command,
    ) -> Result<bool, Error<SPI::Error>> {
        let count = self.count(line);
        if count == 0 {
            return Ok(true);
        }

        self.note(line, command.frame());
        let result = self.line_mut(line).poll(command, count).await;
        if let Err(err) = &result {
            match line {
                LineId::A => {
                    self.line_a_error_count += 1;
                    self.most_recent_line_a_error = Some(err.to_kind());
                    #[cfg(feature = "defmt")]
                    defmt::error!("ADBMS6830B: API: In `poll_line()`: SPI transaction to Line A failed: {}", err.to_kind())
                },
                LineId::B => {
                    self.line_b_error_count += 1;
                    self.most_recent_line_b_error = Some(err.to_kind());
                    #[cfg(feature = "defmt")]
                    defmt::error!("ADBMS6830B: API: In `poll_line()`: SPI transaction to Line B failed: {}", err.to_kind())
                }

            }
        }

        result
    }

    /// Wakes every chip on both lines out of the idle or sleep state.
    ///
    /// Chips that were asleep come back with their counters at 0.
    pub async fn wakeup(&mut self) -> Result<(), Error<SPI::Error>> {
        let (count_a, count_b) = (self.count(LineId::A), self.count(LineId::B));

        let result_a = self.line_a.wakeup(count_a).await;
        if let Err(err) = &result_a {
            self.line_a_error_count += 1;
            self.most_recent_line_a_error = Some(err.to_kind());
            #[cfg(feature = "defmt")]
            defmt::error!("ADBMS6830B: API: In `wakeup()`: SPI transaction to Line A failed: {}", err.to_kind());
        }

        let result_b = self.line_b.wakeup(count_b).await;
        if let Err(err) = &result_b {
            self.line_b_error_count += 1;
            self.most_recent_line_b_error = Some(err.to_kind());
            #[cfg(feature = "defmt")]
            defmt::error!("ADBMS6830B: API: In `wakeup()`: SPI transaction to Line B failed: {}", err.to_kind());
        }

        result_a.and(result_b)
    }

    /// CRATE PRIVATE! Counts the chips reachable on one line. This stops at the first bad PEC (note: this means that comm
    /// errors that result in a bad PEC could look like the line end, so probably call this a few times).
    ///
    /// Useful for locating a break. This doesn't change the split.
    #[allow(dead_code)]
    pub(crate) async fn detect_chips(&mut self, line: LineId) -> Result<usize, Error<SPI::Error>> {
        let result = self.line_mut(line).detect_chips().await;
        if let Err(err) = &result {
            match line {
                LineId::A => {
                    self.line_a_error_count += 1;
                    self.most_recent_line_a_error = Some(err.to_kind());
                    #[cfg(feature = "defmt")]
                    defmt::error!("ADBMS6830B: API: In `detect_chips()`: SPI transaction to Line A failed: {}", err.to_kind());
                },
                LineId::B => {
                    self.line_b_error_count += 1;
                    self.most_recent_line_b_error = Some(err.to_kind());
                    #[cfg(feature = "defmt")]
                    defmt::error!("ADBMS6830B: API: In `detect_chips()`: SPI transaction to Line B failed: {}", err.to_kind());
                }
            }
        }

        result
    }
}

/// # Automatic conversions
///
/// Each helper starts a conversion, waits the time it is expected to take, and then polls until every
/// chip reports that it's finished.
impl<SPI: SpiDevice, const N: usize> Api<SPI, N> {
    /// Sends the `start` command, waits `conversion_ms` milliseconds, and then polls until done (or `timeout_ms` elapses).
    async fn autoconvert(
        &mut self,
        start: Command,
        poll: Command,
        conversion_ms: u64,
        timeout_ms: u64,
    ) -> Result<(), Error<SPI::Error>> {
        use embassy_time::{Duration, Instant, Timer};

        /// How long to wait between polls once the expected conversion time has elapsed.
        const POLL_INTERVAL_MS: u64 = 1;

        let deadline = Instant::now() + Duration::from_millis(timeout_ms);

        self.command(start).await?;
        Timer::after_millis(conversion_ms).await;

        loop {
            if self.poll(poll).await? {
                return Ok(());
            }

            if Instant::now() >= deadline {
                return Err(Error::Timeout);
            }

            Timer::after_millis(POLL_INTERVAL_MS).await;
        }
    }

    /// Starts a cell voltage conversion (ADCV) and waits for it to finish.
    pub async fn adcv_autoconvert(
        &mut self,
        redundancy: commands::adc::AdcvRedundancy,
        acquisition: commands::adc::AutoAcquisition,
        reset_filter: commands::adc::ResetFilter,
        open_wire: commands::adc::OpenWire,
        timeout_ms: u64,
    ) -> Result<(), Error<SPI::Error>> {
        let conversion_ms = match redundancy {
            commands::adc::AdcvRedundancy::Enabled => conversion_times::S_ADC_MS,
            commands::adc::AdcvRedundancy::Disabled => conversion_times::C_ADC_MS,
        };

        self.autoconvert(
            commands::adc::adcv(redundancy, acquisition.into(), reset_filter, open_wire),
            commands::adc::plcadc(),
            conversion_ms,
            timeout_ms,
        )
        .await
    }

    /// Starts an S-ADC conversion (ADSV) and waits for it to finish.
    pub async fn adsv_autoconvert(
        &mut self,
        acquisition: commands::adc::AutoAcquisition,
        open_wire: commands::adc::OpenWire,
        timeout_ms: u64,
    ) -> Result<(), Error<SPI::Error>> {
        self.autoconvert(
            commands::adc::adsv(acquisition.into(), open_wire),
            commands::adc::plsadc(),
            conversion_times::S_ADC_MS,
            timeout_ms,
        )
        .await
    }

    /// Starts an AUX conversion (ADAX) and waits for it to finish.
    /// 
    /// This doesn't account for time added by SOAKON (see Config A). It should still work if SOAKON is
    /// enabled, it just might not be as efficient since it will poll at the same frequency as when SOAKON
    /// is not enabled.
    pub async fn adax_autoconvert(
        &mut self,
        open_wire: commands::adc::OpenWireAux,
        pull: commands::adc::Pull,
        channel: commands::adc::Aux1InputSelection,
        timeout_ms: u64,
    ) -> Result<(), Error<SPI::Error>> {
        self.autoconvert(
            commands::adc::adax(open_wire, pull, channel),
            commands::adc::plaux(),
            conversion_times::AUX_MS,
            timeout_ms,
        )
        .await
    }

    /// Starts an AUX2 conversion (ADAX2) and waits for it to finish.
    pub async fn adax2_autoconvert(
        &mut self,
        channel: commands::adc::Aux2InputSelection,
        timeout_ms: u64,
    ) -> Result<(), Error<SPI::Error>> {
        self.autoconvert(
            commands::adc::adax2(channel),
            commands::adc::plaux2(),
            conversion_times::AUX2_MS,
            timeout_ms,
        )
        .await
    }
}
