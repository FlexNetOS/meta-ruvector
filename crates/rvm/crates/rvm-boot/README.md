# rvm-boot

Deterministic, witness-gated phased boot sequence for the RVM microhypervisor.

This crate implements **two** boot models. Each phase is gated by a witness
entry and must complete before the next begins; out-of-order completion is
rejected.

- **ADR-137** -- the 7-phase deterministic boot the hypervisor actually runs,
  modeled by `BootSequence` (`BootStage` / `PhaseTiming`), with timing and
  per-stage witness digests, plus the `run_boot_sequence` driver and
  `BootContext`.
- **ADR-140** -- a simpler legacy phase model (`BootPhase` / `BootTracker`)
  retained for compatibility.

## ADR-137: 7-Phase Deterministic Boot (`BootSequence`)

```
Phase 0: Reset vector       (initial entry from firmware)
Phase 1: Hardware detect    (enumerate CPUs, memory, devices)
Phase 2: MMU setup          (stage-2 page tables)
Phase 3: Hypervisor mode    (enter EL2)
Phase 4: Kernel object init (cap table, IPC, etc.)
Phase 5: First witness      (genesis attestation)
Phase 6: Scheduler entry    (hand off to scheduler loop)
```

### Key Types

- `BootStage` -- enum of 7 stages (`ResetVector` through `SchedulerEntry`); has
  `next()`, `name()`, and `all()`. `BOOT_STAGE_COUNT` (7) and `TARGET_BOOT_MS`
  (250) are exported from the `sequence` module.
- `PhaseTiming` -- per-phase `start_tick` / `end_tick` with `duration_ticks()`.
- `BootSequence` -- the 7-phase manager. `new()` starts at `ResetVector`;
  `begin_stage(stage, tick)` and `complete_stage(stage, tick, witness_digest)`
  advance strictly forward; `current_stage()` returns the next expected stage.
- `run_boot_sequence(..)` / `BootContext` -- driver that runs the full sequence
  (in the `entry` module).
- `MeasuredBootState` -- measured-boot hash-chain accumulation (`measured`).
- `HalInit`, `StubHal`, `MmuConfig`, `InterruptConfig`, `UartConfig` -- HAL
  initialization traits and a stub implementation (`hal_init`).

## ADR-140: Legacy Phase Model (`BootTracker`)

```
Phase 0: HAL init       (timer, MMU, interrupts)
Phase 1: Memory init    (physical page allocator)
Phase 2: Capability init
Phase 3: Witness init
Phase 4: Scheduler init
Phase 5: Root partition creation
Phase 6: Hand-off to root partition
```

### Key Types

- `BootPhase` -- enum of 7 phases (`HalInit` through `Handoff`)
- `BootTracker` -- state machine enforcing sequential phase completion
  - `new()` -- starts at `HalInit`
  - `complete_phase(phase)` -- marks current phase done, advances to next
  - `is_complete()` -- true when all 7 phases have completed
  - `current_phase()` -- returns the current phase, or `None` if complete

### Example

```rust
use rvm_boot::{BootTracker, BootPhase};

let mut tracker = BootTracker::new();
assert_eq!(tracker.current_phase(), Some(BootPhase::HalInit));

tracker.complete_phase(BootPhase::HalInit).unwrap();
assert_eq!(tracker.current_phase(), Some(BootPhase::MemoryInit));

// Out-of-order is rejected:
assert!(tracker.complete_phase(BootPhase::WitnessInit).is_err());
```

## Design Constraints

- **DC-15**: `#![no_std]`, `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`
- ADR-137 / ADR-140: deterministic, witness-gated boot sequence

## Workspace Dependencies

- `rvm-types`
- `rvm-hal`
- `rvm-partition`
- `rvm-witness`
- `rvm-sched`
- `rvm-memory`
