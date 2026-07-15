// SPDX-License-Identifier: Apache-2.0

//! Mapping a chancery send onto the domain-neutral gate context.

use action_gate_core::ActionContext;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::checks::attr;

/// The chancery-shaped description of a proposed send, before it becomes a
/// domain-neutral [`ActionContext`]. The pre-computed guardrail signals
/// (grounding verdict, suppression membership, injection flag, contact count,
/// tone flag) are filled in by the TS context-assembly step; here they are plain
/// fields so the mapping is exercisable in pure Rust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendContext {
    pub channel: String,
    pub intent: String,
    pub investor_id: String,
    pub play_id: String,
    pub draft_body: String,
    // Pre-computed guardrail signals (the attribute contract with TS).
    pub suppressed: bool,
    /// `"entailed"` | `"unsupported"` | `"unknown"`.
    pub grounding_verdict: String,
    pub injection_detected: bool,
    pub contact_count: i64,
    pub tone_violation: bool,
}

impl SendContext {
    /// The `(channel, intent)` ladder segment key (decision #5).
    pub fn segment(&self) -> String {
        format!("{}::{}", self.channel, self.intent)
    }
}

/// Map a chancery send onto the domain-neutral [`ActionContext`] the gate reads.
/// Mirrors OAP's `to_action_context`: the typed extras become attributes; the
/// gate core never inspects them, only chancery's checks do.
pub fn to_action_context(sc: &SendContext) -> ActionContext {
    ActionContext::new(format!("message.send:{}", sc.channel))
        .with_summary(sc.draft_body.clone())
        .with_body(sc.draft_body.clone())
        .with_attr(attr::SUPPRESSED, json!(sc.suppressed))
        .with_attr(attr::GROUNDING_VERDICT, json!(sc.grounding_verdict))
        .with_attr(attr::INJECTION_DETECTED, json!(sc.injection_detected))
        .with_attr(attr::CONTACT_COUNT, json!(sc.contact_count))
        .with_attr(attr::TONE_VIOLATION, json!(sc.tone_violation))
}
