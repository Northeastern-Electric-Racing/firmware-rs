use embedded_hal::digital::OutputPin;
use embedded_hal_async::i2c::I2c;

use super::VL6180X;
use crate::{
    error::{Error, Error2},
    register::{Register8Bit::*, SysInterruptClearCode},
};

impl<MODE, I2C, E> VL6180X<MODE, I2C>
where
    I2C: I2c<Error = E>,
{
    pub(crate) async fn read_model_id_direct(&mut self) -> Result<u8, Error<E>> {
        let id = self.read_named_register(IDENTIFICATION__MODEL_ID).await?;
        Ok(id)
    }

    pub(crate) async fn read_interrupt_status_direct(&mut self) -> Result<u8, Error<E>> {
        let status = self
            .read_named_register(RESULT__INTERRUPT_STATUS_GPIO)
            .await?;
        Ok(status)
    }

    pub(crate) async fn clear_error_interrupt_direct(&mut self) -> Result<(), Error<E>> {
        self.clear_interrupt(SysInterruptClearCode::Error as u8)
            .await?;
        Ok(())
    }

    pub(crate) async fn clear_ambient_interrupt_direct(&mut self) -> Result<(), Error<E>> {
        self.clear_interrupt(SysInterruptClearCode::Ambient as u8)
            .await?;
        Ok(())
    }

    pub(crate) async fn clear_range_interrupt_direct(&mut self) -> Result<(), Error<E>> {
        self.clear_interrupt(SysInterruptClearCode::Range as u8)
            .await?;
        Ok(())
    }

    pub(crate) async fn clear_all_interrupts_direct(&mut self) -> Result<(), Error<E>> {
        self.clear_interrupt(
            SysInterruptClearCode::Range as u8
                | SysInterruptClearCode::Ambient as u8
                | SysInterruptClearCode::Error as u8,
        )
        .await?;
        Ok(())
    }

    async fn clear_interrupt(&mut self, code: u8) -> Result<(), E> {
        self.write_named_register(SYSTEM__INTERRUPT_CLEAR, code)
            .await
    }

    pub(crate) async fn change_i2c_address_direct(
        &mut self,
        new_address: u8,
    ) -> Result<(), Error<E>> {
        if !(0x08..=0x77).contains(&new_address) {
            return Err(Error::InvalidAddress(new_address));
        }
        self.write_only_named_register(I2C_SLAVE__DEVICE_ADDRESS, new_address)
            .await?;
        self.config.address = new_address;

        Ok(())
    }

    pub(crate) fn power_off_direct<PE, P: OutputPin<Error = PE>>(
        &self,
        x_shutdown_pin: &mut P,
    ) -> Result<(), Error<PE>> {
        x_shutdown_pin.set_low().map_err(|e| Error::GpioPinError(e))
    }

    pub(crate) async fn power_on_and_init_direct<PE, P: OutputPin<Error = PE>>(
        &mut self,
        x_shutdown_pin: &mut P,
    ) -> Result<(), Error2<E, PE>> {
        x_shutdown_pin
            .set_high()
            .map_err(|e| Error2::GpioPinError(e))?;
        self.wait_device_booted()
            .await
            .map_err(|e| Error2::<E, PE>::BusError(e))?;
        self.init_hardware()
            .await
            .map_err(|e| Error2::<E, PE>::BusError(e))?;
        Ok(())
    }

    async fn wait_device_booted(&mut self) -> Result<(), E> {
        loop {
            if let Ok(result) = self.read_named_register(SYSTEM__FRESH_OUT_OF_RESET).await
                && result == 0x01
            {
                break;
            }
        }
        Ok(())
    }
}
