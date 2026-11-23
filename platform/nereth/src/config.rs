use crate::ZenohConfig;
use atomic_float::AtomicF32;
use core::str::from_utf8;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use core::time::Duration;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::channel::Channel;
use phf::phf_map;
use static_cell::StaticCell;
use zenoh_nostd::session::zenoh::{error, info, keyexpr};
use zenoh_nostd::session::*;

// Each param starts at its default and is overwritten when zenoh sends a value
pub static MAX_TORQ: IntData = IntData::new(0);
pub static MAX_TEMP: FloatData = FloatData::new(0.0);
pub static SHUTDOWN_ENABLED: BoolData = BoolData::new(true);

/// Zenoh key -> param, built at compile time, lookups are constant time
/// Adding a param: declare the static above + add one line here
static PARAMS: phf::Map<&'static str, &'static (dyn Param + Sync)> = phf_map! {
    "NERData/max_torq" => &MAX_TORQ,
    "NERData/max_temp" => &MAX_TEMP,
    "NERData/shutdown_enabled" => &SHUTDOWN_ENABLED,
};

/// Every param key must live under this prefix so one query + one subscriber covers them all
pub const PATH: &str = "NERData/**";

/// Shared behaviour so query replies and subscriber samples can update any param type
trait Param {
    /// Parse the payload and store it, returns false if the payload couldn't be parsed
    fn update(&self, payload: &[u8]) -> bool;
}

fn payload_str(payload: &[u8]) -> Option<&str> {
    from_utf8(payload).ok().map(str::trim)
}

pub struct IntData {
    value: AtomicI32,
}
impl IntData {
    const fn new(default: i32) -> Self {
        Self {
            value: AtomicI32::new(default),
        }
    }
    pub fn get(&self) -> i32 {
        self.value.load(Ordering::Relaxed)
    }
}
impl Param for IntData {
    fn update(&self, payload: &[u8]) -> bool {
        match payload_str(payload).and_then(|s| s.parse::<i32>().ok()) {
            Some(v) => {
                self.value.store(v, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }
}

pub struct FloatData {
    value: AtomicF32,
}
impl FloatData {
    const fn new(default: f32) -> Self {
        Self {
            value: AtomicF32::new(default),
        }
    }
    pub fn get(&self) -> f32 {
        self.value.load(Ordering::Relaxed)
    }
}
impl Param for FloatData {
    fn update(&self, payload: &[u8]) -> bool {
        match payload_str(payload).and_then(|s| s.parse::<f32>().ok()) {
            Some(v) => {
                self.value.store(v, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }
}

pub struct BoolData {
    value: AtomicBool,
}
impl BoolData {
    const fn new(default: bool) -> Self {
        Self {
            value: AtomicBool::new(default),
        }
    }
    pub fn get(&self) -> bool {
        self.value.load(Ordering::Relaxed)
    }
}
impl Param for BoolData {
    fn update(&self, payload: &[u8]) -> bool {
        let parsed = match payload_str(payload) {
            Some("true") | Some("1") => Some(true),
            Some("false") | Some("0") => Some(false),
            _ => None,
        };
        match parsed {
            Some(v) => {
                self.value.store(v, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }
}

/// Route a key and payload pair from zenoh to the matching param
fn handle_update(key: &str, payload: &[u8]) {
    match PARAMS.get(key) {
        Some(p) => {
            if !p.update(payload) {
                error!(
                    "[Config] Couldn't parse value for '{}': {:?}",
                    key,
                    from_utf8(payload)
                );
            }
        }
        None => info!("[Config] Ignoring unknown key '{}'", key),
    }
}

/// Callback for the startup query
fn on_query_reply(resp: &GetResponse<'_>) {
    match resp {
        GetResponse::Ok(reply) => handle_update(reply.keyexpr().as_str(), reply.payload()),
        GetResponse::Err(reply) => error!(
            "[Config] Query error from '{}': {:?}",
            reply.keyexpr().as_str(),
            from_utf8(reply.payload())
        ),
    }
}

/// Queries every param once, then keeps them updated from a subscriber
#[embassy_executor::task]
pub async fn config_task(session: &'static Session<'static, ZenohConfig>) {
    static CHANNEL: StaticCell<Channel<NoopRawMutex, FixedCapacitySample<128, 128>, 8>> =
        StaticCell::new();
    let channel: &'static _ = CHANNEL.init(Channel::new());

    let ke = match keyexpr::new(PATH) {
        Ok(ke) => ke,
        Err(e) => {
            error!("[Config] Invalid key expression '{}': {}", PATH, e);
            return;
        }
    };

    // Subscribe first so no update published during the query gets missed
    let subscriber = match session
        .declare_subscriber(ke)
        .channel(channel.dyn_sender(), channel.dyn_receiver())
        .finish()
        .await
    {
        Ok(sub) => sub,
        Err(e) => {
            error!("[Config] Couldn't declare subscriber: {}", e);
            return;
        }
    };

    // Startup query for whatever values already exist
    match session
        .declare_querier(ke)
        .timeout(Duration::from_secs(1))
        .finish()
        .await
    {
        Ok(querier) => {
            if let Err(e) = querier.get().callback_sync(on_query_reply).finish().await {
                error!("[Params] Startup query failed, using defaults: {}", e);
            }
        }
        Err(e) => error!("[Params] Couldn't declare querier, using defaults: {}", e),
    }

    // Live updates
    while let Some(sample) = subscriber.recv().await {
        handle_update(sample.keyexpr().as_str(), sample.payload());
    }
}
