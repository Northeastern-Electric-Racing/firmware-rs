use core::convert::TryFrom;

use embedded_hal_async::i2c::I2c;

use crate::{
    VL6180X,
    error::Error,
    register::{
        self, AmbientStatusErrorCode, RangeStatusErrorCode, Register8Bit, Register16Bit,
        ResultInterruptStatusGpioCode,
    },
};

impl<MODE, I2C, E> VL6180X<MODE, I2C>
where
    I2C: I2c<Error = E>,
{
    pub(crate) async fn read_range_mm_blocking_direct(&mut self) -> Result<u16, Error<E>> {
        let mut c = 0;
        while ResultInterruptStatusGpioCode::has_status(
            ResultInterruptStatusGpioCode::NoRangeEvents,
            self.read_named_register(Register8Bit::RESULT__INTERRUPT_STATUS_GPIO)
                .await?,
        ) {
            c += 1;
            if c == self.config.poll_max_loop {
                return Err(Error::Timeout);
            }
        }

        self.get_range_val_and_status().await
    }

    pub(crate) async fn read_range_mm_direct(&mut self) -> Result<u16, Error<E>> {
        let interrupt_status = self
            .read_named_register(Register8Bit::RESULT__INTERRUPT_STATUS_GPIO)
            .await?;
        if ResultInterruptStatusGpioCode::has_status(
            ResultInterruptStatusGpioCode::NoRangeEvents,
            interrupt_status,
        ) {
            return Err(Error::ResultNotReady);
        }
        self.get_range_val_and_status().await
    }

    async fn get_range_val_and_status(&mut self) -> Result<u16, Error<E>> {
        let status = self
            .read_named_register(Register8Bit::RESULT__RANGE_STATUS)
            .await?;
        self.clear_range_interrupt_direct().await?;
        let error = RangeStatusErrorCode::try_from(status)
            .map_err(|_| Error::UnknownRegisterCode(status))?;
        if error != RangeStatusErrorCode::NoError {
            return Err(Error::RangeStatusError(error));
        }
        let raw_range = self
            .read_named_register(Register8Bit::RESULT__RANGE_VAL)
            .await?;
        Ok(self.convert_raw_range_to_mm(raw_range))
    }

    fn convert_raw_range_to_mm(&self, raw_range: u8) -> u16 {
        self.config.range_scaling as u16 * raw_range as u16
    }

    pub(crate) async fn read_ambient_lux_blocking_direct(&mut self) -> Result<f32, Error<E>> {
        let mut c = 0;
        while ResultInterruptStatusGpioCode::has_status(
            ResultInterruptStatusGpioCode::NoAmbientEvents,
            self.read_named_register(Register8Bit::RESULT__INTERRUPT_STATUS_GPIO)
                .await?,
        ) {
            c += 1;
            if c == self.config.poll_max_loop {
                return Err(Error::Timeout);
            }
        }
        let raw_ambient = self.get_ambient_val_and_status().await?;
        Ok(self.convert_raw_ambient_to_lux(raw_ambient))
    }

    pub(crate) async fn read_ambient_lux_direct(&mut self) -> Result<f32, Error<E>> {
        if ResultInterruptStatusGpioCode::has_status(
            ResultInterruptStatusGpioCode::NoAmbientEvents,
            self.read_named_register(Register8Bit::RESULT__INTERRUPT_STATUS_GPIO)
                .await?,
        ) {
            return Err(Error::ResultNotReady);
        }
        let raw_ambient = self.get_ambient_val_and_status().await?;
        Ok(self.convert_raw_ambient_to_lux(raw_ambient))
    }

    pub(crate) async fn read_ambient_blocking_direct(&mut self) -> Result<u16, Error<E>> {
        let mut c = 0;
        while ResultInterruptStatusGpioCode::has_status(
            ResultInterruptStatusGpioCode::NoAmbientEvents,
            self.read_named_register(Register8Bit::RESULT__INTERRUPT_STATUS_GPIO)
                .await?,
        ) {
            c += 1;
            if c == self.config.poll_max_loop {
                return Err(Error::Timeout);
            }
        }
        self.get_ambient_val_and_status().await
    }

    pub(crate) async fn read_ambient_direct(&mut self) -> Result<u16, Error<E>> {
        if ResultInterruptStatusGpioCode::has_status(
            ResultInterruptStatusGpioCode::NoAmbientEvents,
            self.read_named_register(Register8Bit::RESULT__INTERRUPT_STATUS_GPIO)
                .await?,
        ) {
            return Err(Error::ResultNotReady);
        }
        self.get_ambient_val_and_status().await
    }

    async fn get_ambient_val_and_status(&mut self) -> Result<u16, Error<E>> {
        let status = self
            .read_named_register(Register8Bit::RESULT__ALS_STATUS)
            .await?;
        self.clear_ambient_interrupt_direct().await?;
        let error = AmbientStatusErrorCode::try_from(status)
            .map_err(|_| Error::UnknownRegisterCode(status))?;
        if error != AmbientStatusErrorCode::NoError {
            return Err(Error::AmbientStatusError(error));
        }
        let raw_ambient = self
            .read_named_register_16bit(Register16Bit::RESULT__ALS_VAL)
            .await?;
        Ok(raw_ambient)
    }

    fn convert_raw_ambient_to_lux(&self, raw_ambient: u16) -> f32 {
        let analogue_gain =
            register::AMBIENT_ANALOGUE_GAIN_VALUE[self.config.ambient_analogue_gain_level as usize];

        let integration_period = self.config.ambient_integration_period;

        const LUX_RESOLUTION_FACTOR: f32 = 0.32_f32;

        (LUX_RESOLUTION_FACTOR * 100.0 / analogue_gain)
            * (raw_ambient as f32 / integration_period as f32)
    }
}
