//! Co-evolving adversary: an escalating frontier the population must keep beating, with an
//! **honesty rollback** so the pressure never becomes unfair. This is the anti-saturation heart of
//! the engine — without it a population coasts the moment it finds a local optimum; with it,
//! candidates must keep clearing a rising bar (the operational meaning of "robustness under
//! judge"). The honesty loop is the safeguard: if the reference anchors (e.g. the GR/ΛCDM
//! baseline that must *never* be killed) start failing the frontier, the pressure has overreached
//! and is rolled back.
//!
//! Ported from the legacy `zyal_judge` escalation (frontier_margin / ESCALATION_STEP / CAP /
//! ANCHOR_SURVIVAL_FLOOR) into the new engine's `final_fitness` space.

use super::CandidateAssessment;

/// Default escalation step per generation.
const ESCALATION_STEP: f64 = 0.02;
/// Maximum frontier margin (escalation cap).
const ESCALATION_CAP: f64 = 0.6;
/// If the anchors' mean pressured fitness falls below this, the pressure is unfair → roll back.
const ANCHOR_SURVIVAL_FLOOR: f64 = 0.10;

/// An escalating adversarial frontier with honesty rollback.
#[derive(Debug, Clone)]
pub struct Adversary {
    /// The current frontier margin subtracted from every candidate's fitness.
    pub frontier_margin: f64,
    step: f64,
    cap: f64,
    anchor_floor: f64,
}

impl Default for Adversary {
    fn default() -> Self {
        Adversary {
            frontier_margin: 0.0,
            step: ESCALATION_STEP,
            cap: ESCALATION_CAP,
            anchor_floor: ANCHOR_SURVIVAL_FLOOR,
        }
    }
}

impl Adversary {
    pub fn new() -> Self {
        Self::default()
    }

    /// The adversary-adjusted fitness: a candidate must beat the rising frontier. Its
    /// `final_fitness` minus the current margin, floored at 0.
    pub fn pressured_fitness(&self, assessment: &CandidateAssessment) -> f64 {
        (assessment.final_fitness - self.frontier_margin).max(0.0)
    }

    /// Mean pressured fitness of a set of anchors (1.0 if there are none to protect).
    fn anchor_health(&self, anchors: &[CandidateAssessment]) -> f64 {
        if anchors.is_empty() {
            return 1.0;
        }
        anchors
            .iter()
            .map(|a| self.pressured_fitness(a))
            .sum::<f64>()
            / anchors.len() as f64
    }

    /// Advance the frontier one generation. Escalate while the protected anchors stay healthy;
    /// **roll back** the moment their mean pressured fitness drops below the survival floor, so the
    /// escalation can never unfairly kill the references. Returns the new margin.
    pub fn update(&mut self, anchors: &[CandidateAssessment]) -> f64 {
        if self.anchor_health(anchors) < self.anchor_floor {
            self.frontier_margin = (self.frontier_margin - self.step).max(0.0);
        } else {
            self.frontier_margin = (self.frontier_margin + self.step).min(self.cap);
        }
        self.frontier_margin
    }
}

#[cfg(test)]
mod tests {
    use super::super::{assess, Theory};
    use super::*;
    use crate::cosmology::BackgroundForwardModel;
    use crate::types::ObservableRecord;

    fn obs() -> Vec<ObservableRecord> {
        vec![ObservableRecord {
            observable_id: "dm_over_rd@0.510".into(),
            kind: "bao".into(),
            value: 13.62,
            uncertainty: 0.25,
            unit: "dimensionless".into(),
            source: None,
        }]
    }

    fn baseline_assessment() -> CandidateAssessment {
        assess(
            &Theory::baseline_lcdm(),
            &obs(),
            &BackgroundForwardModel,
            0.0,
        )
    }

    #[test]
    fn pressure_reduces_fitness_by_the_margin() {
        let a = baseline_assessment();
        let mut adv = Adversary::new();
        assert_eq!(adv.pressured_fitness(&a), a.final_fitness);
        adv.frontier_margin = 0.05;
        assert!((adv.pressured_fitness(&a) - (a.final_fitness - 0.05)).abs() < 1e-12);
    }

    #[test]
    fn escalates_while_anchors_are_healthy() {
        let anchor = baseline_assessment();
        let mut adv = Adversary::new();
        let m0 = adv.frontier_margin;
        let m1 = adv.update(std::slice::from_ref(&anchor));
        assert!(m1 > m0);
    }

    #[test]
    fn honesty_rollback_never_permanently_kills_the_anchor() {
        // Escalate for many generations; the rollback must keep the protected baseline anchor's
        // pressured fitness from being driven to zero in the steady state.
        let anchor = baseline_assessment();
        let mut adv = Adversary::new();
        for _ in 0..200 {
            adv.update(std::slice::from_ref(&anchor));
        }
        // In steady state the anchor still survives the frontier (pressured fitness > 0).
        assert!(
            adv.pressured_fitness(&anchor) > 0.0,
            "anchor killed: margin={} fitness={}",
            adv.frontier_margin,
            anchor.final_fitness
        );
        // And the margin has not run away to the cap (it self-limits near the anchor's fitness).
        assert!(adv.frontier_margin < anchor.final_fitness + adv.step + 1e-9);
    }

    #[test]
    fn with_no_anchors_escalation_runs_to_the_cap() {
        let mut adv = Adversary::new();
        for _ in 0..100 {
            adv.update(&[]);
        }
        assert!((adv.frontier_margin - ESCALATION_CAP).abs() < 1e-9);
    }
}
