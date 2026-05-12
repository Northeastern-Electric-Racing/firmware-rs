# MSB-FW.rs
Mechanical Sensor Board Firmware in Rust (experimental, not for usage on car)


### Commands

First, [get rustup](https://www.rust-lang.org/learn/get-started).

Second, clone and open this root folder.

To build `msb-fw-rs`, just run `cargo build --release`.  Note the first time you do this it will take a while, as `rustup` has to install the correct version of rust for this project.

To deploy onto an embedded chip locally connected, run `cargo run --release`.

To build a different project other than `msb-fw-rs` (of which none exist), cd into that directory and run build.

To format, run `cargo format`. 

To lint and check stuff, run `cargo clippy`.

###  On car stuff

To run a RTT terminal dedicated:
`cargo embed --release rtt`

To run a GDB terminal dedicated:
`cargo embed --release gdb`

To flash and leave code:
`cargo embed --release`



### Coding tips and tricks

- Use defmt macros to print stuff
- 



### Repository structure

This is a mega-repo configured as a normal embassy-styled project with multiple dependency crates and sub-projects.

Various files:

Top level `Cargo.toml`, `Embed.toml`, and `rust-toolchain.toml` define the various parts of the embassy project.  See comments inside for how these were structure, but most follow embassy specification.

The `crates` folder defines drivers or other code shared between projects.

Top level folders like `msb-fw-rs`, and any other project, define projects which inherit explicity defined `Cargo.toml` dependencies and `Embed.toml` settings, and more.  They can also depend on a crate in the `crates` folder.

This structure has multiple benefits, including:
- Static versioning of all embassy and other dependencies, eliminating version conflicts for in-tree code
- Inherited build settings so like-microcontroller projects share all of that boilerplate
- Shared `target` folder meaning a shared build cache for quicker and space-saving builds
- Other quirks, such as vscode `settings.json` and `config.toml`, are shared between projects


### Versioning

Updating versions is as follows

1. Update the rust-toolchain to the version found in embassy repo
2. Update all embassy versions to the versions found in the embassy repo, use x.y.z, in main Cargo.toml
3. Update all features, especially ones that say the version in them (ex. "defmt-03")
3. Update major package versions of other projects, use x.y, in main Cargo.toml
4. Update package versions of other projects, in individual Cargo.toml
4. Fix any build issues
