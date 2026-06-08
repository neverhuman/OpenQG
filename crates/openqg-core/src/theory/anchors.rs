//! Frozen anchor/decoy calibration set — the references the engine must *always* get right.
//! "Good" anchors (the GR baseline; a GW170817-safe, screened, ghost-free modified-gravity
//! theory with a provenanced parameter) must survive the veto cascade and be credible; "decoy"
//! anchors (ghost, gray-box free parameter, quintic/GW170817-violating, BBN-violating) must always
//! be killed — either by a hard veto or by failing the cross-domain unification channel.
//!
//! Calibration drift here means the engine's honesty has broken (the adversary has become unfair,
//! or a veto has regressed). The live pipeline runs this every generation and rolls the adversary
//! back if a `Survive` anchor starts dying (see [`super::adversary`]).

use super::{
    assess, flip_to_quintic_decoy, inject_free_parameter, run_veto_cascade, AlphaBasis, Parameter,
    Provenance, Stability, Theory,
};
use crate::cosmology::{CosmologyParams, ForwardModel};
use crate::types::ObservableRecord;

/// Expected calibration outcome for an anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorKind {
    /// Must survive the vetoes and be a credible candidate.
    Survive,
    /// Must be killed — vetoed, or not credible (cross-domain failure).
    Die,
}

/// A frozen calibration anchor.
#[derive(Debug, Clone)]
pub struct Anchor {
    pub theory: Theory,
    pub kind: AnchorKind,
}

/// A GW170817-safe, screened, ghost-free modified-gravity theory with a provenanced parameter —
/// exactly the kind of non-degenerate candidate the engine must keep.
fn good_screened_mg() -> Theory {
    let mut t = Theory::baseline_lcdm();
    t.id = "anchor-good-screened-mg".into();
    t.alpha = AlphaBasis {
        alpha_m: 0.05,
        alpha_b: -0.03,
        alpha_k: 0.1,
        alpha_t: 0.0,
    };
    t.screening = Some("chameleon".into());
    t.stability = Stability {
        kinetic_coefficient: 0.8,
        q_s: 0.5,
        sound_speed_sq: 0.4,
        has_nondegenerate_higher_derivatives: false,
    };
    t.parameters.push(Parameter {
        symbol: "alpha_M0".into(),
        value: 0.05,
        physical_meaning: "Planck-mass run amplitude".into(),
        provenance: Provenance::derived("conformal coupling beta in the chameleon potential"),
    });
    t
}

/// A theory that over-produces primordial helium (BBN-violating): NOT a hard veto, but the
/// cross-domain unification channel must disqualify it.
fn decoy_bbn_violating() -> Theory {
    let mut t = Theory::baseline_lcdm();
    t.id = "anchor-decoy-bbn-violating".into();
    t.background.n_eff = 4.6;
    t
}

/// The frozen calibration set.
pub fn anchor_set() -> Vec<Anchor> {
    let base = Theory::baseline_lcdm();
    vec![
        Anchor {
            theory: base.clone(),
            kind: AnchorKind::Survive,
        },
        Anchor {
            theory: good_screened_mg(),
            kind: AnchorKind::Survive,
        },
        Anchor {
            theory: flip_to_quintic_decoy(&base),
            kind: AnchorKind::Die,
        },
        Anchor {
            theory: inject_free_parameter(&base, "f_ede", 0.07),
            kind: AnchorKind::Die,
        },
        Anchor {
            theory: decoy_bbn_violating(),
            kind: AnchorKind::Die,
        },
    ]
}

/// Run the calibration: returns the ids of any anchors that violated their expected outcome
/// (empty ⇒ the engine is honestly calibrated).
pub fn miscalibrated<M>(
    observables: &[ObservableRecord],
    model: &M,
    baseline_log_likelihood: f64,
) -> Vec<String>
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    anchor_set()
        .into_iter()
        .filter_map(|a| {
            let vetoed = !run_veto_cascade(&a.theory).is_empty();
            let credible =
                assess(&a.theory, observables, model, baseline_log_likelihood).is_credible();
            let ok = match a.kind {
                AnchorKind::Survive => !vetoed && credible,
                AnchorKind::Die => vetoed || !credible,
            };
            if ok {
                None
            } else {
                Some(a.theory.id)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmology::BackgroundForwardModel;
    use crate::types::ObservableRecord;

    fn obs() -> Vec<ObservableRecord> {
        vec![
            ObservableRecord {
                observable_id: "dm_over_rd@0.510".into(),
                kind: "bao".into(),
                value: 13.62,
                uncertainty: 0.25,
                unit: "dimensionless".into(),
                source: None,
            },
            ObservableRecord {
                observable_id: "bbn_yp".into(),
                kind: "bbn".into(),
                value: 0.2453,
                uncertainty: 0.0034,
                unit: "dimensionless".into(),
                source: None,
            },
        ]
    }

    #[test]
    fn the_anchor_set_is_honestly_calibrated() {
        let bad = miscalibrated(&obs(), &BackgroundForwardModel, 0.0);
        assert!(bad.is_empty(), "miscalibrated anchors: {bad:?}");
    }

    #[test]
    fn good_anchors_survive_and_decoys_die() {
        for a in anchor_set() {
            let vetoed = !run_veto_cascade(&a.theory).is_empty();
            let credible = assess(&a.theory, &obs(), &BackgroundForwardModel, 0.0).is_credible();
            match a.kind {
                AnchorKind::Survive => {
                    assert!(!vetoed && credible, "{} should survive", a.theory.id)
                }
                AnchorKind::Die => assert!(vetoed || !credible, "{} should die", a.theory.id),
            }
        }
    }
}
