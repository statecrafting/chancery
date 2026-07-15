// SPDX-License-Identifier: Apache-2.0
//
// End-to-end composition tests: the four primitives, assembled into chancery's
// governance, from a send context through the gate, the ledger, and the
// autonomy policy. These are the C0 "the whole thing wires together" proofs.

use chancery_kernel::{
    AutonomyOutcome, GateParams, Level, MessageDecision, Outcome, RiskTier, SendContext,
    assemble_gate, autonomy_policy, build_record, to_action_context, verify_message_chain,
};

/// A clean send: nothing pre-flagged, grounding entailed.
fn clean_context() -> SendContext {
    SendContext {
        channel: "email".into(),
        intent: "intro".into(),
        investor_id: "inv-1".into(),
        play_id: "play-1".into(),
        draft_body: "Hi Dana, quick note on our seed round.".into(),
        suppressed: false,
        grounding_verdict: "entailed".into(),
        injection_detected: false,
        contact_count: 0,
        tone_violation: false,
    }
}

fn decision_for(sc: &SendContext, params: &GateParams) -> (MessageDecision, chancery_kernel::Gate) {
    let gate = assemble_gate(params);
    let ctx = to_action_context(sc);
    let d = gate.evaluate(&ctx);
    let autonomy = autonomy_policy(&d, Level::Full, RiskTier::Tier1);
    let md = MessageDecision::from_decision(
        &d,
        &gate,
        autonomy,
        "sha256:ctx".into(),
        "sha256:play".into(),
        sc.segment(),
        "claude-opus-4-8".into(),
        None,
    );
    (md, gate)
}

#[test]
fn clean_send_allows_and_auto_sends_at_full_trust() {
    let gate = assemble_gate(&GateParams::default());
    let d = gate.evaluate(&to_action_context(&clean_context()));
    assert!(d.is_allow(), "clean send should pass every check");
    assert_eq!(
        autonomy_policy(&d, Level::Full, RiskTier::Tier1),
        AutonomyOutcome::AutoSend
    );
}

#[test]
fn suppression_hard_blocks_regardless_of_trust() {
    let mut sc = clean_context();
    sc.suppressed = true;
    let gate = assemble_gate(&GateParams::default());
    let d = gate.evaluate(&to_action_context(&sc));
    assert!(matches!(d.outcome, Outcome::Deny));
    assert!(d.blocking, "suppression must be a blocking deny");
    // Even at Full trust and the lowest tier, a blocking deny is Blocked.
    assert_eq!(
        autonomy_policy(&d, Level::Full, RiskTier::Tier0),
        AutonomyOutcome::Blocked
    );
}

#[test]
fn unsupported_grounding_denies() {
    let mut sc = clean_context();
    sc.grounding_verdict = "unsupported".into();
    let gate = assemble_gate(&GateParams::default());
    let d = gate.evaluate(&to_action_context(&sc));
    assert!(matches!(d.outcome, Outcome::Deny));
    assert_eq!(d.reason, "gate:deny:grounding:unsupported_claim");
}

#[test]
fn unknown_grounding_degrades_to_review() {
    let mut sc = clean_context();
    sc.grounding_verdict = "unknown".into();
    let gate = assemble_gate(&GateParams::default());
    let d = gate.evaluate(&to_action_context(&sc));
    assert!(matches!(d.outcome, Outcome::Degrade));
    assert_eq!(
        autonomy_policy(&d, Level::Full, RiskTier::Tier1),
        AutonomyOutcome::ReviewRequired
    );
}

#[test]
fn over_budget_fatigue_degrades() {
    let mut sc = clean_context();
    sc.contact_count = 5; // budget default is 5, so >= trips.
    let gate = assemble_gate(&GateParams::default());
    let d = gate.evaluate(&to_action_context(&sc));
    assert!(matches!(d.outcome, Outcome::Degrade));
    assert_eq!(d.reason, "gate:degrade:fatigue:budget_exceeded");
}

#[test]
fn suppression_short_circuits_before_fatigue() {
    // Both would trigger; suppression is registered first, so its blocking deny
    // wins the first-match short-circuit.
    let mut sc = clean_context();
    sc.suppressed = true;
    sc.contact_count = 99;
    let gate = assemble_gate(&GateParams::default());
    let d = gate.evaluate(&to_action_context(&sc));
    assert_eq!(d.reason, "gate:deny:suppression:on_list");
}

#[test]
fn decision_chain_builds_and_verifies() {
    let (md0, _) = decision_for(&clean_context(), &GateParams::default());
    let mut sc1 = clean_context();
    sc1.intent = "followup".into();
    let (md1, _) = decision_for(&sc1, &GateParams::default());

    let anchor = "sha256:thread-anchor";
    let r0 = build_record(anchor, "rec-0".into(), "2026-07-15T00:00:00Z".into(), &md0);
    let r1 = build_record(
        &r0.record_hash,
        "rec-1".into(),
        "2026-07-15T00:01:00Z".into(),
        &md1,
    );
    assert_eq!(r0.previous_record_hash, anchor);
    assert_eq!(r1.previous_record_hash, r0.record_hash);
    verify_message_chain(&[r0, r1]).expect("intact chain verifies");
}

#[test]
fn tampering_a_record_breaks_verification() {
    let (md0, _) = decision_for(&clean_context(), &GateParams::default());
    let (md1, _) = decision_for(&clean_context(), &GateParams::default());
    let r0 = build_record(
        "sha256:a",
        "rec-0".into(),
        "2026-07-15T00:00:00Z".into(),
        &md0,
    );
    let r1 = build_record(
        &r0.record_hash,
        "rec-1".into(),
        "2026-07-15T00:01:00Z".into(),
        &md1,
    );
    let mut tampered = r1.clone();
    tampered.payload = serde_json::json!({ "tampered": true });
    assert!(
        verify_message_chain(&[r0, tampered]).is_err(),
        "a mutated payload must fail verification"
    );
}

#[test]
fn record_hash_is_deterministic() {
    let (md, _) = decision_for(&clean_context(), &GateParams::default());
    let a = build_record("sha256:x", "id".into(), "2026-07-15T00:00:00Z".into(), &md);
    let b = build_record("sha256:x", "id".into(), "2026-07-15T00:00:00Z".into(), &md);
    assert_eq!(
        a.record_hash, b.record_hash,
        "same inputs, same record hash"
    );
}

#[test]
fn gate_config_hash_is_stable() {
    let a = assemble_gate(&GateParams::default());
    let b = assemble_gate(&GateParams::default());
    assert_eq!(a.config_hash(), b.config_hash());
    // A different fatigue budget is a different configuration.
    let c = assemble_gate(&GateParams { fatigue_budget: 10 });
    assert_ne!(a.config_hash(), c.config_hash());
}

// Golden determinism gate: pinned hashes for a fixed input. A diff here means
// either an intended semantics change (update the constant deliberately) or an
// accidental byte-level drift in the composed hashing (a regression to catch).
// This is the assertion the ecosystem `determinism.yml` gate will run per
// platform to prove "same inputs, same hash, everywhere".
const GOLDEN_GATE_CONFIG_HASH: &str =
    "sha256:d85f5aa0f69a5945dd3fb3b056bcc1a5f2d9e4a7f0f53cb9b20919dcd79c8452";
const GOLDEN_RECORD_HASH: &str =
    "sha256:d151dcb360cb900e370f742c1922b9a3a5af830fd4247a551ffe5109ac88ad59";

#[test]
fn golden_hashes_are_stable() {
    let (md, gate) = decision_for(&clean_context(), &GateParams::default());
    assert_eq!(
        gate.config_hash(),
        GOLDEN_GATE_CONFIG_HASH,
        "gate config hash drifted"
    );
    let r = build_record(
        "sha256:thread-anchor",
        "rec-0".into(),
        "2026-07-15T00:00:00Z".into(),
        &md,
    );
    assert_eq!(r.record_hash, GOLDEN_RECORD_HASH, "record hash drifted");
}
