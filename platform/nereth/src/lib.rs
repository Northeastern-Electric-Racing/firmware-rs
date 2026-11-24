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
//! 3. Declare the node's outgoing topics with [`publishers!`] and create them
//!    with `Publishers::topics`.
//! 4. Declare the node's incoming topics with [`subscribers!`] and create them
//!    with `Subscribers::topics`.
//! 5. Publish from a single task that owns the publishers, and feed it from the
//!    rest of the firmware through a channel. Received messages arrive through
//!    a second channel, decoded and tagged with their `SubTopic`.
//!
//! ```ignore
//! use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
//! use nereth::{NerEth, publishers, subscribers};
//!
//! publishers! {
//!     Acceleration => "VCU_Ethernet/A/Acceleration", "mg";
//!     Gyro         => "VCU_Ethernet/A/Gyro",         "mdps";
//! }
//!
//! subscribers! {
//!     Throttle => "TPU/Throttle";
//!     Mode     => "TPU/Mode";
//! }
//!
//! // Other tasks drop packets in here. `try_send` never blocks, so a dead or
//! // missing Ethernet link can't stall them; packets are just dropped.
//! pub static OUTGOING: Channel<CriticalSectionRawMutex, Packet<3>, 8> = Channel::new();
//!
//! // Received messages land here. If nobody drains it, new messages are
//! // dropped (and logged) rather than stalling the Zenoh session.
//! pub static INCOMING: Channel<CriticalSectionRawMutex, Incoming, 8> = Channel::new();
//!
//! pub fn try_publish(topic: PubTopic, values: [f32; 3]) -> bool {
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
//!     let Some(mut publishers) = Publishers::topics(&eth).await else {
//!         defmt::error!("Could not create publishers");
//!         return;
//!     };
//!
//!     // Bind to a name (not `_`) so the subscribers live as long as this task.
//!     let Some(_subscribers) = Subscribers::topics(&eth, INCOMING.dyn_sender()).await else {
//!         defmt::error!("Could not create subscribers");
//!         return;
//!     };
//!
//!     loop {
//!         let packet = OUTGOING.receive().await;
//!         // Failures are logged inside `send_values`.
//!         let _ = publishers.send_values(packet.topic, packet.values).await;
//!     }
//! }
//!
//! #[embassy_executor::task]
//! async fn incoming_task() {
//!     loop {
//!         let msg = INCOMING.receive().await;
//!         match msg.topic {
//!             SubTopic::Throttle => { /* use msg.data.values */ }
//!             SubTopic::Mode => { /* ... */ }
//!         }
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
use embassy_sync::channel::DynamicSender;
use heapless::{String, Vec};
use micropb::{MessageDecode, MessageEncode, PbDecoder, PbEncoder, size::max_encoded_size};
use static_cell::StaticCell;
use zenoh_embassy::EmbassyLinkManager;
use zenoh_nostd::session::zenoh::keyexpr;
use zenoh_nostd::session::zenoh::storage::RawOrBox;
use zenoh_nostd::session::*;

// Re-exported so the `subscribers!` macro can name `DynamicSender` from
// crates that don't depend on embassy-sync directly.
#[doc(hidden)]
pub use embassy_sync;

mod serverdata {
    #![allow(clippy::all)]
    #![allow(nonstandard_style, unused, irrefutable_let_patterns)]
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
// This unfortunately must not be async, and must be mutexed
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

// PUBLISHING -----------------------------------------------------------------

pub struct NerPublisher<'a> {
    publ: Publisher<'a, 'static, ZenohConfig>,
    enc: PbEncoder<Vec<u8, CAPACITY>>,
    unit: &'static str,
}

impl<'a> NerPublisher<'a> {
    pub(crate) fn new(publ: Publisher<'a, 'static, ZenohConfig>, unit: &'static str) -> Self {
        NerPublisher {
            publ,
            enc: PbEncoder::new(Vec::<u8, CAPACITY>::new()),
            unit,
        }
    }

    /// Publishes data, taking in a ServerData object. Returns true on failure.
    /// Only required if you need to override time or unit behavior.
    pub async fn send(&mut self, payload: ServerData) -> bool {
        if let Err(e) = payload.encode(&mut self.enc) {
            warn!("Could not serialize protobuf, error {}", e);
            return true;
        }
        self.release_and_clear().await
    }

    /// Publishes data, taking in a single point. Returns true on failure.
    // note: heapless will compile time error if size too big
    pub async fn send_values<const N: usize>(&mut self, value: [f32; N]) -> bool {
        let payload = ServerData {
            unit: String::from_str(self.unit).expect("Critical parse failure"),
            time_us: 0, // TODO ptp
            values: Vec::from_array(value),
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

        self.enc = PbEncoder::new(Vec::<u8, CAPACITY>::new());

        res
    }
}

// SUBSCRIBING ----------------------------------------------------------------

/// Keeps a Zenoh subscription alive. Received messages are delivered through
/// the channel passed to [`NerEth::get_subscriber`], not through this handle.
pub struct NerSubscriber<'a> {
    #[allow(dead_code)]
    sub: Subscriber<'a, 'static, ZenohConfig>,
}

impl<'a> NerSubscriber<'a> {
    pub(crate) fn new(sub: Subscriber<'a, 'static, ZenohConfig>) -> Self {
        NerSubscriber { sub }
    }
}

/// A received message, decoded and tagged with the topic it arrived on.
pub struct NerMessage<T> {
    pub topic: T,
    pub data: ServerData,
}

// ZENOH CONFIG ---------------------------------------------------------------

type LinkManager = EmbassyLinkManager<'static, 512, 3>;

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

// TASKS ----------------------------------------------------------------------

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

// PHY ------------------------------------------------------------------------

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

// MACROS ---------------------------------------------------------------------

/// Defines every outgoing Ethernet topic in one place.
///
/// Generates:
/// - `PubTopic`, an enum with one variant per topic, in declaration order.
/// - `Publishers`, which owns one publisher per topic. Create it with
///   `Publishers::topics(&eth)` and publish with `send_values`.
/// - `Packet`, a topic plus values, for feeding the publishing task.
#[macro_export]
macro_rules! publishers {
    ($($name:ident => $key:literal, $unit:literal;)*) => {
        #[derive(Clone, Copy)]
        pub enum PubTopic {
            $($name,)*
        }

        impl PubTopic {
            pub const COUNT: usize = [$(stringify!($name)),*].len();
        }

        pub struct Publishers([$crate::NerPublisher<'static>; PubTopic::COUNT]);

        impl Publishers {
            /// Declares a publisher for every topic. Returns None if any fails.
            pub async fn topics<const ID: u8>(eth: &$crate::NerEth<ID>) -> Option<Self> {
                Some(Publishers([$(
                    eth.get_publisher($key, Some($unit)).await?,
                )*]))
            }

            /// The publisher for one topic, for when you need `send`.
            pub fn get(&mut self, topic: PubTopic) -> &mut $crate::NerPublisher<'static> {
                &mut self.0[topic as usize]
            }

            /// Publishes values on a topic. Returns true on failure.
            pub async fn send_values<const N: usize>(
                &mut self,
                topic: PubTopic,
                values: [f32; N],
            ) -> bool {
                self.get(topic).send_values(values).await
            }
        }

        pub struct Packet<const SIZE: usize> {
            pub topic: PubTopic,
            pub values: [f32; SIZE],
        }
    };
}

/// Defines every incoming Ethernet topic in one place.
///
/// Generates:
/// - `SubTopic`, an enum with one variant per topic, in declaration order.
/// - `Subscribers`, which keeps every subscription alive. Create it with
///   `Subscribers::topics(&eth, sender)`; messages arrive through `sender`.
/// - `Incoming`, the received message type: a `SubTopic` plus decoded data.
#[macro_export]
macro_rules! subscribers {
    ($($name:ident => $key:literal;)*) => {
        #[derive(Clone, Copy)]
        pub enum SubTopic {
            $($name,)*
        }

        impl SubTopic {
            pub const COUNT: usize = [$(stringify!($name)),*].len();
        }

        pub type Incoming = $crate::NerMessage<SubTopic>;

        /// Dropping this ends the subscriptions, so keep it alive.
        pub struct Subscribers(
            #[allow(dead_code)] [$crate::NerSubscriber<'static>; SubTopic::COUNT],
        );

        impl Subscribers {
            /// Subscribes to every topic. Returns None if any fails.
            pub async fn topics<const ID: u8>(
                eth: &$crate::NerEth<ID>,
                sender: $crate::embassy_sync::channel::DynamicSender<'static, Incoming>,
            ) -> Option<Self> {
                Some(Subscribers([$(
                    eth.get_subscriber($key, SubTopic::$name, sender).await?,
                )*]))
            }
        }
    };
}

// NER ETH --------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum NerEthError {
    EthernetInitError,
    DeviceInitError,
    SetIpError,
    ConnectionError,
}

pub struct NerEth<const ID: u8> {
    session: &'static Session<'static, ZenohConfig>,
}

impl<const ID: u8> NerEth<ID> {
    const _ASSERT_VALID_ID: () = {
        assert!(ID >= 1, "ID must not be zero!");
        assert!(ID <= MAX_NODES, "ID must not be greater than max nodes");
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
        // Referencing the const forces the ID check to run at compile time.
        #[allow(clippy::let_unit_value)]
        let () = Self::_ASSERT_VALID_ID;

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

        // PLCA_CTRL1: bits 7:0 are the node ID, bits 15:8 the node count.
        // Only the coordinator (node 0, i.e. ID 1) sets the node count.
        // TODO: test coordinator
        let node_id = (ID - 1) as u16;
        let plca_ctrl1 = if ID == 1 {
            node_id | ((MAX_NODES as u16) << 8)
        } else {
            node_id
        };
        write_lan8670_vendor_reg(device.phy_mut().station_management(), 0xCA02, plca_ctrl1);

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

        let builder = zenoh::session!(static ZenohConfig: ex);
        let Ok(session) = builder.connect(endpoint).await else {
            return Err(NerEthError::ConnectionError);
        };

        spawner.spawn(session_task(session).expect("Failed to spawn nereth::session_task()."));

        Ok(NerEth::<ID> { session })
    }

    /// Retrieves a publisher to send data over a topic.
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

    /// Subscribes to a topic. Each received message is decoded as `ServerData`,
    /// tagged with `topic`, and sent into `sender`. If the channel is full or
    /// the payload doesn't decode, the message is dropped and a warning logged.
    ///
    /// The returned [`NerSubscriber`] must be kept alive for as long as the
    /// subscription should stay active.
    pub async fn get_subscriber<T: Copy + 'static>(
        &self,
        key: &'static str,
        topic: T,
        sender: DynamicSender<'static, NerMessage<T>>,
    ) -> Option<NerSubscriber<'static>> {
        let Ok(keyexpr) = keyexpr::new(key) else {
            warn!("Could not parse keyexpr: {}", key);
            return None;
        };

        // Runs inline on the session's receive path, so it must not block.
        // Captures: sender (2 words) + key (2 words) + topic, which fits in the
        // RawOrBox<56> callback storage configured in ZenohConfig.
        let Ok(sub) = self
            .session
            .declare_subscriber(keyexpr)
            .callback_sync(move |sample| {
                let payload = sample.payload();
                let mut data = ServerData::default();
                if data
                    .decode(&mut PbDecoder::new(payload), payload.len())
                    .is_err()
                {
                    warn!("Could not decode protobuf on {}", key);
                    return;
                }
                if sender.try_send(NerMessage { topic, data }).is_err() {
                    warn!("Subscriber channel full, dropping message on {}", key);
                }
            })
            .finish()
            .await
        else {
            warn!("Could not create subscriber: {}", key);
            return None;
        };

        Some(NerSubscriber::new(sub))
    }
}