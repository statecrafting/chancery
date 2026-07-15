// SPDX-License-Identifier: Apache-2.0

//! The chancery ledger payload and record construction over attest-ledger.

use action_gate_core::{Decision, Gate, Outcome};
use attest_ledger_core::{LedgerRecord, RecordChain, VerifyError, verify_chain};
use serde::{Deserialize, Serialize};

use crate::autonomy::AutonomyOutcome;

/// The chancery domain payload carried inside a generic attest-ledger
/// [`LedgerRecord`]. The ledger core treats it as an opaque, key-sorted JSON
/// value; this typed wrapper is chancery's view of that payload.
///
/// Binding the gate's `config_hash` makes each recorded decision provable
/// against the exact gate configuration that produced it: an auditor can replay
/// the gate over the recorded context and confirm the recorded outcome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageDecision {
    /// Hash of the exact agent-context bundle the draft was produced from.
    pub context_hash: String,
    /// Hash of the play-spec that authorized the send.
    pub play_hash: String,
    /// `"allow"` | `"deny"` | `"degrade"`.
    pub gate_outcome: String,
    /// The gate reason code, e.g. `"gate:deny:grounding:unsupported_claim"`.
    pub gate_reason: String,
    /// The check ids consulted for this decision.
    pub check_ids: Vec<String>,
    /// `Gate::config_hash()`: binds the decision to the exact gate configuration.
    pub gate_config_hash: String,
    /// `"auto_send"` | `"review_required"` | `"blocked"`.
    pub autonomy_outcome: String,
    /// Provenance of the drafting model.
    pub model_snapshot: String,
    /// The approving principal (a rauthy subject), when a human approved.
    pub approver: Option<String>,
    /// The `(channel, intent)` segment key.
    pub segment: String,
}

fn outcome_str(o: &Outcome) -> &'static str {
    match o {
        Outcome::Allow => "allow",
        Outcome::Deny => "deny",
        Outcome::Degrade => "degrade",
    }
}

impl MessageDecision {
    /// Build a `MessageDecision` from a gate decision and the surrounding facts.
    #[allow(clippy::too_many_arguments)]
    pub fn from_decision(
        decision: &Decision,
        gate: &Gate,
        autonomy: AutonomyOutcome,
        context_hash: String,
        play_hash: String,
        segment: String,
        model_snapshot: String,
        approver: Option<String>,
    ) -> Self {
        Self {
            context_hash,
            play_hash,
            gate_outcome: outcome_str(&decision.outcome).to_string(),
            gate_reason: decision.reason.clone(),
            check_ids: decision.check_ids.clone(),
            gate_config_hash: gate.config_hash(),
            autonomy_outcome: autonomy.as_str().to_string(),
            model_snapshot,
            approver,
            segment,
        }
    }
}

/// Build a single hash-linked ledger record for a `MessageDecision`, linking to
/// `prev_hash`.
///
/// Stateless by design: the chain state (the previous record hash) is owned by
/// the consumer's store (CoreLedger in chancery), not by this crate.
/// `RecordChain::new(prev_hash)` seeds the link, and the single append produces
/// a record whose `previous_record_hash == prev_hash`. `timestamp` is
/// caller-supplied (no wall clock), so the record hash is reproducible: same
/// inputs, same hash, on every platform.
///
/// For the very first record of a thread, pass the chain's anchor hash as
/// `prev_hash`.
pub fn build_record(
    prev_hash: &str,
    id: String,
    timestamp: String,
    decision: &MessageDecision,
) -> LedgerRecord {
    let payload = serde_json::to_value(decision).expect("MessageDecision serializes to JSON");
    let mut chain = RecordChain::new(prev_hash.to_string());
    chain.append(id, timestamp, payload)
}

/// Verify a full decision chain (integrity only): re-hash every record and check
/// the links. This is exactly what the stock `attest-ledger verify` CLI does,
/// so a chancery chain exported from CoreLedger verifies with no bespoke code.
pub fn verify_message_chain(records: &[LedgerRecord]) -> Result<(), VerifyError> {
    verify_chain(records)
}
