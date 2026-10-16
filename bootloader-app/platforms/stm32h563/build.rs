use std::{env, path::PathBuf};

fn main() {
    let memory = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("memory.x");
    println!("cargo::rerun-if-changed=memory.x");
    // Direct dependents receive this as DEP_STM32H563_BOOTLOADER_APP_MEMORY_X.
    println!("cargo::metadata=memory_x={}", memory.display());
}
