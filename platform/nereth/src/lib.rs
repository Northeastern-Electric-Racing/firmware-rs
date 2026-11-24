#![no_std]
//! NER Ethernet handler.
//!
//! Takes the STM32 Ethernet peripheral and pins and brings up the whole stack:
//!
//! - **PHY:** a Microchip LAN8670 (10BASE-T1S) configured for PLCA.
//! - **Network:** `embassy-net` with a static IPv4 address on `10.0.0.0/24`.
//! - **Messaging:** a Zenoh client session over UDP to the TPU at `10.0.0.1:7447`.
//!   Payloads are protobuf `ServerData` messages encoded with `micropb`.
//!
//! Timestamps are not set yet (`time_us` is always 0). PTP time sync is planned.
//!
//! # Node ID
//!
//! The const generic `ID` sets every per-node identifier:
//!
//! | Identifier | Value                                      |
//! |------------|--------------------------------------------|
//! | IPv4       | `10.0.0.ID`                                |
//! | MAC        | `06:00:00:00:00:ID` (locally administered) |
//! | PLCA node  | `ID - 1`                                   |
//!
//! `ID` must be between 1 and 8 (the PLCA node count). ID 1 is PLCA node 0,
//! the coordinator.
//!
//! # Usage
//!
//! 1. Reset the PHY. This crate does not do it, and the board may be warm
//!    booting with the PHY in an unknown state.
//! 2. Call [`NerEth::new`] once. It spawns the network and Zenoh session tasks.
//! 3. Declare the node's topics with [`topics!`] and create their publishers
//!    with `Topic::publishers`.
//! 4. Publish from a single task that owns the publishers, and feed it from the
//!    rest of the firmware through a channel.
//!
//! ```ignore
//! use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
//! use nereth::{NerEth, topics};
//!
//! topics! {
//!     Acceleration => "VCU_Ethernet/A/Acceleration", "mg";
//!     Gyro         => "VCU_Ethernet/A/Gyro",         "mdps";
//! }
//!
//! // Other tasks drop packets in here. `try_send` never blocks, so a dead or
//! // missing Ethernet link can't stall them; packets are just dropped.
//! pub static OUTGOING: Channel<CriticalSectionRawMutex, Packet<3>, 8> = Channel::new();
//!
//! pub fn try_publish(topic: Topic, values: [f32; 3]) -> bool {
//!     OUTGOING.try_send(Packet { topic, values }).is_ok()
//! }
//!
//! // `r` is the board's struct of Ethernet peripherals and pins.
//! #[embassy_executor::task]
//! async fn ethernet_task(spawner: Spawner, r: EthernetResources) {
//!     // Hold the PHY in reset for 500 ms.
//!     let mut phy_reset = Output::new(r.phy_reset, Level::Low, Speed::Low);
//!     Timer::after_millis(500).await;
//!     phy_reset.set_high();
//!
//!     let Ok(eth) = NerEth::<4>::new(
//!         spawner, r.rng, r.eth, r.ref_clk, r.crs, r.rx_d0, r.rx_d1,
//!         r.tx_d0, r.tx_d1, r.tx_en, r.sma, r.mdio, r.mdc,
//!     )
//!     .await
//!     else {
//!         defmt::error!("Ethernet init failed; running without Ethernet");
//!         return;
//!     };
//!
//!     // One publisher per topic, in declaration order.
//!     let mut publishers = Topic::publishers(&eth).await;
//!
//!     loop {
//!         let packet = OUTGOING.receive().await;
//!         // Failures are logged inside `send_values`.
//!         let _ = publishers[packet.topic as usize]
//!             .send_values(packet.values)
//!             .await;
//!     }
//! }
//! ```

use core::{cell::RefCell, str::FromStr};

use defmt::{error, warn};
use embassy_executor::Spawner;
use embassy_net::{StackStorage, StaticPool, wire::IpCidr};
use embassy_stm32::eth::{self, CRSPin, Ethernet, GenericPhy};
use embassy_stm32::eth::{MDCPin, MDIOPin, RXD0Pin, RXD1Pin, RefClkPin};
use embassy_stm32::eth::{Sma, StationManagement, TXD0Pin, TXD1Pin, TXEnPin};
use embassy_stm32::mode::Blocking;
use embassy_stm32::peripherals;
use embassy_stm32::{Peri, bind_interrupts, rng};
use embassy_sync::blocking_mutex::{Mutex as BlockingMutex, raw::ThreadModeRawMutex};
use embassy_sync::channel::{DynamicReceiver, DynamicSender};
use micropb::{MessageEncode, PbEncoder, size::max_encoded_size};
use static_cell::StaticCell;
use zenoh_embassy::EmbassyLinkManager;
use zenoh_nostd::session::zenoh::keyexpr;
use zenoh_nostd::session::zenoh::storage::RawOrBox;
use zenoh_nostd::session::*;

mod serverdata {
    #![allow(clippy::all)]
    #![allow(nonstandard_style, unused, irrefutable_let_patterns)]
    // Let's assume that Example is the only message define in the .proto file that has been
    // converted into a Rust struct
    include!(concat!(env!("OUT_DIR"), "/serverdata.rs"));
}

pub type ServerData = serverdata::serverdata_::v2_::ServerData;

const CAPACITY: usize = max_encoded_size::<ServerData>().next_power_of_two();

type Device = Ethernet<'static, peripherals::ETH, GenericPhy<Sma<'static, peripherals::ETH_SMA>>>;

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

pub struct NerSubscriber<'a> {
    subscriber: Subscriber<'a, 'static, ZenohConfig, FixedCapacitySample<128, 128>, true>,
    last: Option<FixedCapacitySample<128, 128>>,
}

pub struct NerMessage<'a> {
    pub keyexpr: &'a str,
    pub payload: &'a [u8],
}

impl<'a> NerSubscriber<'a> {
    pub(crate) fn new(
        subscriber: Subscriber<'a, 'static, ZenohConfig, FixedCapacitySample<128, 128>, true>,
    ) -> Self {
        NerSubscriber { subscriber, last: None }
    }

    /// Receives a message. Returns None on failure.
    pub async fn receive(&mut self) -> Option<NerMessage<'_>> {
        let sample = self.subscriber.recv().await?;
        let sample = self.last.insert(sample);
        Some(NerMessage {
            keyexpr: sample.keyexpr(),
            payload: sample.payload(),
        })
    }
}

type LinkManager = EmbassyLinkManager<'static, 512, 3>;

pub struct NerEth<const ID: u8> {
    session: &'static Session<'static, ZenohConfig>,
}

pub struct ZenohConfig {
    transports: TransportLinkManager<LinkManager>,
}

const BUFF_SIZE: u16 = 512u16;
impl ZSessionConfig for ZenohConfig {
    type Buff = [u8; BUFF_SIZE as usize];

    type LinkManager = LinkManager;

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
    let _ = sm.smi_write(0, 0x0D, 0x1F);

    // write address
    let _ = sm.smi_write(0, 0x0E, reg);

    // write normal data, keep vendor specific location
    let _ = sm.smi_write(0, 0x0D, 0x1F | 1 << 14);

    // write payload
    let _ = sm.smi_write(0, 0x0E, val);
}

/// Defines every Ethernet topic in one place.
/// Generates the `Topic` enum and a matching (key, unit) table, in the same order.
#[macro_export]
macro_rules! topics {
    ($($name:ident => $key:literal, $unit:literal;)*) => {
        #[derive(Clone, Copy)]
        pub enum Topic {
            $($name,)*
        }

        impl Topic {
            pub const COUNT: usize = [$(stringify!($name)),*].len();

            async fn publishers<const ID: u8>(eth: &$crate::NerEth<ID>)
                -> Option<[$crate::NerPublisher<'static>; Self::COUNT]> {
                Some([$(
                    eth.get_publisher($key, Some($unit)).await?,
                )*])
            }
        }

        pub struct Packet<const SIZE: usize> {
            pub topic: Topic,
            pub values: [f32; SIZE],
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum NerEthError {
    EthernetInitError,
    DeviceInitError,
    SetIpError,
    ConnectionError,
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
    ) -> Result<Self, NerEthError> {
        static PACKETS: StaticCell<eth::PacketQueue<8, 8>> = StaticCell::new();

        let mut rng = rng::Rng::new_blocking(peri_rand);
        let mut seed = [0; 8];
        rng.blocking_fill_bytes(&mut seed);
        let seed = u64::from_le_bytes(seed);
        RNG.lock(|cell| *cell.borrow_mut() = Some(rng));

        // 06 is a LAA (cannot be taken globally)
        // for private networks this is chill
        let mac_addr: [u8; 6] = [0x6, 0x0, 0x0, 0x0, 0x0, ID];

        let mut device = match Ethernet::new(
            PACKETS.init(eth::PacketQueue::<8, 8>::new()),
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
        ) {
            Ok(o) => o,
            Err(e) => {
                error!("Could not initialize ethernet! {}", e);
                return Err(NerEthError::EthernetInitError);
            }
        };

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
        static POOL: StaticPool<1520, 16, 8> = StaticPool::new();
        let (stack, runner) = embassy_net::Stack::new(STACK.init(StackStorage::new()), &POOL, seed);

        // Add the network interface to the stack.
        static DEVICE: StaticCell<Device> = StaticCell::new();
        let Ok(iface) = stack.add_iface_borrowed(DEVICE.init(device)) else {
            return Err(NerEthError::DeviceInitError);
        };
        // We run our network on 10.0.0.0/24

        if iface
            .set_ip_addrs([IpCidr::new(embassy_net::wire::IpAddr::v4(10, 0, 0, ID), 24)])
            .is_err()
        {
            return Err(NerEthError::SetIpError);
        }

        // Launch network task
        spawner.spawn(net_task(runner).expect("Failed to spawn embassy_net::net_task()."));

        // ZENOH ---------------------------------------------------------------
        let ex = ZenohConfig {
            transports: TransportLinkManager::from(EmbassyLinkManager::new(stack)),
        };
        // For now we UDP client all to TPU, making TPU required
        // this also makes inter-firmware comms NON-CRITICAL over Zenoh
        // TODO: peer connectivity or appoint critical node host (ex. VCU)
        let endpoint = Endpoint::try_from("udp/10.0.0.1:7447")
            .expect("Failed to parse Zenoh endpoint address");

        // static SESSION: static_cell::StaticCell<Session<'static, ZenohConfig>> =
        //     static_cell::StaticCell::new();
        // let session: &'static Session<'static, ZenohConfig> =
        //     SESSION.init(connect(resources, config, endpoint).await.unwrap());

        let builder = zenoh::session!(static ZenohConfig: ex);
        let Ok(session) = builder.connect(endpoint).await else {
            return Err(NerEthError::ConnectionError);
        };

        spawner.spawn(session_task(session).expect("Failed to spawn nereth::session_task()."));

        Ok(NerEth::<ID> { session })
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

        let Ok(keyexpr) = keyexpr::new(key) else {
            warn!("Could not parse keyexpr: {}", key);
            return None;
        };

        let Ok(publisher) = self.session.declare_publisher(keyexpr).finish().await else {
            warn!("Could not create publisher: {}", key);
            return None;
        };

        Some(NerPublisher::new(publisher, unit))
    }

    pub async fn get_subscriber<'a>(
        &'a self,
        key: &'static str,
        sender: DynamicSender<'static, FixedCapacitySample<128, 128>>,
        receiver: DynamicReceiver<'static, FixedCapacitySample<128, 128>>,
    ) -> Option<NerSubscriber<'a>> {
        let Ok(keyexpr) = keyexpr::new(key) else {
            warn!("Could not parse keyexpr: {}", key);
            return None;
        };

        let Ok(subscriber) = self
            .session
            .declare_subscriber(keyexpr)
            .channel(sender, receiver)
            .finish()
            .await
        else {
            warn!("Could not create subscriber: {}", key);
            return None;
        };

        Some(NerSubscriber::new(subscriber))
    }
}
