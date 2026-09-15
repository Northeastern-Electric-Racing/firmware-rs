//! Module representing a single Line of daisy-chained devices

use embedded_hal_async::spi::{Operation, SpiDevice};

use crate::chip::commands::{self, Command, CommandFrame};
use crate::chip::pec::{DataPecRx, DataPecTx};
use crate::chip::registers::{ReadableGroup, WritableGroup, GROUP_BYTES};
use crate::docs;

/// Bytes one device sends or receives per register group (its data plus the data PEC).
const BLOCK_BYTES: usize = GROUP_BYTES + 2;

/// Errors returned by this driver.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    /// The underlying SPI transaction failed.
    Spi(E),
    /// More devices were addressed than there is buffer space for.
    TooManyDevices,
    /// A user-provided timeout elapsed before the operation finished.
    Timeout,
}
impl<E: embedded_hal_async::spi::Error> Error<E> {
    /// Converts the error into an error with embedded_hal_async::spi::ErrorKind.
    pub fn to_kind(&self) -> Error<embedded_hal_async::spi::ErrorKind> {
        match self {
            Error::Spi(err)       => Error::Spi(err.kind()),
            Error::TooManyDevices => Error::TooManyDevices,
            Error::Timeout        => Error::Timeout,
        }
    }
}


/// Result of a device's data PEC check.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PecStatus {
    /// The data PEC matched. This means the response data can be trusted.
    Success,
    /// The data PEC did not match. This means that the data was likely corrupted in transit.
    Failed,
}

impl PecStatus {
    /// Whether the PEC check passed.
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    /// Whether the PEC check failed.
    pub const fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// One device's answer to a read.
///
/// The data and command counter are readable even when the PEC failed, so check `pec()` before
/// trusting either of them.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ChipResponse<G> {
    data: G,
    command_counter: u8,
    pec: PecStatus,
}

impl<G: Copy> ChipResponse<G> {
    /// The register group the device sent back.
    pub const fn data(&self) -> G {
        self.data
    }

    /// Command counter (`CCNT[5:0]`) the device reported alongside its data.
    pub const fn command_counter(&self) -> u8 {
        self.command_counter
    }

    /// Whether this device's data PEC was found as valid.
    pub const fn pec(&self) -> PecStatus {
        self.pec
    }
}

impl<G: ReadableGroup> ChipResponse<G> {
    /// PRIVATE! Decodes one device's block off the wire (`GROUP_BYTES` of data, then PEC0/PEC1).
    fn from_block(block: &[u8; BLOCK_BYTES]) -> Self {
        let mut data = [0u8; GROUP_BYTES];
        data.copy_from_slice(&block[..GROUP_BYTES]);
        let pec = DataPecRx::from_bytes([block[GROUP_BYTES], block[GROUP_BYTES + 1]]);

        Self {
            data: G::from_bytes(data),
            command_counter: pec.ccnt(),
            pec: if pec.verify(&data) {
                PecStatus::Success
            } else {
                PecStatus::Failed
            },
        }
    }

    /// PRIVATE! Filler for slots that were never read.
    fn blank() -> Self {
        Self {
            data: G::from_bytes([0; GROUP_BYTES]),
            command_counter: 0,
            pec: PecStatus::Failed,
        }
    }
}

#[cfg(feature = "defmt")]
impl<G: defmt::Format> defmt::Format for ChipResponse<G> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "ChipResponse {{ data: {}, command_counter: {=u8}, pec: {} }}",
            self.data,
            self.command_counter,
            self.pec
        )
    }
}

/// Per-device results of a read on a line, nearest the host first.
///
/// Derefs to `[ChipResponse<G>]`. This means you can index it, iterate it, or use `.get()`/`.len()` on it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Responses<G, const N: usize> {
    chips: [ChipResponse<G>; N],
    used: usize,
}

impl<G: ReadableGroup, const N: usize> Responses<G, N> {
    /// PRIVATE! Decodes the first `used` blocks that came back off from SPI.
    fn from_blocks(blocks: &[[u8; BLOCK_BYTES]; N], used: usize) -> Self {
        Self {
            chips: core::array::from_fn(|i| {
                if i < used {
                    ChipResponse::from_block(&blocks[i])
                } else {
                    ChipResponse::blank()
                }
            }),
            used,
        }
    }

    /// A response covering no devices.
    pub(crate) fn empty() -> Self {
        Self {
            chips: core::array::from_fn(|_| ChipResponse::blank()),
            used: 0,
        }
    }
}

impl<G, const N: usize> Responses<G, N> {
    /// Whether every device covered by this response passed its PEC check.
    pub fn all_ok(&self) -> bool {
        self.chips[..self.used]
            .iter()
            .all(|chip| chip.pec.is_success())
    }
}

impl<G, const N: usize> core::ops::Deref for Responses<G, N> {
    type Target = [ChipResponse<G>];

    fn deref(&self) -> &Self::Target {
        &self.chips[..self.used]
    }
}

#[cfg(feature = "defmt")]
impl<G, const N: usize> defmt::Format for Responses<G, N> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "Responses {{ len: {=usize}, all_ok: {=bool} }}",
            self.used,
            self.all_ok()
        )
    }
}

/// An individual SPI/isoSPI line that goes up to `N` daisy-chained ADBMS6830B devices.
///
/// Index `0` is the device nearest this end of the line.
#[doc = docs::isospi_indexing_example!()]
pub struct Line<SPI, const N: usize> {
    spi: SPI,
    last_activity: embassy_time::Instant,
}

#[cfg(feature = "defmt")]
impl<SPI, const N: usize> defmt::Format for Line<SPI, N> {
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Line {{ max_chips: {=usize} }}", N)
    }
}

impl<SPI: SpiDevice, const N: usize> Line<SPI, N> {
    /// Builds a line on `spi`.
    /// 
    /// WARNING: The SpiDevice must support DelayNs, or else this will panic whenever you attempt to send a transaction. For example, don't use `ExclusiveDevice::new_no_delay()`.
    pub const fn new(spi: SPI) -> Self {
        Self { spi, last_activity: embassy_time::Instant::from_ticks(0) }
    }

    /// Releases the underlying SPI device.
    pub fn release(self) -> SPI {
        self.spi
    }

    /// PRIVATE!
    /// SPI bus transaction.
    /// `count` is the number of devices on the chain you are trying to communicate with.
    /// `frame` is the command frame you want to send.
    /// `payload` is the payload if there is one.
    async fn transact(
        &mut self,
        count: usize,
        frame: CommandFrame,
        payload: Option<Operation<'_, u8>>,
    ) -> Result<(), Error<SPI::Error>> {
        let bytes = frame.to_bytes();

        // make sure they are woke
        self.ensure_woke(count).await?;

        let err = match payload {
            Some(payload) => {
                self.spi
                    .transaction(&mut [Operation::Write(&bytes), payload])
                    .await
            }
            // this is only used for `command()` commands where they are just a command with no payload
            None => self.spi.transaction(&mut [Operation::Write(&bytes)]).await,
        }
        .map_err(Error::Spi);

        if err.is_ok() {
            self.last_activity = embassy_time::Instant::now();
        }

        err
    }

    /// Sends a command that carries no payload.
    /// 
    /// ### Parameters
    /// - `count`: Number of chips to send the command to. This is used for the wakeup logic.
    pub async fn command(&mut self, count: usize, command: Command) -> Result<(), Error<SPI::Error>> {
        self.transact(count, command.frame(), None).await
    }

    /// Reads a register group from the `count` devices nearest this end of the line.
    pub async fn read<G: ReadableGroup>(
        &mut self,
        count: usize,
    ) -> Result<Responses<G, N>, Error<SPI::Error>> {
        if count > N {
            return Err(Error::TooManyDevices);
        }

        let mut blocks = [[0u8; BLOCK_BYTES]; N];
        self.transact(
            count,
            G::READ_COMMAND,
            Some(Operation::Read(
                &mut blocks.as_flattened_mut()[..count * BLOCK_BYTES],
            )),
        )
        .await?;

        Ok(Responses::from_blocks(&blocks, count))
    }

    /// Writes one register group per device.
    pub async fn write<G: WritableGroup>(&mut self, groups: &[G]) -> Result<(), Error<SPI::Error>> {
        let count = groups.len();
        if count > N {
            return Err(Error::TooManyDevices);
        }

        // The first block on the wire ends up in the furthest device so we need to reverse the payload
        let mut blocks = [[0u8; BLOCK_BYTES]; N];
        for (i, group) in groups.iter().enumerate() {
            let block = &mut blocks[count - 1 - i];
            let data = group.to_bytes();
            block[..GROUP_BYTES].copy_from_slice(&data);
            let pec = DataPecTx::new(&data);
            block[GROUP_BYTES] = pec.pec0();
            block[GROUP_BYTES + 1] = pec.pec1();
        }

        self.transact(
            count,
            G::WRITE_COMMAND,
            Some(Operation::Write(
                &blocks.as_flattened()[..count * BLOCK_BYTES],
            )),
        )
        .await
    }

    /// Sends a poll command and reports whether all `count` devices have finished.
    ///
    /// This does not wait! You need to keep polling it until it returns `true`, or use the Api's
    /// `*_autoconvert()` helpers (which do this automatically).
    pub async fn poll(&mut self, command: Command, count: usize) -> Result<bool, Error<SPI::Error>> {
        if count > N {
            return Err(Error::TooManyDevices);
        }

        // Poll status is only valid after 2*N clock pulses and updates every pulse after that, so
        // this will clock past the invalid window and take the byte after it. See the "POLLING METHODS" section on page 54.
        let used = (2 * count).div_ceil(8) + 1;

        let mut buffer = [[0u8; BLOCK_BYTES]; N];
        self.transact(
            count,
            command.frame(),
            Some(Operation::Read(&mut buffer.as_flattened_mut()[..used])),
        )
        .await?;

        Ok(buffer.as_flattened()[used - 1] == 0xFF)
    }

    /// PRIVATE! Helper that emits one long isoSPI pulse pair per device, with each pulse being spaced by `gap_us`.
    /// 
    /// ### Parameters
    /// - `count`: Number of pulse pairs to emit.
    /// - `gap_us`: Gap between pulses in microseconds.
    async fn pulse_line(&mut self, count: usize, gap_us: u32) -> Result<(), Error<SPI::Error>> {
        use embedded_hal_async::spi::{Operation};
        for _ in 0..count {
            self.spi.transaction(&mut [
                Operation::DelayNs(gap_us * 1000),
            ]).await.map_err(Error::Spi)?;
        }

        Ok(())
    }

    /// PRIVATE! This is a helper that wakes up chips from the IDLE state.
    /// 
    /// This only works for the IDLE state. It won't work for the SLEEP state, since the IDLE wakeup time is shorter. As such, this
    /// is a private helper since it could be accidentally misused.
    async fn wakeup_from_idle(&mut self, count: usize) -> Result<(), Error<SPI::Error>> {
        /// Pulse gap to wake chips up from IDLE.
        /// 
        /// This needs to be larger than t_READY (~10us) but smaller than t_IDLE (~4.3ms). See Table 9 on page 8 of the datasheet.
        const IDLE_PULSE_GAP_US: u32 = 30;

        self.pulse_line(count, IDLE_PULSE_GAP_US).await?;

        Ok(())
    }
    
    /// PRIVATE! Makes sure the chips are awake.
    /// 
    /// This will typically issue a `wakeup_from_idle()`, but may issue a full `wakeup()` if the `Line` hasn't been
    /// used in a very long time. For good measure, this always issues `wakeup_from_idle()` no matter what, even if the
    /// `Line` has been used recently enough to where a wakeup technically shouldn't be needed.
    async fn ensure_woke(&mut self, count: usize) -> Result<(), Error<SPI::Error>> {
        use embassy_time::Duration;
        
        /// If the `Line` was last used more than `IDLE_ELAPSED_TIME` ago, then it is likely that the chips have entered the IDLE state and need to be woken up.
        /// The datasheet (Table 9) lists tIDLE as 4.3ms but this is a bit lower than that for good measure.
        const IDLE_ELAPSED_TIME: Duration = Duration::from_micros(3500);

        /// If the `Line` was last used more than `SLEEP_ELAPSED_TIME` ago, then it is likely that the chips have entered the SLEEP state and need to be woken up.
        /// The datasheet (Table 7) lists tSLEEP as 1.8s but this is a bit lower than that for good measure.
        const SLEEP_ELAPSED_TIME: Duration = Duration::from_millis(1500);
        
        let elapsed = self.last_activity.elapsed();
        
        // elapsed time since last use is less than IDLE_MARGIN, so chips should not have fallen into the IDLE state yet
        if elapsed < IDLE_ELAPSED_TIME {
            // at least for now, still do `wakeup_from_idle()` even though we technicalyl shouldn't need to, just for good measure.
            // u_TODO - marking this since it is something we could probably easily remove if we want to in the future (i.e., we probably don't actually need to wakeup() in this case). it is just here for good measure
            self.wakeup_from_idle(count).await?;
        }
        // elapsed time since last use is larger than IDLE_ELAPSED_TIME (so chips should have fallen IDLE), but is less than SLEEP_ELAPSED_TIME (so chips have not fallen fully asleep yet) 
        else if (elapsed >= IDLE_ELAPSED_TIME) && (elapsed < SLEEP_ELAPSED_TIME) {
            self.wakeup_from_idle(count).await?;
        }
        // elapsed time is greater than SLEEP_ELAPSED_TIME so chips have fallen fully asleep
        else {
            self.wakeup(count).await?;
        }

        Ok(())
    }

    /// Wakes `count` devices out of the idle or sleep state.
    ///
    /// This sends one pulse pair per device, since each device must wake up before it propagates the pulse to the
    /// next one. See the "Waking Up the Serial Interface" section on page 51 of the datasheet.
    pub async fn wakeup(&mut self, count: usize) -> Result<(), Error<SPI::Error>> {

        // t_WAKE is 500 us max (from sleep), so the gap has to be at least that long for a device to
        // power up, and under t_IDLE (4.3 ms min) or devices that already woke drop back to idle
        // before the chain finishes.
        const FULL_WAKEUP_PULSE_GAP_US: u32 = 500;

        self.pulse_line(count, FULL_WAKEUP_PULSE_GAP_US).await?;

        Ok(())
    }

    /// Counts the devices reachable on this line. This stops at the first bad PEC.
    pub async fn detect_chips(&mut self) -> Result<usize, Error<SPI::Error>> {
        let mut blocks = [[0u8; BLOCK_BYTES]; N];

        // this uses `N` for count because there could be a max of `N` chips on the line, we just don't know the exact/current runtime amount (which is what this function is supposed to determine)
        self.transact(
            N,
            commands::misc::rdsid().frame(),
            Some(Operation::Read(blocks.as_flattened_mut())),
        )
        .await?;

        Ok(blocks
            .iter()
            .take_while(|block| {
                DataPecRx::from_bytes([block[GROUP_BYTES], block[GROUP_BYTES + 1]])
                    .verify(&block[..GROUP_BYTES])
            })
            .count())
    }
}

/// Conversion times from the datasheet (milliseconds).
pub mod conversion_times {
    /// C-ADC single shot conversion.
    /// 
    /// The C-ADC conversion time is cited as 1ms in the "CONTINUOUS OR SINGLE SHOT MEASUREMENTS" section on page 20 of the datasheet.
    pub const C_ADC_MS: u64 = 1;
    /// S-ADC conversion (and a redundant ADCV for RD = 1).
    /// 
    /// This is tSADC from page 18 on the datasheet.
    pub const S_ADC_MS: u64 = 8;
    /// AUX ADC conversion (ADAX).
    /// 
    /// This is tAUX from page 18 of the datasheet.
    pub const AUX_MS: u64 = 1;
    /// AUX2 ADC conversion (ADAX2).
    /// 
    /// This is tAUX2 on page 18 of the datasheet.
    pub const AUX2_MS: u64 = 8;
    /// Added to any of the above when starting from the standby state (max).
    /// 
    /// This is tREFUP from Table 7 on page 8 of the datasheet.
    pub const REFUP_MS: u64 = 5;
}
