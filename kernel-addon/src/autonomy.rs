// SPDX-License-Identifier: Apache-2.0

//! The autonomy policy: how a proposed send is disposed, given the gate
//! decision, the play's trust level, and the send's risk tier.

use action_gate_core::{Decision, Outcome};
use serde::{Deserialize, Serialize};
use trust_window::Level;

use crate::tier::RiskTier;

/// What the outbox should do with a proposed send. This is chancery policy
/// layered on top of the pure gate: a gate check yields allow / deny / degrade,
/// and autonomy is a separate dimension (the gate library deliberately does not
/// know about it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyOutcome {
    /// Send without a human in the loop.
    AutoSend,
    /// Route to a human reviewer before sending.
    ReviewRequired,
    /// Do not send.
    Blocked,
}

impl AutonomyOutcome {
    /// A stable machine string, for the ledger payload.
    pub fn as_str(self) -> &'static str {
        match self {
            AutonomyOutcome::AutoSend => "auto_send",
            AutonomyOutcome::ReviewRequired => "review_required",
            AutonomyOutcome::Blocked => "blocked",
        }
    }
}

/// Fuse the gate decision, the play's trust level, and the send's risk tier into
/// an autonomy outcome. A pure, total function.
///
/// Invariants:
/// - A [`blocking`](action_gate_core::Decision) decision is `Blocked`
///   regardless of trust or tier: a hard stop overrides everything.
/// - `Deny` is `Blocked`; `Degrade` is `ReviewRequired`.
/// - On `Allow`, the tier gates auto-send: a hard-pinned tier is always
///   `ReviewRequired`; otherwise a sufficiently high trust level unlocks
///   `AutoSend` (the bar rises with the tier).
pub fn autonomy_policy(gate: &Decision, trust: Level, tier: RiskTier) -> AutonomyOutcome {
    if gate.blocking {
        return AutonomyOutcome::Blocked;
    }
    match &gate.outcome {
        Outcome::Deny => AutonomyOutcome::Blocked,
        Outcome::Degrade => AutonomyOutcome::ReviewRequired,
        Outcome::Allow => match tier {
            // Hard pin and elevated both route to review under the C0 policy.
            RiskTier::Tier3 | RiskTier::Tier2 => AutonomyOutcome::ReviewRequired,
            RiskTier::Tier1 => {
                if trust == Level::Full {
                    AutonomyOutcome::AutoSend
                } else {
                    AutonomyOutcome::ReviewRequired
                }
            }
            RiskTier::Tier0 => match trust {
                Level::Full | Level::Restricted => AutonomyOutcome::AutoSend,
                _ => AutonomyOutcome::ReviewRequired,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use action_gate_core::Decision;

    fn allow() -> Decision {
        Decision::allow()
    }
    fn deny() -> Decision {
        Decision::deny("gate:deny:x", vec!["x".into()])
    }
    fn degrade() -> Decision {
        Decision::degrade("gate:degrade:x", vec!["x".into()])
    }
    fn blocking() -> Decision {
        Decision::deny("gate:deny:hard", vec!["hard".into()]).blocking()
    }

    #[test]
    fn blocking_overrides_everything() {
        assert_eq!(
            autonomy_policy(&blocking(), Level::Full, RiskTier::Tier0),
            AutonomyOutcome::Blocked
        );
    }

    #[test]
    fn deny_blocks_degrade_reviews() {
        assert_eq!(
            autonomy_policy(&deny(), Level::Full, RiskTier::Tier0),
            AutonomyOutcome::Blocked
        );
        assert_eq!(
            autonomy_policy(&degrade(), Level::Full, RiskTier::Tier0),
            AutonomyOutcome::ReviewRequired
        );
    }

    #[test]
    fn allow_tier_matrix() {
        // Tier3 pinned + Tier2 elevated: always review on allow.
        for tier in [RiskTier::Tier3, RiskTier::Tier2] {
            assert_eq!(
                autonomy_policy(&allow(), Level::Full, tier),
                AutonomyOutcome::ReviewRequired
            );
        }
        // Tier1: auto only at Full.
        assert_eq!(
            autonomy_policy(&allow(), Level::Full, RiskTier::Tier1),
            AutonomyOutcome::AutoSend
        );
        assert_eq!(
            autonomy_policy(&allow(), Level::Restricted, RiskTier::Tier1),
            AutonomyOutcome::ReviewRequired
        );
        // Tier0: auto from Restricted up, review below.
        assert_eq!(
            autonomy_policy(&allow(), Level::Restricted, RiskTier::Tier0),
            AutonomyOutcome::AutoSend
        );
        assert_eq!(
            autonomy_policy(&allow(), Level::ReadOnly, RiskTier::Tier0),
            AutonomyOutcome::ReviewRequired
        );
    }
}
