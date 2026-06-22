# ruvix-cli

Host-side CLI tool for the RuVix cognition kernel.

`ruvix` is the host-side tooling for building, flashing, configuring, and
monitoring RuVix kernel images on AArch64 bare-metal targets. It supports
secure boot, cryptographic key management, Device Tree Blob (DTB) validation,
serial/UART monitoring, and security auditing. The crate builds a single
binary named `ruvix`.

## Install / Build

```bash
cargo build --release -p ruvix-cli
# binary at target/release/ruvix
```

## Subcommands

| Command | Purpose |
|---------|---------|
| `build` | Compile the kernel image for bare-metal targets (optional secure-boot signing, feature flags, custom linker script). |
| `flash` | Write the compiled kernel image and optional DTB to a target device's boot partition. |
| `config` | Get, set, or list kernel configuration options (TOML or JSON). |
| `keys` | Generate, sign, verify, and manage cryptographic keys for secure boot. |
| `dtb` | Validate, inspect, and dump Device Tree Blob files. |
| `monitor` | Connect to the target's serial port for real-time console output and debugging. |
| `security` | Run security audits on kernel configuration and generate reports. |

## Global Options

- `-v, --verbose` — enable verbose output (prints the full error chain on failure)
- `-q, --quiet` — suppress all output except errors
- `--format <text|json>` — output format (default: `text`)

## Usage Examples

```bash
# Build a release kernel with secure boot
ruvix build --release --secure-boot --target aarch64-unknown-none

# Flash to a Raspberry Pi 4
ruvix flash --device /dev/sdb --image target/kernel8.img

# Flash with a DTB
ruvix flash --device /dev/sdb --image kernel8.img --dtb bcm2711-rpi-4-b.dtb

# Generate signing keys
ruvix keys generate --output keys/

# Monitor serial output
ruvix monitor --port /dev/ttyUSB0 --baud 115200

# Run a security audit
ruvix security audit --depth full
```

## Features

- `default` — no optional features enabled
- `rsa` — enables RSA key support (in addition to the default Ed25519)

For more information, see <https://github.com/ruvnet/ruvector>.
