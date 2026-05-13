#![no_std]
#![no_main]

use cortex_m::peripheral::SCB;
use cortex_m_rt::{exception, ExceptionFrame};
use defmt::{info, unwrap};
use embassy_executor::Spawner;
use embassy_futures::select::select_array;
use embassy_stm32::{
    bind_interrupts,
    can::{
        Can, Frame, Rx0InterruptHandler, Rx1InterruptHandler, SceInterruptHandler, StandardId,
        TxInterruptHandler,
    },
    exti,
    exti::ExtiInput,
    gpio::Level,
    interrupt,
    peripherals::CAN1,
};
use embassy_stm32::{
    peripherals,
    usart::{self},
    Config,
};
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

bind_interrupts!(struct IrqsExti4 {
    EXTI4 => exti::InterruptHandler<interrupt::typelevel::EXTI4>;
});
bind_interrupts!(struct IrqsExti5 {
    EXTI9_5 => exti::InterruptHandler<interrupt::typelevel::EXTI9_5>;
});
bind_interrupts!(struct IrqsExti0 {
    EXTI0 => exti::InterruptHandler<interrupt::typelevel::EXTI0>;
});
bind_interrupts!(struct IrqsExti1 {
    EXTI1 => exti::InterruptHandler<interrupt::typelevel::EXTI1>;
});
bind_interrupts!(struct IrqsExti2 {
    EXTI2 => exti::InterruptHandler<interrupt::typelevel::EXTI2>;
});
bind_interrupts!(struct IrqsExti3 {
    EXTI3 => exti::InterruptHandler<interrupt::typelevel::EXTI3>;
});

const SEND_MSG_ID: StandardId = StandardId::new(0x680).expect("Could not parse ID");

// main should be where the peripheral object is used, and then peripherals are init-ed and sent to the threads
// periph. obj sent to threads should not be mut, they can be edited in threads
// the loop at the end of main should be to refresh the watchdog, however main can return if needed
// put the obj used in the thread immediately before the thread instantiation
#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    info!("Initializing wheel...");
    // initialize the project, ensure we can debug during sleep
    let p = embassy_stm32::init(Config::default());

    // embassy enforces pin mappings to their correct functions for the most at compile time
    let mut can = Can::new(p.CAN1, p.PA11, p.PA12, IrqsCAN);
    can.set_bitrate(500_000);
    can.enable().await;

    // let mut usart = Uart::new(
    //     p.USART2,
    //     p.PA3,
    //     p.PA2,
    //     IrqsUsart,
    //     p.DMA1_CH6,
    //     p.DMA1_CH5,
    //     usart::Config::default(),
    // )
    // .unwrap();
    // let mut s: String<128> = String::new();
    // core::write!(&mut s, "Hello DMA World!\r\n",).unwrap();
    // unwrap!(usart.write(s.as_bytes()).await);

    let mut button1 = ExtiInput::new(p.PA1, p.EXTI1, embassy_stm32::gpio::Pull::Up, IrqsExti1);
    let mut button2 = ExtiInput::new(p.PA2, p.EXTI2, embassy_stm32::gpio::Pull::Up, IrqsExti2);
    let mut button3 = ExtiInput::new(p.PA3, p.EXTI3, embassy_stm32::gpio::Pull::Up, IrqsExti3);
    let mut button4 = ExtiInput::new(p.PA4, p.EXTI4, embassy_stm32::gpio::Pull::Up, IrqsExti4);
    let mut button5 = ExtiInput::new(p.PC5, p.EXTI5, embassy_stm32::gpio::Pull::Up, IrqsExti5);
    let mut button6 = ExtiInput::new(p.PC6, p.EXTI6, embassy_stm32::gpio::Pull::Up, IrqsExti5);

    loop {
        select_array([
            button1.wait_for_falling_edge(),
            button2.wait_for_falling_edge(),
            button3.wait_for_falling_edge(),
            button4.wait_for_falling_edge(),
            button5.wait_for_falling_edge(),
            button6.wait_for_falling_edge(),
        ])
        .await;

        let but1_val = (button1.get_level() == Level::Low) as u8;
        let but2_val = (button2.get_level() == Level::Low) as u8;
        let but3_val = (button3.get_level() == Level::Low) as u8;
        let but4_val = (button4.get_level() == Level::Low) as u8;
        let but5_val = (button5.get_level() == Level::Low) as u8;
        let but6_val = (button6.get_level() == Level::Low) as u8;

        can.write(&unwrap!(Frame::new_data(
            SEND_MSG_ID,
            &[but1_val, but2_val, but3_val, but4_val, but5_val, but6_val]
        )))
        .await;
    }
}

#[exception]
unsafe fn HardFault(_frame: &ExceptionFrame) -> ! {
    SCB::sys_reset() // <- you could do something other than reset
}
