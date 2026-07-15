// SPDX-License-Identifier: Apache-2.0

//! chancery's domain gate checks.
//!
//! Each check is a pure `impl Check`: it reads only the pre-computed
//! [`ActionContext`](action_gate_core::ActionContext) attributes and never calls
//! a model or the host, so the assembled gate stays deterministic and
//! golden-testable. I/O-bound signals (grounding entailment, suppression
//! membership, the fatigue count, tone/injection classification) are computed by
//! the TS context-assembly step and fed in as attributes. The attribute keys are
//! the pre-computation contract, collected in [`attr`].

mod fatigue;
mod grounding;
mod injection;
mod suppression;
mod tone;

pub use fatigue::FatigueCheck;
pub use grounding::GroundingCheck;
pub use injection::InjectionCheck;
pub use suppression::SuppressionCheck;
pub use tone::ToneCheck;

/// The attribute keys chancery's checks read: the pre-computation contract the
/// TS context-assembly step fills in before the gate runs.
pub mod attr {
    /// `bool`: the recipient is on a global unsubscribe / bounce / do-not-contact
    /// list. Resolved from the suppression store TS-side.
    pub const SUPPRESSED: &str = "suppressed";
    /// `string`: `"entailed"` | `"unsupported"` | `"unknown"`. The verdict of the
    /// upstream entailment step over the approved fact sheet.
    pub const GROUNDING_VERDICT: &str = "grounding_verdict";
    /// `bool`: preprocessing flagged prompt-injection from inbound content that
    /// leaked into the draft.
    pub const INJECTION_DETECTED: &str = "injection_detected";
    /// `i64`: the recipient's current contact count in the fatigue window,
    /// read from a hiqlite counter TS-side.
    pub const CONTACT_COUNT: &str = "contact_count";
    /// `bool`: the draft violates the tone / style guardrail.
    pub const TONE_VIOLATION: &str = "tone_violation";
}
