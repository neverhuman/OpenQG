//! V8 Phase 28 (#18): Budget-aware routing + marginal-token-productivity stopping rule.
//!
//! Two orthogonal pieces:
//!
//! 1. [`RouteScore`] — ranks candidate actions by expected value per token cost, plus an
//!    exploration bonus (UCB1). A router that always picks the highest `route_score` is
//!    provably near-optimal for a multi-armed bandit with known costs. SYNTHESIS #18 target:
//!    "route_score = value/cost + UCB".
//!
//! 2. [`MtpStoppingRule`] — implements the "marginal-token-productivity" (MTP) stopping rule:
//!    pause the search when MTP < 0.25 pts / 100k tokens for ≥ 3 consecutive token windows.
//!    SYNTHESIS #18 target: "pause when MTP < 0.25 pts/100k tokens over 3 windows".
//!
//! Neither piece requires an LLM call; both are unit-testable with synthetic data.

/// Number of tokens per MTP measurement window.
pub const MTP_WINDOW_SIZE_TOKENS: u64 = 100_000;

/// MTP threshold in pts per token (= 0.25 pts / 100k tokens).
pub const MTP_THRESHOLD_PER_TOKEN: f64 = 0.25 / MTP_WINDOW_SIZE_TOKENS as f64;

/// Number of consecutive low-MTP windows before the stopping rule fires.
pub const MTP_CONSECUTIVE_REQUIRED: u64 = 3;

/// Budget-aware route score for one candidate action.
///
/// `route_score = (expected_value / expected_cost) + ucb_bonus`
///
/// Higher is preferred. When `expected_cost == 0` the ratio is clamped to 0 (the action is
/// free but also adds no value — a degenerate entry). The UCB bonus is large for under-explored
/// arms and decays as `O(1/sqrt(n))` with the arm's trial count.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteScore {
    /// Expected score gain from this action (pts, may be negative for exploratory actions).
    pub expected_value: f64,
    /// Expected token cost (tokens ≥ 0; 0 means cost-free but also uninteresting).
    pub expected_cost: f64,
    /// UCB1 exploration bonus added to the value/cost ratio.
    pub ucb_bonus: f64,
    /// The final routing score: `expected_value / expected_cost + ucb_bonus`.
    pub score: f64,
}

impl RouteScore {
    /// Compute a route score from raw inputs.
    ///
    /// - `expected_value`: anticipated score gain (pts).
    /// - `expected_cost`: anticipated token cost (tokens). Clamped to 0 on the low side.
    /// - `ucb_bonus`: exploration bonus; use [`ucb1_bonus`] for the standard formula.
    pub fn compute(expected_value: f64, expected_cost: f64, ucb_bonus: f64) -> Self {
        let cost = expected_cost.max(0.0);
        let ratio = if cost > 0.0 {
            expected_value / cost
        } else {
            0.0
        };
        RouteScore {
            expected_value,
            expected_cost,
            ucb_bonus,
            score: ratio + ucb_bonus,
        }
    }

    /// True when this route score beats `other`.
    pub fn beats(&self, other: &RouteScore) -> bool {
        self.score > other.score
    }
}

/// Compute the UCB1 exploration bonus for an arm with `arm_trials` pulls out of `total_trials`
/// total pulls across all arms.
///
/// UCB1: `sqrt(2 * ln(N) / n_i)` (Auer, Cesa-Bianchi & Fischer 2002).
/// Returns `f64::INFINITY` when `arm_trials == 0` (an unexplored arm is always preferred).
pub fn ucb1_bonus(total_trials: u64, arm_trials: u64) -> f64 {
    if arm_trials == 0 {
        return f64::INFINITY;
    }
    let n = total_trials.max(1) as f64;
    let ni = arm_trials as f64;
    (2.0 * n.ln() / ni).sqrt()
}

/// One completed MTP measurement window.
#[derive(Debug, Clone, PartialEq)]
pub struct MtpWindow {
    /// Sequential index of this window (0-based).
    pub index: u64,
    /// Score gain observed during this window (pts).
    pub score_gain: f64,
    /// Tokens consumed during this window.
    pub tokens_spent: u64,
    /// MTP = score_gain / tokens_spent (pts/token). 0 when tokens_spent == 0.
    pub mtp: f64,
}

impl MtpWindow {
    fn new(index: u64, score_gain: f64, tokens_spent: u64) -> Self {
        let mtp = if tokens_spent > 0 {
            score_gain / tokens_spent as f64
        } else {
            0.0
        };
        MtpWindow {
            index,
            score_gain,
            tokens_spent,
            mtp,
        }
    }

    /// True when MTP is below `threshold` (a low-productivity window).
    pub fn is_low_mtp(&self, threshold: f64) -> bool {
        self.mtp < threshold
    }
}

/// Marginal-token-productivity (MTP) stopping rule.
///
/// Records score and token spend event by event. Automatically closes a window when the
/// cumulative token spend within the current window crosses [`MTP_WINDOW_SIZE_TOKENS`].
/// Fires `should_pause() = true` when [`MTP_CONSECUTIVE_REQUIRED`] consecutive completed
/// windows all have MTP below [`MTP_THRESHOLD_PER_TOKEN`].
///
/// # Usage
/// ```ignore
/// use openqg_bench::zyal_genome::routing::MtpStoppingRule;
/// let mut rule = MtpStoppingRule::standard();
/// rule.record(0.5, 80_000);   // 0.5 pts gained, 80k tokens spent
/// rule.record(0.0, 30_000);   // window closes here (cumulative = 110k > 100k)
/// // … more events
/// if rule.should_pause() { /* pause the search loop */ }
/// ```
pub struct MtpStoppingRule {
    window_size_tokens: u64,
    mtp_threshold: f64,
    consecutive_required: u64,
    completed_windows: Vec<MtpWindow>,
    current_score: f64,
    current_tokens: u64,
}

impl MtpStoppingRule {
    /// Standard rule: 100k-token windows, threshold 0.25 pts/100k, pause after 3 low windows.
    pub fn standard() -> Self {
        MtpStoppingRule {
            window_size_tokens: MTP_WINDOW_SIZE_TOKENS,
            mtp_threshold: MTP_THRESHOLD_PER_TOKEN,
            consecutive_required: MTP_CONSECUTIVE_REQUIRED,
            completed_windows: Vec::new(),
            current_score: 0.0,
            current_tokens: 0,
        }
    }

    /// Custom rule (for testing or alternate configurations).
    pub fn custom(window_size_tokens: u64, mtp_threshold: f64, consecutive_required: u64) -> Self {
        MtpStoppingRule {
            window_size_tokens,
            mtp_threshold,
            consecutive_required,
            completed_windows: Vec::new(),
            current_score: 0.0,
            current_tokens: 0,
        }
    }

    /// Record `score_gain` pts and `tokens` token spend for one event.
    ///
    /// Automatically closes and starts a new window when cumulative tokens in the current window
    /// cross `window_size_tokens`. If a single event is larger than one window, multiple windows
    /// are closed proportionally.
    pub fn record(&mut self, score_gain: f64, tokens: u64) {
        let mut remaining_score = score_gain;
        let mut remaining_tokens = tokens;
        loop {
            let budget_left = self.window_size_tokens.saturating_sub(self.current_tokens);
            if remaining_tokens <= budget_left {
                self.current_score += remaining_score;
                self.current_tokens += remaining_tokens;
                break;
            }
            // Proportionally allocate to fill the current window.
            let fraction = budget_left as f64 / remaining_tokens.max(1) as f64;
            let window_score = remaining_score * fraction;
            self.current_score += window_score;
            self.current_tokens += budget_left;
            self.close_current_window();
            remaining_score -= window_score;
            remaining_tokens -= budget_left;
        }
    }

    /// Flush the in-progress window as a completed window. Call at the end of a campaign to
    /// evaluate the partial final window.
    pub fn flush(&mut self) {
        if self.current_tokens > 0 {
            self.close_current_window();
        }
    }

    /// Number of completed windows (not including any in-progress partial window).
    pub fn completed_window_count(&self) -> usize {
        self.completed_windows.len()
    }

    /// Number of consecutive completed windows whose MTP is below the threshold, counting
    /// backwards from the most recent window.
    pub fn consecutive_low_mtp_windows(&self) -> u64 {
        let mut count = 0u64;
        for w in self.completed_windows.iter().rev() {
            if w.is_low_mtp(self.mtp_threshold) {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// True when the stopping rule has fired: ≥ `consecutive_required` consecutive completed
    /// windows all have MTP below `mtp_threshold`.
    pub fn should_pause(&self) -> bool {
        self.consecutive_low_mtp_windows() >= self.consecutive_required
    }

    /// MTP of the most recent completed window, or `None` if no window has completed yet.
    pub fn last_window_mtp(&self) -> Option<f64> {
        self.completed_windows.last().map(|w| w.mtp)
    }

    /// All completed windows, in chronological order.
    pub fn completed_windows(&self) -> &[MtpWindow] {
        &self.completed_windows
    }

    fn close_current_window(&mut self) {
        let idx = self.completed_windows.len() as u64;
        let w = MtpWindow::new(idx, self.current_score, self.current_tokens);
        self.completed_windows.push(w);
        self.current_score = 0.0;
        self.current_tokens = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- RouteScore ----

    #[test]
    fn higher_value_per_cost_wins() {
        let cheap = RouteScore::compute(1.0, 10_000.0, 0.0); // 1e-4 pts/token
        let costly = RouteScore::compute(0.5, 10_000.0, 0.0); // 5e-5 pts/token
        assert!(cheap.beats(&costly), "higher value/cost must win");
    }

    #[test]
    fn ucb_bonus_promotes_unexplored_arm() {
        // An arm with 0 trials must always beat any finite route score.
        let unexplored = RouteScore::compute(0.0, 1000.0, ucb1_bonus(100, 0));
        let explored = RouteScore::compute(10.0, 1000.0, ucb1_bonus(100, 50));
        assert!(
            unexplored.beats(&explored),
            "unexplored arm (UCB=∞) must always win"
        );
    }

    #[test]
    fn ucb1_bonus_decays_with_arm_trials() {
        let b10 = ucb1_bonus(1000, 10);
        let b100 = ucb1_bonus(1000, 100);
        assert!(b10 > b100, "UCB bonus must decrease as arm trials grow");
    }

    #[test]
    fn zero_cost_route_scores_zero_ratio() {
        let r = RouteScore::compute(5.0, 0.0, 0.0);
        assert_eq!(
            r.score, 0.0,
            "zero-cost route must score 0 (not NaN/infinity)"
        );
    }

    #[test]
    fn synthetic_router_prefers_higher_value_per_budget_unit() {
        // Synthetic verification: given three arms with equal UCB (same trials), the router
        // must rank them by value/cost. Arm A: 2 pts / 50k tokens = 4e-5 pts/token.
        // Arm B: 1 pt / 50k tokens = 2e-5 pts/token.  Arm C: 0.5 pts / 50k tokens = 1e-5.
        let bonus = ucb1_bonus(30, 10); // equal for all three (10 trials each, 30 total)
        let arm_a = RouteScore::compute(2.0, 50_000.0, bonus);
        let arm_b = RouteScore::compute(1.0, 50_000.0, bonus);
        let arm_c = RouteScore::compute(0.5, 50_000.0, bonus);
        assert!(arm_a.beats(&arm_b), "A(4e-5) must beat B(2e-5)");
        assert!(arm_b.beats(&arm_c), "B(2e-5) must beat C(1e-5)");
        assert!(arm_a.beats(&arm_c), "A(4e-5) must beat C(1e-5)");
    }

    // ---- MtpWindow ----

    #[test]
    fn low_mtp_window_detected_correctly() {
        let threshold = MTP_THRESHOLD_PER_TOKEN;
        let low = MtpWindow::new(0, 0.01, MTP_WINDOW_SIZE_TOKENS); // 0.01/100k = 1e-7 << threshold
        let high = MtpWindow::new(1, 1.0, MTP_WINDOW_SIZE_TOKENS); // 1/100k >> threshold
        assert!(
            low.is_low_mtp(threshold),
            "0.01 pts/100k must be below threshold"
        );
        assert!(
            !high.is_low_mtp(threshold),
            "1.0 pts/100k must be above threshold"
        );
    }

    // ---- MtpStoppingRule ----

    #[test]
    fn stopping_rule_does_not_fire_after_one_low_window() {
        let mut rule = MtpStoppingRule::standard();
        // One low-MTP window (0 pts, full window of tokens).
        rule.record(0.0, MTP_WINDOW_SIZE_TOKENS);
        rule.flush();
        assert_eq!(rule.consecutive_low_mtp_windows(), 1);
        assert!(
            !rule.should_pause(),
            "one low window is not enough to pause"
        );
    }

    #[test]
    fn stopping_rule_fires_after_three_consecutive_low_windows() {
        let mut rule = MtpStoppingRule::standard();
        for _ in 0..3 {
            rule.record(0.0, MTP_WINDOW_SIZE_TOKENS); // zero score → low MTP
        }
        rule.flush();
        assert!(
            rule.should_pause(),
            "three consecutive zero-MTP windows must trigger pause"
        );
    }

    #[test]
    fn high_mtp_window_resets_consecutive_count() {
        let mut rule = MtpStoppingRule::standard();
        // Two low windows, then one high window.
        rule.record(0.0, MTP_WINDOW_SIZE_TOKENS);
        rule.record(0.0, MTP_WINDOW_SIZE_TOKENS);
        // High-productivity window: 100 pts in 100k tokens >> 0.25 pts/100k threshold.
        rule.record(100.0, MTP_WINDOW_SIZE_TOKENS);
        rule.flush();
        assert_eq!(
            rule.consecutive_low_mtp_windows(),
            0,
            "a high-MTP window must reset the consecutive low-MTP counter"
        );
        assert!(!rule.should_pause());
    }

    #[test]
    fn custom_rule_with_short_windows() {
        // Use 1000-token windows and a low threshold = 0.1 pts/1000 tokens.
        let mut rule = MtpStoppingRule::custom(1_000, 0.1 / 1_000.0, 2);
        rule.record(0.0, 1_000); // window 0: 0 pts/1000 tokens → low
        rule.record(0.0, 1_000); // window 1: 0 pts/1000 tokens → low → fires
        rule.flush();
        assert!(
            rule.should_pause(),
            "custom rule must fire after 2 consecutive low windows"
        );
    }

    #[test]
    fn partial_window_not_counted_until_flushed() {
        let mut rule = MtpStoppingRule::standard();
        // Spend 50k tokens (half a window), no score.
        rule.record(0.0, 50_000);
        // No complete windows yet, so should_pause is false.
        assert_eq!(rule.completed_window_count(), 0);
        assert!(!rule.should_pause());
        // Flush closes the partial window.
        rule.flush();
        assert_eq!(rule.completed_window_count(), 1);
    }

    #[test]
    fn mtp_threshold_constant_matches_spec() {
        // SYNTHESIS #18: "0.25 pts / 100k tokens".
        let expected = 0.25 / 100_000.0_f64;
        assert!(
            (MTP_THRESHOLD_PER_TOKEN - expected).abs() < 1e-15,
            "MTP threshold must exactly match the SYNTHESIS spec (0.25 / 100k)"
        );
    }
}
