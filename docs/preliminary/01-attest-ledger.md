# 01: attest-ledger library (PRELIMINARY)

> Working name `attest-ledger`. Extracted from OAP
> `crates/policy-kernel/src/proof_chain.rs` (596 LoC) and `audit.rs`
> (876 LoC). Coupling to OAP domain: 6 field-name references each; the
> chain, signing, and verify mechanics are domain-neutral.

## What it is

A tamper-evident record ledger: append-only, hash-linked, Ed25519-signed,
with an independent verifier that does not trust the producer. It is the
**run-time** counterpart to `spec-spine`'s build-time registry: same
primitive (typed, hash-verified, append-only), different clock. chancery's
`Decision` artifact is one record in this ledger; OAP's `ProofRecord` is
another.

Two layers, both already present in OAP and both worth carrying over:

- **Record chain** (`proof_chain.rs`): individual records, each hashing the
  canonical JSON of the prior record's hash into its own; a signed genesis
  anchor pins the chain to a bundle/context and is Ed25519-signed.
- **Audit segments** (`audit.rs`): a rotating, size-bounded, hash-chained
  segment log for higher-volume append (the `AuditLogger` OAP uses for
  per-dispatch audit lines), with its own `verify_audit_chain`.

## The generalization (what changes on extraction)

OAP's `ProofRecord` bakes gate semantics into the record shape:

```rust
// OAP today (proof_chain.rs:48): gate-specific fields inline.
pub struct ProofRecord {
    pub id: String,
    pub timestamp: String,
    pub policy_bundle_hash: String,   // gate-specific
    pub rule_ids: Vec<String>,        // gate-specific
    pub input_context_hash: String,   // gate-specific
    pub decision: ProofRecordDecision,
    pub privilege_level: ProofPrivilege,
    pub previous_record_hash: String, // chain mechanism (generic)
    pub record_hash: String,          // chain mechanism (generic)
}
```

Extraction splits the **envelope** (generic) from the **payload** (domain):

```rust
// attest-ledger-types
pub struct LedgerRecord {
    pub id: String,
    pub timestamp: String,             // caller-supplied; no wall clock in core
    pub previous_record_hash: String,  // chain link
    pub record_hash: String,           // hash over canonical JSON of everything above + payload
    pub payload: serde_json::Value,    // opaque, canonical-serialized domain payload
}

pub struct ChainAnchor {               // signed genesis (from ProofChainAnchor)
    pub chain_id: String,
    pub anchor_hash: String,           // was policy_bundle_hash; now a generic "pinned root"
    pub genesis_timestamp: String,
    pub attestation: GenesisAttestation,   // Ed25519 pubkey + signature, or unsigned
}
```

`decision` / `privilege_level` / `rule_ids` / `policy_bundle_hash` move
**into** the `payload` for the consumers that want them. The ledger core no
longer knows they exist. The `NF004_MAX_BYTES_EXCLUDING_CONTEXT` budget and
canonical-JSON hashing carry over unchanged.

## Public API sketch (three crates)

```
attest-ledger-types   LedgerRecord, ChainAnchor, GenesisAttestation, VerifyError
attest-ledger-core    RecordChain::append, compute_record_hash, sha256_hex,
                      verify_chain / verify_chain_with_anchor,
                      sign_anchor / verify_anchor (ed25519),
                      AuditLog::append, verify_audit_chain
attest-ledger-cli     `attest-ledger verify <chain.jsonl> [--anchor a.json] [--require-signed]`
                      `attest-ledger verify-audit <segments-dir>`
```

```rust
// core, the load-bearing calls (names carried from OAP)
impl RecordChain {
    pub fn append(&mut self, id: String, timestamp: String, payload: Value) -> LedgerRecord;
    pub fn last_link_hash(&self) -> &str;
    pub fn build_anchor(&self, chain_id: String, genesis_timestamp: String) -> ChainAnchor;
}
pub fn verify_chain(records: &[LedgerRecord]) -> Result<(), VerifyError>;
pub fn verify_chain_with_anchor(anchor: &ChainAnchor, records: &[LedgerRecord]) -> Result<(), VerifyError>;
```

Determinism discipline (carried from OAP, and the C3 lesson from the OAP
review): the core takes **all** inputs including `timestamp` as arguments
(no `Date.now()`, no wall clock), so `compute_record_hash` is a pure
function of its inputs and the verifier is reproducible. Add a byte-equality
golden test in the extracted repo so "same inputs, same hash on every
platform" is tested, not just claimed.

## How chancery maps onto it

`Decision` = `LedgerRecord` whose `payload` is a `MessageDecision`:

| `LedgerRecord` | chancery `Decision` |
|---|---|
| `record_hash` / `previous_record_hash` | the per-thread hash link |
| `payload.context_hash` | hash of the exact `AgentContext` bundle |
| `payload.play_hash` | hash of the play-spec that authorized the send |
| `payload.gate_verdict` | the `action-gate` `Decision` (allow/deny/degrade + check ids) |
| `payload.model_snapshot`, `payload.approver` | provenance |
| `ChainAnchor` | pins the thread's chain to the deployment's signing key |

"Every send is reconstructable to context + prompt + model + guardrail
verdict + approver" is then a property of the ledger, verified offline by
`attest-ledger verify`, not bespoke chancery code.

## How OAP re-consumes it

OAP's `policy-kernel::proof_chain` becomes a thin module that constructs
`LedgerRecord` with an OAP payload (`{policy_bundle_hash, rule_ids,
input_context_hash, decision, privilege_level}`). `verify_proof_chain` /
`verify_audit_chain` binaries re-target the extracted core. No behavior
change; OAP's existing proof-chain tests are the regression guard.

## Open questions

- **Typed vs opaque payload.** `payload: serde_json::Value` (opaque) keeps
  the core domain-free but loses compile-time payload typing. Alternative:
  `RecordChain<P: Serialize>` generic over the payload type. Generic is
  nicer for consumers but complicates the CLI verifier (which must verify
  chains whose payload type it does not know). Recommendation: opaque
  `Value` in the core + verifier; consumers wrap with typed helpers.
- **Storage.** The core is storage-agnostic (produces/verifies records).
  Persistence (JSONL files like OAP, or Postgres append-only tables like
  chancery) is the consumer's concern. Do not put a DB in the library.
- **canonical-json** dependency (Open Decision 3 in `00`): the hash is only
  reproducible with deterministic key-sorted serialization.
