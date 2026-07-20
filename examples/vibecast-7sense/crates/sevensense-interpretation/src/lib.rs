//! # sevensense-interpretation
//!
//! Evidence-based interpretation for the 7sense bioacoustics platform.
//!
//! **Status: work-in-progress skeleton.** The crate root currently exposes only
//! the [`VERSION`] constant. The domain, application, and infrastructure source
//! files exist in the tree (evidence packs, claim generation, formatting
//! templates) but are **not yet wired into the public API** — they are not
//! declared as modules here, so nothing beyond [`VERSION`] is reachable from
//! `sevensense_interpretation::`.
//!
//! ## Planned design
//!
//! The intended bounded context is structured around "evidence packs" that
//! document why a prediction was made, using nearest-neighbor, cluster, and
//! sequence context. Once wired up, the public surface is expected to include:
//!
//! - `domain::entities` — `EvidencePack`, `EmbeddingId`, `Interpretation`, `Claim`, …
//! - `domain::repository` — `EvidencePackRepository` (with an in-memory impl)
//! - `application::services` — `InterpretationService` building evidence packs
//! - `infrastructure` — `EvidenceBuilder`, `ClaimGenerator`
//! - `templates` — `InterpretationTemplates`, `EvidencePackFormatter`
//!
//! These modules are not currently part of the compiled public API.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

// NOTE: The domain/application/infrastructure/templates source files exist but
// are intentionally not declared as `pub mod` yet — the crate currently only
// publishes `VERSION`. Wiring them into the public API is tracked as WIP.

/// Crate version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
