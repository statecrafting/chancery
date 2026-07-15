// SPDX-License-Identifier: Apache-2.0

use action_gate_core::{ActionContext, Check, Decision};

use super::attr;

/// The highest-stakes guardrail: a drafted claim must be entailed by the
/// approved fact sheet. The entailment itself is computed upstream (a model /
/// the TS context-assembly step) and delivered as
/// `attributes["grounding_verdict"]`; this check only reads the verdict, which
/// is what keeps the gate pure.
///
/// - `"entailed"` -> pass (`None`).
/// - `"unsupported"` -> `Deny` (a claim the fact sheet does not support).
/// - `"unknown"` or missing -> `Degrade` (unverified; route to a human). Missing
///   is treated as unknown on purpose: a send whose grounding was never computed
///   must not sail through.
pub struct GroundingCheck;

impl Check for GroundingCheck {
    fn id(&self) -> &str {
        "grounding"
    }

    fn evaluate(&self, ctx: &ActionContext) -> Option<Decision> {
        match ctx.attr_str(attr::GROUNDING_VERDICT) {
            Some("entailed") => None,
            Some("unsupported") => Some(Decision::deny(
                "gate:deny:grounding:unsupported_claim",
                vec!["grounding".into()],
            )),
            _ => Some(Decision::degrade(
                "gate:degrade:grounding:unverified",
                vec!["grounding".into()],
            )),
        }
    }
}
