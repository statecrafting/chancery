# 02: action-gate library (PRELIMINARY)

> Working name `action-gate`. Extracted from OAP
> `crates/policy-kernel/src/lib.rs` (792 LoC, 149 OAP-coupling hits). This
> is the Case-B extraction: the gate machinery is generic, but the six
> checks are OAP-domain and must be lifted into a pluggable registry.

## The problem being fixed

OAP's `evaluate()` hardcodes six checks as private functions in a fixed
order and takes a code-governance-shaped context:

```rust
// OAP today (lib.rs:117): six gates, hardcoded, private, non-extensible.
pub fn evaluate(ctx: &ToolCallContext, bundle: &PolicyBundle) -> PolicyDecision {
    if let Some(d) = gate_secrets_scanner(ctx, bundle) { return d; }
    if let Some(d) = gate_destructive_operation(ctx, bundle) { return d; }
    if let Some(d) = gate_tool_allowlist(ctx, bundle) { return d; }
    if let Some(d) = gate_spec_status(ctx, bundle) { return d; }        // OAP-specific
    if let Some(d) = gate_spec_risk(ctx, bundle) { return d; }          // OAP-specific
    if let Some(d) = gate_diff_size_limiter(ctx, bundle) { return d; }  // code-specific
    PolicyDecision::allow()
}
```

You cannot register "grounding against a fact sheet" without editing this
function. And `ToolCallContext` carries `diff_lines`, `spec_statuses`,
`feature_ids`, etc. To reuse the gate for message sends, the check set and
the context must both become open.

## The generalization

Three moves. The outcome enum and the pure-function property are preserved.

**1. A domain-neutral context.** Replace `ToolCallContext` with:

```rust
// action-gate-types
pub struct ActionContext {
    pub action: String,               // was tool_name; "email.send", "workspace.write_file"
    pub payload_summary: String,      // was arguments_summary; canonical string for scanning
    pub payload_body: Option<String>, // was proposed_file_content; the content under review
    pub attributes: BTreeMap<String, serde_json::Value>, // everything domain-specific
}
```

OAP's `diff_lines`, `spec_statuses`, `max_spec_risk` become entries in
`attributes` that OAP's own checks read. chancery's checks read
`attributes["fact_sheet_id"]`, `attributes["investor_id"]`, etc. The core
never inspects `attributes`; only checks do.

**2. A `Check` trait + ordered registry.**

```rust
pub enum Outcome { Allow, Deny, Degrade }

pub struct Decision {
    pub outcome: Outcome,
    pub reason: String,     // stable code, e.g. "gate:deny:grounding:unsupported_claim"
    pub check_ids: Vec<String>,
    pub blocking: bool,     // forces block regardless of downstream trust/consumer policy
}

pub trait Check: Send + Sync {
    fn id(&self) -> &str;
    /// Return Some(decision) to short-circuit, None to pass to the next check.
    fn evaluate(&self, ctx: &ActionContext) -> Option<Decision>;
}

pub struct Gate { checks: Vec<Box<dyn Check>> }
impl Gate {
    pub fn builder() -> GateBuilder;
    pub fn evaluate(&self, ctx: &ActionContext) -> Decision; // first Some wins; else Allow
}
```

`Gate::evaluate` is a **pure function** of `(ctx, checks)`: deterministic,
no host calls, unit-testable, byte-stable via a `decision_to_canonical_json`
carried from OAP. Determinism now depends on check ordering being stable,
which the builder guarantees (insertion order).

**3. Config lives in the checks, not a global bundle.** OAP's `PolicyBundle`
(constitution + shards of `PolicyRule`) is OAP's config model. Do not adopt
it into the generic library. Each check is constructed with its own params
(a `SecretsCheck` owns its regex set; an `AllowlistCheck` owns its list).
The consumer assembles the gate:

```rust
let gate = Gate::builder()
    .check(SecretsCheck::default())
    .check(AllowlistCheck::new(allowed))
    .check(GroundingCheck::new(fact_sheet))   // chancery
    .build();
```

If a consumer wants OAP-style compiled bundles, that is a consumer-side
loader that constructs checks from a bundle file; it is not in the core.

## What the library ships vs what consumers register

- **Library core**: `ActionContext`, `Decision`, `Outcome`, `Check`, `Gate`,
  `GateBuilder`, canonical serialization. Plus, optionally, two genuinely
  generic reference checks that many consumers want: `SecretsCheck` and
  `AllowlistCheck` (both already domain-neutral in OAP today). Ship these as
  an optional `checks-common` feature, not baked into the core.
- **OAP registers**: `DestructiveOpCheck`, `SpecStatusCheck`, `SpecRiskCheck`,
  `DiffSizeCheck` (its four domain checks) plus the two common ones. OAP's
  `evaluate()` becomes `oap_gate().evaluate(ctx)`.
- **chancery registers**: `GroundingCheck` (draft claims entailed by the
  approved fact sheet, the highest-stakes guardrail), `SuppressionCheck`
  (global unsubscribe/bounce/DNC), `FatigueCheck` (the global per-investor
  contact budget), `InjectionCheck` (inbound-content-as-data boundary),
  `ToneCheck`, plus the common ones.

## Degrade is a gate outcome, not a gate action (important)

The gate returns `Degrade`; it does **not** perform a degraded action. In
OAP the router collapses `Degrade` to a hard deny; the unused tool-registry
bridge maps it to a confirmation prompt. Neither executes a reduced action.
Keep this honest in the library: `Outcome::Degrade` is a signal, and the
**consumer** decides its meaning. chancery maps `Degrade -> review_required`
(route to a human) and `Deny -> blocked`; that mapping is chancery policy,
layered on top of the pure gate, not inside it. Document this so nobody
claims the library "degrades" anything.

## Interaction with trust-window (the autonomy dimension)

The gate is per-check (allow/deny/degrade). The **autonomy** decision
(auto_send vs review_required vs blocked) is a separate function of
`(gate Decision, play trust level, risk tier)`. Keep it out of the gate
library. In chancery it is a small `autonomy_policy(decision, trust, tier)`
that sits between the gate and the outbox, and the `trust` input comes from
`trust-window` (`03`). A `blocking: true` check overrides any trust level.

## Open questions

- **Async checks.** OAP's live checks are synchronous and pure. chancery's
  `GroundingCheck` may call an entailment model (async, fallible). Options:
  keep `Check` sync and pre-compute grounding into `attributes` before the
  gate runs (preserves purity and determinism, recommended), or add a
  parallel `AsyncCheck` trait. Recommendation: keep the gate pure; do model
  calls in the context-assembly step, feed results in as attributes.
- **Check-set versioning.** For the ledger to prove "this decision came from
  this gate configuration," the gate needs a stable hash of its check set +
  params. Add `Gate::config_hash()`; record it in the ledger payload.
