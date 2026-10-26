use std::{env, fs, path::PathBuf};

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let memory = out.join("memory.x");
    fs::write(&memory, include_bytes!("memory.x")).unwrap();
    println!("cargo::metadata=memory_x={}", memory.display());
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
}
