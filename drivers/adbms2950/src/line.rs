//! SPI transport for one isoSPI line to an ADBMS2950B.
//!
//! The chip has two isoSPI ports onto the same die, so "line" is one of those two connections --
//! [`crate::api::Api`] holds both and fails over between them.
//!
//! Frames a command, appends and verifies the data PEC, extracts the command counter, and holds
//! the bus idle for t5. Anything deciding *what* to send is policy and lives in [`crate::api`].
//!
//! Private, because `Api` owns its lines. Its public types are re-exported from [`crate::api`].

use embedded_hal_async::spi::{Operation, SpiDevice};

use crate::chip::commands::CommandFrame;
use crate::chip::pec::{DataPecRx, DataPecTx};
use crate::chip::registers::{GROUP_BYTES, ReadableGroup, WritableGroup};
use crate::spi_timing::BusIdle;

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
/// Chip select belongs to the `SpiDevice`, so t6 and t7 do too; t5 is the one this can enforce,
/// and `DELAY` is what it enforces it with. See [`crate::spi_timing`].
///
/// **The `SpiDevice` must support [`Operation::DelayNs`]**, because the wake-up pulse is a
/// delay-only transaction. `ExclusiveDevice::new_no_delay(...)` panics on one.
pub struct Line<SPI, DELAY> {
    spi: SPI,
    delay: DELAY,
}

impl<SPI: SpiDevice, DELAY: BusIdle> Line<SPI, DELAY> {
    /// Wraps an [`SpiDevice`] and the delay used to honour t5.
    pub const fn new(spi: SPI, delay: DELAY) -> Self {
        Self { spi, delay }
    }

    /// Returns the underlying device and delay.
    pub fn release(self) -> (SPI, DELAY) {
        (self.spi, self.delay)
    }

    /// Runs one transaction, then holds the bus idle for t5.
    ///
    /// Every bus access routes through here, so t5 covers the wake pulse and back-to-back reads
    /// alike. Unconditional rather than measured, and on the error path too, since chip select
    /// was released either way.
    async fn transact(
        &mut self,
        operations: &mut [Operation<'_, u8>],
    ) -> Result<(), Error<SPI::Error>> {
        let result = self.spi.transaction(operations).await.map_err(Error::Spi);
        self.delay.idle();
        result
    }

    /// Sends a command with no data payload.
    pub async fn command(&mut self, frame: CommandFrame) -> Result<(), Error<SPI::Error>> {
        let bytes = frame.to_bytes();
        self.pulse().await?;
        self.transact(&mut [Operation::Write(&bytes)]).await
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
        self.transact(&mut [Operation::Write(&command), Operation::Read(&mut block)])
            .await?;

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
        self.transact(&mut [Operation::Write(&command), Operation::Write(&block)])
            .await
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
        self.transact(&mut [Operation::Write(&command), Operation::Read(&mut status)])
            .await?;

        Ok(status[1] == 0xFF)
    }

    /// Sends a wake-up pulse and waits out the regulator startup time.
    ///
    /// Only needed after power-up or an `SRST`. This chip has no isoSPI
    /// idle timeout and no sleep state, so there is no reason to call this before an ordinary
    /// transaction.
    ///
    /// The pulse is a delay-only SPI transaction, which asserts chip select, waits, and
    /// deasserts. See the note on [`Line`] about needing a device that implements delays.
    pub async fn wakeup(&mut self) -> Result<(), Error<SPI::Error>> {
        self.pulse().await
    }

    /// Generates one chip-select pulse to wake the line.
    ///
    /// Called before every bus access, matching what the vendor reference code does. Costs about
    /// 4 us for a single device.
    async fn pulse(&mut self) -> Result<(), Error<SPI::Error>> {
        self.transact(&mut [Operation::DelayNs(conversion_times::WAKE_PULSE_US * 1_000)])
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
