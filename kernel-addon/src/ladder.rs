// SPDX-License-Identifier: Apache-2.0

//! The play-scoped autonomy ladder over the trust window.

use serde::{Deserialize, Serialize};
use trust_window::{Direction, Level, Sample, WindowConfig, WindowScorer, WindowSnapshot};

/// The outcome of a human review of a drafted send, the source of a trust
/// sample. On each review the scorer for the play's `(channel, intent)` segment
/// records one of these.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReviewOutcome {
    /// Accepted verbatim: the strongest positive signal (1.0).
    AcceptedAsIs,
    /// Accepted after edits; `distance` is the normalized edit distance in
    /// `[0, 1]` (0 = no change, 1 = fully rewritten). Sample value is
    /// `1 - distance`.
    Edited { distance: f64 },
    /// Rejected outright: the strongest negative signal (0.0).
    Rejected,
}

impl ReviewOutcome {
    /// Map a review outcome to a trust-window sample.
    pub fn to_sample(self) -> Sample {
        match self {
            ReviewOutcome::AcceptedAsIs => Sample::new(1.0),
            ReviewOutcome::Edited { distance } => Sample::new(1.0 - distance),
            ReviewOutcome::Rejected => Sample::new(0.0),
        }
    }
}

/// The chancery per-(play, segment) window configuration: bidirectional, so a
/// play can be promoted to auto-send as its acceptance record improves and
/// demoted on regression. `promote_min_samples` is the promotion floor: a play
/// must accrue that many observations in the window before it is trusted to
/// reach `Full`.
pub fn chancery_window_config() -> WindowConfig {
    WindowConfig {
        direction: Direction::Bidirectional,
        promote_min_samples: 10,
        ..WindowConfig::default()
    }
}

/// The result of scoring new samples against a (possibly rehydrated) window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreResult {
    pub score: f64,
    pub level: Level,
    /// The new snapshot to persist back to the consumer's store.
    pub snapshot: WindowSnapshot,
}

/// Score new review samples against a per-(play, segment) window.
///
/// The snapshot is loaded from and persisted back to the consumer's store
/// (CoreLedger); the scorer itself never touches a database, which is what keeps
/// this deterministic (given the same sample sequence, `score` and `level` are
/// reproducible; there is no wall clock). Pass `None` for a fresh window.
pub fn score(
    config: WindowConfig,
    snapshot: Option<WindowSnapshot>,
    samples: &[Sample],
) -> ScoreResult {
    let mut scorer = match snapshot {
        Some(s) => WindowScorer::from_snapshot(config, s),
        None => WindowScorer::new(config),
    };
    for s in samples {
        scorer.record(*s);
    }
    ScoreResult {
        score: scorer.score(),
        level: scorer.level(),
        snapshot: scorer.snapshot(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_outcome_samples() {
        assert_eq!(ReviewOutcome::AcceptedAsIs.to_sample().value, 1.0);
        assert_eq!(ReviewOutcome::Rejected.to_sample().value, 0.0);
        assert_eq!(
            ReviewOutcome::Edited { distance: 0.25 }.to_sample().value,
            0.75
        );
    }

    #[test]
    fn sustained_acceptance_promotes_to_full() {
        let cfg = chancery_window_config();
        let samples: Vec<Sample> = (0..15)
            .map(|_| ReviewOutcome::AcceptedAsIs.to_sample())
            .collect();
        let result = score(cfg, None, &samples);
        assert_eq!(result.level, Level::Full);
    }

    #[test]
    fn regression_demotes_from_full() {
        let cfg = chancery_window_config();
        // Earn Full, persist the snapshot, then regress with rejections.
        let up: Vec<Sample> = (0..15)
            .map(|_| ReviewOutcome::AcceptedAsIs.to_sample())
            .collect();
        let first = score(cfg.clone(), None, &up);
        assert_eq!(first.level, Level::Full);

        let down: Vec<Sample> = (0..15)
            .map(|_| ReviewOutcome::Rejected.to_sample())
            .collect();
        let second = score(cfg, Some(first.snapshot), &down);
        assert_ne!(second.level, Level::Full);
    }
}
