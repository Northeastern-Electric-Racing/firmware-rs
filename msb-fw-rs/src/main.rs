#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use core::fmt::Write;

use cortex_m::{peripheral::SCB, singleton};
use cortex_m_rt::{exception, ExceptionFrame};
use defmt::{debug, info, unwrap};
use embassy_executor::Spawner;
use embassy_stm32::{
    adc::{Adc, SampleTime, Sequence},
    bind_interrupts,
    can::{Can, Rx0InterruptHandler, Rx1InterruptHandler, SceInterruptHandler, TxInterruptHandler},
    i2c::{self, I2c},
    peripherals::CAN1,
    time::Hertz,
};
use embassy_stm32::{
    can::Frame,
    gpio::{Input, Level, Output, Pull, Speed},
    peripherals,
    usart::{self, Uart},
    wdg::IndependentWatchdog,
    Config,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel, mutex::Mutex};
use embassy_time::Timer;
use heapless::String;
use msb_fw_rs::{can_handler, controllers, readers, DeviceLocation, SharedI2c3};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

// here are our interrupts.  Embassy is interrupt by default
bind_interrupts!(struct IrqsCAN {
    CAN1_RX0 => Rx0InterruptHandler<CAN1>;
    CAN1_RX1 => Rx1InterruptHandler<CAN1>;
    CAN1_SCE => SceInterruptHandler<CAN1>;
    CAN1_TX => TxInterruptHandler<CAN1>;
});

bind_interrupts!(struct IrqsUsart {
    USART2 => usart::InterruptHandler<peripherals::USART2>;
});

bind_interrupts!(struct IrqsI2c {
    I2C3_EV => i2c::EventInterruptHandler<peripherals::I2C3>;
    I2C3_ER => i2c::ErrorInterruptHandler<peripherals::I2C3>;
});

// channels are like RTOS queues, with a limit.  They are MPMC easy to pass around in threads.
static CAN_CHANNEL: Channel<ThreadModeRawMutex, Frame, 25> = Channel::new();

// main should be where the peripheral object is used, and then peripherals are init-ed and sent to the threads
// periph. obj sent to threads should not be mut, they can be edited in threads
// the loop at the end of main should be to refresh the watchdog, however main can return if needed
// put the obj used in the thread immediately before the thread instantiation
#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    info!("Initializing MSB-FW...");
    // initialize the project, ensure we can debug during sleep
    let mut p = embassy_stm32::init(Config::default());

    // create some GPIO on input mode and read from them
    let pin0 = Input::new(p.PC10, Pull::None);
    let addr0 = pin0.get_level() == Level::High;

    let pin1 = Input::new(p.PC11, Pull::None);
    let addr1 = pin1.get_level() == Level::High;

    let pin2 = Input::new(p.PC12, Pull::None);
    let addr2 = pin2.get_level() == Level::High;

    // create our MSB device location from the pin states
    let loc = DeviceLocation::from((addr0, addr1, addr2));
    info!("MSB Location is: {}", loc);

    // create a thread to hold some LEDs and blink them or whatever
    let led1 = Output::new(p.PC4, Level::High, Speed::Low);
    let led2 = Output::new(p.PC5, Level::High, Speed::Low);
    spawner.must_spawn(controllers::control_leds(
        // note that most types have an internal generic holding the pin or bus itself, this can be removed by degrade
        // this makes types more generic and should be done for all pins, but is not necessary for multi-bus i2c or whatnot
        led1,
        led2,
        loc.clone(),
    ));
    // embassy enforces pin mappings to their correct functions for the most at compile time
    let can = Can::new(p.CAN1, p.PA11, p.PA12, IrqsCAN);
    // pass in a can channel consumer to get the frames from any producer
    spawner.must_spawn(can_handler::can_handler(can, CAN_CHANNEL.receiver(), loc));

    // checkout this fuckery, the official way to have two things use one i2c bus
    // see here: https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/shared_bus.rs
    // this uses the embassy_embedded_hal extension, which basically converts these wierd ass types to embedded_hal compatable traits
    static I2C_BUS: StaticCell<SharedI2c3> = StaticCell::new();
    let i2c = I2c::new(
        p.I2C3,
        p.PA8,
        p.PC9,
        IrqsI2c,
        p.DMA1_CH4, // for must things embassy is DMA by default, allowing for bet use of the async executer.  NoDma can be passed to disable that
        p.DMA1_CH2,
        Hertz(100_000),
        i2c::Config::default(),
    );
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));

    #[cfg(feature = "temp-sensor")]
    spawner.must_spawn(readers::temperature_reader(i2c_bus, CAN_CHANNEL.sender()));

    #[cfg(feature = "tof-sensor")]
    spawner.must_spawn(readers::tof_reader(i2c_bus, CAN_CHANNEL.sender()));

    #[cfg(feature = "imu-sensor")]
    spawner.must_spawn(readers::imu_reader(i2c_bus, CAN_CHANNEL.sender()));

    // this pretty much straight from docs, adc dma is very new in embassy stm32 hal
    const ADC_BUF_SIZE: usize = 1024;
    let adc1 = Adc::new(p.ADC1);
    let adc_data = singleton!(ADCDAT : [u16; ADC_BUF_SIZE] = [0u16; ADC_BUF_SIZE])
        .expect("Could not init adc buffer");
    let mut adc1 = adc1.into_ring_buffered(p.DMA2_CH0, adc_data);
    adc1.set_sample_sequence(Sequence::One, &mut p.PA0, SampleTime::CYCLES112); // SHOCKPOT
    adc1.set_sample_sequence(Sequence::Two, &mut p.PA5, SampleTime::CYCLES112); // STRAIN 1
    adc1.set_sample_sequence(Sequence::Three, &mut p.PA6, SampleTime::CYCLES112); // STRAIN 2
    spawner.must_spawn(readers::adc1_reader(adc1, CAN_CHANNEL.sender()));

    let mut usart = Uart::new(
        p.USART2,
        p.PA3,
        p.PA2,
        IrqsUsart,
        p.DMA1_CH6,
        p.DMA1_CH5,
        usart::Config::default(),
    )
    .unwrap();
    let mut s: String<128> = String::new();
    core::write!(&mut s, "MSB-FW.rs prints in RTT, not UART!\r\n",).unwrap();
    unwrap!(usart.write(s.as_bytes()).await);

    let mut watchdog = IndependentWatchdog::new(p.IWDG, 1000000);
    watchdog.unleash();
    loop {
        debug!("Status: Alive");
        Timer::after_millis(500).await;
        watchdog.pet();
    }
}

#[exception]
unsafe fn HardFault(_frame: &ExceptionFrame) -> ! {
    SCB::sys_reset() // <- you could do something other than reset
}

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
