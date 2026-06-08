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

use super::{
    adjudicate, flip_to_quintic_decoy, inject_free_parameter, run_veto_cascade, unification_report,
    CandidateAssessment, Theory,
};

/// A class of hard negative the adversary fabricates each generation. Every kind MUST be killed by
/// some deterministic gate — a decoy that survives is an honesty failure (the gates missed a
/// known-bad theory). This is what makes the adversary a real *opponent* that generates new
/// falsification challenges, not merely the scalar `frontier_margin` pressure schedule (the M6
/// hardening the v3.0.0 review demanded).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DecoyKind {
    /// A free fitting knob — killed by the whitebox provenance veto.
    GrayBoxKnob,
    /// A quintic (G5) sector ⇒ α_T ≠ 0 + non-degenerate higher derivatives — killed by GW170817 +
    /// Ostrogradsky.
    QuinticGhost,
    /// Extra radiation (large N_eff) overproduces primordial helium — killed by the BBN
    /// cross-domain channel.
    BbnViolating,
    /// Modifies gravity on linear scales with no declared screening — killed by the PPN gate.
    UnscreenedModifiedGravity,
}

impl DecoyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DecoyKind::GrayBoxKnob => "gray_box_knob",
            DecoyKind::QuinticGhost => "quintic_ghost",
            DecoyKind::BbnViolating => "bbn_violating",
            DecoyKind::UnscreenedModifiedGravity => "unscreened_modified_gravity",
        }
    }
}

/// A fabricated adversarial decoy: a theory that SHOULD die, plus the held-out observable the
/// champion has not been probed on this generation (the "regime the champion didn't anticipate").
#[derive(Debug, Clone)]
pub struct Decoy {
    pub theory: Theory,
    pub kind: DecoyKind,
    pub held_out_observable: String,
}

/// Held-out observables the adversary rotates through — regimes a typical background+growth run
/// does not score the champion on (high-z growth, the lensing amplitude, the drag horizon, the
/// local-ladder H0).
const HELD_OUT: [&str; 4] = ["fsigma8@2.0", "s8", "r_drag", "h0_local"];

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

    /// Health of the *best-surviving* reference anchor: the maximum pressured fitness over the
    /// protected anchors (1.0 if there are none). The honesty loop protects the strongest
    /// reference (the GR baseline that must never die), so one aspirational anchor that happens to
    /// fit a given dataset poorly cannot stall the escalation by dragging a mean down.
    pub fn anchor_health(&self, anchors: &[CandidateAssessment]) -> f64 {
        if anchors.is_empty() {
            return 1.0;
        }
        anchors
            .iter()
            .map(|a| self.pressured_fitness(a))
            .fold(0.0_f64, f64::max)
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

    /// Fabricate a decoy for `generation`, rotating through the [`DecoyKind`]s. Deterministic in the
    /// generation number (no RNG) so a run reproduces. Each decoy is a theory the gates MUST kill
    /// and a held-out observable the champion has not been probed on — the adversary's per-generation
    /// falsification challenge.
    pub fn generate_decoy(&self, generation: usize) -> Decoy {
        let base = Theory::baseline_lcdm();
        let kind = match generation % 4 {
            0 => DecoyKind::GrayBoxKnob,
            1 => DecoyKind::QuinticGhost,
            2 => DecoyKind::BbnViolating,
            _ => DecoyKind::UnscreenedModifiedGravity,
        };
        let theory = match kind {
            DecoyKind::GrayBoxKnob => inject_free_parameter(&base, "f_ede", 0.07),
            DecoyKind::QuinticGhost => flip_to_quintic_decoy(&base),
            DecoyKind::BbnViolating => {
                let mut t = base.clone();
                t.id = format!("{}-bbn-decoy", base.id);
                t.background.n_eff = 4.5; // extra radiation overproduces primordial helium
                t
            }
            DecoyKind::UnscreenedModifiedGravity => {
                let mut t = base.clone();
                t.id = format!("{}-unscreened-decoy", base.id);
                t.alpha.alpha_m = 0.1; // modifies gravity on linear scales…
                t.screening = None; // …with no declared screening ⇒ violates Cassini γ
                t
            }
        };
        Decoy {
            theory,
            kind,
            held_out_observable: HELD_OUT[generation % HELD_OUT.len()].to_string(),
        }
    }

    /// Whether the decoy is correctly KILLED by the deterministic gates — the honesty check the
    /// adversary applies to itself each generation. A decoy that SURVIVES is an honesty failure (the
    /// gates missed a known-bad theory). Uses the structural veto cascade, the M3 adjudication pass,
    /// and the cross-domain unification channel — a kill from any one suffices.
    pub fn decoy_is_killed(&self, decoy: &Decoy) -> bool {
        !run_veto_cascade(&decoy.theory).is_empty()
            || !adjudicate(&decoy.theory).is_empty()
            || !unification_report(&decoy.theory).is_unified()
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

    // --- v3.0.0 M6: the decoy-generating adversary (a real opponent, not just a margin) ---

    #[test]
    fn every_generated_decoy_is_killed_by_the_gates() {
        let adv = Adversary::new();
        let mut kinds = std::collections::BTreeSet::new();
        for gen in 0..12 {
            let decoy = adv.generate_decoy(gen);
            kinds.insert(decoy.kind);
            assert!(
                adv.decoy_is_killed(&decoy),
                "gen {gen}: decoy {:?} (held-out {}) SURVIVED the gates — honesty failure",
                decoy.kind,
                decoy.held_out_observable
            );
        }
        // All four decoy kinds are exercised by the rotation.
        assert_eq!(kinds.len(), 4, "all decoy kinds should be generated");
    }

    #[test]
    fn generate_decoy_is_deterministic_in_the_generation() {
        let adv = Adversary::new();
        let a = adv.generate_decoy(5);
        let b = adv.generate_decoy(5);
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.held_out_observable, b.held_out_observable);
        assert_eq!(a.theory.id, b.theory.id);
    }

    #[test]
    fn the_gr_baseline_is_not_a_killed_decoy() {
        // Sanity: the honesty check must NOT flag the legitimate baseline as a (killed) decoy.
        let adv = Adversary::new();
        let good = Decoy {
            theory: Theory::baseline_lcdm(),
            kind: DecoyKind::GrayBoxKnob,
            held_out_observable: "s8".into(),
        };
        assert!(
            !adv.decoy_is_killed(&good),
            "the GR baseline must survive the gates"
        );
    }
}
