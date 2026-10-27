# can-flash-cli

Command-line tool for flashing application firmware onto an ECU over CAN.

The CLI uploads the application ELF to the CAN flashing server, which transfers it
over CAN to the ECU's custom bootloader and reports firmware update progress.

## Setup

1. Build the CLI from this directory:

   ```sh
   cargo build --release
   ```

2. Source the environment script from Bash or Zsh to add the `can` command:

   ```sh
   source <path-to-firmware-rs>/utilities/can-flash-cli/can-flash-env.sh
   ```

   Add this line to `~/.bashrc` (Bash) or `~/.zshrc` (Zsh) to keep `can` available
   in new shells.

## Usage

Before flashing, make sure [Ajax](https://github.com/Northeastern-Electric-Racing/Ajax)
is running in server mode on the TPU.

Build the application, then run `can flash` from its project directory with Ajax's
URL, including scheme and port:

```sh
cargo build --release
can flash --host http://<server>:<port>
```

The CLI reads the ECU name from the project's `Cargo.toml`:

```toml
[package.metadata.can-flash]
ecu = "<ecu>"
```

The name must match an ECU in Odyssey-Definitions’ `can-bootloader/ecus.json`.
It flashes the release ELF from the Cargo target directory, so build with
`--release` first.

From a workspace root, select the project with `-p`:

```sh
can flash --host <url> -p <package>
```

To flash an ELF outside a Cargo project, pass both the file and the ECU:

```sh
can flash --host <url> --elf path/to/firmware --ecu <ecu>
```

Run `can flash --help` for all options.

## Notes

- By default, `can flash` has Ajax send a CAN boot-entry request to the
  running application, which must handle it with `boot-control`. Use
  `--already-in-bootloader` to skip this if the ECU has no valid application or is
  already in update mode.
- Ajax handles one flash at a time and rejects uploads while busy. Check it
  with `curl <url>/status`, which returns `idle` or `busy`.
- The CLI does not retry uploads. If the connection drops, Ajax stops the
  flash before its next CAN operation. Wait for Ajax to report `idle` before
  trying again. If the ECU stayed in update mode, use `--already-in-bootloader`.
