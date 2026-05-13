#![no_std]
#![no_main]

use core::fmt::Write;
use cortex_m::peripheral::SCB;
use cortex_m_rt::{ExceptionFrame, exception};
use defmt::debug;
use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_stm32::usart::Uart;
use embassy_stm32::{Config, dma, peripherals, usart};
use embassy_stm32::{bind_interrupts, wdg::IndependentWatchdog};
use embassy_time::Timer;
use heapless::String;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct IrqsUsart {
    LPUART1 => usart::InterruptHandler<peripherals::LPUART1>;
    GPDMA1_CHANNEL0 => dma::InterruptHandler<peripherals::GPDMA1_CH0>;
    GPDMA1_CHANNEL1 => dma::InterruptHandler<peripherals::GPDMA1_CH1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    info!("Initializing wheel...");
    // initialize the project, ensure we can debug during sleep
    let p = embassy_stm32::init(Config::default());

    let mut usart_config = usart::Config::default();
    usart_config.swap_rx_tx = true;
    let mut usart = Uart::new(
        p.LPUART1,
        p.PA10,
        p.PA9,
        p.GPDMA1_CH0,
        p.GPDMA1_CH1,
        IrqsUsart,
        usart_config,
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
