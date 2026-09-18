//! Transport for a single ADBMS2950B.
//!
//! This is the layer that actually talks to the chip: it frames commands, appends and verifies
//! the data PEC, extracts the command counter, and waits out the datasheet's conversion times.
//! Everything it sends and decodes is built from [`crate::chip`].
//!
//! ### Why this is much simpler than the ADBMS6830B's equivalent
//!
//! The ADBMS6830B driver's `line` module carries a lot of machinery this one does not need:
//!
//! - **No sleep detection.** The ADBMS6830B's core sleeps after roughly 1.5 s of quiet, so its
//!   driver has to notice that and recover. The ADBMS2950B has no sleep state at all: the word
//!   does not appear anywhere in its datasheet, and unlike the ADBMS6830B its status registers
//!   carry no `SLEEP` bit for a host to poll. So there is no `last_activity` tracking, no sleep
//!   threshold, and no startup-from-sleep path.
//!
//!   Note this driver still sends a wake-up pulse before **every** transaction, which is what the
//!   vendor's reference code does. The pulse is cheap -- a chip-select toggle with a 2 us delay
//!   either side, so about 4 us for one device -- and it is emphatically *not* the 500 us
//!   `tWAKE` regulator startup, which is only owed after power-up or an `SRST`
//!   (see [`Line::wait_after_reset`]). Paying 4 us unconditionally is a much better trade than
//!   reasoning about whether the isoSPI port can time out while idle, which the datasheet does
//!   not actually say.
//! - **No daisy chain.** This driver targets one device measuring the tractive system as a whole,
//!   so there is no device count, no per-device response array, and no reversing the write
//!   payload to account for the first block landing in the furthest device. Adding chain support
//!   later means making `read`/`write` loop over blocks; nothing else here would change.
//! - **PEC failure is an error, not per-device data.** With one device a bad PEC means the read
//!   is simply invalid, so it comes back as [`Error::Pec`] rather than a status flag you have to
//!   remember to check.

use embedded_hal_async::spi::{Operation, SpiDevice};

use crate::chip::commands;
use crate::chip::commands::{
    CommandFrame,
    adc::{Acquisition, Diagnostic, OpenWire, OpenWireVoltage, Redundancy, VoltageChannel},
};
use crate::chip::pec::{DataPecRx, DataPecTx};
use crate::chip::registers::{GROUP_BYTES, ReadableGroup, WritableGroup};

/// Wire length of one register group plus its data PEC.
const BLOCK_BYTES: usize = GROUP_BYTES + 2;

/// Largest command counter value before it rolls over.
///
/// The counter is six bits and rolls over past its maximum to 1, not 0, because 0 is reserved for
/// the reset that `RSTCC` and `SRST` perform. See the "Command Counter" section on page 24 of the
/// datasheet.
pub const COMMAND_COUNTER_MAX: u8 = 63;

/// Something went wrong talking to the chip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    /// The underlying SPI device failed.
    Spi(E),
    /// The data PEC on a read did not match, so the returned bytes cannot be trusted.
    Pec,
    /// A conversion did not report completion within the allotted time.
    Timeout,
    /// The device did not identify itself as an ADBMS2950B.
    ///
    /// Carries the `DEVID` that was read instead. See
    /// [`crate::chip::registers::serial_id::ADBMS2950B_DEVICE_ID`].
    WrongDevice(u8),
}

impl<E> Error<E> {
    /// Erases the SPI error type, so diagnostics structs can stay `Copy + Eq` without carrying
    /// an `E` parameter around.
    pub fn to_kind(self) -> Error<embedded_hal_async::spi::ErrorKind>
    where
        E: embedded_hal_async::spi::Error,
    {
        match self {
            Self::Spi(err) => Error::Spi(err.kind()),
            Self::Pec => Error::Pec,
            Self::Timeout => Error::Timeout,
            Self::WrongDevice(id) => Error::WrongDevice(id),
        }
    }
}

/// A successfully read register group, plus the command counter that came with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Response<G> {
    /// The decoded register group.
    pub data: G,
    /// The chip's command counter (`CCNT[5:0]`), piggybacked on the first data PEC byte.
    ///
    /// Compare this against your own expected count to detect a missed or corrupted command. It
    /// increments on every command except reads, `RSTCC`, and `SRST`; the latter two reset it
    /// to 0.
    pub command_counter: u8,
}

/// One ADBMS2950B on a SPI or isoSPI link.
///
/// `SPI` is an [`SpiDevice`], not a bus, so chip-select handling and inter-operation delays come
/// from the device implementation rather than being managed here.
///
/// **The `SpiDevice` must support [`Operation::DelayNs`]**, because the wake-up pulse before every transaction generates it
/// pulse by running a delay-only transaction. With `embedded-hal-bus` that means
/// `ExclusiveDevice::new(...)` with a real delay, not `ExclusiveDevice::new_no_delay(...)`, which
/// panics on a delay operation.
pub struct Line<SPI> {
    spi: SPI,
}

impl<SPI: SpiDevice> Line<SPI> {
    /// Wraps an [`SpiDevice`].
    pub const fn new(spi: SPI) -> Self {
        Self { spi }
    }

    /// Returns the underlying device.
    pub fn release(self) -> SPI {
        self.spi
    }

    /// Sends a command with no data payload.
    pub async fn command(&mut self, frame: CommandFrame) -> Result<(), Error<SPI::Error>> {
        let bytes = frame.to_bytes();
        self.pulse().await?;
        self.spi
            .transaction(&mut [Operation::Write(&bytes)])
            .await
            .map_err(Error::Spi)
    }

    /// Reads a register group.
    ///
    /// Sends the group's read command, then clocks in its six data bytes followed by the two PEC
    /// bytes. Returns [`Error::Pec`] if the PEC does not verify, in which case the data is
    /// discarded rather than handed back.
    pub async fn read<G: ReadableGroup>(&mut self) -> Result<Response<G>, Error<SPI::Error>> {
        let command = G::READ_COMMAND.to_bytes();
        let mut block = [0u8; BLOCK_BYTES];

        self.pulse().await?;
        self.spi
            .transaction(&mut [Operation::Write(&command), Operation::Read(&mut block)])
            .await
            .map_err(Error::Spi)?;

        let data: [u8; GROUP_BYTES] = {
            let mut out = [0u8; GROUP_BYTES];
            out.copy_from_slice(&block[..GROUP_BYTES]);
            out
        };
        let pec = DataPecRx::from_bytes([block[GROUP_BYTES], block[GROUP_BYTES + 1]]);

        if !pec.verify(&data) {
            return Err(Error::Pec);
        }

        Ok(Response {
            data: G::from_bytes(data),
            command_counter: pec.ccnt(),
        })
    }

    /// Writes a register group.
    ///
    /// Sends the group's write command, then its six data bytes followed by the two PEC bytes the
    /// chip will check. A write whose PEC does not match is ignored by the chip and does not
    /// increment its command counter, so a silent no-op here shows up as a counter mismatch on
    /// the next read.
    pub async fn write<G: WritableGroup>(&mut self, group: G) -> Result<(), Error<SPI::Error>> {
        let command = G::WRITE_COMMAND.to_bytes();
        let data = group.to_bytes();
        let pec = DataPecTx::new(&data);

        let mut block = [0u8; BLOCK_BYTES];
        block[..GROUP_BYTES].copy_from_slice(&data);
        block[GROUP_BYTES] = pec.pec0();
        block[GROUP_BYTES + 1] = pec.pec1();

        self.pulse().await?;
        self.spi
            .transaction(&mut [Operation::Write(&command), Operation::Write(&block)])
            .await
            .map_err(Error::Spi)
    }

    /// Sends a poll command once and reports whether the conversion has finished.
    ///
    /// The chip holds SDO low while a conversion is in progress and releases it high when done,
    /// so "finished" means the clocked-in byte reads back as all ones. Two bytes are clocked
    /// rather than one because the first is inside the window the datasheet marks invalid; only
    /// the last is inspected.
    ///
    /// Use one of the `poll` commands from [`crate::chip::commands::poll`].
    pub async fn poll(&mut self, frame: CommandFrame) -> Result<bool, Error<SPI::Error>> {
        let command = frame.to_bytes();
        let mut status = [0u8; 2];

        self.pulse().await?;
        self.spi
            .transaction(&mut [Operation::Write(&command), Operation::Read(&mut status)])
            .await
            .map_err(Error::Spi)?;

        Ok(status[1] == 0xFF)
    }

    /// Polls until a conversion finishes, or gives up.
    ///
    /// Waits `settle` first, since polling before the conversion could possibly be done is just
    /// wasted bus traffic, then polls every millisecond until `timeout` elapses. The sensible
    /// `settle` values are in [`conversion_times`].
    pub async fn poll_until(
        &mut self,
        frame: CommandFrame,
        settle: embassy_time::Duration,
        timeout: embassy_time::Duration,
    ) -> Result<(), Error<SPI::Error>> {
        embassy_time::Timer::after(settle).await;

        let deadline = embassy_time::Instant::now() + timeout;
        loop {
            if self.poll(frame).await? {
                return Ok(());
            }
            if embassy_time::Instant::now() >= deadline {
                #[cfg(feature = "defmt")]
                defmt::warn!("ADBMS2950: Line: poll_until: conversion did not complete in time");
                return Err(Error::Timeout);
            }
            embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
        }
    }

    /// Sends a wake-up pulse and waits out the regulator startup time.
    ///
    /// Only needed after power-up or an `SRST`. Unlike the ADBMS6830B, this chip has no isoSPI
    /// idle timeout and no sleep state, so there is no reason to call this before an ordinary
    /// transaction.
    ///
    /// The pulse is a delay-only SPI transaction, which asserts chip select, waits, and
    /// deasserts. See the note on [`Line`] about needing a device that implements delays.
    pub async fn wakeup(&mut self) -> Result<(), Error<SPI::Error>> {
        self.pulse().await
    }

    /// Waits out the regulator startup time, `tWAKE`.
    ///
    /// Owed after power-up or an `SRST`, and **not** something an ordinary transaction needs --
    /// this is a 500 us wait, versus the ~4 us [`Line::wakeup`] pulse. After this the chip is in
    /// STANDBY and accepting commands; it then reaches REFUP on its own, which
    /// [`Line::wait_for_reference`] detects.
    pub async fn wait_after_reset(&mut self) {
        embassy_time::Timer::after(embassy_time::Duration::from_micros(
            conversion_times::WAKE_MAX_US as u64,
        ))
        .await;
    }

    /// Generates one chip-select pulse to wake the line.
    ///
    /// Called before every bus access, matching what the vendor reference code does. Costs about
    /// 4 us for a single device.
    async fn pulse(&mut self) -> Result<(), Error<SPI::Error>> {
        self.spi
            .transaction(&mut [Operation::DelayNs(conversion_times::WAKE_PULSE_US * 1_000)])
            .await
            .map_err(Error::Spi)
    }

    /// Waits for the voltage references to come up, by polling the `REFUP` bit in CFGA.
    ///
    /// After power-up or an `SRST` the chip passes through STANDBY into REFUP on its own; this is
    /// how you find out it has arrived. Measurements taken before this are not trustworthy.
    pub async fn wait_for_reference(&mut self) -> Result<(), Error<SPI::Error>> {
        use crate::chip::registers::config_a::{ConfigA, types::ReferencePowered};

        let deadline = embassy_time::Instant::now()
            + embassy_time::Duration::from_millis(conversion_times::REFUP_MAX_MS as u64 * 2);

        loop {
            if let Ok(response) = self.read::<ConfigA>().await
                && response.data.refup() == ReferencePowered::Powered
            {
                return Ok(());
            }
            if embassy_time::Instant::now() >= deadline {
                #[cfg(feature = "defmt")]
                defmt::warn!("ADBMS2950: Line: wait_for_reference: REFUP never asserted");
                return Err(Error::Timeout);
            }
            embassy_time::Timer::after(embassy_time::Duration::from_micros(500)).await;
        }
    }

    /// Confirms the attached device is an ADBMS2950B, returning its serial ID.
    ///
    /// Reads `RDSID` and checks the `DEVID` bits. Useful both as a presence check and to catch a
    /// miswired bus where an ADBMS6830B answered instead -- the two chips share this command's
    /// opcode but report different device IDs.
    pub async fn detect(
        &mut self,
    ) -> Result<crate::chip::registers::serial_id::SerialId, Error<SPI::Error>> {
        use crate::chip::registers::serial_id::SerialId;

        let response = self.read::<SerialId>().await?;
        if !response.data.is_adbms2950b() {
            return Err(Error::WrongDevice(response.data.device_id()));
        }
        Ok(response.data)
    }

    /// Starts an I1ADC conversion and waits for it to finish.
    ///
    /// Always a single-shot conversion: a continuous one never reports completion, so there
    /// would be nothing to poll. To run continuously, send
    /// [`crate::chip::commands::adc::adi1`] yourself with [`Acquisition::Continuous`] and track
    /// progress through the FLAG register's `i1pha`/`i1cnt` counters instead.
    ///
    /// The settle wait is `tIxADC_STARTUP`, which is what the *first* conversion after power-up
    /// or `SRST` costs. Subsequent conversions finish in about
    /// [`conversion_times::IXADC_CONVERSION_MS`], so polling simply returns on the first try.
    pub async fn adi1_autoconvert(
        &mut self,
        rd: Redundancy,
        diag: Diagnostic,
        ow: OpenWire,
        timeout: embassy_time::Duration,
    ) -> Result<(), Error<SPI::Error>> {
        let start = commands::adc::adi1(rd, Acquisition::SingleShot, diag, ow).frame();
        self.command(start).await?;
        self.poll_until(
            commands::poll::pli1().frame(),
            embassy_time::Duration::from_millis(conversion_times::IXADC_STARTUP_MAX_MS as u64),
            timeout,
        )
        .await
    }

    /// Starts an I2ADC conversion and waits for it to finish.
    ///
    /// Single-shot only, for the same reason as [`Line::adi1_autoconvert`]. The second current
    /// channel is independent of the first, so this has no redundancy parameter.
    pub async fn adi2_autoconvert(
        &mut self,
        diag: Diagnostic,
        ow: OpenWire,
        timeout: embassy_time::Duration,
    ) -> Result<(), Error<SPI::Error>> {
        let start = commands::adc::adi2(Acquisition::SingleShot, diag, ow).frame();
        self.command(start).await?;
        self.poll_until(
            commands::poll::pli2().frame(),
            embassy_time::Duration::from_millis(conversion_times::IXADC_STARTUP_MAX_MS as u64),
            timeout,
        )
        .await
    }

    /// Starts a V1ADC/V2ADC conversion and waits for it to finish.
    ///
    /// The settle wait scales with how many channels `vch` sweeps, since a round robin converts
    /// them one after another. **Any `SOAK` time configured in CFGA is added by the chip on top
    /// of this** and is not accounted for here -- if you enable soak, widen `timeout` to match.
    pub async fn adv_autoconvert(
        &mut self,
        ow: OpenWireVoltage,
        vch: VoltageChannel,
        timeout: embassy_time::Duration,
    ) -> Result<(), Error<SPI::Error>> {
        let start = commands::adc::adv(ow, vch).frame();
        let settle_us =
            conversion_times::VADC_CONVERSION_MAX_US as u64 * vch.channel_count() as u64;
        self.command(start).await?;
        self.poll_until(
            commands::poll::plv().frame(),
            embassy_time::Duration::from_micros(settle_us),
            timeout,
        )
        .await
    }

    /// Starts an AUX ADC conversion and waits for it to finish.
    ///
    /// The AUX ADC sweeps its whole set of internal rails and both temperature sensors, so
    /// unlike [`Line::adv_autoconvert`] there is nothing to select.
    pub async fn adx_autoconvert(
        &mut self,
        timeout: embassy_time::Duration,
    ) -> Result<(), Error<SPI::Error>> {
        self.command(commands::adc::adx().frame()).await?;
        self.poll_until(
            commands::poll::plx().frame(),
            embassy_time::Duration::from_micros(conversion_times::VADC_CONVERSION_MAX_US as u64),
            timeout,
        )
        .await
    }
}

/// Datasheet timings, for deciding how long to wait before reading a result.
///
/// All values are the datasheet's **maximum** where one is specified, so that waiting this long
/// is always sufficient. See Table 8 on page 9 of the datasheet for the startup timings and the
/// per-ADC specifications for the conversion times.
pub mod conversion_times {
    /// Width of the chip-select pulse used to wake the line, in microseconds.
    ///
    /// Not a datasheet parameter. This matches the vendor reference code's single-device setting,
    /// which pulses chip select with a 2 us delay either side. The vendor's alternative
    /// millisecond-scale path exists for long daisy chains, which this driver does not target.
    pub const WAKE_PULSE_US: u32 = 2;

    /// Regulator startup, `tWAKE`. Typical 200 us, maximum 500 us.
    ///
    /// How long after VDD crosses the `VDRUV` threshold before VREG is up and the chip reaches
    /// STANDBY. Also the wait after an `SRST`.
    pub const WAKE_MAX_US: u32 = 500;

    /// Reference startup, `tREFUP`. Typical 3.5 ms, maximum 4.5 ms; rounded up.
    ///
    /// Time to transition into the REFUP state after power-up or `SRST`.
    pub const REFUP_MAX_MS: u32 = 5;

    /// First IxADC result availability, `tIxADC_STARTUP`. Typical 9 ms, maximum 9.9 ms; rounded up.
    ///
    /// Applies to the first `ADI1`/`ADI2` in the REFUP state after power-up or `SRST`.
    pub const IXADC_STARTUP_MAX_MS: u32 = 10;

    /// IxADC initialization, `tIxADC_INIT`. Typical 127 ms, maximum 140 ms.
    ///
    /// Runs after `IXADC_STARTUP_MAX_MS` expires. The `i1cal` and `i2cal` bits in STAT report
    /// when it has finished.
    pub const IXADC_INIT_MAX_MS: u32 = 140;

    /// IxADC and VBxADC conversion time, in milliseconds.
    ///
    /// The accumulator registers update every `ACCN` of these instead, so their update period is
    /// this multiplied by `ACCN`.
    pub const IXADC_CONVERSION_MS: u32 = 1;

    /// V1ADC/V2ADC single conversion with no soak time. Typical 265 us, maximum 292 us.
    ///
    /// A round-robin `ADV` takes this per channel, and any `SOAK` time configured in CFGA is
    /// added on top.
    pub const VADC_CONVERSION_MAX_US: u32 = 292;

    /// OCxADC conversion time, `tOCxADC`. Typical 62.3 us, maximum 69 us.
    ///
    /// This is the `OCxR` register update rate.
    pub const OCADC_CONVERSION_MAX_US: u32 = 69;
}
