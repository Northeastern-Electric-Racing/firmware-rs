//! Higher-level API over one or two [`Line`]s.
//!
//! [`Line`] is deliberately dumb: it frames a transaction and hands back bytes. Everything that
//! needs *memory* between transactions lives here -- the command counter, PEC tallies, the
//! cached configuration, and per-line error counts. Without this layer each of those ends up
//! reimplemented in the application, which is where they were before this module existed.

use embassy_time::{Duration, Instant};
use embedded_hal_async::spi::SpiDevice;

use crate::chip::commands::{self, Command};
use crate::chip::registers::config_a::ConfigA;
use crate::chip::registers::config_b::ConfigB;
use crate::chip::registers::{ReadableGroup, WritableGroup};
use crate::line::{COMMAND_COUNTER_MAX, Error, Line, conversion_times};

/// Which isoSPI line a transaction went out on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LineId {
    /// Port A.
    A,
    /// Port B.
    B,
}

/// Which overcurrent comparator channel a result code came from.
///
/// Needed because each channel has its own gain bit in `ConfigB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum OverCurrentChannel {
    /// OC1ADC.
    Oc1,
    /// OC2ADC.
    Oc2,
    /// OC3ADC.
    Oc3,
}

/// What we know about the device's health, accumulated across transactions.
///
/// This is the natural home for anything that only means something *between* reads. A single
/// register reading carries no metadata of its own -- if a read's PEC fails the driver returns
/// an error and there is no reading at all -- so the counters live here instead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct DeviceState {
    expected_command_counter: u8,
    reported_command_counter: Option<u8>,
    pec_success_count: u32,
    pec_failed_count: u32,
    last_contacted: Option<Instant>,
}

impl DeviceState {
    /// State for a device we have not talked to yet.
    pub const fn new() -> Self {
        Self {
            expected_command_counter: 0,
            reported_command_counter: None,
            pec_success_count: 0,
            pec_failed_count: 0,
            last_contacted: None,
        }
    }

    /// The counter value we believe the device should report next.
    pub const fn expected_command_counter(&self) -> u8 {
        self.expected_command_counter
    }

    /// The counter the device actually reported on the most recent successful read.
    pub const fn reported_command_counter(&self) -> Option<u8> {
        self.reported_command_counter
    }

    /// Reads whose data PEC verified.
    pub const fn pec_success_count(&self) -> u32 {
        self.pec_success_count
    }

    /// Reads whose data PEC did not verify.
    ///
    /// A steadily climbing count here is the leading indicator of a marginal isoSPI link, and is
    /// what a future failover policy would key off.
    pub const fn pec_failed_count(&self) -> u32 {
        self.pec_failed_count
    }

    /// When any transaction last succeeded.
    pub const fn last_contacted(&self) -> Option<Instant> {
        self.last_contacted
    }

    /// Whether the device's reported counter matches what we expected.
    ///
    /// `None` until the first successful read.
    pub const fn command_counter_matches(&self) -> Option<bool> {
        match self.reported_command_counter {
            Some(reported) => Some(reported == self.expected_command_counter),
            None => None,
        }
    }

    /// Whether the device looks like it reset itself without us asking.
    ///
    /// The counter is only ever cleared to 0 by `RSTCC` or `SRST`, and otherwise counts up and
    /// wraps to 1. So a device reporting 0 when we expect something else has rebooted -- and
    /// crucially, has lost the configuration we wrote.
    pub const fn suspected_reset(&self) -> bool {
        match self.reported_command_counter {
            Some(reported) => reported == 0 && self.expected_command_counter != 0,
            None => false,
        }
    }

    /// Advances the expected counter the way the device would.
    ///
    /// Wraps to 1 rather than 0, because 0 is reserved to mean "was just reset".
    const fn advance(&mut self) {
        self.expected_command_counter = if self.expected_command_counter >= COMMAND_COUNTER_MAX {
            1
        } else {
            self.expected_command_counter + 1
        };
    }
}

impl Default for DeviceState {
    fn default() -> Self {
        Self::new()
    }
}

/// Stateful API over one ADBMS2950B.
pub struct Api<SPI> {
    line_a: Line<SPI>,
    line_b: Line<SPI>,
    active: LineId,
    device: DeviceState,
    config_a: ConfigA,
    config_b: ConfigB,
    line_a_error_count: u32,
    line_b_error_count: u32,
}

impl<SPI: SpiDevice> Api<SPI> {
    /// Wraps both isoSPI lines, with line A active.
    ///
    /// The cached configuration starts at `ConfigA::new()` (the datasheet reset values); it
    /// becomes accurate as soon as you call [`Api::set_configa`].
    pub const fn new(line_a: Line<SPI>, line_b: Line<SPI>) -> Self {
        Self {
            line_a,
            line_b,
            active: LineId::A,
            device: DeviceState::new(),
            config_a: ConfigA::new(),
            config_b: ConfigB::new(),
            line_a_error_count: 0,
            line_b_error_count: 0,
        }
    }

    /// Device health: command counter, PEC tallies, last contact.
    pub const fn device(&self) -> &DeviceState {
        &self.device
    }

    /// Which line transactions currently go out on.
    pub const fn active_line(&self) -> LineId {
        self.active
    }

    /// Switches which line transactions go out on.
    ///
    /// Two things this intentionally does **not** do:
    ///
    /// - It does not reset the PEC tallies in [`Api::device`]. Those are lifetime totals, and a
    ///   windowed failure detector wants to diff them over time rather than have them cleared
    ///   out from under it.
    /// - It does not invalidate the cached `ConfigA`. Both ports lead to the same die, so the
    ///   configuration the chip is holding does not change just because we started talking to it
    ///   through the other one.
    pub fn set_active_line(&mut self, line: LineId) {
        if self.active == line {
            return;
        }

        #[cfg(feature = "defmt")]
        defmt::warn!(
            "ADBMS2950: Api: set_active_line: switching from {} to {}",
            self.active,
            line
        );

        self.active = line;
    }

    /// How many transactions have failed on a given line.
    pub const fn line_error_count(&self, line: LineId) -> u32 {
        match line {
            LineId::A => self.line_a_error_count,
            LineId::B => self.line_b_error_count,
        }
    }

    /// The most recently written `ConfigA`.
    ///
    /// Kept so a single-bit change is a write, not a read followed by a write.
    pub const fn config_a(&self) -> ConfigA {
        self.config_a
    }

    /// The most recently written `ConfigB`.
    ///
    /// Worth caching for the same reason as `ConfigA`, plus two specific to this register:
    ///
    /// - **The four GPIOs live here**, not in `ConfigA`. `GPO1`-`GPO6` are `ConfigA`;
    ///   `GPIO1C`-`GPIO4C` are `ConfigB`. Driving a GPIO needs this cache.
    /// - **Overcurrent results cannot be scaled without it.** `OCxR` is a raw code whose LSB is
    ///   5 mV or 2.5 mV depending on the matching `OCxGC` gain bit, which lives here. See
    ///   [`Api::overcurrent_microvolts`].
    pub const fn config_b(&self) -> ConfigB {
        self.config_b
    }

    /// The active line, for the rare case you need the transport directly.
    fn line(&mut self) -> &mut Line<SPI> {
        match self.active {
            LineId::A => &mut self.line_a,
            LineId::B => &mut self.line_b,
        }
    }

    /// Records a failed transaction against the active line.
    fn note_error(&mut self) {
        match self.active {
            LineId::A => self.line_a_error_count = self.line_a_error_count.saturating_add(1),
            LineId::B => self.line_b_error_count = self.line_b_error_count.saturating_add(1),
        }
    }

    /// Records a successful transaction.
    fn note_success(&mut self) {
        self.device.last_contacted = Some(Instant::now());
    }

    /// Reads a register group.
    ///
    /// Unlike [`Line::read`] this returns the group alone: the command counter that came with it
    /// is folded into [`Api::device`] instead, alongside the PEC tallies, so callers do not have
    /// to remember to thread per-reading metadata around.
    ///
    /// Reads do not increment the device's command counter, so this only ever *checks* it.
    pub async fn read<G: ReadableGroup>(&mut self) -> Result<G, Error<SPI::Error>> {
        match self.line().read::<G>().await {
            Ok(response) => {
                self.device.pec_success_count = self.device.pec_success_count.saturating_add(1);
                self.device.reported_command_counter = Some(response.command_counter);
                self.note_success();

                if self.device.suspected_reset() {
                    #[cfg(feature = "defmt")]
                    defmt::warn!(
                        "ADBMS2950: Api: read: device reported command counter 0 while we expected {}; it has reset and lost its configuration",
                        self.device.expected_command_counter
                    );
                }

                Ok(response.data)
            }
            Err(err) => {
                if matches!(err, Error::Pec) {
                    self.device.pec_failed_count = self.device.pec_failed_count.saturating_add(1);
                }
                self.note_error();
                Err(err)
            }
        }
    }

    /// Writes a register group, keeping the expected command counter in step.
    pub async fn write<G: WritableGroup>(&mut self, group: G) -> Result<(), Error<SPI::Error>> {
        let increments = G::WRITE_COMMAND.increments();
        match self.line().write(group).await {
            Ok(()) => {
                if increments {
                    self.device.advance();
                }
                self.note_success();
                Ok(())
            }
            Err(err) => {
                self.note_error();
                Err(err)
            }
        }
    }

    /// Sends a command, keeping the expected command counter in step.
    ///
    /// Takes a [`Command`] rather than a frame so it can see whether the command increments the
    /// counter or resets it -- `RSTCC` and `SRST` both set it to 0.
    pub async fn command(&mut self, command: Command) -> Result<(), Error<SPI::Error>> {
        let increments = command.increments();
        let resets = command.resets_counter();

        match self.line().command(command.frame()).await {
            Ok(()) => {
                if resets {
                    self.device.expected_command_counter = 0;
                } else if increments {
                    self.device.advance();
                }
                self.note_success();
                Ok(())
            }
            Err(err) => {
                self.note_error();
                Err(err)
            }
        }
    }

    /// Writes `ConfigA` and caches it.
    pub async fn set_configa(&mut self, config: ConfigA) -> Result<(), Error<SPI::Error>> {
        self.write(config).await?;
        self.config_a = config;
        Ok(())
    }

    /// Writes `ConfigB` and caches it.
    pub async fn set_configb(&mut self, config: ConfigB) -> Result<(), Error<SPI::Error>> {
        self.write(config).await?;
        self.config_b = config;
        Ok(())
    }

    /// Changes `ConfigA` without reading it back first.
    ///
    /// Applies `f` to the cached configuration and writes the result. This is how to drive a GPO
    /// or GPIO -- for example an external relay wired to GPO4:
    ///
    /// ```ignore
    /// api.modify_configa(|cfg| cfg.with_gpo4c(GpoOutputState::Driven)).await?;
    /// ```
    ///
    /// The driver deliberately has no notion of what any GPO is wired to; that belongs to the
    /// board. Note the cache is only as good as the device's compliance with the last write --
    /// if [`DeviceState::suspected_reset`] goes true, the device is back at its reset defaults
    /// and the cache is stale, so re-run your startup configuration.
    pub async fn modify_configa(
        &mut self,
        f: impl FnOnce(ConfigA) -> ConfigA,
    ) -> Result<(), Error<SPI::Error>> {
        self.set_configa(f(self.config_a)).await
    }

    /// Changes `ConfigB` without reading it back first. The `ConfigB` counterpart of
    /// [`Api::modify_configa`].
    pub async fn modify_configb(
        &mut self,
        f: impl FnOnce(ConfigB) -> ConfigB,
    ) -> Result<(), Error<SPI::Error>> {
        self.set_configb(f(self.config_b)).await
    }

    /// Converts an overcurrent result code to microvolts, using the cached channel gain.
    ///
    /// `OCxR` codes are meaningless without knowing whether that channel is running at gain 1
    /// (5 mV per code) or gain 2 (2.5 mV), and that bit lives in `ConfigB`. This reads the gain
    /// out of the cache so callers do not have to plumb it themselves.
    ///
    /// Only as good as the cache: if [`DeviceState::suspected_reset`] is true the chip is back
    /// at its reset defaults and this will scale with the wrong gain until the configuration is
    /// rewritten.
    pub const fn overcurrent_microvolts(
        &self,
        code: crate::chip::registers::results::types::OverCurrentCode,
        channel: OverCurrentChannel,
    ) -> i32 {
        let gain = match channel {
            OverCurrentChannel::Oc1 => self.config_b.oc1gc(),
            OverCurrentChannel::Oc2 => self.config_b.oc2gc(),
            OverCurrentChannel::Oc3 => self.config_b.oc3gc(),
        };
        code.as_microvolts(gain)
    }

    /// Software-resets the device and waits out the regulator startup.
    ///
    /// Leaves the device in STANDBY with its command counter at 0 and its registers at their
    /// reset defaults, which means the cached `ConfigA` is reset to match. Follow this with
    /// [`Api::wait_for_reference`] before trusting any measurement.
    pub async fn reset(&mut self) -> Result<(), Error<SPI::Error>> {
        self.command(commands::misc::srst()).await?;
        self.line().wait_after_reset().await;
        self.config_a = ConfigA::new();
        self.config_b = ConfigB::new();
        Ok(())
    }

    /// Resets just the command counter, on both the device and our expectation of it.
    pub async fn reset_command_counter(&mut self) -> Result<(), Error<SPI::Error>> {
        self.command(commands::misc::rstcc()).await
    }

    /// Sends a wake-up pulse on the active line.
    pub async fn wakeup(&mut self) -> Result<(), Error<SPI::Error>> {
        self.line().wakeup().await
    }

    /// Waits for the voltage references to come up.
    pub async fn wait_for_reference(&mut self) -> Result<(), Error<SPI::Error>> {
        self.line().wait_for_reference().await
    }

    /// Confirms the attached device is an ADBMS2950B, returning its serial ID.
    pub async fn detect(
        &mut self,
    ) -> Result<crate::chip::registers::serial_id::SerialId, Error<SPI::Error>> {
        self.line().detect().await
    }

    /// Sends a conversion command, then polls until it completes.
    ///
    /// Poll commands increment the device's command counter, and a poll loop runs an
    /// unpredictable number of times, so this drives the loop itself and counts each attempt.
    /// Delegating to [`Line::poll_until`] would silently desynchronize
    /// [`DeviceState::expected_command_counter`] and make every subsequent read look like a
    /// counter mismatch.
    async fn autoconvert(
        &mut self,
        start: Command,
        poll: Command,
        settle: Duration,
        timeout: Duration,
    ) -> Result<(), Error<SPI::Error>> {
        self.command(start).await?;

        embassy_time::Timer::after(settle).await;
        let deadline = Instant::now() + timeout;
        let poll_frame = poll.frame();

        loop {
            let done = match self.line().poll(poll_frame).await {
                Ok(done) => done,
                Err(err) => {
                    self.note_error();
                    return Err(err);
                }
            };
            if poll.increments() {
                self.device.advance();
            }
            self.note_success();

            if done {
                return Ok(());
            }
            if Instant::now() >= deadline {
                #[cfg(feature = "defmt")]
                defmt::warn!("ADBMS2950: Api: autoconvert: conversion did not complete in time");
                return Err(Error::Timeout);
            }
            embassy_time::Timer::after(Duration::from_millis(1)).await;
        }
    }

    /// Starts an I1ADC conversion and waits for it to finish.
    ///
    /// Single-shot: a continuous conversion never reports completion, so there is nothing to
    /// poll. To run continuously, send [`commands::adc::adi1`] through [`Api::command`] with
    /// [`commands::adc::Acquisition::Continuous`] and watch the FLAG register's `i1pha`/`i1cnt`.
    pub async fn adi1_autoconvert(
        &mut self,
        rd: commands::adc::Redundancy,
        diag: commands::adc::Diagnostic,
        ow: commands::adc::OpenWire,
        timeout: Duration,
    ) -> Result<(), Error<SPI::Error>> {
        self.autoconvert(
            commands::adc::adi1(rd, commands::adc::Acquisition::SingleShot, diag, ow),
            commands::poll::pli1(),
            Duration::from_millis(conversion_times::IXADC_STARTUP_MAX_MS as u64),
            timeout,
        )
        .await
    }

    /// Starts an I2ADC conversion and waits for it to finish. Single-shot, as above.
    pub async fn adi2_autoconvert(
        &mut self,
        diag: commands::adc::Diagnostic,
        ow: commands::adc::OpenWire,
        timeout: Duration,
    ) -> Result<(), Error<SPI::Error>> {
        self.autoconvert(
            commands::adc::adi2(commands::adc::Acquisition::SingleShot, diag, ow),
            commands::poll::pli2(),
            Duration::from_millis(conversion_times::IXADC_STARTUP_MAX_MS as u64),
            timeout,
        )
        .await
    }

    /// Starts a V1ADC/V2ADC conversion and waits for it to finish.
    ///
    /// The settle wait scales with how many channels `vch` sweeps. Any `SOAK` time configured in
    /// CFGA is added by the chip on top of that and is not accounted for here.
    pub async fn adv_autoconvert(
        &mut self,
        ow: commands::adc::OpenWireVoltage,
        vch: commands::adc::VoltageChannel,
        timeout: Duration,
    ) -> Result<(), Error<SPI::Error>> {
        let settle_us =
            conversion_times::VADC_CONVERSION_MAX_US as u64 * vch.channel_count() as u64;
        self.autoconvert(
            commands::adc::adv(ow, vch),
            commands::poll::plv(),
            Duration::from_micros(settle_us),
            timeout,
        )
        .await
    }

    /// Starts an AUX ADC conversion and waits for it to finish.
    pub async fn adx_autoconvert(&mut self, timeout: Duration) -> Result<(), Error<SPI::Error>> {
        self.autoconvert(
            commands::adc::adx(),
            commands::poll::plx(),
            Duration::from_micros(conversion_times::VADC_CONVERSION_MAX_US as u64),
            timeout,
        )
        .await
    }
}
