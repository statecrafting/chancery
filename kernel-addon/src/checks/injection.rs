// SPDX-License-Identifier: Apache-2.0

use action_gate_core::{ActionContext, Check, Decision};

use super::attr;

/// Inbound-content-as-data boundary. If preprocessing flagged prompt-injection
/// in inbound content that leaked into the draft, deny the send. The detection
/// runs upstream and arrives as `attributes["injection_detected"]`.
pub struct InjectionCheck;

impl Check for InjectionCheck {
    fn id(&self) -> &str {
        "injection"
    }

    fn evaluate(&self, ctx: &ActionContext) -> Option<Decision> {
        let detected = ctx
            .attr(attr::INJECTION_DETECTED)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if detected {
            Some(Decision::deny(
                "gate:deny:injection:boundary_violation",
                vec!["injection".into()],
            ))
        } else {
            None
        }
    }
}
