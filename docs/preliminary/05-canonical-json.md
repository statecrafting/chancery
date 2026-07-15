# 05: canonical-json library (PRELIMINARY)

> Repo/crate `canonical-json`. Extracted from OAP
> `crates/canonical-json/src/lib.rs` (124 LoC, 0 OAP-coupling). The leaf
> dependency: `attest-ledger` and `action-gate` both need it for
> reproducible hashing and canonical serialization. Extract it first.

## What it is

A single function that recursively lex-sorts object keys at the JSON
serialization boundary, so a value serializes byte-identically regardless of
`serde_json`'s `preserve_order` feature state elsewhere in the dependency
graph. This is the substrate that makes every downstream hash reproducible:
if two producers serialize the same logical value differently, their record
hashes diverge and the ledger's tamper-evidence breaks.

```rust
// the whole public surface today (OAP crates/canonical-json)
pub fn canonicalize_value(v: Value) -> Value; // recursive object-key sort; arrays/scalars pass through
```

## Why it must be its own crate

`serde_json`'s `preserve_order` unifies monotonically under Cargo resolver 2:
any crate in the graph enabling it flips `serde_json::Map` from lexicographic
to insertion order for every dependent. OAP hit exactly this (`crates/xray`
enables `preserve_order`). The only robust fix is explicit canonicalization at
the emission boundary, in a shared crate every hashing consumer routes
through. Inlining it per library would risk two libraries disagreeing on
canonical form; a shared crate guarantees one canonical form ecosystem-wide.

Note: OAP today has **two** copies of this logic (the standalone
`crates/canonical-json` and an inline `canonical_json_sorted` inside
`policy-kernel/src/lib.rs`). The extraction unifies both on this crate; the
OAP re-consumption step deletes the inline copy.

## Scope for the extracted crate

- Carry `canonicalize_value` verbatim (it is already domain-neutral) plus the
  six existing tests.
- Add one convenience: `to_canonical_string(&Value) -> String` (canonicalize
  then `serde_json::to_string`) since every consumer immediately serializes.
- Add a byte-equality determinism test asserting stable output across a
  shuffled-key input, so `determinism.yml` (the ecosystem CI golden gate) has
  something to guard. This closes the C3 gap flagged in the OAP review: prove
  byte-stability, do not just assert it in prose.
- **No** SHA-256 helper here. Hashing lives in `attest-ledger` (which owns
  `sha256_hex` and the record-hash construction). This crate only guarantees
  the canonical byte string; what you hash it with is the consumer's concern.

## Consumers

- `attest-ledger`: `to_canonical_string(record_without_hash)` then SHA-256 to
  compute `record_hash`. Reproducibility of the whole ledger rests here.
- `action-gate`: canonical serialization of `Decision` for stable
  `config_hash` and for recording into the ledger payload.
- OAP re-consumes: `policy-kernel` drops its inline sorter and depends on this
  crate; `spec-spine` / tenant-emit / tenant-tail can adopt it too (they each
  do canonical hashing today).

## Shape

Single crate (no `types` / `core` / `cli` split; there is no CLI and the type
surface is `serde_json::Value`). Repo still carries the ecosystem
self-governance skeleton (`spec-spine.toml`, `specs/000-bootstrap`,
`standards/`, committed `.derived/`, `determinism.yml`) so it is a first-class
member, not a loose crate.

## Resolved: crates.io name

`canonical-json` (and its normalized twin `canonical_json`) is **taken** by
Mozilla's `canonicaljson-rs`, which implements gibson042's Canonical JSON spec:
it escapes non-ASCII to `\uXXXX` and normalizes floats to scientific notation,
so it is **byte-incompatible** with this minimal key-sort (and would break OAP
`proof_chain`/`audit` hash parity if adopted, plus add `regex`+`hex` deps).

Decision: keep this implementation, publish as **`canonical-keysort-json`**
(import `canonical_keysort_json`). The name carries both the `canonical`
identity and the honest `keysort` signal (key-ordering only, not full canonical
JSON). The repo directory and spec-governance namespace stay `canonical-json`;
only the published package name and import path differ.
