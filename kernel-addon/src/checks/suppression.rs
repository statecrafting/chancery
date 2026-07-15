// SPDX-License-Identifier: Apache-2.0

use action_gate_core::{ActionContext, Check, Decision};

use super::attr;

/// Global unsubscribe / bounce / do-not-contact. A hard, blocking stop: `Deny`
/// with `blocking = true`, so no trust level and no autonomy policy can override
/// it. Placed first in the gate so it short-circuits before any softer check.
pub struct SuppressionCheck;

impl Check for SuppressionCheck {
    fn id(&self) -> &str {
        "suppression"
    }

    fn evaluate(&self, ctx: &ActionContext) -> Option<Decision> {
        let suppressed = ctx
            .attr(attr::SUPPRESSED)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if suppressed {
            Some(
                Decision::deny("gate:deny:suppression:on_list", vec!["suppression".into()])
                    .blocking(),
            )
        } else {
            None
        }
    }
}
