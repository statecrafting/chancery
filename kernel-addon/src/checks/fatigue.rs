// SPDX-License-Identifier: Apache-2.0

use action_gate_core::{ActionContext, Check, Decision};

use super::attr;

/// Per-investor contact-frequency budget. The current contact count is read from
/// a hiqlite counter TS-side and delivered as `attributes["contact_count"]`;
/// this check compares it against its configured budget. Over budget ->
/// `Degrade` (route to a human), not a hard deny: the budget is a guardrail, not
/// a wall.
///
/// The budget is genuine gate configuration (a policy value, not a per-request
/// signal), so it is folded into [`config_fingerprint`](Check::config_fingerprint)
/// and thereby into the gate's `config_hash`: a recorded decision binds to the
/// exact budget in force when it was made.
pub struct FatigueCheck {
    budget: i64,
}

impl FatigueCheck {
    pub fn new(budget: i64) -> Self {
        Self { budget }
    }
}

impl Check for FatigueCheck {
    fn id(&self) -> &str {
        "fatigue"
    }

    fn evaluate(&self, ctx: &ActionContext) -> Option<Decision> {
        let count = ctx
            .attr(attr::CONTACT_COUNT)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if count >= self.budget {
            Some(Decision::degrade(
                "gate:degrade:fatigue:budget_exceeded",
                vec!["fatigue".into()],
            ))
        } else {
            None
        }
    }

    fn config_fingerprint(&self) -> String {
        format!("fatigue:{}", self.budget)
    }
}
