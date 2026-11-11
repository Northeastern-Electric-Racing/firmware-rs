use std::{env, path::PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=proto");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Replaces protoc: compile the .proto in pure Rust (same as -Iproto)
    let fds = protox::Compiler::new(["proto"])
        .unwrap()
        .include_imports(true)
        .open_files(["serverdata.proto"])
        .unwrap()
        .encode_file_descriptor_set();

    let fds_path = out_dir.join("serverdata.fds");
    std::fs::write(&fds_path, fds).unwrap();

    let mut generator = micropb_gen::Generator::new();
    generator
        .use_container_heapless()
        .configure(
            ".serverdata.v2.ServerData.values",
            micropb_gen::Config::new().max_len(4),
        )
        .configure(
            ".serverdata.v2.ServerData.unit",
            micropb_gen::Config::new().max_bytes(18),
        );

    generator
        .compile_fdset_file(&fds_path, out_dir.join("serverdata.rs"))
        .unwrap();
}