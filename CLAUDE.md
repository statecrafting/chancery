# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

`chancery` is the governance kernel for a message-decision engine (fundraising
outreach: gate a message send, record a signed Decision, accrue per-play trust).
It does not reinvent that machinery; it **composes four extracted Apache-2.0
primitives** into chancery's domain policy:

- **`action-gate-core`**: the pure decision gate. `evaluate(ActionContext, checks) -> Allow/Deny/Degrade`.
- **`attest-ledger-core`**: the Ed25519-signed, hash-linked decision ledger + independent verifier.
- **`trust-window`**: the rolling-window trust scorer (sample stream -> score -> privilege `Level`).
- **`canonical-keysort-json`**: deterministic key-sorted serialization the ledger hashes through (transitive).

Each primitive has its own repo and is also cloned locally under `~/DevWork/`
(`action-gate`, `attest-ledger`, `trust-window`, `canonical-keysort-json`); read
those sources when you need a primitive's exact API. chancery is
application-zero of them, not their owner.

### Current state (read this before assuming files exist)

Only **WU-B is implemented**: the `kernel-addon/` Rust crate (the pure,
golden-tested core + its napi surface). The wider plan (`docs/plans/C0-chancery-kernel.md`)
stamps the **enrahitu** chassis (Encore.ts + rauthy + hiqlite + libSQL) around
it, adding `backend/kernel/` (a thin Encore service), CoreLedger entities, and TS
orchestration. **None of that chassis code is in the repo yet.** `docs/preliminary/`
holds the superseded pre-chassis design; `docs/plans/C0-chancery-kernel.md` is the
live plan and its "Status" section is the source of truth for what is done.

## Commands

All Rust commands run from `kernel-addon/` (the only cargo project). Toolchain is
pinned to `1.92.0` via `rust-toolchain.toml`; rustup auto-installs it.

```bash
cargo test --locked                 # 22 tests: 11 unit (incl. the wire boundary) + 11 integration
cargo test --locked --test composition            # just the composition + golden-determinism suite
cargo test --locked golden_hashes_are_stable      # a single test by name

cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings                 # pure core + wire
cargo clippy --all-targets --features napi --locked -- -D warnings # + the napi surface

cargo build --locked                # pure core build

# napi addon (needs the chassis npm toolchain / @napi-rs/cli):
npm run build                       # napi build --platform --release --features napi -> prebuilt .node + index.d.ts
npm run build:debug
```

CI (`.github/workflows/ci.yml`) runs fmt + both clippy modes + build + test.
`determinism.yml` runs `cargo test --test composition` across Linux/macOS/Windows.

## Architecture and invariants

### The napi feature is opt-in, and that shapes everything

`default = []` (napi **off**). A napi crate cannot link a test executable (Node
supplies the Node-API symbols at load time), so:

- **`cargo test` builds the pure core + the napi-free `wire` boundary** and runs clean.
- The `#[napi]` surface (`src/napi_api.rs`) is built only with `--features napi`
  and is **clippy-checked, never unit-tested**.
- **Therefore all real logic must live in `wire.rs`**, whose functions are pure
  `&str -> Result<String, String>` with no napi types. `napi_api.rs` is a thin
  delegator that only maps the `String` error onto `napi::Error`. Do not put
  behaviour in the napi shim; it would be untestable. `build.rs` calls
  `napi_build::setup()` only when `CARGO_FEATURE_NAPI` is set.

### The kernel is a pure function of its inputs

No host calls, no wall clock, no database. **Timestamps are caller-supplied**;
the previous-record hash is passed in (`build_record(prev_hash, ...)`), not held.
Persistence (the ledger chain, per-play trust snapshots) and pre-computing
I/O-bound gate signals (grounding entailment, suppression membership, the fatigue
count) are the **TypeScript consumer's job**. This purity is exactly what makes
the core byte-deterministic and golden-testable across platforms. Preserve it:
never introduce `SystemTime`, RNG, or I/O into the crate.

### Data flow through the modules (`src/`)

```
SendContext (context.rs)            chancery-shaped send; pre-computed guardrail signals as fields
   -> to_action_context             maps typed extras to ActionContext attributes (the attr:: contract in checks/mod.rs)
   -> assemble_gate (gate.rs)       registers the 6 checks in short-circuit order
   -> gate.evaluate -> Decision     first check to fire wins
   -> autonomy_policy (autonomy.rs) fuses (Decision, trust Level, RiskTier) -> AutoSend/ReviewRequired/Blocked
   -> MessageDecision (payload.rs)  the ledger payload; build_record / verify_message_chain over attest-ledger
```

`ladder.rs` is the separate trust path: a human `ReviewOutcome` becomes a
`trust_window::Sample`, `score()` runs the bidirectional window over a rehydrated
snapshot, and the new snapshot is returned for the consumer to persist.

### Load-bearing conventions

- **Checks are pure `impl Check`** (`src/checks/`). They read only pre-computed
  `ActionContext` attributes (keys defined in `checks::attr`), never a model or
  the host. Reason codes follow `gate:<outcome>:<check>:<detail>`
  (e.g. `gate:deny:grounding:unsupported_claim`).
- **Gate registration order = short-circuit order.** Blocking/safety checks
  first: suppression (blocking deny) -> secrets -> injection -> grounding ->
  fatigue -> tone. See `assemble_gate` in `gate.rs`.
- **`config_hash` binds a decision to its exact gate config.** Genuine
  configuration (e.g. the fatigue budget) is folded into a check's
  `config_fingerprint()` and thus into `Gate::config_hash()`, which is recorded
  in `MessageDecision.gate_config_hash`. Per-request signals ride in attributes,
  not in `GateParams`.
- **The autonomy policy is deliberately separate from the gate**: the gate
  library does not know about autonomy. A `blocking` decision forces `Blocked`
  regardless of trust/tier.

### The golden determinism gate

`tests/composition.rs` pins `GOLDEN_GATE_CONFIG_HASH` and `GOLDEN_RECORD_HASH`
for a fixed input. If you change gate composition, check semantics, payload
shape, or serialization, these hashes drift. A drift is either an **intended
semantics change** (update the constants deliberately, in the same commit) or an
**accidental byte-level regression** (fix the cause). `determinism.yml` re-checks
them on all three OSes, so the update must be genuinely platform-stable.

## House rules

Apache-2.0; every source file carries an `SPDX-License-Identifier: Apache-2.0`
header. Keep the pinned primitive versions (`0.1`) in sync with what the composed
hashes were computed against. Governance graduation (chancery domain specs under
`specs/` via spec-spine) is deferred to WU-F, per the C0 plan.
