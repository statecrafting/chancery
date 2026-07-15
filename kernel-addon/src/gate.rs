// SPDX-License-Identifier: Apache-2.0

//! Assembling the chancery gate from its checks.

use action_gate_core::{Gate, checks::SecretsCheck};
use serde::{Deserialize, Serialize};

use crate::checks::{FatigueCheck, GroundingCheck, InjectionCheck, SuppressionCheck, ToneCheck};

/// Parameters that configure the chancery gate. Only genuine gate-configuration
/// values live here; per-request signals ride in the
/// [`ActionContext`](action_gate_core::ActionContext) attributes instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateParams {
    /// Per-investor contact budget for the fatigue check.
    pub fatigue_budget: i64,
}

impl Default for GateParams {
    fn default() -> Self {
        Self { fatigue_budget: 5 }
    }
}

/// Assemble the chancery gate. The registration order is the short-circuit
/// order, so the harder, blocking stops come first:
///
/// 1. `suppression` (blocking `Deny`)  hard, unconditional stop
/// 2. `secrets` (`Deny`)               credential leak in the draft
/// 3. `injection` (`Deny`)             inbound-content boundary violation
/// 4. `grounding` (`Deny` / `Degrade`) unsupported or unverified claim
/// 5. `fatigue` (`Degrade`)            over the contact budget
/// 6. `tone` (`Degrade`)               tone / style violation
pub fn assemble_gate(params: &GateParams) -> Gate {
    Gate::builder()
        .check(SuppressionCheck)
        .check(SecretsCheck::default())
        .check(InjectionCheck)
        .check(GroundingCheck)
        .check(FatigueCheck::new(params.fatigue_budget))
        .check(ToneCheck)
        .build()
}
