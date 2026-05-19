use embedded_hal::digital::OutputPin;
use embedded_hal_async::{delay::DelayNs, digital::Wait, spi::SpiBus};

/// Enum containing all possible types of errors when interacting with Adbms6830
#[derive(Debug)]
pub enum AdbmsError<E, H> {
    /// A HAL derived SPI comms error
    CommunicationError(E),
    /// A HAL derived CS pin error
    CSControlError(H),
    /// A CRC communication error to the chip zero-indexed below
    PECError(usize),
    /// A buffer length mismatch (uncommon)
    LengthMismatch { expected: usize, got: usize },
    /// An error with the GPIO based polling
    PollError,
}

// TODO delete these if we ever get generic_const_exprs
/// The TX size buffer
pub const TX_SIZE: usize = 6;
/// The RX size buffer
pub const RX_SIZE: usize = 8;

/// Poll the ADC by reading SPI bytes from SDO and observing when they go high
/// R: The amount of bytes to check at a time --> more bytes, slower response to ADC but less CPU time
/// Choose 1 at first and if you need more CPU raise it
pub struct SpiPollAdc<const R: usize> {}

/// Poll the ADC by waiting until a GPIO goes high.
pub struct GpioPollAdc<PIN: Wait> {
    pin_to_wait: PIN,
}

/// Sleep for a duration to assume poll finished
pub struct SleepPollAdc {
    pub duration: core::time::Duration,
}

/// The Adbms6830 chain
/// T:
/// C:
/// D:
/// P
/// N: the number of ICs in the chain
/// NR:
/// NT:
pub struct Adbms6830<
    'a,
    T: SpiBus,
    C: OutputPin,
    D: DelayNs,
    P,
    const N: usize,
    const NR: usize,
    const NT: usize,
> {
    pub(crate) device: T,
    pub(crate) cs_pin: C,
    pub(crate) delay: D,
    pub(crate) poll_mode: P,

    pub(crate) tx_buffer: &'a mut [u8],
    pub(crate) rx_buffer: &'a mut [u8],

    pub(crate) cmd_cntr: [u32; N],
}

const CRC15_TABLE: [u16; 256] = [
    0x0000, 0xc599, 0xceab, 0xb32, 0xd8cf, 0x1d56, 0x1664, 0xd3fd, 0xf407, 0x319e, 0x3aac, 0xff35,
    0x2cc8, 0xe951, 0xe263, 0x27fa, 0xad97, 0x680e, 0x633c, 0xa6a5, 0x7558, 0xb0c1, 0xbbf3, 0x7e6a,
    0x5990, 0x9c09, 0x973b, 0x52a2, 0x815f, 0x44c6, 0x4ff4, 0x8a6d, 0x5b2e, 0x9eb7, 0x9585, 0x501c,
    0x83e1, 0x4678, 0x4d4a, 0x88d3, 0xaf29, 0x6ab0, 0x6182, 0xa41b, 0x77e6, 0xb27f, 0xb94d, 0x7cd4,
    0xf6b9, 0x3320, 0x3812, 0xfd8b, 0x2e76, 0xebef, 0xe0dd, 0x2544, 0x2be, 0xc727, 0xcc15, 0x98c,
    0xda71, 0x1fe8, 0x14da, 0xd143, 0xf3c5, 0x365c, 0x3d6e, 0xf8f7, 0x2b0a, 0xee93, 0xe5a1, 0x2038,
    0x7c2, 0xc25b, 0xc969, 0xcf0, 0xdf0d, 0x1a94, 0x11a6, 0xd43f, 0x5e52, 0x9bcb, 0x90f9, 0x5560,
    0x869d, 0x4304, 0x4836, 0x8daf, 0xaa55, 0x6fcc, 0x64fe, 0xa167, 0x729a, 0xb703, 0xbc31, 0x79a8,
    0xa8eb, 0x6d72, 0x6640, 0xa3d9, 0x7024, 0xb5bd, 0xbe8f, 0x7b16, 0x5cec, 0x9975, 0x9247, 0x57de,
    0x8423, 0x41ba, 0x4a88, 0x8f11, 0x57c, 0xc0e5, 0xcbd7, 0xe4e, 0xddb3, 0x182a, 0x1318, 0xd681,
    0xf17b, 0x34e2, 0x3fd0, 0xfa49, 0x29b4, 0xec2d, 0xe71f, 0x2286, 0xa213, 0x678a, 0x6cb8, 0xa921,
    0x7adc, 0xbf45, 0xb477, 0x71ee, 0x5614, 0x938d, 0x98bf, 0x5d26, 0x8edb, 0x4b42, 0x4070, 0x85e9,
    0xf84, 0xca1d, 0xc12f, 0x4b6, 0xd74b, 0x12d2, 0x19e0, 0xdc79, 0xfb83, 0x3e1a, 0x3528, 0xf0b1,
    0x234c, 0xe6d5, 0xede7, 0x287e, 0xf93d, 0x3ca4, 0x3796, 0xf20f, 0x21f2, 0xe46b, 0xef59, 0x2ac0,
    0xd3a, 0xc8a3, 0xc391, 0x608, 0xd5f5, 0x106c, 0x1b5e, 0xdec7, 0x54aa, 0x9133, 0x9a01, 0x5f98,
    0x8c65, 0x49fc, 0x42ce, 0x8757, 0xa0ad, 0x6534, 0x6e06, 0xab9f, 0x7862, 0xbdfb, 0xb6c9, 0x7350,
    0x51d6, 0x944f, 0x9f7d, 0x5ae4, 0x8919, 0x4c80, 0x47b2, 0x822b, 0xa5d1, 0x6048, 0x6b7a, 0xaee3,
    0x7d1e, 0xb887, 0xb3b5, 0x762c, 0xfc41, 0x39d8, 0x32ea, 0xf773, 0x248e, 0xe117, 0xea25, 0x2fbc,
    0x846, 0xcddf, 0xc6ed, 0x374, 0xd089, 0x1510, 0x1e22, 0xdbbb, 0xaf8, 0xcf61, 0xc453, 0x1ca,
    0xd237, 0x17ae, 0x1c9c, 0xd905, 0xfeff, 0x3b66, 0x3054, 0xf5cd, 0x2630, 0xe3a9, 0xe89b, 0x2d02,
    0xa76f, 0x62f6, 0x69c4, 0xac5d, 0x7fa0, 0xba39, 0xb10b, 0x7492, 0x5368, 0x96f1, 0x9dc3, 0x585a,
    0x8ba7, 0x4e3e, 0x450c, 0x8095,
];

impl<'a, T: SpiBus, C: OutputPin, D: DelayNs, P, const N: usize, const NR: usize, const NT: usize>
    Adbms6830<'a, T, C, D, P, N, NR, NT>
{
    /// Calculate the CRC15, used for the command PEC
    pub(crate) fn pec15_calc(data: [u8; 2]) -> u16 {
        let mut remainder = 16u16;
        for i in 0..2 {
            let addr = ((remainder >> 7) ^ data[i] as u16) & 0xFFu16;
            remainder = (remainder << 8) ^ CRC15_TABLE[addr as usize];
        }

        remainder * 2
    }

    /// Calculate the CRC10, used for the data PEC
    /// I is the size of the whole register, and R if true, indicates this is an Rx command
    pub(crate) fn pec10_calc<const I: usize, const R: bool>(data: &[u8; I]) -> u16 {
        let mut remainder = 16u16;
        const POLYNOMIAL: u16 = 0x8Fu16;

        let data_size = if R { I - 2 } else { I };

        for byte_index in 0..data_size {
            remainder ^= (data[byte_index] as u16) << 2u16;

            for _ in (1..=8).rev() {
                if (remainder & 0x200u16) > 0 {
                    remainder = remainder << 1;
                    remainder = remainder ^ POLYNOMIAL;
                } else {
                    remainder = remainder << 1;
                }
            }
        }

        if R {
            remainder ^= ((data[I - 2] as u16) & 0xFCu16) << 2u16;
        }
        /* Perform modulo-2 division, a bit at a time */
        for _ in (1..=6).rev() {
            /* Try to divide the current data bit */
            if (remainder & 0x200u16) > 0 {
                remainder = remainder << 1;
                remainder = remainder ^ POLYNOMIAL;
            } else {
                remainder = remainder << 1;
            }
        }

        remainder
    }

    async fn wake_ic(&mut self) -> Result<(), AdbmsError<T::Error, C::Error>> {
        // TODO tune
        for _ in 0..N {
            self.cs_pin.set_low().map_err(AdbmsError::CSControlError)?;
            self.delay.delay_us(500).await;
            self.cs_pin.set_high().map_err(AdbmsError::CSControlError)?;
            self.delay.delay_us(500).await;
        }
        Ok(())
    }

    /// Send a isoSPI command
    /// B - if true, do not release CS (useful for eventual ADC poll).  Will still flush
    async fn send_command<const B: bool>(
        &mut self,
        cmd: [u8; 2],
    ) -> Result<(), AdbmsError<T::Error, C::Error>> {
        let mut tx_buf = [0u8; 4];
        tx_buf[0] = cmd[0];
        tx_buf[1] = cmd[1];
        let pec = Self::pec15_calc(cmd);
        tx_buf[2] = (pec >> 8) as u8;
        tx_buf[3] = pec as u8;

        self.wake_ic().await?;

        self.cs_pin.set_low().map_err(AdbmsError::CSControlError)?;
        self.device
            .write(&mut tx_buf)
            .await
            .map_err(AdbmsError::CommunicationError)?;

        self.device
            .flush()
            .await
            .map_err(AdbmsError::CommunicationError)?;

        if !B {
            self.cs_pin.set_high().map_err(AdbmsError::CSControlError)
        } else {
            Ok(())
        }
    }

    /// Read a register
    pub(crate) async fn read_register(
        &mut self,
        cmd: [u8; 2],
    ) -> Result<[u8; NR], AdbmsError<T::Error, C::Error>> {
        self.send_command::<true>(cmd).await?;

        self.device
            .read(self.rx_buffer)
            .await
            .map_err(AdbmsError::CommunicationError)?;
        self.device
            .flush()
            .await
            .map_err(AdbmsError::CommunicationError)?;
        self.cs_pin.set_high().map_err(AdbmsError::CSControlError)?;

        let mut rx_data = [0; NR];

        for c in 0..N {
            let idex = c * RX_SIZE;
            rx_data[idex..idex + RX_SIZE].copy_from_slice(&self.rx_buffer[idex..idex + RX_SIZE]);

            self.cmd_cntr[c] = (self.rx_buffer[idex + RX_SIZE - 2] >> 2) as u32;

            let recv_pec = (((self.rx_buffer[idex + (RX_SIZE - 2)] & 0x03) << 8)
                | self.rx_buffer[idex + RX_SIZE - 1]) as u16;

            let calculated_pec = Self::pec10_calc::<RX_SIZE, true>(
                self.rx_buffer[idex..idex + RX_SIZE].try_into().unwrap(),
            );

            if calculated_pec != recv_pec {
                return Err(AdbmsError::PECError(c));
            }
        }

        Ok(rx_data)
    }

    /// Write a register
    pub(crate) async fn write_register(
        &mut self,
        cmd: [u8; 2],
        data: [u8; NT],
    ) -> Result<(), AdbmsError<T::Error, C::Error>> {
        self.tx_buffer.fill_with(Default::default);
        self.tx_buffer[0] = cmd[0];
        self.tx_buffer[1] = cmd[1];
        let pec = Self::pec15_calc(cmd);
        self.tx_buffer[2] = (pec >> 8) as u8;
        self.tx_buffer[3] = pec as u8;

        let mut buf_index = 4usize;

        for c in (1..=N).rev() {
            let src_addr = (c - 1) * TX_SIZE;
            for cb in 0..TX_SIZE {
                self.tx_buffer[buf_index] = data[((c - 1) * TX_SIZE) + cb];
                buf_index = buf_index + 1;
            }
            let data_pec = Self::pec10_calc::<TX_SIZE, false>(
                data[src_addr..src_addr + TX_SIZE].try_into().unwrap(),
            );
            self.tx_buffer[buf_index] = (data_pec >> 8) as u8;
            buf_index = buf_index + 1;
            self.tx_buffer[buf_index] = data_pec as u8;
            buf_index = buf_index + 1;
        }

        self.wake_ic().await?;

        self.cs_pin.set_low().map_err(AdbmsError::CSControlError)?;
        self.device
            .write(self.tx_buffer)
            .await
            .map_err(AdbmsError::CommunicationError)?;
        self.device
            .flush()
            .await
            .map_err(AdbmsError::CommunicationError)?;
        self.cs_pin.set_high().map_err(AdbmsError::CSControlError)
    }

    /// Creates a new Adbms6830 chain
    ///
    /// # Panics
    /// Panics if `N` || `NR` || `NT` == 0, `NR` not divisible by RX_SIZE, and `NT` not divisible by TX_SIZE
    ///
    /// Example instantiation
    /// ```
    /// const IC_CNT: usize = 4;
    /// const CNT_TX: usize = IC_CNT * TX_SIZE;
    /// const CNT_RX: usize = IC_CNT * RX_SIZE;
    /// let client = Adbms6830::<_, _, IC_CNT, CNT_RX, CNT_TX>::new(device, cs_pin);
    /// ```
    pub fn new(
        device: T,
        cs_pin: C,
        poll_mode: P,
        delay: D,
        tx_buffer: &'a mut [u8],
        rx_buffer: &'a mut [u8],
    ) -> Self {
        const {
            assert!(N > 0);
            assert!(NR % RX_SIZE == 0 && NR != 0);
            assert!(NT % TX_SIZE == 0 && NT != 0);
        }
        assert!(tx_buffer.len() >= 4 + (RX_SIZE * N));
        assert!(rx_buffer.len() >= RX_SIZE * N);
        Self {
            device,
            cs_pin,
            delay,
            poll_mode,
            tx_buffer,
            rx_buffer,
            cmd_cntr: [0u32; N],
        }
    }
}

trait PollAdc {
    type Output;
    /// Returns when the ADC has finished its single shot conversion
    async fn poll_adc(&mut self) -> Self::Output;
}

impl<
    'a,
    T: SpiBus,
    C: OutputPin,
    D: DelayNs,
    const N: usize,
    const NR: usize,
    const NT: usize,
    const R: usize,
> PollAdc for Adbms6830<'a, T, C, D, SpiPollAdc<R>, N, NR, NT>
{
    type Output = Result<(), AdbmsError<T::Error, C::Error>>;
    async fn poll_adc(&mut self) -> Self::Output {
        let mut buf = [0u8; R];
        while buf[0] == 0 {
            self.device
                .read(&mut buf)
                .await
                .map_err(AdbmsError::CommunicationError)?;
        }

        Ok(())
    }
}

impl<
    'a,
    T: SpiBus,
    C: OutputPin,
    D: DelayNs,
    const N: usize,
    const NR: usize,
    const NT: usize,
    PIN: Wait,
> PollAdc for Adbms6830<'a, T, C, D, GpioPollAdc<PIN>, N, NR, NT>
{
    type Output = Result<(), AdbmsError<T::Error, C::Error>>;
    async fn poll_adc(&mut self) -> Self::Output {
        match self.poll_mode.pin_to_wait.wait_for_high().await {
            Ok(_) => Ok(()),
            Err(_) => Err(AdbmsError::PollError),
        }
    }
}

impl<'a, T: SpiBus, C: OutputPin, D: DelayNs, const N: usize, const NR: usize, const NT: usize>
    PollAdc for Adbms6830<'a, T, C, D, SleepPollAdc, N, NR, NT>
{
    type Output = Result<(), AdbmsError<T::Error, C::Error>>;
    async fn poll_adc(&mut self) -> Self::Output {
        todo!("aaa");
    }
}
