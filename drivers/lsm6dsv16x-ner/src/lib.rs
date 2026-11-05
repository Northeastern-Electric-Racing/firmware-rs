#![no_std]

#[allow(dead_code)]
use defmt::{error, info, warn};
use embedded_hal_async::delay::DelayNs;
use embedded_hal_async::spi::SpiDevice;
use lsm6dsv16x_rs::asynchronous::Error as LsmError;
use lsm6dsv16x_rs::asynchronous::Lsm6dsv16x;
use lsm6dsv16x_rs::asynchronous::register::MainBank;
use lsm6dsv16x_rs::asynchronous::register::main::FiltGyLp1Bandwidth;
use lsm6dsv16x_rs::asynchronous::register::main::FiltSettlingMask;
use lsm6dsv16x_rs::asynchronous::register::main::FiltXlLp2Bandwidth;
use lsm6dsv16x_rs::asynchronous::register::main::GyFullScale;
use lsm6dsv16x_rs::asynchronous::register::main::Odr;
use lsm6dsv16x_rs::asynchronous::register::main::Reset;
use lsm6dsv16x_rs::asynchronous::register::main::XlFullScale;
use st_mems_bus::asynchronous::SpiBus;
use uom::si::acceleration::meter_per_second_squared;
use uom::si::angular_velocity::degree_per_second;
use uom::si::f32::*;
#[macro_use]
extern crate uom;

unit! {
    system: uom::si;
    quantity: uom::si::acceleration;
    @milligravity: 9.80665e-3; "mG", "milligravity", "milligravities"; // Singular abbreviation, singular name, plural name
}

// each unit! expansion emits its own `__system`/`__quantity`/`Conversion`/`Unit`
// imports and `mod f32`, so a second invocation needs its own module
pub mod mdps {
    unit! {
        system: uom::si;
        quantity: uom::si::angular_velocity;
        @millidegree_per_second: 1.745_329_251_994_329_5_E-5; "mdps", "millidegree per second", "millidegrees per second"; // Singular abbreviation, singular name, plural name
        // coefficient is relative to the base unit rad/s: (pi/180)/1000
    }
}
use mdps::millidegree_per_second;

const ID: u8 = 0x70;
pub struct NerLsm6dsv16x<SPI: SpiDevice, D: DelayNs> {
    imu: Lsm6dsv16x<SpiBus<SPI>, D, MainBank>,
}

pub struct AccelVec {
    pub x: Acceleration,
    pub y: Acceleration,
    pub z: Acceleration,
}

impl defmt::Format for AccelVec {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "x: {} mG, y: {} mG, z: {} mG",
            self.x.get::<milligravity>(),
            self.y.get::<milligravity>(),
            self.z.get::<milligravity>()
        )
    }
}

#[derive(Debug)]
pub enum ImuInitError<E> {
    WrongId(u8),
    DriverError(LsmError<E>),
}
impl<E> From<LsmError<E>> for ImuInitError<E> {
    fn from(value: LsmError<E>) -> Self {
        ImuInitError::DriverError(value)
    }
}

pub struct AngularVelVec {
    pub roll_rate: AngularVelocity,
    pub pitch_rate: AngularVelocity,
    pub yaw_rate: AngularVelocity,
}

impl defmt::Format for AngularVelVec {
    fn format(&self, fmt: defmt::Formatter) {
        defmt::write!(
            fmt,
            "roll: {} mdps, pitch: {} mdps, yaw: {} mdps",
            self.roll_rate.get::<millidegree_per_second>(),
            self.pitch_rate.get::<millidegree_per_second>(),
            self.yaw_rate.get::<millidegree_per_second>()
        )
    }
}

impl AngularVelVec {
    pub fn from_rads_per_s(p: f32, q: f32, r: f32) -> Self {
        AngularVelVec {
            roll_rate: AngularVelocity::new::<degree_per_second>(p),
            pitch_rate: AngularVelocity::new::<degree_per_second>(q),
            yaw_rate: AngularVelocity::new::<degree_per_second>(r),
        }
    }
    pub fn from_raws(raws: [i16; 3]) -> Self {
        AngularVelVec {
            roll_rate: AngularVelocity::new::<millidegree_per_second>(raws[0] as f32),
            pitch_rate: AngularVelocity::new::<millidegree_per_second>(raws[1] as f32),
            yaw_rate: AngularVelocity::new::<millidegree_per_second>(raws[2] as f32),
        }
    }
}

impl AccelVec {
    pub fn from_mps_sq(x: f32, y: f32, z: f32) -> Self {
        AccelVec {
            x: Acceleration::new::<meter_per_second_squared>(x),
            y: Acceleration::new::<meter_per_second_squared>(y),
            z: Acceleration::new::<meter_per_second_squared>(z),
        }
    }
    pub fn from_raws(raw: [i16; 3]) -> Self {
        use lsm6dsv16x_rs::asynchronous::from_fs2_to_mg;
        AccelVec {
            x: Acceleration::new::<milligravity>(from_fs2_to_mg(raw[0])),
            y: Acceleration::new::<milligravity>(from_fs2_to_mg(raw[1])),
            z: Acceleration::new::<milligravity>(from_fs2_to_mg(raw[2])),
        }
    }
}
impl<SPI: SpiDevice, D: DelayNs> NerLsm6dsv16x<SPI, D> {
    pub fn new(spi: SPI, delay: D) -> Self {
        NerLsm6dsv16x {
            imu: Lsm6dsv16x::new_spi(spi, delay),
        }
    }

    pub async fn get_accel(&mut self) -> Option<AccelVec> {
        match self.imu.flag_data_ready_get().await {
            Ok(f) => {
                let raw_accel = self.imu.acceleration_raw_get().await;
                if f.drdy_xl == 1 && raw_accel.is_ok() {
                    let accel = AccelVec::from_raws(raw_accel.unwrap());
                    Some(accel)
                } else {
                    None
                }
            }
            Err(_) => {
                warn!("Reading accelerometer failed!");
                None
            }
        }
    }
    pub async fn get_anguar_vel(&mut self) -> Option<AngularVelVec> {
        match self.imu.flag_data_ready_get().await {
            Ok(f) => {
                let raw_rate = self.imu.angular_rate_raw_get().await;
                if f.drdy_gy == 1 && raw_rate.is_ok() {
                    let vel = AngularVelVec::from_raws(raw_rate.unwrap());
                    Some(vel)
                } else {
                    None
                }
            }
            Err(_) => {
                warn!("Reading gryo failed!");
                None
            }
        }
    }
    pub async fn init(&mut self) -> Result<(), ImuInitError<SPI::Error>> {
        self.imu.tim.delay_ms(5).await;

        // Check device ID
        let id = self.imu.device_id_get().await?;
        if id != ID {
            error!("Unexpected device ID: {:x}", id);
            return Err(ImuInitError::WrongId(id));
        }

        // Restore default configuration
        self.imu.reset_set(Reset::RestoreCtrlRegs).await?;
        let mut rst: Reset = Reset::RestoreCtrlRegs;
        while rst != Reset::Ready {
            rst = self.imu.reset_get().await?;
        }

        // Enable Block Data Update
        self.imu.block_data_update_set(1).await?;

        // Set Output Data Rate for accelerometer and gyroscope
        self.imu.xl_data_rate_set(Odr::_7_5hz).await?;
        self.imu.gy_data_rate_set(Odr::_15hz).await?;

        // Set full scale for accelerometer and gyroscope
        self.imu.xl_full_scale_set(XlFullScale::_2g).await?;
        self.imu.gy_full_scale_set(GyFullScale::_2000dps).await?;

        // Configure filtering chain
        let filt_settling_mask = FiltSettlingMask {
            drdy: 1,
            ois_drdy: 1,
            irq_xl: 1,
            irq_g: 1,
        };
        self.imu.filt_settling_mask_set(filt_settling_mask).await?;
        self.imu.filt_gy_lp1_set(1).await?;
        self.imu
            .filt_gy_lp1_bandwidth_set(FiltGyLp1Bandwidth::UltraLight)
            .await?;
        self.imu.filt_xl_lp2_set(1).await?;
        self.imu
            .filt_xl_lp2_bandwidth_set(FiltXlLp2Bandwidth::Strong)
            .await?;

        info!("IMU init complete");
        Ok(())
    }
}
