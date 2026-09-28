#![no_std]
//! The NER Ethernet Handler.
//! Takes in an ethernet peripheral and pins, and handles all setup.
//! The NER Ethernet stack currently includes a LAN8670 running in PLCA mode.
//! On top of the PLCA mode we run Zenoh to communicate data.
//! We also run PTP over UDP to synchronize time effectively.
//!
//! ## ID
//! The generic ID sources the various identifiers:
//!  - The IP is `10.0.0.ID`
//!  - The mac is `06:00:00:00:00:ID`
//!  - The PLCA node is `ID-1`
//!
//! Therefore an ID of 0 is disallowed, and an ID of one assumes PLCA coordinator.
//!
//!
//! Example initilization of ethernet using this crate's utilities:
//! ```
//! // First the PHY must be reset -- this handler does not take care of that.
//! let mut phy_reset = Output::new(p.PE10, Level::Low, Speed::Low);
//! phy_reset.set_low();
//! Timer::after_millis(500).await;
//! phy_reset.set_high();
//!
//! // Now we initialize the stack
//!
//! // Use the returned object to retrieve the data channels for rx/tx and get time
//!
//! ```
//!
//!
//!

use defmt::unwrap;
use embassy_executor::Spawner;
use embassy_net::{Stack, StackStorage, wire::IpCidr};
use embassy_stm32::{
    Peri, bind_interrupts,
    eth::{
        self, CRSPin, Ethernet, GenericPhy, MDCPin, MDIOPin, RXD0Pin, RXD1Pin, RefClkPin, Sma,
        StationManagement, TXD0Pin, TXD1Pin, TXEnPin,
    },
    peripherals::{self, ETH_SMA},
    rng,
};
use static_cell::StaticCell;

pub type Device = Ethernet<'static, peripherals::ETH, GenericPhy<Sma<'static, ETH_SMA>>>;

// max PLCA nodes (only effective if coordinator)
const MAX_NODES: u8 = 8;

bind_interrupts!(struct IrqsEth {
    ETH => eth::InterruptHandler<peripherals::ETH>;
    RNG => rng::InterruptHandler<peripherals::RNG>;
});

pub struct NerEth<const ID: u8> {
    stack: Stack<'static>,
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static>) -> ! {
    runner.run().await
}

fn write_lan8670_vendor_reg(sm: &mut impl StationManagement, reg: u16, val: u16) {
    // enable vendor specific access and address write
    sm.smi_write(0, 0x0D, 0x1F);

    // write address
    sm.smi_write(0, 0x0E, reg);

    // write normal data, keep vendor specific location
    sm.smi_write(0, 0x0D, 0x1F | 1 << 14);

    // write payload
    sm.smi_write(0, 0x0E, val);
}

impl<const ID: u8> NerEth<ID> {
    const _ASSERT_VALID_SIZE: () = {
        assert!(ID > 0, "ID must not be zero!");
    };

    /// This constructs and initializes the ethernet and network stack.
    /// MAY BLOCK: MDIO calls are synchronous
    pub async fn new(
        spawner: Spawner,
        peri_rand: Peri<'static, peripherals::RNG>,
        peri_eth: Peri<'static, peripherals::ETH>,
        ref_clk: Peri<'static, impl RefClkPin<peripherals::ETH>>,
        crs: Peri<'static, impl CRSPin<peripherals::ETH>>,
        rx_d0: Peri<'static, impl RXD0Pin<peripherals::ETH>>,
        rx_d1: Peri<'static, impl RXD1Pin<peripherals::ETH>>,
        tx_d0: Peri<'static, impl TXD0Pin<peripherals::ETH>>,
        tx_d1: Peri<'static, impl TXD1Pin<peripherals::ETH>>,
        tx_en: Peri<'static, impl TXEnPin<peripherals::ETH>>,
        sma: Peri<'static, peripherals::ETH_SMA>,
        mdio: Peri<'static, impl MDIOPin<peripherals::ETH_SMA>>,
        mdc: Peri<'static, impl MDCPin<peripherals::ETH_SMA>>,
    ) -> Self {
        static PACKETS: StaticCell<eth::PacketQueue<4, 4>> = StaticCell::new();

        let mut rng = rng::Rng::new(peri_rand, IrqsEth);
        let mut seed = [0; 8];
        unwrap!(rng.fill_bytes(&mut seed).await);
        let seed = u64::from_le_bytes(seed);

        // 06 is a LAA (cannot be taken globally)
        let mac_addr: [u8; 6] = [06, 00, 00, 00, 00, ID];

        let mut device = eth::Ethernet::new(
            PACKETS.init(eth::PacketQueue::<4, 4>::new()),
            peri_eth,
            ref_clk,
            crs,
            rx_d0,
            rx_d1,
            tx_d0,
            tx_d1,
            tx_en,
            mac_addr,
            sma,
            mdio,
            mdc,
            IrqsEth,
        );

        // embassy-stm32's eth v2 driver unconditionally configures the MAC for
        // 100 Mbps full duplex, but 10BASE-T1S (LAN8670) is always 10 Mbps
        // half duplex, so it must be corrected here after construction.
        embassy_stm32::pac::ETH.ethernet_mac().maccr().modify(|w| {
            w.set_fes(false);
            w.set_dm(false);
        });

        // sets node ID
        // TODO: if coordinator (ID == 0) set group count
        if (ID > 1) {
            write_lan8670_vendor_reg(
                device.phy_mut().station_management(),
                0xCA02,
                (ID - 1) as u16,
            );
        } else {
            // must also set node count if coordinator
            write_lan8670_vendor_reg(
                device.phy_mut().station_management(),
                0xCA02,
                ((ID - 1) as u16) | ((MAX_NODES as u16) << 8),
            );
        }
        // turn on PLCA
        write_lan8670_vendor_reg(device.phy_mut().station_management(), 0xCA01, 1 << 15);

        static STACK: StaticCell<StackStorage> = StaticCell::new();
        let (stack, runner) = embassy_net::Stack::new(STACK.init(StackStorage::new()), seed);

        // Add the network interface to the stack.
        static DEVICE: StaticCell<Device> = StaticCell::new();
        let iface = unwrap!(stack.add_iface(DEVICE.init(device)));
        // We run our network on 10.0.0.0/24
        unwrap!(iface.set_ip_addrs([IpCidr::new(
            embassy_net::wire::IpAddress::v4(10, 0, 0, ID),
            24,
        )]));

        // Launch network task
        spawner.spawn(net_task(runner).unwrap());

        NerEth::<ID> { stack }
    }
}
