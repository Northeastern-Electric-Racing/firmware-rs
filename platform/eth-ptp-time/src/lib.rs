#![no_std]

use embassy_stm32::pac::{
    self,
    eth::regs::{Macstnur, Macstsur},
};

pub struct HardwareClockTime {
    pub seconds: u32,
    pub ns: u32,
}

pub trait HardwareClock {
    fn get_time(&self) -> HardwareClockTime;

    fn set_time(&self, time: HardwareClockTime);

    fn adjust_time(&self, time_ns: i32);
}

pub struct Stm32EthPtpClock;

impl Stm32EthPtpClock {
    pub fn init() -> Self {
        pac::ETH
            .ethernet_mac()
            .macier()
            .modify(|m| m.set_tsie(false));
        pac::ETH.ethernet_mac().mactscr().modify(|m| {
            m.set_tsena(true);
            m.set_tsctrlssr(true);
        });

        // TODO get rcc clock
        let c = Stm32EthPtpClock;

        c.set_time(HardwareClockTime {
            seconds: 1000000000,
            ns: 0,
        });

        Stm32EthPtpClock
    }
}

impl HardwareClock for Stm32EthPtpClock {
    fn get_time(&self) -> HardwareClockTime {
        HardwareClockTime {
            seconds: pac::ETH.ethernet_mac().macstsr().read().tss(),
            ns: pac::ETH.ethernet_mac().macstnr().read().tsss(),
        }
    }

    fn set_time(&self, time: HardwareClockTime) {
        pac::ETH.ethernet_mac().macstnur().write(|w| {
            w.set_addsub(false);
            w.set_tsss(time.ns);
        });

        pac::ETH.ethernet_mac().macstsur().write(|w| {
            w.set_tss(time.seconds);
        });

        pac::ETH.ethernet_mac().mactscr().modify(|m| {
            m.set_tsinit(true);
        });

        while pac::ETH.ethernet_mac().mactscr().read().tsinit() {}

        pac::ETH.ethernet_mac().macstsur().write_value(Macstsur(0));
    }

    fn adjust_time(&self, time_ns: i32) {
        if time_ns >= 0 {
            pac::ETH
                .ethernet_mac()
                .macstnur()
                .write_value(Macstnur(time_ns as u32));
        } else {
            let comp: u32 = 1000000000 - time_ns.unsigned_abs();

            pac::ETH.ethernet_mac().macstnur().write(|w| {
                w.set_tsss(comp);
                w.set_addsub(true);
            });
        }

        pac::ETH.ethernet_mac().mactscr().modify(|m| {
            m.set_tsupdt(true);
        });
    }
}
