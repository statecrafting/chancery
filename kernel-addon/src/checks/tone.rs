// SPDX-License-Identifier: Apache-2.0

use action_gate_core::{ActionContext, Check, Decision};

use super::attr;

/// Tone / style guardrail. A tone violation classified upstream and delivered as
/// `attributes["tone_violation"]` routes the send to review (`Degrade`).
pub struct ToneCheck;

impl Check for ToneCheck {
    fn id(&self) -> &str {
        "tone"
    }

    fn evaluate(&self, ctx: &ActionContext) -> Option<Decision> {
        let violation = ctx
            .attr(attr::TONE_VIOLATION)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if violation {
            Some(Decision::degrade(
                "gate:degrade:tone:violation",
                vec!["tone".into()],
            ))
        } else {
            None
        }
    }
}
