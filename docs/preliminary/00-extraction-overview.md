# 00: Reusable-primitive extraction overview (PRELIMINARY)

> Status: preliminary draft. Not yet a spec-spine spec. Lives in
> `docs/preliminary/` until the crate boundaries are settled, then each
> library graduates to its own governed `specs/` corpus in its own repo.
>
> Scope: what to extract from `open-agentic-platform` (OAP) into standalone
> reusable libraries **before** chancery-specific work begins. chancery is
> application-zero of these libraries, not their owner.

## Why this comes first

chancery's runtime governance (gate a message send, record the Decision,
accrue play trust) is not chancery-specific. The same primitives are already
implemented inside OAP's `crates/policy-kernel/`, welded together with
OAP-domain logic. Extracting the domain-neutral cores into their own
Apache-2.0 libraries buys three things at once:

1. **chancery imports instead of reinvents.** The hard parts (a signed
   hash-linked ledger with an independent verifier; a deterministic gate;
   a rolling-window trust scorer) already exist and are tested.
2. **OAP gets better.** OAP becomes consumer-zero of the extracted crates
   (its own `policy-kernel` collapses to a thin shim of OAP-specific checks
   over the shared cores). This is the exact pattern OAP already follows
   with the external `spec-spine` CLI.
3. **The ecosystem gets primitives.** tenant-emit, tenant-tail, and future
   tenant apps can all consume the same ledger, gate, and canonical-json.

## What is extractable, with evidence

Coupling measured by counting OAP-domain term hits (`spec_status`,
`tool_name`, `diff_lines`, `mcp`, `factory`, `shard`, ...) per source module
in `crates/policy-kernel/src/`:

| Source | LoC | OAP-coupling hits | Extractable as | Confidence |
|---|---:|---:|---|---|
| `crates/canonical-json/src/lib.rs` | 124 | 0 | `canonical-json` | Very high (already standalone) |
| `proof_chain.rs` | 596 | 6 (field names only) | `attest-ledger` (core) | High |
| `audit.rs` | 876 | 6 (field names only) | `attest-ledger` (audit segments) | High |
| `coherence.rs` | 272 | **0** | `trust-window` | Very high |
| `lib.rs` (`evaluate` + 6 gates) | 792 | 149 | `action-gate` (shell generic; 6 checks stay in OAP) | Medium (needs Case-B refactor) |
| `permission.rs`, `merge.rs`, `settings.rs`, `watcher.rs`, `denial.rs` | ~1134 | spec-068 family, **unwired** | **do not extract** (stays OAP) | n/a |
| `provenance_policy.rs` | 112 | spec-121, OAP-specific | **do not extract** | n/a |

## What happens to `policy-kernel` (answering "why don't I see it?")

`policy-kernel` is not one of the extracted libraries because **it is not one
thing**. In OAP it is a grab-bag crate that fuses six separable concerns:

1. a gate / decision engine (`evaluate` + the six gate functions),
2. a signed hash-linked ledger (`proof_chain.rs`),
3. a rotating audit chain (`audit.rs`),
4. a rolling-window trust scorer (`coherence.rs`),
5. OAP's own six domain checks (destructive-op, spec-status, spec-risk,
   diff-size, plus the two generic ones),
6. a pile of **unwired** spec-068 permission-runtime code
   (`permission.rs`, `merge.rs`, `settings.rs`, `watcher.rs`, `denial.rs`).

The extraction **dissolves** it: the four reusable primitives (1, 2+3, 4, and
the canonical-JSON helper it leans on) become `action-gate`, `attest-ledger`,
`trust-window`, and `canonical-json`. What is left (5 and 6) is OAP's own
**deployment config**: its specific check-set plus dead code. That stays
inside OAP as an internal crate (keep the name `policy-kernel` or rename to
`oap-checks`), consuming `action-gate` and registering OAP's six checks. It is
the direct analog of chancery's own private checks crate. It is not a shared
library because there is nothing domain-neutral left to share.

So the "kernel" concept does survive extraction: it becomes **`action-gate`**,
the pure decision core. The name `policy-kernel` was OAP-flavored ("policy"
= spec/rule governance); the generalized crate evaluates any action against
any registered checks, so `action-gate` is the honest domain-neutral name. If
you would rather the gate crate keep the name `policy-kernel`, that is
available, but it re-imports OAP's framing into a domain-neutral library.

## The four libraries (DECIDED)

Four standalone repos in the `stagecraft-ing/*` org, **Apache-2.0**, edition
2024, self-governed by their own `specs/` corpus compiled by a pinned
`spec-spine` (the tenant-emit / tenant-tail dogfooding pattern). Crate count
per repo follows need: the CLI-bearing libraries use the three-crate
`types` / `core` / `cli` shape; the pure libraries are single-crate.

| Repo / crate | From | Shape | One line | Doc |
|---|---|---|---|---|
| `canonical-json` | `crates/canonical-json` | single crate | deterministic recursive lex-sort of object keys at the serialization boundary | `05-canonical-json.md` |
| `attest-ledger` | `proof_chain.rs` + `audit.rs` | types/core/cli | append-only, Ed25519-signed, hash-linked record chain + rotating audit segments + independent verifier | `01-attest-ledger.md` |
| `action-gate` | `lib.rs` (generic shell) | types/core (+cli?) | pure `evaluate(ActionContext, checks) -> Allow/Deny/Degrade` over a pluggable `Check` registry | `02-action-gate.md` |
| `trust-window` | `coherence.rs` | single crate | rolling-window sample stream -> coherence score -> graduated privilege level | `03-trust-window.md` |

Dependency graph (this sets build order):

```
canonical-json  (leaf, no internal deps)
   ^        ^
   |        |
attest-ledger   action-gate       trust-window  (no canonical-json dep; serde only)
```

Design intent: the libraries stay **independent**. The ledger does not depend
on the gate; the gate is a pure function that does not depend on the ledger;
trust is a standalone scorer. `attest-ledger` and `action-gate` both depend on
`canonical-json` for reproducible hashing / serialization. Consumers (OAP's
axiomregent router, chancery's outbox) **compose** them.

## Packaging: four separate repos (DECIDED)

Four repos, one per primitive, exactly like `tenant-emit` / `tenant-tail`.
Maximum independence and per-primitive versioning, at the cost of 4x repo
overhead (4 CI setups, 4 `spec-spine.toml`, 4 release cadences). Each repo
carries the ecosystem's self-governance skeleton (`spec-spine.toml`, `specs/`,
`standards/`, committed `.derived/` shards, `.github/workflows` including a
`determinism.yml` golden gate, the npm binary-distribution shim where a CLI
ships).

## Relicensing (Apache-2.0)

Each extracted file carries an `AGPL-3.0-or-later` SPDX header in OAP.
Relicensing this extracted source to Apache-2.0 is the prerogative of the sole
copyright holder (Bartek Kus) and is an explicit, authorized act, recorded in
each repo's `NOTICE` exactly as `tenant-tail` / `tenant-emit` already do. A
forthcoming OAP extraction spec (analogous to spec 219 for tenant-tail)
formalizes the vend; until then these preliminary docs are the record.

## Dogfooding: OAP stays consumer-zero

The extraction is a net-positive refactor of OAP, not a fork:

1. Extract `canonical-json`, `attest-ledger`, `trust-window` (near-zero
   coupling) first.
2. Rewrite OAP's `policy-kernel::{proof_chain, audit, coherence}` and the
   inline `canonical_json_sorted` helper to re-export / thin-wrap the
   extracted crates. OAP's verifier binaries keep working.
3. Do the Case-B `action-gate` extraction: OAP's six gates become six
   `impl Check` that live in OAP and register into the shared `Gate`.
4. Only then does chancery import the crates and register its own checks /
   payloads.

## Sequencing

```
Phase L-1  Extract canonical-json  (leaf; unblocks the ledger + gate)   -> repo, crate green
Phase L0   Extract attest-ledger   (proof_chain + audit)                -> repo + specs, verifier CLI green
Phase L1   Extract trust-window    (coherence, 0 coupling)              -> repo + specs
Phase L2   Extract action-gate     (Case-B: Check registry)             -> repo + specs; OAP re-consumes all four
Phase C0   chancery imports the four; registers grounding/suppression/fatigue checks,
           MessageDecision ledger payload, play-scoped autonomy ladder on trust-window
```

L-1, L0, L1 are low-risk and independently valuable to OAP even if chancery
never ships. L2 carries the real design work. C0 is the first chancery-owned
code and waits on L-1..L2.

## What does NOT extract (guard against over-reach)

- The spec-068 permission-runtime family (`permission.rs`, `merge.rs`,
  `settings.rs`, `watcher.rs`, `denial.rs`): 5-tier settings merge with zero
  non-test callers in OAP. Unwired and OAP-shaped. Leave it in OAP.
- `provenance_policy.rs` (spec 121): OAP workspace-provenance specific.
- `PolicyRule` / `PolicyBundle` (constitution + shards): the OAP config
  model. The generic gate does not adopt it; checks self-configure
  (see `02-action-gate.md`).

## Resolved decisions

1. **Packaging**: four separate repos (`canonical-json`, `attest-ledger`,
   `action-gate`, `trust-window`). DECIDED.
2. **Names**: `canonical-json`, `attest-ledger`, `action-gate`,
   `trust-window`. DECIDED (open sub-question: keep `action-gate` vs revive
   `policy-kernel` for the gate crate; see the policy-kernel section).
3. **canonical-json**: its own crate, not inlined. DECIDED. Published on
   crates.io as `canonical-keysort-json` (plain `canonical-json` is taken by
   Mozilla's byte-incompatible gibson-spec crate); repo + spec namespace stay
   `canonical-json`. See `05-canonical-json.md`.
4. **License**: Apache-2.0 for all four, matching tenant-emit / tenant-tail
   and enabling the open-core boundary (permissive engine, private config).
   DECIDED.

See `04-deployment-and-portability.md` for the deployment posture, which is a
chancery-service concern and does not touch these libraries.
