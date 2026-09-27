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
//! // Make sure to reset on boot, as this might not be a cold boot
//! let mut phy_reset = Output::new(p.PE10, Level::Low, Speed::Low);
//! phy_reset.set_low();
//! Timer::after_millis(500).await;
//! phy_reset.set_high();
//!
//! // Now we initialize the stack
//!
//! let nereth = nereth::NerEth::<1>::new(
//! _spawner, p.RNG, p.ETH, p.PA1, p.PA7, p.PC4, p.PC5, p.PB12, p.PB15, p.PA5,
//! p.ETH_SMA, p.PA2, p.PC1).await;
//!
//!
//! // Use the returned object to retrieve the data channels for rx/tx and get time
//!
//! ```
//!
//!
//!

use core::{cell::RefCell, str::FromStr};

use defmt::{error, expect, unwrap, warn};
use embassy_executor::Spawner;
use embassy_net::{StackStorage, wire::IpCidr};
use embassy_stm32::{
    Peri, bind_interrupts,
    eth::{
        self, CRSPin, Ethernet, GenericPhy, MDCPin, MDIOPin, RXD0Pin, RXD1Pin, RefClkPin, Sma,
        StationManagement, TXD0Pin, TXD1Pin, TXEnPin,
    },
    mode::Blocking,
    peripherals::{self, ETH_SMA},
    rng,
};
use embassy_sync::blocking_mutex::{Mutex as BlockingMutex, raw::ThreadModeRawMutex};
use micropb::{MessageEncode, PbEncoder, size::max_encoded_size};
use static_cell::StaticCell;
use zenoh_embassy::EmbassyLinkManager;
use zenoh_nostd::session::{
    Endpoint, FixedCapacityGetCallbacks, FixedCapacityQueryableCallbacks,
    FixedCapacitySubCallbacks, Publisher, Resources, Session, TransportLinkManager, ZSessionConfig,
    zenoh::{connect, keyexpr, storage::RawOrBox},
};

mod serverdata {
    #![allow(clippy::all)]
    #![allow(nonstandard_style, unused, irrefutable_let_patterns)]
    // Let's assume that Example is the only message define in the .proto file that has been
    // converted into a Rust struct
    include!(concat!(env!("OUT_DIR"), "/serverdata.rs"));
}

pub type ServerData = serverdata::serverdata_::v2_::ServerData;

const CAPACITY: usize = max_encoded_size::<ServerData>().next_power_of_two();

type Device = Ethernet<'static, peripherals::ETH, GenericPhy<Sma<'static, ETH_SMA>>>;

// max PLCA nodes (coordinator MUST set this number)
const MAX_NODES: u8 = 8;

bind_interrupts!(struct IrqsEth {
    ETH => eth::InterruptHandler<peripherals::ETH>;
});

// getrandom is used by Zenoh crypto libraries, therefore we need to override
// This unofrtunately must not be async, and must be mutexed
static RNG: BlockingMutex<ThreadModeRawMutex, RefCell<Option<rng::Rng<'static, Blocking>>>> =
    BlockingMutex::new(RefCell::new(None));

getrandom::register_custom_getrandom!(getrandom_custom);
fn getrandom_custom(bytes: &mut [u8]) -> Result<(), getrandom::Error> {
    RNG.lock(|rng| {
        rng.borrow_mut()
            .as_mut()
            .expect("RNG not initialized before use")
            .blocking_fill_bytes(bytes);
    });
    Ok(())
}

pub struct NerPublisher<'a> {
    publ: Publisher<'a, 'static, ZenohConfig>,
    enc: PbEncoder<heapless::Vec<u8, CAPACITY>>,
    unit: &'static str,
}

impl<'a> NerPublisher<'a> {
    pub(crate) fn new(publ: Publisher<'a, 'static, ZenohConfig>, unit: &'static str) -> Self {
        let under = heapless::Vec::<u8, CAPACITY>::new();
        NerPublisher {
            publ,
            enc: PbEncoder::new(under),
            unit,
        }
    }

    /// Publishes data, taking in a ServerData object.  Returns true if failure
    /// Only required if you need to override time or unit behavior
    pub async fn send(&mut self, payload: ServerData) -> bool {
        if let Err(e) = payload.encode(&mut self.enc) {
            warn!("Could not serialize protobuf, error {}", e);
            return true;
        }
        self.release_and_clear().await
    }

    /// Publishes data, taking in a single point.  Returns true if failure
    // note: heapless will compile time error if size too big
    pub async fn send_values<const N: usize>(&mut self, value: [f32; N]) -> bool {
        let payload = ServerData {
            unit: heapless::String::from_str(self.unit).expect("Critical parse failure"),
            time_us: 0, // TODO ptp
            values: heapless::Vec::from_array(value),
        };
        self.send(payload).await
    }

    async fn release_and_clear(&mut self) -> bool {
        let res = match self.publ.put(self.enc.as_writer()).finish().await {
            Ok(()) => false,
            Err(e) => {
                warn!(
                    "Failure to publish zenoh value at key {} with error {}",
                    self.publ.keyexpr(),
                    e
                );
                true
            }
        };

        self.enc = PbEncoder::new(heapless::Vec::<u8, CAPACITY>::new());

        res
    }
}

type LinkManager = zenoh_embassy::EmbassyLinkManager<'static, 512, 3>;

pub struct NerEth<const ID: u8> {
    session: &'static Session<'static, ZenohConfig>,
}

pub struct ZenohConfig {
    transports: TransportLinkManager<LinkManager>,
}

const BUFF_SIZE: u16 = 512u16;
impl ZSessionConfig for ZenohConfig {
    type LinkManager = LinkManager;

    type Buff = [u8; BUFF_SIZE as usize];

    type SubCallbacks<'res> = FixedCapacitySubCallbacks<'res, 8, RawOrBox<56>, RawOrBox<600>>;

    type GetCallbacks<'res> = FixedCapacityGetCallbacks<'res, 8, RawOrBox<1>, RawOrBox<32>>;

    type QueryableCallbacks<'res> =
        FixedCapacityQueryableCallbacks<'res, Self, 8, RawOrBox<32>, RawOrBox<952>>;

    fn transports(&self) -> &TransportLinkManager<Self::LinkManager> {
        &self.transports
    }

    fn buff(&self) -> Self::Buff {
        [0u8; BUFF_SIZE as usize]
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn session_task(session: &'static Session<'static, ZenohConfig>) {
    if let Err(e) = session.run().await {
        error!("Error in Zenoh session task: {}", e);
    }
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
        assert!(ID < MAX_NODES, "ID must not be greater than max nodes");
    };

    /// This constructs and initializes the ethernet and network stack.
    /// MAY BLOCK: MDIO calls are synchronous
    #[allow(clippy::too_many_arguments)]
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

        let mut rng = rng::Rng::new_blocking(peri_rand);
        let mut seed = [0; 8];
        rng.blocking_fill_bytes(&mut seed);
        let seed = u64::from_le_bytes(seed);
        RNG.lock(|cell| *cell.borrow_mut() = Some(rng));

        // 06 is a LAA (cannot be taken globally)
        // for private networks this is chill
        let mac_addr: [u8; 6] = [0x6, 0x0, 0x0, 0x0, 0x0, ID];

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
        // TODO: upstream
        embassy_stm32::pac::ETH.ethernet_mac().maccr().modify(|w| {
            w.set_fes(false);
            w.set_dm(false);
        });

        // sets node ID
        if ID > 0 {
            write_lan8670_vendor_reg(
                device.phy_mut().station_management(),
                0xCA02,
                (ID - 1) as u16,
            );
        } else {
            // must also set node count if coordinator
            // TODO: test
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

        // ZENOH ---------------------------------------------------------------
        let ex = ZenohConfig {
            transports: TransportLinkManager::from(EmbassyLinkManager::new(stack)),
        };

        static RESOURCES: static_cell::StaticCell<Resources<'static, ZenohConfig>> =
            static_cell::StaticCell::new();
        static CONFIG: static_cell::StaticCell<ZenohConfig> = static_cell::StaticCell::new();
        let config = CONFIG.init(ex);
        let resources = RESOURCES.init(Resources::default());

        // For now we UDP client all to TPU, making TPU required
        // this also makes inter-firmware comms NON-CRITICAL over Zenoh
        // TODO: peer connectivity or appoint critical node host (ex. VCU)
        let endpoint = Endpoint::try_from("udp/10.0.0.1:7447").unwrap();

        static SESSION: static_cell::StaticCell<Session<'static, ZenohConfig>> =
            static_cell::StaticCell::new();
        let session: &'static Session<'static, ZenohConfig> =
            SESSION.init(connect(resources, config, endpoint).await.unwrap());

        spawner.spawn(session_task(session).unwrap());

        NerEth::<ID> { session }
    }

    /// Retreives a publisher to send data over a topic.
    pub async fn get_publisher<'a>(
        &self,
        key: &'a str,
        unit: Option<&'static str>,
    ) -> Option<NerPublisher<'a>> {
        // TODO: make compile time
        let test = ServerData::default();
        let unit = unit.unwrap_or("");
        assert!(unit.len() < test.unit.capacity(), "Unit too big!");

        match self
            .session
            .declare_publisher(expect!(keyexpr::new(key), "Invalid key expression"))
            .finish()
            .await
        {
            Ok(res) => Some(NerPublisher::new(res, unit)),
            Err(e) => {
                warn!("Could not create publisher: {}", e);
                None
            }
        }
    }
}
