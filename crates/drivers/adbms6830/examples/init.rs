#![no_std]
#![no_main]

use adbms6830::{
    client::{Adbms6830, RX_SIZE, SpiPollAdc, TX_SIZE},
    types::ConfigA,
};

async fn main() {
    const IC_CNT: usize = 1;
    const CNT_TX: usize = IC_CNT * TX_SIZE;
    const CNT_RX: usize = IC_CNT * RX_SIZE;
    let mut tx_buffer = [0u8; 4 + RX_SIZE * IC_CNT];
    let mut rx_buffer = [0u8; RX_SIZE * IC_CNT];
    let client = Adbms6830::<_, _, _, SpiPollAdc<1>, IC_CNT, CNT_RX, CNT_TX>::new(
        device,
        cs_pin,
        SpiPollAdc {},
        delay,
        &mut tx_buffer,
        &mut rx_buffer,
    );

    let a = ConfigA::new();
    a.set_fc(bits);
    client.write::<ConfigA>([].into());
    let r: [ConfigA; 1] = client.read::<ConfigA>().await.unwrap();
}
