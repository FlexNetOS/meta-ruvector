# ruvector-mmwave

Streaming parser for the Seeed MR60BHA2 60 GHz radar's UART
protocol. Pure-Rust, no_std-compatible, zero-allocation hot path.

> **Status:** library, **11 unit tests** passing (in `src/lib.rs`
> `mod tests`). Shared between the host-side bridge in
> `crates/ruvector-hailo-cluster/src/bin/mmwave-bridge.rs` (parses
> serial input → emits NL events → cluster embed RPC) and the
> firmware self-test in `examples/esp32-mmwave-sensor/` that runs on
> the radar's MCU directly.

## Why a separate crate

ADR-178 Gap H: keeping the parser separate from
`ruvector-hailo-cluster` (the bridge's home crate) means a regression
in either side surfaces independently. The parser is byte-for-byte
deterministic against fuzzed inputs; the bridge layers transport,
TLS, fingerprinting on top.

## Wire format (Seeed MR60BHA2 v0.3)

```text
8-byte header  | variable payload | trailing checksum
[0x01]         | <up to 64 bytes>  | invert_xor(payload)
[frame_id_hi]
[frame_id_lo]
[length_hi]    ← 16-bit big-endian payload length
[length_lo]
[type_hi]      ← 16-bit big-endian frame type
[type_lo]
[invert_xor of 7 prior bytes]
```

Frame types currently parsed (see `decode_event` in `src/lib.rs`):

| `frame_type` | meaning | payload shape | `Event` variant |
|--------------|---------|---------------|-----------------|
| `0x0A14` | breathing rate | `[bpm: u8]` | `Event::Breathing { bpm }` |
| `0x0A15` | heart rate | `[bpm: u8]` | `Event::HeartRate { bpm }` |
| `0x0A16` | nearest target distance | `[cm: u16 BE]` | `Event::Distance { cm }` |
| `0x0F09` | presence flag | `[present: u8]` (0 = absent, non-zero = present) | `Event::Presence { present }` |
| anything else | (iter 249) `payload_len` is `u16` | `Event::Unknown { frame_type, payload_len }` |

## API surface

```rust
use ruvector_mmwave::{Event, Mr60Parser};

let mut p = Mr60Parser::new();
let frame: &[u8] = /* 60 bytes from /dev/ttyUSB0 */;
p.feed_slice(frame, |ev| match ev {
    Event::Breathing { bpm } => println!("breathing {} bpm", bpm),
    Event::HeartRate { bpm } => println!("heart rate {} bpm", bpm),
    Event::Distance { cm } => println!("distance {} cm", cm),
    Event::Presence { present } => println!("present={}", present),
    Event::Unknown { frame_type, payload_len } => {
        eprintln!("unknown frame 0x{:04x} len={}", frame_type, payload_len);
    }
    Event::ChecksumError => eprintln!("dropped frame, parser resynced"),
    Event::Resync => eprintln!("desync byte, scanning for next SOF"),
});
```

The closure signature is `FnMut(Event)`; `feed_slice` invokes it once
per emitted event (each byte produces at most one `Event`, via
`feed` returning `Option<Event>`). The state machine resyncs cleanly on
checksum failure or unexpected SOF — call `reset()` if you need to
force resync after a disconnect/reconnect.

## Performance characteristics

The hot path is allocation-free by construction: the parser holds a
fixed 8-byte header buffer and a fixed `MAX_PAYLOAD`-byte (64) payload
buffer, so `feed`/`feed_slice` never allocate per byte. This keeps the
state machine cheap relative to typical UART rates (115200 baud →
~14 KB/s).

> **Planned/WIP:** there is no `benches/` directory or criterion
> harness in this crate yet, so no measured throughput figures are
> published. Add a `benches/` target before quoting GB/s numbers.

## Tests

The `mod tests` block in `src/lib.rs` covers:
- per-frame-type decoding (breathing, heart rate, distance, presence)
- unknown frame types surfacing as `Event::Unknown` (incl. the iter-249
  `payload_len: u16` widening)
- corrupted header and data checksums producing `Event::ChecksumError`
- split byte streams (frame fed one byte at a time)
- recovery after a garbage prefix (`Event::Resync` then a valid frame)
- the `invert_xor` checksum matching the Seeed reference

## See also

- `crates/ruvector-hailo-cluster/src/bin/mmwave-bridge.rs` — the
  host-side bridge that consumes this parser and posts NL events
  to the hailo-backend cluster.
- `examples/esp32-mmwave-sensor/` — firmware-side use of this
  parser on an ESP32 paired to the radar over UART.
- ADR-063 — original mmwave integration design.
- ADR-178 Gap H — rationale for the separate crate boundary.
