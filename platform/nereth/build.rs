fn main() {
    println!("cargo::rerun-if-changed=proto");

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
        )
        .add_protoc_arg("-Iproto");
    // Compile example.proto into a Rust module
    generator
        .compile_protos(
            &["serverdata.proto"],
            std::env::var("OUT_DIR").unwrap() + "/serverdata.rs",
        )
        .unwrap();
}
