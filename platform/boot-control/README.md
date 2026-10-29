# boot-control

Application-side support for the Atlas CAN bootloader. Boot-control confirms
application startup and handles CAN boot-entry requests. CAN initialization,
receive filters, and frame dispatch stay with the application.

## Application integration

Add `boot-control` as a workspace dependency:

```toml
# Workspace Cargo.toml
[workspace.dependencies]
boot-control = { git = "https://github.com/Northeastern-Electric-Racing/firmware-rs.git", rev = "<rev>" }
```

```toml
# Project Cargo.toml
[dependencies]
boot-control.workspace = true
```

Create a `boot-control` instance using the flash peripheral, confirm startup, and
pass it to the CAN task:

```rust
let mut bootloader = boot_control::Bootloader::new(p.FLASH);
// Confirm basic startup to prevent rollback on the next reset.
// Device initialization continues in application tasks.
bootloader
    .confirm_boot()
    .expect("Failed to confirm application startup");
can::start(spawner, can_resources, bootloader);
```

Calling `confirm_boot()` keeps the new firmware from being rolled back on the next
reset. Call it once the application's required startup checks have succeeded.

Add the ECU's boot-request ID to the CAN receive filter. In the receive task, use
the generated parser to identify the request and pass its data to `boot-control`:

```rust
match Msg::decode(&frame) {
    Some(Msg::<Ecu>BootRequest(_)) => bootloader
        .handle_request(frame.data())
        .unwrap_or_else(|_| defmt::error!("Failed to persist bootloader request")),
    _ => {}
}
```

Replace `<Ecu>BootRequest` with the cangen-generated boot-request message for the
application's ECU. Boot-control validates the boot-request data, saves the
boot-entry request, and resets only after a successful state write. Invalid data is
ignored. Perform any application-specific shutdown before calling `handle_request()`.

## Memory layout

`boot-control`'s `memory.x` divides flash into these regions:

- `BOOTLOADER`: the bootloader.
- `FLASH`: the running application.
- `DFU`: new firmware staged by the bootloader until it is swapped into `FLASH`.
- `BOOTLOADER_STATE`: boot and update flags shared by the bootloader and the application.

Applications must link with the layout provided by `boot-control`, which matches the
bootloader. `boot-control` exports its `memory.x` path through Cargo build metadata as
`DEP_BOOT_CONTROL_MEMORY_X`. Copy it in the application's `build.rs`:

```rust
let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
let memory = env::var_os("DEP_BOOT_CONTROL_MEMORY_X")
    .expect("boot-control must provide the application memory layout");
fs::copy(memory, out.join("memory.x")).unwrap();
println!("cargo:rustc-link-search={}", out.display());
```
