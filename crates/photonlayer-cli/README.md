# photonlayer-cli

PhotonLayer command-line studio: run simulations, demos, benchmarks, and verify receipts (ADR-260).

## Overview

`photonlayer-cli` is the user-facing entry point to the PhotonLayer optical-computing
simulator within the meta-ruvector workspace. It ties together `photonlayer-core` (the
optical simulator, masks, metrics, and receipts), `photonlayer-bench` (benchmarks and the
mask learner), and `photonlayer-ruvector` (experiment memory) into a single `photonlayer`
binary. Optical computing here is framed as a front end that performs useful computation
before digitization — lower latency, narrower sensor bandwidth, compressed measurements —
and the flagship demo shows consented 1:1 verification without storing a recoverable image.

## Usage

```bash
cargo run -p photonlayer-cli -- <subcommand> [args...]
```

Subcommands:

- `bench [classification|compression]` — run accuracy / compression benchmarks (defaults to both).
- `barcode` — optical encode/decode demo; renders ASCII frames and decodes the hidden class.
- `edge` — optical edge-detection demo using a high-pass lens mask.
- `privacy-gate` — flagship consented biometric verification demo (EER, privacy leakage, and a tamper-evident receipt).
- `verify-receipt <path.json>` — verify the integrity of a stored experiment receipt.
- `help` (or no args) — print usage.

Examples:

```bash
cargo run -p photonlayer-cli -- bench compression
cargo run -p photonlayer-cli -- privacy-gate
cargo run -p photonlayer-cli -- verify-receipt /tmp/receipt.json
```

## License

MIT
