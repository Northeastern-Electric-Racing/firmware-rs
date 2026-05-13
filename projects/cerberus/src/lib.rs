#![no_std]

pub mod bms;
pub mod can_handler;
pub mod dti;
pub mod fault;
pub mod monitor;
pub mod state_machine;

pub type SharedI2c = embassy_sync::mutex::Mutex<
    embassy_sync::blocking_mutex::raw::NoopRawMutex,
    embassy_stm32::i2c::I2c<'static, embassy_stm32::mode::Async, embassy_stm32::i2c::mode::Master>,
>;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum FunctionalType {
    READY,
    /* F means functional */
    FPit,
    FPerformance,
    FEfficiency,
    REVERSE,
    FAULTED,
}
#[derive(Copy, Clone)]
pub enum NeroType {
    OFF,
    PIT,         //SPEED_LIMITIED
    PERFORMANCE, //AUTOCROSS
    EFFICIENCY,  //ENDURANCE
    DEBUG,
    CONFIGURATION,
    FlappyBird,
    EXIT,
}
#[derive(Copy, Clone)]
pub enum StateTransition {
    Functional(FunctionalType),
    Nero(NeroType),
}

// TODO: this is a breaking change and is also ugly and non-exhuastive in terms of IDs
// However it is centralized which is better than the C impl

#[repr(u8)]
pub enum FaultSeverity {
    Defcon1 = 1,
    Defcon2 = 2,
    Defcon3 = 3,
    Defcon4 = 4,
    Defcon5 = 5,
}

#[derive(Copy, Clone)]
pub enum FaultCode {
    FaultsClear = 0x0,
    BmsCanMonitorFault = 0x800,
}

impl FaultCode {
    fn get_severity(&self) -> FaultSeverity {
        match self {
            FaultCode::FaultsClear => FaultSeverity::Defcon5,
            FaultCode::BmsCanMonitorFault => FaultSeverity::Defcon4,
        }
    }
}

pub enum PduCommand {
    WritePump(bool),
    WriteBrakelight(bool),
    WriteFault(bool),
    SoundRtds,
}
