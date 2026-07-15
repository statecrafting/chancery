// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Bartek Kus

//! `chancery-kernel`: chancery's runtime governance, composed from four
//! domain-neutral primitives.
//!
//! - [`action-gate-core`](action_gate_core): the pure decision gate. chancery
//!   registers its own [`checks`] (grounding, suppression, injection, fatigue,
//!   tone) into it.
//! - [`attest-ledger-core`](attest_ledger_core): the signed, hash-linked
//!   decision ledger. chancery's [`MessageDecision`](payload::MessageDecision)
//!   is one record's payload.
//! - [`trust-window`](trust_window): the rolling-window trust scorer. chancery's
//!   play-scoped autonomy [`ladder`] drives it bidirectionally.
//! - [`canonical-keysort-json`](canonical_keysort_json): the canonical
//!   serialization the ledger hashes through (a transitive dependency).
//!
//! The whole crate is a pure function of its inputs: no host calls, no wall
//! clock, no database. Persistence (the ledger chain, the per-play trust
//! snapshots) and the pre-computation of I/O-bound gate signals (grounding
//! entailment, suppression membership, the fatigue count) are the consumer's
//! job. That is what keeps the kernel deterministic and golden-testable.

pub mod autonomy;
pub mod checks;
pub mod context;
pub mod gate;
pub mod ladder;
pub mod payload;
pub mod tier;
pub mod wire;

/// The napi-rs addon surface: JSON-in / JSON-out `#[napi]` functions the
/// enrahitu Encore.ts app calls, thin delegators over [`wire`]. Present only
/// under the `napi` feature (opt-in); the pure core does not depend on it.
#[cfg(feature = "napi")]
pub mod napi_api;

pub use autonomy::{AutonomyOutcome, autonomy_policy};
pub use context::{SendContext, to_action_context};
pub use gate::{GateParams, assemble_gate};
pub use ladder::{ReviewOutcome, ScoreResult, chancery_window_config, score};
pub use payload::{MessageDecision, build_record, verify_message_chain};
pub use tier::RiskTier;

// Re-export the primitive types a consumer of this kernel touches directly, so
// they need not depend on the four crates individually.
pub use action_gate_core::{ActionContext, Decision, Gate, Outcome};
pub use attest_ledger_core::{LedgerRecord, VerifyError};
pub use trust_window::{Level, Sample, WindowSnapshot};
