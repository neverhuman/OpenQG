//! V8 Phase 3 (#18): Budget-aware routing and stopping rules.
//!
//! Routes proposer effort to model/lane combinations with the highest expected
//! value per token spent. The routing objective is:
//!
//! ```text
//! value(model, lane)     = E[survive | model, lane] * E[max(score-baseline, 0) | survive]
//! cost(model, lane)      = E[usd] + λ_tok * E[tokens/1000] + λ_lat * E[latency_seconds]
//! ucb(model, lane)       = exploration_c * sqrt(ln(total_calls + 1) / (calls + 1))
//! route_score(m, l)      = value / max(cost, floor_cost) + ucb
//! ```
//!
//! Per-model/per-lane Beta priors track survival; Gamma estimates track tokens/cost.
//! The stopping rule pauses when marginal token productivity (MTP) drops below threshold
//! for three consecutive rolling windows.
//!
//! Spec reference: S12 §7.

use serde::{Deserialize, Serialize};

/// The contribution breakdown of a routing score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteScore {
    pub model: String,
    pub lane: String,
    /// Expected value: survive_rate × E[score_delta | survive].
    pub expected_value: f64,
    /// Expected cost in USD.
    pub expected_cost_usd: f64,
    /// UCB exploration bonus.
    pub ucb_bonus: f64,
    /// `value / max(cost, floor_cost) + ucb`.
    pub route_score: f64,
    /// Total calls to this (model, lane) pair so far.
    pub total_calls: u64,
}

/// Bayesian prior for one (model, lane) arm.
///
/// Survival follows Beta(alpha, beta); cost follows a running mean.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandPrior {
    pub model: String,
    pub lane: String,
    /// Beta(alpha, beta): alpha = survive_count + 1.
    pub alpha: f64,
    /// Beta(alpha, beta): beta = fail_count + 1.
    pub beta_: f64,
    /// Total calls to this arm.
    pub total_calls: u64,
    /// Total tokens consumed.
    pub total_tokens: u64,
    /// Total USD consumed.
    pub total_usd: f64,
    /// Running mean of score delta for survivors.
    pub mean_score_delta: f64,
    /// Number of survivors contributing to `mean_score_delta`.
    pub survivor_count: u64,
    /// Running mean latency in seconds.
    pub mean_latency_seconds: f64,
}

impl BandPrior {
    /// Create a new arm with uniform Beta prior (1,1) = equal chance of survive/fail.
    pub fn new(model: impl Into<String>, lane: impl Into<String>) -> Self {
        BandPrior {
            model: model.into(),
            lane: lane.into(),
            alpha: 1.0,
            beta_: 1.0,
            total_calls: 0,
            total_tokens: 0,
            total_usd: 0.0,
            mean_score_delta: 0.0,
            survivor_count: 0,
            mean_latency_seconds: 0.0,
        }
    }

    /// Expected survival rate: Beta posterior mean = alpha / (alpha + beta).
    pub fn expected_survive_rate(&self) -> f64 {
        self.alpha / (self.alpha + self.beta_)
    }

    /// Expected cost per call in USD.
    pub fn expected_cost_per_call(&self) -> f64 {
        if self.total_calls == 0 {
            0.001 // default prior: 1 milli-USD
        } else {
            self.total_usd / self.total_calls as f64
        }
    }

    /// Record a completed call outcome.
    pub fn update(
        &mut self,
        survived: bool,
        score_delta: Option<f64>,
        tokens: u64,
        usd: f64,
        latency_seconds: f64,
    ) {
        self.total_calls += 1;
        self.total_tokens += tokens;
        self.total_usd += usd;

        // Running mean latency (Welford-style).
        let n = self.total_calls as f64;
        self.mean_latency_seconds += (latency_seconds - self.mean_latency_seconds) / n;

        if survived {
            self.alpha += 1.0;
            if let Some(delta) = score_delta {
                self.survivor_count += 1;
                let k = self.survivor_count as f64;
                self.mean_score_delta += (delta - self.mean_score_delta) / k;
            }
        } else {
            self.beta_ += 1.0;
        }
    }
}

/// Marginal token productivity for one rolling window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginalTokenProductivity {
    /// Size of the window in generations.
    pub window_size: u32,
    /// Generation index at the end of the window.
    pub generation_end: u64,
    /// Score improvement over the window: `best_score[end] - best_score[start]`.
    pub score_delta: f64,
    /// Tokens consumed in the window.
    pub tokens_in_window: u64,
    /// MTP = score_delta / (tokens_in_window / 100_000).
    /// Units: scorecard points per 100k tokens.
    pub mtp: f64,
    /// Wilson upper confidence bound on survivor rate over the window.
    pub survivor_rate_ucb: f64,
}

impl MarginalTokenProductivity {
    fn compute(
        generation_end: u64,
        window_size: u32,
        score_delta: f64,
        tokens: u64,
        n_survivors: u64,
        n_proposals: u64,
    ) -> Self {
        let mtp = if tokens == 0 {
            f64::INFINITY
        } else {
            score_delta / (tokens as f64 / 100_000.0)
        };

        // Wilson UCB on survivor/proposal rate.
        let survivor_rate_ucb = wilson_ucb(n_survivors, n_proposals);

        MarginalTokenProductivity {
            window_size,
            generation_end,
            score_delta,
            tokens_in_window: tokens,
            mtp,
            survivor_rate_ucb,
        }
    }
}

/// Compute Wilson upper confidence bound for a proportion.
fn wilson_ucb(successes: u64, total: u64) -> f64 {
    if total == 0 {
        return 1.0;
    }
    let n = total as f64;
    let p = successes as f64 / n;
    // 95% UCB: z ≈ 1.645 (one-tailed)
    let z = 1.645_f64;
    let center = (p + z * z / (2.0 * n)) / (1.0 + z * z / n);
    let spread = z * f64::sqrt(p * (1.0 - p) / n + z * z / (4.0 * n * n)) / (1.0 + z * z / n);
    (center + spread).min(1.0)
}

/// Parameters for the MTP-based stopping rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoppingRule {
    /// Rolling window size in generations.
    pub window_size: u32,
    /// Minimum adjudicated proposals before stopping is considered.
    pub min_proposals: u32,
    /// Pause when MTP falls below this threshold for `consecutive_windows` windows.
    /// Units: scorecard points per 100k tokens.
    pub mtp_threshold: f64,
    /// Pause when survivor UCB rate falls below this threshold.
    /// Units: survivors per 100k tokens.
    pub survivor_rate_ucb_threshold: f64,
    /// Number of consecutive windows all conditions must hold before pausing.
    pub consecutive_windows_required: u32,
}

impl Default for StoppingRule {
    fn default() -> Self {
        StoppingRule {
            window_size: 50,
            min_proposals: 20,
            mtp_threshold: 0.25,
            survivor_rate_ucb_threshold: 1.0,
            consecutive_windows_required: 3,
        }
    }
}

/// Decision from the stopping rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoppingDecision {
    pub should_pause: bool,
    pub reason: String,
    pub mtp_windows: Vec<MarginalTokenProductivity>,
    pub consecutive_low_windows: u32,
}

/// One generation entry for stopping-rule input.
#[derive(Debug, Clone)]
pub struct GenerationEntry {
    pub generation: u64,
    /// Best score seen at or before this generation.
    pub best_score: f64,
    /// Cumulative tokens at this generation.
    pub cumulative_tokens: u64,
    /// Cumulative adjudicated proposals at this generation.
    pub cumulative_proposals: u64,
    /// Cumulative survivors at this generation.
    pub cumulative_survivors: u64,
}

/// Evaluate whether the campaign should pause based on the stopping rule.
///
/// `history` must be sorted by generation ascending and have at least `window_size` entries
/// for the stopping rule to fire; if fewer entries exist, returns `should_pause = false`.
pub fn should_stop(history: &[GenerationEntry], rule: &StoppingRule) -> StoppingDecision {
    if history.len() < rule.window_size as usize || history.is_empty() {
        return StoppingDecision {
            should_pause: false,
            reason: format!(
                "insufficient history: {} generations, need {}",
                history.len(),
                rule.window_size
            ),
            mtp_windows: vec![],
            consecutive_low_windows: 0,
        };
    }

    // Check minimum proposals at the end of history.
    let total_proposals = history.last().map(|e| e.cumulative_proposals).unwrap_or(0);
    if total_proposals < rule.min_proposals as u64 {
        return StoppingDecision {
            should_pause: false,
            reason: format!(
                "insufficient proposals: {total_proposals}, need {}",
                rule.min_proposals
            ),
            mtp_windows: vec![],
            consecutive_low_windows: 0,
        };
    }

    // Compute MTP for each rolling window ending at each generation.
    let w = rule.window_size as usize;
    let mut mtp_windows: Vec<MarginalTokenProductivity> = Vec::new();

    for i in w..=history.len() {
        let start = &history[i - w];
        let end = &history[i - 1];

        let score_delta = (end.best_score - start.best_score).max(0.0);
        let tokens = end
            .cumulative_tokens
            .saturating_sub(start.cumulative_tokens);
        let survivors = end
            .cumulative_survivors
            .saturating_sub(start.cumulative_survivors);
        let proposals = end
            .cumulative_proposals
            .saturating_sub(start.cumulative_proposals);

        mtp_windows.push(MarginalTokenProductivity::compute(
            end.generation,
            rule.window_size,
            score_delta,
            tokens,
            survivors,
            proposals,
        ));
    }

    // Count consecutive low-MTP windows from the end.
    let mut consecutive = 0u32;
    for mtp in mtp_windows.iter().rev() {
        let mtp_low = mtp.mtp < rule.mtp_threshold;
        // Survivor UCB: convert from rate-per-proposal to per-100k-tokens.
        let survivor_ucb_per_100k = if mtp.tokens_in_window == 0 {
            f64::INFINITY
        } else {
            mtp.survivor_rate_ucb * (mtp.tokens_in_window as f64 / 100_000.0).recip()
        };
        let survivor_low = survivor_ucb_per_100k < rule.survivor_rate_ucb_threshold;

        if mtp_low && survivor_low {
            consecutive += 1;
        } else {
            break;
        }
    }

    let should_pause = consecutive >= rule.consecutive_windows_required;
    let reason = if should_pause {
        format!(
            "stopping: {consecutive} consecutive windows below MTP={:.2} pts/100k tokens and \
             survivor UCB < {:.1}/100k tokens",
            rule.mtp_threshold, rule.survivor_rate_ucb_threshold
        )
    } else {
        format!(
            "continuing: only {consecutive}/{} consecutive low-MTP windows",
            rule.consecutive_windows_required
        )
    };

    StoppingDecision {
        should_pause,
        reason,
        mtp_windows,
        consecutive_low_windows: consecutive,
    }
}

/// Compute route scores for all arms, sorted by `route_score` descending.
pub fn compute_route_scores(
    priors: &[BandPrior],
    exploration_c: f64,
    floor_cost_usd: f64,
) -> Vec<RouteScore> {
    let total_calls: u64 = priors.iter().map(|p| p.total_calls).sum();

    let mut scores: Vec<RouteScore> = priors
        .iter()
        .map(|p| {
            let survive_rate = p.expected_survive_rate();
            let expected_score_delta = if p.survivor_count == 0 {
                1.0 // prior: survivors earn 1 point on average
            } else {
                p.mean_score_delta.max(0.0)
            };
            let value = survive_rate * expected_score_delta;
            let cost = p.expected_cost_per_call().max(floor_cost_usd);
            let ucb = exploration_c
                * f64::sqrt(f64::ln(total_calls as f64 + 1.0) / (p.total_calls as f64 + 1.0));
            let route_score = value / cost + ucb;

            RouteScore {
                model: p.model.clone(),
                lane: p.lane.clone(),
                expected_value: value,
                expected_cost_usd: cost,
                ucb_bonus: ucb,
                route_score,
                total_calls: p.total_calls,
            }
        })
        .collect();

    scores.sort_by(|a, b| {
        b.route_score
            .partial_cmp(&a.route_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scores
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_history(scores: &[(u64, f64, u64, u64, u64)]) -> Vec<GenerationEntry> {
        scores
            .iter()
            .map(
                |(gen, score, tokens, proposals, survivors)| GenerationEntry {
                    generation: *gen,
                    best_score: *score,
                    cumulative_tokens: *tokens,
                    cumulative_proposals: *proposals,
                    cumulative_survivors: *survivors,
                },
            )
            .collect()
    }

    /// Spec S12 rank-1: synthetic router test prefers higher expected value per budget unit.
    #[test]
    fn router_prefers_high_expected_value_arm() {
        let priors = vec![
            {
                let mut p = BandPrior::new("claude-sonnet-4-6", "physics");
                for _ in 0..30 {
                    p.update(true, Some(5.0), 10_000, 0.005, 1.0);
                }
                for _ in 0..10 {
                    p.update(false, None, 8_000, 0.004, 0.9);
                }
                p
            },
            {
                let mut p = BandPrior::new("claude-haiku-4-5", "physics");
                for _ in 0..5 {
                    p.update(true, Some(1.0), 4_000, 0.001, 0.5);
                }
                for _ in 0..15 {
                    p.update(false, None, 3_000, 0.001, 0.4);
                }
                p
            },
        ];

        let scores = compute_route_scores(&priors, 1.0, 0.001);
        assert_eq!(
            scores[0].model, "claude-sonnet-4-6",
            "high-value arm must rank first"
        );
    }

    /// Unexplored arms get UCB bonus and can outrank well-explored bad arms.
    #[test]
    fn unexplored_arm_gets_ucb_bonus() {
        let priors = vec![
            {
                let mut p = BandPrior::new("model-a", "lane-x");
                for _ in 0..100 {
                    p.update(false, None, 10_000, 0.01, 1.0);
                }
                p
            },
            BandPrior::new("model-b", "lane-y"), // unexplored
        ];
        let scores = compute_route_scores(&priors, 2.0, 0.001);
        // Unexplored model-b should be ranked first due to UCB bonus.
        assert_eq!(
            scores[0].model, "model-b",
            "unexplored arm must rank first with UCB bonus"
        );
    }

    /// Stopping rule fires after 3 consecutive low-MTP windows.
    #[test]
    fn stopping_rule_fires_after_three_low_windows() {
        let rule = StoppingRule {
            window_size: 10,
            min_proposals: 20,
            mtp_threshold: 0.25,
            survivor_rate_ucb_threshold: 1.0,
            consecutive_windows_required: 3,
        };

        // 70 generations: first 40 have score improvement, last 30 are flat (MTP = 0).
        let mut history = Vec::new();
        for i in 0u64..40 {
            history.push(GenerationEntry {
                generation: i,
                best_score: 50.0 + i as f64 * 0.1, // improving
                cumulative_tokens: i * 5_000,
                cumulative_proposals: i,
                cumulative_survivors: i / 3,
            });
        }
        for i in 40u64..70 {
            history.push(GenerationEntry {
                generation: i,
                best_score: 54.0, // flat
                cumulative_tokens: i * 5_000,
                cumulative_proposals: i,
                cumulative_survivors: 14, // flat
            });
        }

        let decision = should_stop(&history, &rule);
        assert!(
            decision.should_pause,
            "stopping rule must fire after 3 consecutive low-MTP windows; got: {}",
            decision.reason
        );
        assert!(decision.consecutive_low_windows >= 3);
    }

    /// Stopping rule does NOT fire when score is still improving.
    #[test]
    fn stopping_rule_does_not_fire_while_improving() {
        let rule = StoppingRule::default();
        let history: Vec<GenerationEntry> = (0u64..60)
            .map(|i| GenerationEntry {
                generation: i,
                best_score: 50.0 + i as f64 * 0.05, // steady improvement
                cumulative_tokens: i * 8_000,
                cumulative_proposals: i,
                cumulative_survivors: i / 2,
            })
            .collect();

        let decision = should_stop(&history, &rule);
        assert!(
            !decision.should_pause,
            "stopping rule must not fire while score is improving"
        );
    }

    /// Below minimum proposals, stopping rule defers.
    #[test]
    fn stopping_rule_defers_below_min_proposals() {
        let rule = StoppingRule::default(); // min_proposals = 20
        let history: Vec<GenerationEntry> = (0u64..60)
            .map(|i| GenerationEntry {
                generation: i,
                best_score: 50.0,
                cumulative_tokens: i * 10_000,
                cumulative_proposals: 5, // always below minimum
                cumulative_survivors: 0,
            })
            .collect();
        let decision = should_stop(&history, &rule);
        assert!(
            !decision.should_pause,
            "must defer when proposals < min_proposals"
        );
    }

    #[test]
    fn wilson_ucb_is_bounded() {
        assert!(wilson_ucb(0, 0) <= 1.0);
        assert!(wilson_ucb(10, 10) <= 1.0);
        assert!(wilson_ucb(0, 100) < 0.1);
        assert!(wilson_ucb(100, 100) > 0.95);
    }

    #[test]
    fn band_prior_survive_rate_updates_correctly() {
        let mut p = BandPrior::new("m", "l");
        // Initial: Beta(1,1) = 0.5.
        assert!((p.expected_survive_rate() - 0.5).abs() < 1e-9);

        // After 9 survivors and 1 fail: Beta(10,2) = 10/12 ≈ 0.833.
        for _ in 0..9 {
            p.update(true, Some(5.0), 5000, 0.005, 1.0);
        }
        p.update(false, None, 5000, 0.005, 1.0);
        assert!(
            (p.expected_survive_rate() - 10.0 / 12.0).abs() < 1e-9,
            "rate = {}",
            p.expected_survive_rate()
        );
    }
}
