// SPDX-License-Identifier: Apache-2.0

//! The JSON-in / JSON-out boundary the enrahitu Encore.ts app drives.
//!
//! Every function here is pure `&str -> Result<String, String>` with no napi
//! types, so it compiles and unit-tests under a plain `cargo test`. The
//! `#[napi]` layer (`src/napi_api.rs`, behind the `napi` feature) is a thin
//! delegator that only maps the `String` error onto `napi::Error`. Keeping the
//! logic here, not in the napi shim, is what lets the boundary be tested without
//! a Node runtime.
//!
//! The DTOs below are the wire contract: the primitive types' Rust shapes are
//! deliberately not exposed across the boundary.

use action_gate_core::{Decision, Outcome};
use serde::{Deserialize, Serialize};
use trust_window::{Level, Sample, WindowConfig, WindowSnapshot};

use crate::{
    GateParams, MessageDecision, RiskTier, SendContext, assemble_gate, autonomy_policy,
    build_record as core_build_record, chancery_window_config, score as core_score,
    to_action_context, verify_message_chain,
};

/// A serializable view of a gate [`Decision`].
#[derive(Serialize, Deserialize)]
struct DecisionDto {
    /// `"allow"` | `"deny"` | `"degrade"`.
    outcome: String,
    reason: String,
    check_ids: Vec<String>,
    blocking: bool,
}

impl DecisionDto {
    fn from_decision(d: &Decision) -> Self {
        let outcome = match d.outcome {
            Outcome::Allow => "allow",
            Outcome::Deny => "deny",
            Outcome::Degrade => "degrade",
        };
        Self {
            outcome: outcome.to_string(),
            reason: d.reason.clone(),
            check_ids: d.check_ids.clone(),
            blocking: d.blocking,
        }
    }

    /// Rebuild a [`Decision`] carrying enough to drive the autonomy policy
    /// (outcome + blocking). An unknown outcome string is treated as `allow`.
    fn to_decision(&self) -> Decision {
        let mut d = match self.outcome.as_str() {
            "deny" => Decision::deny(self.reason.clone(), self.check_ids.clone()),
            "degrade" => Decision::degrade(self.reason.clone(), self.check_ids.clone()),
            _ => Decision::allow(),
        };
        if self.blocking {
            d = d.blocking();
        }
        d
    }
}

#[derive(Serialize)]
struct GateResultDto {
    decision: DecisionDto,
    config_hash: String,
}

#[derive(Serialize)]
struct VerifyDto {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn e<E: std::fmt::Display>(ctx: &str, err: E) -> String {
    format!("chancery-kernel {ctx}: {err}")
}

/// Deserialize a bare string (e.g. `"full"`, `"tier1"`) into a serde enum.
fn parse_enum_str<T: for<'de> Deserialize<'de>>(ctx: &str, s: &str) -> Result<T, String> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).map_err(|err| e(ctx, err))
}

/// Evaluate the chancery gate over a send context.
///
/// Input: a `SendContext` JSON (guardrail signals pre-computed as fields) and a
/// `GateParams` JSON. Output:
/// `{ "decision": { outcome, reason, check_ids, blocking }, "config_hash": "..." }`.
pub fn evaluate_gate(context_json: &str, params_json: &str) -> Result<String, String> {
    let sc: SendContext = serde_json::from_str(context_json).map_err(|err| e("context", err))?;
    let params: GateParams = serde_json::from_str(params_json).map_err(|err| e("params", err))?;
    let gate = assemble_gate(&params);
    let decision = gate.evaluate(&to_action_context(&sc));
    let out = GateResultDto {
        decision: DecisionDto::from_decision(&decision),
        config_hash: gate.config_hash(),
    };
    serde_json::to_string(&out).map_err(|err| e("result", err))
}

/// Fuse a gate decision, trust level, and risk tier into an autonomy outcome
/// string (`"auto_send"` | `"review_required"` | `"blocked"`).
///
/// `decision_json` is the `decision` object from [`evaluate_gate`]; `trust_level`
/// is `"full" | "restricted" | "read-only" | "suspended"`; `tier` is
/// `"tier0" | "tier1" | "tier2" | "tier3"`.
pub fn decide_autonomy(
    decision_json: &str,
    trust_level: &str,
    tier: &str,
) -> Result<String, String> {
    let dto: DecisionDto = serde_json::from_str(decision_json).map_err(|err| e("decision", err))?;
    let level: Level = parse_enum_str("trust_level", trust_level)?;
    let tier: RiskTier = parse_enum_str("tier", tier)?;
    Ok(autonomy_policy(&dto.to_decision(), level, tier)
        .as_str()
        .to_string())
}

/// Build one hash-linked ledger record for a `MessageDecision`, linking to
/// `prev_hash` (the thread's anchor hash for the first record). Output: a
/// `LedgerRecord` JSON. `timestamp` is caller-supplied (no wall clock).
pub fn build_record(
    prev_hash: &str,
    id: &str,
    timestamp: &str,
    message_decision_json: &str,
) -> Result<String, String> {
    let md: MessageDecision =
        serde_json::from_str(message_decision_json).map_err(|err| e("message_decision", err))?;
    let rec = core_build_record(prev_hash, id.to_string(), timestamp.to_string(), &md);
    serde_json::to_string(&rec).map_err(|err| e("record", err))
}

/// Verify a full decision chain (integrity only). Input: a JSON array of
/// `LedgerRecord`. Output: `{ "ok": bool, "error"?: String }`. Equivalent to the
/// stock `attest-ledger verify` CLI over the same records.
pub fn verify_chain(records_json: &str) -> Result<String, String> {
    let records: Vec<attest_ledger_core::LedgerRecord> =
        serde_json::from_str(records_json).map_err(|err| e("records", err))?;
    let res = verify_message_chain(&records);
    let dto = VerifyDto {
        ok: res.is_ok(),
        error: res.err().map(|x| x.to_string()),
    };
    serde_json::to_string(&dto).map_err(|err| e("verify", err))
}

/// Score new review samples against a per-(play, segment) trust window. Inputs: a
/// `WindowConfig` JSON, an optional `WindowSnapshot` JSON (omit for a fresh
/// window), and a JSON array of `Sample`. Output: `{ score, level, snapshot }`.
pub fn score(
    config_json: &str,
    snapshot_json: Option<&str>,
    samples_json: &str,
) -> Result<String, String> {
    let config: WindowConfig = serde_json::from_str(config_json).map_err(|err| e("config", err))?;
    let snapshot: Option<WindowSnapshot> = match snapshot_json {
        Some(s) => Some(serde_json::from_str(s).map_err(|err| e("snapshot", err))?),
        None => None,
    };
    let samples: Vec<Sample> =
        serde_json::from_str(samples_json).map_err(|err| e("samples", err))?;
    let result = core_score(config, snapshot, &samples);
    serde_json::to_string(&result).map_err(|err| e("score", err))
}

/// The default chancery per-(play, segment) `WindowConfig` as JSON, for the TS
/// side to persist alongside a new play so scoring is reproducible.
pub fn default_window_config() -> Result<String, String> {
    serde_json::to_string(&chancery_window_config()).map_err(|err| e("window_config", err))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLEAN_CTX: &str = r#"{
        "channel":"email","intent":"intro","investor_id":"i","play_id":"p",
        "draft_body":"hi","suppressed":false,"grounding_verdict":"entailed",
        "injection_detected":false,"contact_count":0,"tone_violation":false
    }"#;

    #[test]
    fn evaluate_gate_boundary() {
        let out = evaluate_gate(CLEAN_CTX, r#"{"fatigue_budget":5}"#).unwrap();
        assert!(out.contains("\"outcome\":\"allow\""), "got: {out}");
        assert!(out.contains("config_hash"));
    }

    #[test]
    fn autonomy_boundary() {
        let dec = r#"{"outcome":"allow","reason":"","check_ids":[],"blocking":false}"#;
        assert_eq!(decide_autonomy(dec, "full", "tier1").unwrap(), "auto_send");
        let blk = r#"{"outcome":"deny","reason":"x","check_ids":["x"],"blocking":true}"#;
        assert_eq!(decide_autonomy(blk, "full", "tier0").unwrap(), "blocked");
    }

    #[test]
    fn record_and_verify_boundary() {
        let md = r#"{
            "context_hash":"sha256:c","play_hash":"sha256:p","gate_outcome":"allow",
            "gate_reason":"","check_ids":[],"gate_config_hash":"sha256:g",
            "autonomy_outcome":"auto_send","model_snapshot":"m","approver":null,
            "segment":"email::intro"
        }"#;
        let rec = build_record("sha256:anchor", "rec-0", "2026-07-15T00:00:00Z", md).unwrap();
        let arr = format!("[{rec}]");
        let v = verify_chain(&arr).unwrap();
        assert!(v.contains("\"ok\":true"), "got: {v}");
    }

    #[test]
    fn score_boundary() {
        let cfg = default_window_config().unwrap();
        let s = score(&cfg, None, r#"[{"value":1.0,"weight":1.0}]"#).unwrap();
        assert!(s.contains("level"), "got: {s}");
        assert!(s.contains("snapshot"));
    }

    #[test]
    fn malformed_json_is_an_error() {
        assert!(evaluate_gate("{ not json", "{}").is_err());
        assert!(decide_autonomy("{}", "nonsense-level", "tier1").is_err());
    }
}
