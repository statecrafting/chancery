# C0: chancery kernel plan (chancery imports the four primitives)

> Status: execution plan (outline), v2. Successor to the `docs/preliminary/00-05`
> design docs, **rebased onto the `enrahitu` chassis** (see §A). Those docs
> settled the four extracted libraries; this doc plans the first
> chancery-owned code that composes them, as an app stamped from enrahitu.
>
> Scope of C0 (from `preliminary/00` sequencing): "chancery imports the four;
> registers grounding/suppression/fatigue checks, MessageDecision ledger
> payload, play-scoped autonomy ladder on trust-window." L-1..L2 are done:
> OAP PR #572 merged, OAP is consumer-zero of all four.

## A. Template rebase (what changed, and why)

C0 is now built on the **`enrahitu`** chassis (`~/DevWork/enrahitu`,
`statecrafting/enrahitu`): **En**core.ts + **ra**uthy + **hi**qlite +
**Tu**rso/libSQL. One Docker image + one volume = a complete authenticated
app, zero managed-infrastructure dependencies. This **supersedes the
`preliminary/04` deployment design** on several load-bearing points:

| `preliminary/04` (old) | enrahitu chassis (now) |
|---|---|
| Two services: `conv-api` (Encore.ts) + `conv-kernel` (Rust axum) | **One** enrahitu app; the Rust kernel is an **in-process napi-rs addon**, not a separate service |
| Postgres append tables for the ledger | **CoreLedger on libSQL/SQLite** (`@Entity`/`@Column` decorators); **no Postgres** (a `postgres.ts` driver exists in the chassis but stays an unused scale-time swap) |
| NSQ / Encore pubsub outbox | No pubsub in the chassis today; async/outbox is C1+ (hiqlite `listen_notify` is a "later" addon feature) |
| Rauthy JWT auth copied into a bespoke axum service | Auth/identity inherited from the chassis (`backend/auth`, `backend/idp`, rauthy same-origin) |
| Kubernetes / Terraform / Flux | Single container + one volume; deployment is chassis-owned, C1+ |

**Consequence for decision #4 (naming).** There is no separate `conv-kernel`
axum service to host a `chancery-kernel` library. Adjusted: the kernel is a
**native addon** `@chancery/kernel-native` (Rust crate `chancery-kernel`),
sibling to the chassis `addon/`; a thin Encore service `backend/kernel/` wraps
it exactly as `backend/hiq/` wraps the hiqlite addon. `conv-api` disappears
into the enrahitu backend. This is the "adjust accordingly" the template
change requires; everything else about #4 (a kernel unit distinct from its
service wrapper) still holds.

**Consequence for decision #2 (governance timing).** Stamping enrahitu gives
chancery the spec-spine governance skeleton *on day one* (the chassis carries
`spec-spine.toml`, `standards/`, `specs/000-019`, committed `.derived/`, the
coupling gate). So nothing about the *skeleton* is deferred. What is deferred
to end-of-C0 is authoring chancery's **own domain specs** (kernel, checks,
ladder) on top of the inherited chassis specs, once the shapes stabilize. #2
still holds, reframed: inherit the skeleton at stamp, author domain specs last.

## B. Locked decisions (this session)

1. **License: Apache-2.0** (matches the chassis; `vendor/encore/` stays MPL at
   file level, inherited).
2. **Governance timing:** inherit skeleton at stamp; author chancery domain
   specs at end of C0. Confirmed (reframed per §A).
3. **Crate layout:** single `chancery-kernel` crate (the addon). Confirmed.
4. **Naming:** `@chancery/kernel-native` addon + `backend/kernel/` service
   (adjusted from `chancery-kernel` lib + `conv-kernel` axum, per §A).
5. **Ladder segment: `(channel, intent)`** to start.
6. **Grounding pre-computed into attributes** before the gate (agreed).
7. **RiskTier taxonomy: open.** A strawman is proposed in §6 to react to.

## Status (2026-07-15)

- **WU-B: DONE and green (pure core + napi addon).** The `chancery-kernel`
  crate lives at `kernel-addon/`. It composes the four published `0.1` crates
  into: the five checks (`suppression`, `grounding`, `injection`, `fatigue`,
  `tone`) + reused `secrets`; `assemble_gate` / `GateParams`; `MessageDecision`
  + `build_record` / `verify_message_chain` over attest-ledger; the
  bidirectional `ladder` (`ReviewOutcome` -> `Sample`, `score`);
  `autonomy_policy` + `RiskTier` (the Tier0-3 strawman); a `wire` module of
  pure JSON-in/JSON-out boundary functions; and a feature-gated `napi_api`
  layer of `#[napi]` delegators over `wire`.
  - **napi is opt-in** (`default = []`): `cargo test` builds the pure core +
    `wire` boundary and runs clean (a napi crate cannot link a test
    executable); the addon builds with `--features napi` (the `napi build`
    script passes it). This keeps the pure core independently verifiable.
  - **Green:** `cargo test` = 22 tests (11 unit incl. `wire` boundary + 11
    integration); `cargo clippy --all-targets` clean both with and without
    `--features napi`; `cargo fmt --check` clean. Addon cdylib builds
    (`--release --features napi` -> 2.0 MB stripped dylib).
  - **Determinism golden pinned** (feeds `determinism.yml`):
    `gate.config_hash = sha256:d85f5aa0...`, `record_hash = sha256:d151dcb3...`.
  - **napi surface:** `evaluate_gate`, `decide_autonomy`, `build_record`,
    `verify_chain`, `score`, `default_window_config`. `index.js` / `index.d.ts`
    are generated by `npm run build` (napi CLI) at chassis integration; not
    committed yet.
- **Not started:** WU-A (stamp enrahitu), WU-C/D (backend/kernel service +
  CoreLedger entities + hiqlite fatigue counter + context assembly + ladder),
  WU-E (e2e), WU-F (domain specs). All best done in a chancery-rooted session
  where the chassis npm/`@napi-rs/cli` toolchain lives.

## 0. Ground truth (verified, not assumed)

### The four primitives (published `0.1`, cloned locally, APIs read from source)

**`canonical-keysort-json` 0.1** (`canonical_keysort_json`)
- `canonicalize_value(Value) -> Value`, `to_canonical_string(&Value) -> String`

**`action-gate-core` 0.1** (+ `-types`)
- `ActionContext { action, payload_summary, payload_body: Option<String>, attributes: BTreeMap<String, Value> }`
  (`new`, `with_summary`, `with_body`, `with_attr`, `attr_str`, `attr`).
- `enum Outcome { Allow, Deny, Degrade }`; `Decision { outcome, reason, check_ids, blocking }`
  (`allow`, `deny`, `degrade`, `blocking`, `is_allow`).
- `trait Check { fn id(&self)->&str; fn evaluate(&self,&ActionContext)->Option<Decision>; fn config_fingerprint(&self)->String { id } }`.
- `Gate { builder(), evaluate(&ctx)->Decision, check_ids(), config_hash()->String }`; `GateBuilder { check<C>(), check_boxed(), build() }`.
- `decision_to_canonical_json(&Decision)->String`, `sha256_hex(&[u8])->String`. Reference checks: `SecretsCheck`, `AllowlistCheck`.

**`attest-ledger-core` 0.1** (+ `-types`, `-cli`)
- `RecordChain::new(anchor_hash)`, `.append(id, timestamp, payload: Value)->LedgerRecord`,
  `.build_anchor(...)`, `.build_anchor_with_key(...)`, `.last_link_hash()`, `.anchor_hash()`.
- `verify_chain(&[LedgerRecord])`, `verify_chain_with_anchor(anchor, records)`.
- `compute_record_hash`, `link_record_hash(value, field)`, `sha256_hex`, `record_payload_bytes`.
- Signing: `sign_anchor`, `verify_anchor`, `resolve_signing_material()->(SigningKey, GenesisAttestation)`. Audit: `AuditChain`, `verify_audit_chain`.
- Independent CLI: `attest-ledger verify <chain.jsonl> [--anchor a.json] [--require-signed]`.

**`trust-window` 0.1**
- `Sample { value, weight }` (`new`, `weighted`, `aligned`); `enum Level`; `LevelThresholds::level_of(score)->Level`.
- `enum Direction { DegradeOnly, Bidirectional }`; `WindowConfig { window_size, direction, decay_lambda, thresholds, violation_threshold, violation_floor, promote_min_samples }`.
- `WindowSnapshot { samples, stuck_severity }` (serde). `WindowScorer::{new, with_defaults, from_snapshot, record, record_aligned, score, violation_count, raw_level, level, restore_max, snapshot}`.

### The enrahitu chassis patterns C0 mirrors (read from source)

- **Native addon** (`addon/`, `@enrahitu/hiqlite-native`): a napi-rs cdylib.
  Exports are `#[napi] pub async fn ...` (e.g. `kv_put`, `counter_add`).
  Deps in `addon/Cargo.toml`: `napi 3` (`napi8`, `tokio_rt`) + `napi-derive`.
  Built with `npm run build:addon`; ships a prebuilt `.node` + `index.d.ts`.
- **Service over addon** (`backend/hiq/`): `init.ts` (start the node at service
  load), `api.ts` (thin Encore `api()` endpoints calling the addon),
  `encore.service.ts`. Very thin; the addon does the work.
- **CoreLedger** (`backend/core/ledger/`): durable data behind stage-3
  `@Entity("table")` / `@Column({type,primary,unique,index,nullable,...})`
  decorators; `ledger().init()`, `ledger().repo(Entity)`, `.insert()`;
  `ensureSchema()` / `migrate()`. Drivers: `LibsqlDriver` (default, local file
  + Turso replica) and `PostgresDriver` (present, **unused for chancery**).
  Real consumer to copy: `backend/auth/entities.ts` (`UserAccount`,
  `RefreshToken`, `AuditLog`).
- **hiqlite counters** (`counter_add`/`counter_get`/`counter_set`): "the
  rate-limit primitive," raft-replicated and atomic. This is the natural
  backing for the fatigue budget.
- **Governance**: `package.json` carries `"spec-spine": { "spec": ... }`;
  `addon/Cargo.toml` carries `[package.metadata.spec-spine]`; edits to a spec
  require `spec-spine compile && spec-spine index` + committing `.derived/`.

## 1. Target architecture (C0 slice)

```
chancery  (stamped from enrahitu; Apache-2.0; single container + one volume)
|
|-- addon/                     inherited @enrahitu/hiqlite-native (KV + counters)
|-- kernel-addon/  (NEW)       @chancery/kernel-native: napi-rs cdylib
|     Cargo.toml               deps: action-gate-core, attest-ledger-core,
|     src/                           trust-window, canonical-keysort-json, napi
|       checks/                 the 5 chancery checks (impl Check, pure)
|       gate.rs payload.rs      assemble_gate, MessageDecision, record build/verify
|       ladder.rs autonomy.rs   window scoring + snapshot; autonomy_policy
|       lib.rs                  the #[napi] surface (stateless pure compute)
|-- backend/
|   |-- kernel/  (NEW)          Encore service over the addon (mirrors hiq/)
|   |     init.ts api.ts        context assembly, evaluate, record, verify
|   |     ladder.ts context.ts  load/persist snapshots, fatigue counter reads
|   |-- core/ledger/            + MessageDecisionRecord, TrustSnapshot entities
|   |-- hiq/                    inherited; counters back FatigueCheck budget
|   |-- auth/ idp/              inherited; rauthy identity = approver
|   `-- health/ web/            inherited
|-- frontend/                   inherited Vue SPA (review UI is C1+)
`-- specs/                      inherited chassis specs 000-019; + chancery domain specs (end of C0)
```

**The addon is stateless pure compute; TypeScript owns orchestration and
persistence.** Rationale: it keeps the addon a pure, byte-deterministic,
golden-testable core (the property that made the four extractions verifiable),
matches enrahitu's "no DB in the addon" posture, and honours
`preliminary/01`'s "storage is the consumer's concern." Concretely:

- **Gate:** TS assembles the `ActionContext` (grounding verdict pre-computed,
  suppression membership resolved, **fatigue count read from the hiqlite
  counter**) and hands it to the addon; the addon runs the pure gate and
  returns `{decision, config_hash}`.
- **Ledger:** the addon builds/hashes/verifies `LedgerRecord`s; TS persists
  them as `MessageDecisionRecord` rows in CoreLedger and reads them back for
  `verify_chain`.
- **Trust:** TS loads the `TrustSnapshot` (JSON) from CoreLedger, hands it +
  new samples to the addon, gets back `{score, level, new_snapshot}`, and
  persists the new snapshot. The scorer never sees a database.

## 2. What C0 is, and what it is not

**In C0:** stamp the enrahitu skeleton; the `chancery-kernel` addon composing
the four (checks, gate, `MessageDecision` payload + record build/verify,
bidirectional window scoring, `autonomy_policy`), golden-tested in Rust; a thin
`backend/kernel/` service + CoreLedger entities + hiqlite fatigue counter to
exercise it end-to-end in the chassis; chancery domain specs authored last.

**Not in C0 (deferred to C1+):** the human review UI (Vue), the ESP send path,
the outbox that makes the gate unbypassable, the grounding/entailment model
itself (a context-assembly concern), Turso/offsite durability, and deployment
(single-container packaging is chassis-owned). Postgres is not on the roadmap
at all unless scale forces the CoreLedger swap.

## 3. Work units

Dependencies: **WU-A** first. **WU-B** (pure Rust addon) is independently
golden-testable and is the verifiable core. **WU-C** and **WU-D** (TS
integration) follow B. **WU-E** closes over all. **WU-F** (domain specs) last.

### WU-A: stamp the enrahitu skeleton for chancery

- Bring the enrahitu chassis into the (near-empty) chancery repo. Two paths:
  **(a) Statecraft factory stamp** (`template.toml` slots: `app_name=chancery`,
  `org=statecrafting`, `frontend=vue`) which also emits the `.statecraft/
  born-with.json` provenance cert (spec 012); or **(b) manual chassis copy**
  into the existing repo. Recommend (a) if the factory stamp path is ready,
  since it yields the born-with cert for free; else (b). **Open decision (§6).**
- License stays Apache-2.0. Reconcile the existing `docs/preliminary/*` and
  this plan into the stamped tree.
- Verify the inherited baseline is green before adding anything:
  `npm run build:addon && npm run build:runtime && npm install && npm run dev`,
  `curl :4000/health`, `npm test`, `spec-spine compile && spec-spine index`.

### WU-B: the `chancery-kernel` native addon (pure Rust, the verifiable core)

New `kernel-addon/` (sibling to `addon/`), napi-rs cdylib. `Cargo.toml` deps:
the four crates at pinned `0.1` + `napi 3`/`napi-derive` (copy the addon's napi
feature set). Contents:

- **The five checks as `impl Check`** (pure; read pre-computed attributes):

  | Check | id | Outcome |
  |---|---|---|
  | `SuppressionCheck` | `suppression` | unsubscribe/bounce/DNC -> `Deny` + `blocking` |
  | `GroundingCheck` | `grounding` | claim not entailed by fact sheet -> `Deny` (highest-stakes) |
  | `InjectionCheck` | `injection` | inbound-content-as-data boundary -> `Deny` |
  | `FatigueCheck` | `fatigue` | per-investor budget exceeded -> `Degrade` |
  | `ToneCheck` | `tone` | tone/style violation -> `Degrade` |
  | common `SecretsCheck`, `AllowlistCheck` | | reused as-is |

  Rules: checks are pure (no host calls, no clock); `GroundingCheck` reads
  `attributes["grounding_verdict"]` (pre-computed TS-side, C1 model);
  `FatigueCheck` reads `attributes["contact_count"]` + `["contact_budget"]`
  (count fetched from the hiqlite counter TS-side). Parameterized checks
  override `config_fingerprint()` (fatigue budget, suppression-list hash,
  fact-sheet id/hash). Reason codes: `gate:<outcome>:<check>:<detail>`.
  `assemble_gate(params)` builds the ordered list (blocking/safety first).
- **`MessageDecision` payload + record build/verify** over attest-ledger
  (`RecordChain::append(id, timestamp, to_value(payload))`; caller-supplied
  timestamp; opaque `Value` in the core, typed wrapper here). Fields in §4.
- **Bidirectional window scoring**: `WindowScorer` with
  `Direction::Bidirectional`, `from_snapshot`/`snapshot` round-trip, sample
  construction from review outcomes.
- **`autonomy_policy(&Decision, Level, RiskTier) -> AutonomyOutcome`** (pure;
  see §4).
- **`#[napi]` surface** (stateless, JSON in/out at the boundary):
  `evaluate(context_json, params_json) -> {decision, config_hash}`,
  `build_record(prev_hash, id, timestamp, payload_json) -> record_json`,
  `verify_chain(records_json) -> {ok, error?}`,
  `score(config_json, snapshot_json, samples_json) -> {score, level, snapshot_json}`,
  `autonomy(decision_json, level, tier) -> outcome`.
- **Determinism golden tests in Rust**: fixed inputs -> fixed `config_hash`,
  `record_hash`, `level`, `AutonomyOutcome`; byte-stable across platforms
  (feeds the inherited `determinism.yml`). This is the analog of v1's
  "pure-library C0," now as the addon core, verifiable before any TS exists.
- Build wiring: `npm run build:addon:kernel` mirroring the chassis addon build;
  emit prebuilt `.node` + `index.d.ts`.

### WU-C: `backend/kernel/` service + CoreLedger entities

- Thin Encore service (mirror `backend/hiq/`): `init.ts` (warm the signing
  material via `resolve_signing_material`), `api.ts` (evaluate + record-decision
  endpoints), `encore.service.ts`.
- CoreLedger entities (copy the `backend/auth/entities.ts` shape; libSQL
  driver, no Postgres):

  ```ts
  @Entity("message_decision")            // the attest-ledger chain, one row/record
  class MessageDecisionRecord {
    @Column({ primary: true }) id = "";
    @Column({ index: true }) threadId = "";
    @Column({ type: "timestamp" }) timestamp = new Date(); // caller-supplied at build
    @Column({ type: "json" }) payload: unknown = {};        // MessageDecision
    @Column() prevHash = "";
    @Column() recordHash = "";
  }
  @Entity("trust_snapshot")              // per (play, segment) window
  class TrustSnapshot {
    @Column({ primary: true }) id = "";  // `${playId}::${channel}::${intent}`
    @Column({ index: true }) playId = "";
    @Column() segment = "";
    @Column({ type: "json" }) snapshot: unknown = {};       // WindowSnapshot
    @Column({ type: "timestamp" }) updatedAt = new Date();
  }
  ```
- `ensureSchema()` for both entities at service init.

### WU-D: context assembly, the ladder, the fatigue counter (TS orchestration)

- `context.ts`: build the `ActionContext` JSON for the addon from a
  `ChancerySendRequest` (channel, intent, investor_id, play_id, fact_sheet_id,
  draft body). Pre-compute: grounding verdict (C1 stub returns "unknown" ->
  routes to review), suppression membership, and the **fatigue count via
  `hiqlite.counterGet(`fatigue:${investorId}`)`**; on send, `counterAdd`.
- `ladder.ts`: load `TrustSnapshot` from CoreLedger for `(playId, channel,
  intent)`; call addon `score` with the new review sample; persist the new
  snapshot; derive promotion/demotion. Promotion `review_required ->
  auto_send` when `level == Full` over `promote_min_samples`; demotion
  automatic when `level()` drops. Hard pins from RiskTier override.
  Sample source: accepted-as-is 1.0, edited `1 - normalized_edit_distance`,
  rejected 0.0. Confidence math: min-samples-at-`Full` to start (Wilson
  interval later; interface unchanged).

### WU-E: end-to-end test + determinism gate

- vitest: `ChancerySendRequest` -> context assembly -> addon `evaluate` ->
  `build_record` -> persist `MessageDecisionRecord` -> read chain back ->
  addon `verify_chain` passes -> `autonomy` yields the expected outcome. One
  test proves addon + CoreLedger + hiqlite wire together in the chassis.
- Bind `config_hash` into `MessageDecision.gate_config_hash` so a recorded
  decision is provable against the exact gate config. **Property:** the stock
  `attest-ledger verify` CLI verifies chancery chains exported from CoreLedger
  (no bespoke verifier).
- The Rust golden gate rides the inherited `determinism.yml`.

### WU-F: chancery domain specs (governance graduation)

- Author `specs/NNN-*` for the kernel addon, the checks, the ledger payload,
  and the ladder, on top of inherited chassis specs 000-019. Add
  `[package.metadata.spec-spine]` to `kernel-addon/Cargo.toml` and
  `"spec-spine": { "spec": ... }` to `backend/kernel`'s manifest linkage.
  `spec-spine compile && spec-spine index`, commit `.derived/`, coupling gate
  green.

## 4. Key type sketches

```rust
// payload.rs (field names to finalize)
pub struct MessageDecision {
    pub context_hash: String,     // hash of the exact AgentContext bundle
    pub play_hash: String,        // hash of the authorizing play-spec
    pub gate_outcome: String,     // Allow/Deny/Degrade
    pub gate_reason: String,      // gate reason code
    pub check_ids: Vec<String>,
    pub gate_config_hash: String, // binds the decision to the exact gate config
    pub autonomy_outcome: String, // AutoSend / ReviewRequired / Blocked
    pub model_snapshot: String,   // provenance
    pub approver: Option<String>, // rauthy principal, when human-approved
    pub segment: String,          // "channel::intent"
}

// autonomy.rs
pub enum AutonomyOutcome { AutoSend, ReviewRequired, Blocked }
pub fn autonomy_policy(gate: &Decision, trust: Level, tier: RiskTier) -> AutonomyOutcome {
    if gate.blocking { return AutonomyOutcome::Blocked; } // overrides trust
    match gate.outcome {
        Outcome::Deny    => AutonomyOutcome::Blocked,
        Outcome::Degrade => AutonomyOutcome::ReviewRequired,
        Outcome::Allow   => match (tier.is_hard_pinned(), trust) {
            (true, _)            => AutonomyOutcome::ReviewRequired,
            (false, Level::Full) => AutonomyOutcome::AutoSend,
            _                    => AutonomyOutcome::ReviewRequired,
        },
    }
}
```

## 5. Sequencing

```
WU-A stamp enrahitu -> chancery (Apache-2.0, governance inherited)
      |
WU-B  chancery-kernel addon (pure Rust; golden-tested standalone)   <- verifiable core
      |
      +--> WU-C backend/kernel service + CoreLedger entities
      +--> WU-D context assembly + ladder + hiqlite fatigue counter
                 |
WU-E  end-to-end test (addon + CoreLedger + hiqlite) + determinism gate
                 |
WU-F  chancery domain specs (graduate governance)
```

## 6. Open decisions

1. **WU-A stamp path:** Statecraft factory stamp (yields the born-with cert)
   vs manual chassis copy into the existing chancery repo. Recommend the
   factory stamp if that path is ready.
2. **Addon boundary confirmation:** the addon is stateless pure compute; TS
   owns persistence (CoreLedger) and the fatigue counter (hiqlite).
   Recommended; confirm.
3. **RiskTier taxonomy (was #7, still open).** Strawman to react to:
   - `Tier0` low: warm/existing thread, non-first-touch -> auto-eligible.
   - `Tier1` standard: routine outreach -> auto-eligible only at `Full` trust.
   - `Tier2` elevated: first-touch, or a claim referencing specifics ->
     review by default.
   - `Tier3` pinned: top-tier investor first-touch, or anything touching
     terms/legal/financial commitments -> always `review_required`
     (`is_hard_pinned() == true`), regardless of trust.
4. **Ledger anchor granularity:** one `RecordChain` (and CoreLedger chain) per
   conversation thread, anchored to the deployment signing key via
   `resolve_signing_material`. Confirm per-thread vs per-play chains.

Carried and already resolved: license (Apache-2.0), governance timing,
single-crate, naming (adjusted §A), segment `(channel, intent)`, grounding
pre-computation.

## 7. Definition of done for C0

- The enrahitu baseline is green in the chancery repo (`npm run dev`, health,
  `npm test`, `spec-spine` clean).
- `chancery-kernel` addon composes the four at pinned `0.1`; Rust golden tests
  green; prebuilt `.node` + `index.d.ts` emitted.
- End-to-end path green: `ChancerySendRequest` -> addon gate -> `MessageDecision`
  -> CoreLedger `message_decision` chain -> read back -> `verify_chain` (and the
  stock `attest-ledger` CLI) pass -> `autonomy_policy` outcome; fatigue budget
  enforced through a hiqlite counter.
- No Postgres anywhere; CoreLedger on libSQL only.
- All §6 open decisions resolved and recorded.
- Ready for C1: review UI, ESP send, the unbypassable outbox, the grounding
  model, and single-container deployment.
