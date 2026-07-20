# rvm-memory

Guest physical address space management for the RVM microhypervisor, built
around a **four-tier, coherence-driven memory model** with reconstruction
capability (ADR-136 / ADR-138).

## Four-Tier Memory Model (ADR-136)

| Tier | Name | Description |
|------|------|-------------|
| 0 | Hot | Per-core SRAM / L1-adjacent; always resident during execution |
| 1 | Warm | Shared DRAM; resident if the residency rule is met |
| 2 | Dormant | Compressed checkpoint + delta; reconstructed on demand |
| 3 | Cold | Persistent archival; accessed only during recovery |

Tier transitions are explicit (not demand-paged) and are driven by coherence,
falling back to static thresholds (DC-1) when the coherence engine is absent.
The crate is `#![no_std]` with zero heap allocation and `#![forbid(unsafe_code)]`.

## Key Components

- `Tier` / `TierManager` / `TierThresholds` / `RegionTierState` -- coherence-driven tier placement and transitions (`tier`)
- `BuddyAllocator` -- power-of-two physical page allocator (`allocator`)
- `RegionManager` / `OwnedRegion` / `RegionConfig` / `AddressMapping` -- owned-region lifecycle and address translation (`region`)
- `ReconstructionPipeline` / `CompressedCheckpoint` / `CheckpointId` / `WitnessDelta` / `ReconstructionResult` / `create_checkpoint` -- dormant-state restoration (`reconstruction`)

## Crate-Root Helpers

- `MemoryRegion` -- a legacy descriptor (ADR-138 compatibility): guest base, host base, page count, permissions, owner. For new code prefer `OwnedRegion`, which carries tier metadata.
- `MemoryPermissions` -- RWX permission flags with constants (`READ_ONLY`, `READ_WRITE`, `READ_EXECUTE`)
- `validate_region(region)` -- checks alignment, page count, and permission validity
- `regions_overlap(a, b)` -- detects overlapping regions within the same partition
- `regions_overlap_host(a, b)` -- detects host-physical overlap across partitions (isolation check)
- `PAGE_SIZE` -- 4 KiB page size constant

## Example

```rust
use rvm_memory::{MemoryRegion, MemoryPermissions, validate_region, PAGE_SIZE};
use rvm_types::{GuestPhysAddr, PhysAddr, PartitionId};

let region = MemoryRegion {
    guest_base: GuestPhysAddr::new(0x8000_0000),
    host_base: PhysAddr::new(0x4000_0000),
    page_count: 16,
    permissions: MemoryPermissions::READ_WRITE,
    owner: PartitionId::new(1),
};
assert!(validate_region(&region).is_ok());
```

## Design Constraints

- **DC-15**: `#![no_std]`, `#![forbid(unsafe_code)]`, `#![deny(missing_docs)]`
- ADR-138: capability-gated mappings; witness-logged operations

## Workspace Dependencies

- `rvm-types`
- `rvm-hal`
- `rvm-partition`
- `rvm-witness`
