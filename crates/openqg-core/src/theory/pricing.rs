//! V8 Phase 9 (SYNTHESIS #3): PricingLedger — audit that every scored DOF choice has an
//! explicit rubric price. Fails closed if any DOF type earned points without a declared price,
//! preventing exotic configurations from silently gaming the rubric.

use serde::{Deserialize, Serialize};

/// One scored degree-of-freedom and its pricing status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PricingEntry {
    /// Category of choice: "solver_tier" | "data_source" | "parameter" | "claim_kind".
    pub dof_kind: String,
    /// Human-readable label for this specific choice (e.g. "T2Boltzmann", "BAO+CMB").
    pub label: String,
    /// Points actually awarded in the scorecard for this DOF choice.
    pub points_awarded: f64,
    /// Whether the rubric has a declared explicit price for this DOF category.
    /// When false, the ledger fails closed and the theory is blocked.
    pub has_explicit_price: bool,
}

/// Audit ledger that tracks whether every scored choice in the rubric has an assigned price.
///
/// The scoring rubric must define explicit prices for every degree-of-freedom category. If a
/// theory exercises an exotic DOF that the rubric never priced, `fails_closed()` returns true
/// and the scorecard must block the theory from earning points on that DOF.
///
/// This prevents iterative rubric evasion: an adversary cannot discover a new uncategorized DOF
/// type and silently earn points from it, because the ledger will catch the gap on audit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PricingLedger {
    pub entries: Vec<PricingEntry>,
}

impl PricingLedger {
    pub fn new() -> Self {
        PricingLedger {
            entries: Vec::new(),
        }
    }

    /// Record one DOF choice in the ledger.
    pub fn add(
        &mut self,
        dof_kind: impl Into<String>,
        label: impl Into<String>,
        points_awarded: f64,
        has_explicit_price: bool,
    ) {
        self.entries.push(PricingEntry {
            dof_kind: dof_kind.into(),
            label: label.into(),
            points_awarded,
            has_explicit_price,
        });
    }

    /// Number of entries that earned points but have no rubric price.
    pub fn unpriced_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| !e.has_explicit_price)
            .count()
    }

    /// True when ANY entry lacks an explicit price.
    ///
    /// A true return value must block the theory from earning scored points on the unpriced DOF.
    /// Downstream scoring gates call `fails_closed()` and, when true, append a kill reason.
    pub fn fails_closed(&self) -> bool {
        self.entries.iter().any(|e| !e.has_explicit_price)
    }

    /// Sum of `points_awarded` across all entries, regardless of pricing status.
    pub fn total_awarded_points(&self) -> f64 {
        self.entries.iter().map(|e| e.points_awarded).sum()
    }

    /// Returns all entries that lack an explicit price (for kill-reason reporting).
    pub fn unpriced_entries(&self) -> Vec<&PricingEntry> {
        self.entries
            .iter()
            .filter(|e| !e.has_explicit_price)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ledger_does_not_fail_closed() {
        let ledger = PricingLedger::new();
        assert!(!ledger.fails_closed());
        assert_eq!(ledger.unpriced_count(), 0);
        assert_eq!(ledger.total_awarded_points(), 0.0);
    }

    #[test]
    fn ledger_fails_closed_on_any_unpriced_entry() {
        let mut ledger = PricingLedger::new();
        ledger.add("solver_tier", "T2Boltzmann", 5.0, true);
        ledger.add("exotic_dof", "novel_param_kind_xyz", 3.0, false);
        assert!(ledger.fails_closed());
        assert_eq!(ledger.unpriced_count(), 1);
        assert_eq!(ledger.unpriced_entries().len(), 1);
        assert_eq!(ledger.unpriced_entries()[0].dof_kind, "exotic_dof");
    }

    #[test]
    fn ledger_passes_when_all_entries_priced() {
        let mut ledger = PricingLedger::new();
        ledger.add("solver_tier", "T1Emulator", 2.0, true);
        ledger.add("data_source", "BAO+CMB", 4.0, true);
        ledger.add("claim_kind", "InterestingFit", 0.0, true);
        assert!(!ledger.fails_closed());
        assert_eq!(ledger.unpriced_count(), 0);
        assert!((ledger.total_awarded_points() - 6.0).abs() < 1e-9);
    }

    #[test]
    fn all_unpriced_entries_are_counted() {
        let mut ledger = PricingLedger::new();
        ledger.add("dof_a", "label_a", 1.0, false);
        ledger.add("dof_b", "label_b", 2.0, false);
        assert_eq!(ledger.unpriced_count(), 2);
        assert!(ledger.fails_closed());
    }

    #[test]
    fn total_awarded_sums_all_entries_regardless_of_pricing_status() {
        let mut ledger = PricingLedger::new();
        ledger.add("dof_a", "label_a", 3.0, true);
        ledger.add("dof_b", "label_b", 7.0, false);
        assert!((ledger.total_awarded_points() - 10.0).abs() < 1e-9);
    }

    #[test]
    fn unpriced_entries_returns_only_unpriced() {
        let mut ledger = PricingLedger::new();
        ledger.add("priced", "ok", 1.0, true);
        ledger.add("unpriced_a", "gap_1", 0.5, false);
        ledger.add("unpriced_b", "gap_2", 1.5, false);
        let unpriced = ledger.unpriced_entries();
        assert_eq!(unpriced.len(), 2);
        assert_eq!(unpriced[0].dof_kind, "unpriced_a");
        assert_eq!(unpriced[1].dof_kind, "unpriced_b");
    }

    #[test]
    fn default_is_same_as_new() {
        let ledger: PricingLedger = Default::default();
        assert!(!ledger.fails_closed());
        assert!(ledger.entries.is_empty());
    }
}
