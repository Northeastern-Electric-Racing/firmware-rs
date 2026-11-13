//! Check application vectors before transferring execution to active flash.

unsafe extern "C" {
    // Absolute linker symbols carry values in their addresses, not stored data.
    static __bootloader_active_address: u8;
    static __bootloader_active_size: u8;
    static __bootloader_sram_start: u8;
    static __bootloader_sram_end: u8;
}

/// Start address of the active partition, where the application's vector table lives.
pub fn active_address() -> u32 {
    core::ptr::addr_of!(__bootloader_active_address) as u32
}

/// Size of the active partition, which bounds the image size the host may send.
pub fn active_size() -> u32 {
    core::ptr::addr_of!(__bootloader_active_size) as u32
}

fn sram_start() -> u32 {
    core::ptr::addr_of!(__bootloader_sram_start) as u32
}

fn sram_end() -> u32 {
    core::ptr::addr_of!(__bootloader_sram_end) as u32
}

/// Read the active image after boot preparation has completed any swap.
pub fn active_valid() -> bool {
    // The linker places the active image in readable flash, separate from DFU writes.
    let vector = unsafe {
        let sp = core::ptr::read_volatile(active_address() as *const u32);
        let reset = core::ptr::read_volatile((active_address() + 4) as *const u32);
        let mut vector = [0; 8];
        vector[..4].copy_from_slice(&sp.to_le_bytes());
        vector[4..].copy_from_slice(&reset.to_le_bytes());
        vector
    };
    vector_valid(&vector)
}

/// Check stack and Thumb entry addresses; this does not verify image integrity.
pub fn vector_valid(vector: &[u8; 8]) -> bool {
    let sp = u32::from_le_bytes([vector[0], vector[1], vector[2], vector[3]]);
    let reset = u32::from_le_bytes([vector[4], vector[5], vector[6], vector[7]]);
    // Bit zero selects Thumb execution and is not part of the code address.
    let reset_addr = reset & !1;

    // A descending stack may start one byte past the last SRAM address.
    (sram_start()..=sram_end()).contains(&sp)
        && (reset & 1) != 0
        && (active_address()..active_address() + active_size()).contains(&reset_addr)
}
