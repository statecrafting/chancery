# 03: trust-window library (PRELIMINARY)

> Working name `trust-window`. Extracted from OAP
> `crates/policy-kernel/src/coherence.rs` (272 LoC, **0 OAP-coupling
> hits**). The cleanest extraction of the three: it is already
> domain-neutral. The work is generalizing its direction, not decoupling it.

## What it is today (OAP `CoherenceScheduler`)

A rolling-window scorer with a graduated privilege output:

- `record_action(aligned: bool)` feeds a fixed-size window (default 50).
- `coherence_score() -> f64` in [0,1], decay-weighted.
- `violation_count()` counts non-aligned entries in the window.
- Score maps to `PrivilegeLevel`: `Full` >= 0.8, `Restricted` 0.5-0.8,
  `ReadOnly` 0.2-0.5, `Suspended` < 0.2, with a `violation_count` floor.
- **Monotonic degradation**: privilege only ratchets down within a session
  until `human_restore()` clears the window and the latch.
- **No wall clock**: deterministic given the recorded action sequence.

This is a real graduated-trust ladder. It is exactly the primitive the
autonomy story needs, but it currently points only one direction (down).

## Why chancery cannot use it as-is

chancery's autonomy ladder differs on three axes, and those differences are
the new engineering:

| Axis | `CoherenceScheduler` (OAP) | chancery autonomy ladder |
|---|---|---|
| Direction | degrade-only, monotonic down | promote **and** demote |
| Subject | one session / agent, live behavior | a **play**, its track record |
| Signal + persistence | aligned/violation booleans, in-memory window, no clock | human-acceptance + eval score + edit-distance, **durable, cross-session** state |

So the extraction generalizes the core scorer and leaves play-scoping,
persistence, and promotion policy to the consumer.

## The generalization

**1. Weighted samples, not just booleans.** OAP feeds `bool`. chancery feeds
several graded signals per send. Generalize the input to a score:

```rust
// trust-window-types
pub struct Sample { pub value: f64, pub weight: f64 } // value in [0,1]
```

`bool` becomes `Sample { value: 1.0|0.0, weight: 1.0 }`, so OAP's usage is a
trivial special case.

**2. Direction is configuration, not a hardcoded latch.**

```rust
pub enum Direction {
    DegradeOnly,          // OAP's current behavior (monotonic down + human_restore)
    Bidirectional,        // chancery: promote when sustained-high, demote on regression
}

pub struct WindowConfig {
    pub window_size: usize,
    pub direction: Direction,
    pub thresholds: LevelThresholds,     // score cutoffs for each level
    pub promote_min_samples: usize,      // Bidirectional: require N samples before promoting
    pub violation_floor: u32,            // carried from OAP
}
```

**3. The scorer stays pure and in-memory; the consumer owns storage.**

```rust
// trust-window-core
pub struct WindowScorer { /* VecDeque<Sample>, config, latch */ }
impl WindowScorer {
    pub fn new(config: WindowConfig) -> Self;
    pub fn from_snapshot(config: WindowConfig, snapshot: WindowSnapshot) -> Self; // rehydrate
    pub fn record(&mut self, sample: Sample);
    pub fn score(&self) -> f64;
    pub fn level(&self) -> Level;            // Full/Restricted/ReadOnly/Suspended (rename per taste)
    pub fn snapshot(&self) -> WindowSnapshot; // serialize for durable persistence
    pub fn restore_max(&mut self);            // OAP human_restore
}
```

`from_snapshot` / `snapshot` are the seam that lets chancery persist
per-play windows to Postgres and rehydrate them, without the library
knowing about a database. Determinism is preserved: given the same sample
sequence, `score()` and `level()` are reproducible (no clock).

## How chancery's autonomy ladder builds on it

`trust-window` gives the score-to-level primitive. chancery adds, on top:

- **One `WindowScorer` per (play, segment)** where segment is (channel,
  intent, maybe investor tier), rehydrated from a durable snapshot store.
- **Sample source**: on each human review, record a `Sample` from acceptance
  (accepted as-is = 1.0, edited = 1 minus normalized edit distance, rejected
  = 0.0), optionally blended with an offline eval score.
- **Promotion transition**: a play moves `review_required -> auto_send` when
  its scorer sits at `Full` over `promote_min_samples` with statistical
  confidence. Demotion is automatic on regression (the scorer drops a level).
- **Shadow sampling**: after promotion, keep recording a sampled fraction of
  auto-sent messages (graded by a human or judge) so drift re-enters the
  window and can demote.
- **Hard pins**: high-risk segments (first-touch top-tier investor, anything
  touching terms) are pinned to `review_required` regardless of level; this
  is chancery policy, not a scorer feature.

None of that bullet list belongs in the library. The library ships the
scorer; chancery ships the ladder.

## How OAP re-consumes it

OAP's `CoherenceScheduler` becomes `WindowScorer` with
`Direction::DegradeOnly`, `Sample`s built from `PolicyOutcome` (aligned =
not a violation), and `restore_max()` for `human_restore()`. OAP's coherence
tests are the regression guard. `PrivilegeLevel` either re-exports the
library `Level` or maps to it.

## Open questions

- **Own repo/crate or a module of `action-gate`?** It is small (272 LoC) and
  its output (a privilege level) feeds gating decisions. Argument for
  separate: it is stateful (a scorer), whereas the gate is pure; mixing them
  muddies the gate's pure-function property. Recommendation: separate crate,
  colocated in the toolkit repo (Option A in `00`).
- **Confidence math for promotion.** "Statistical confidence over N samples"
  needs a concrete test (Wilson interval on acceptance rate, or a simple
  min-samples-at-level gate). Pick the simplest defensible rule first;
  the interface (`promote_min_samples` + `level`) does not change if the
  internal test gets fancier later.
