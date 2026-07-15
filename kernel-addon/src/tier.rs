// SPDX-License-Identifier: Apache-2.0

//! Risk tier of a proposed send.

use serde::{Deserialize, Serialize};

/// Risk tier of a proposed send, one input to the autonomy policy alongside the
/// gate decision and the play's trust level.
///
/// This is the C0 strawman taxonomy; the chancery domain spec (WU-F) refines
/// it. The behavioural contract each tier makes with
/// [`autonomy_policy`](crate::autonomy::autonomy_policy):
///
/// - [`Tier0`](RiskTier::Tier0): warm / existing thread, non-first-touch.
///   Auto-eligible from `Restricted` trust upward.
/// - [`Tier1`](RiskTier::Tier1): standard routine outreach. Auto-eligible only
///   at `Full` trust.
/// - [`Tier2`](RiskTier::Tier2): elevated (first-touch, or a claim referencing
///   specifics). Review by default.
/// - [`Tier3`](RiskTier::Tier3): pinned (top-tier first-touch, or anything
///   touching terms / legal / financial commitments). Always review, never
///   auto, regardless of trust ([`is_hard_pinned`](RiskTier::is_hard_pinned)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskTier {
    Tier0,
    Tier1,
    Tier2,
    Tier3,
}

impl RiskTier {
    /// A hard pin can never be auto-sent, whatever the trust level. Used both by
    /// [`autonomy_policy`](crate::autonomy::autonomy_policy) and by the ladder,
    /// which should not bother recording promotion samples for a pinned segment.
    #[inline]
    pub fn is_hard_pinned(self) -> bool {
        matches!(self, RiskTier::Tier3)
    }
}
