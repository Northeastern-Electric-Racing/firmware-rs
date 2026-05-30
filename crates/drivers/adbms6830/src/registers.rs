//! Generic per-register read/write on [`Adbms6830`].
//!
//! Adding a new register is: define a `#[bitfield(u64)]` struct in
//! [`crate::types`], add `From<u64>` / `Into<u64>` impls (4 trivial lines),
//! and tag it with [`AdbmsReadableRegister`] and/or [`AdbmsWritableRegister`].
//! The transport, striding, and byte packing live here once.

use embedded_hal::digital::OutputPin;
use embedded_hal_async::{delay::DelayNs, spi::SpiBus};

use crate::client::{Adbms6830, AdbmsError, RX_SIZE, TX_SIZE};

/// A register that can be written to every IC on the chain.
///
/// The `Into<u64>` bound is satisfied automatically for any
/// `#[bitfield(u64)]` struct — the `bitfields` macro generates the
/// `From<Self> for u64` impl. Implementors only have to set [`WRITE_CMD`].
///
/// [`WRITE_CMD`]: AdbmsWritableRegister::WRITE_CMD
pub trait AdbmsWritableRegister: Copy + Into<u64> {
    const WRITE_CMD: [u8; 2];
}

/// A register that can be read from every IC on the chain.
///
/// The `From<u64>` bound is satisfied automatically for any
/// `#[bitfield(u64)]` struct. Implementors only have to set [`READ_CMD`].
///
/// [`READ_CMD`]: AdbmsReadableRegister::READ_CMD
pub trait AdbmsReadableRegister: From<u64> {
    const READ_CMD: [u8; 2];
}

/// A command that does not have an additional payload or reception
///
/// May carry data in the command data stream
pub trait AdbmsCommand {
    fn get_command(&self) -> u16;
}

impl<'a, T: SpiBus, C: OutputPin, D: DelayNs, P, const N: usize, const NR: usize, const NT: usize>
    Adbms6830<'a, T, C, D, P, N, NR, NT>
{
    /// Write the same register to every IC in the chain.
    ///
    /// `regs[0]` is the IC closest to the controller.
    pub async fn write<R: AdbmsWritableRegister>(
        &mut self,
        regs: &[R; N],
    ) -> Result<(), AdbmsError<T::Error, C::Error>> {
        let mut data = [0u8; NT];
        for ic in 0..N {
            let off = ic * TX_SIZE;
            let bytes = u64::to_le_bytes(regs[ic].into());
            data[off..off + TX_SIZE].copy_from_slice(&bytes[..TX_SIZE]);
        }
        self.write_register(R::WRITE_CMD, data).await
    }

    /// Read a register from every IC in the chain.
    ///
    /// Result index 0 is the IC closest to the controller.
    pub async fn read<R: AdbmsReadableRegister>(
        &mut self,
    ) -> Result<[R; N], AdbmsError<T::Error, C::Error>> {
        let raw = self.read_register(R::READ_CMD).await?;
        Ok(core::array::from_fn(|ic| {
            let off = ic * RX_SIZE;
            // Each 8-byte RX chunk is 6 bytes register payload + 2 bytes
            // cmd_cntr/PEC (already verified by `read_register`).
            let mut buf = [0u8; 8];
            buf[..6].copy_from_slice(&raw[off..off + 6]);
            R::from(u64::from_le_bytes(buf))
        }))
    }

    /// Send a command
    ///
    /// Result index 0 is the IC closest to the controller.
    pub async fn command<R: AdbmsCommand>(
        &mut self,
        cmd: R,
    ) -> Result<(), AdbmsError<T::Error, C::Error>> {
        self.send_command::<false>(cmd.get_command().to_le_bytes())
            .await
    }
}
