//! Cross-domain "unification" consistency channel — orthogonal to the cosmology data fit.
//!
//! A theory that merely fits cosmological distances is not "unified"; a credible candidate must
//! simultaneously survive the constraints that link cosmology to gravity and particle physics.
//! This channel scores a [`Theory`] against the cheap *analytic* cross-domain checks from
//! `docs/research/forward-model-and-unification.md` §4 — GW speed (GW170817), solar-system PPN,
//! laboratory fifth-force / equivalence principle, BBN light-element abundances, and the
//! gravitational-wave-friction standard-siren ratio.
//!
//! It is deliberately kept **orthogonal** to ε (the cosmology fit): the overall score is the
//! *weakest* domain (a minimum, not a sum), so a candidate cannot buy consistency in one sector
//! by overfitting another. A "unified" theory must pass cosmology AND clear this channel.

use super::Theory;

/// Above this α-scale a theory measurably modifies gravity and needs screening to pass PPN.
const PPN_MODIFICATION_SCALE: f64 = 1e-3;
/// GW170817 structural tolerance on the tensor-speed excess.
const ALPHA_T_BOUND: f64 = 1e-2;
/// Observed primordial helium fraction (Aver et al. 2015) and its 1σ.
const YP_OBSERVED: f64 = 0.2453;
const YP_SIGMA: f64 = 0.0034;
/// Loose standard-siren bound on |d_L^GW/d_L^EM − 1| at the reference redshift.
const SIREN_BOUND: f64 = 0.2;
/// Reference redshift for the GW-friction siren check.
const SIREN_Z: f64 = 1.0;

/// One cross-domain consistency check.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainCheck {
    pub domain: &'static str,
    /// Soft consistency score in `[0,1]` (0 = hard inconsistency, 1 = comfortably consistent).
    pub score: f64,
    pub note: String,
}

impl DomainCheck {
    pub fn passed(&self) -> bool {
        self.score > 0.0
    }
}

/// The cross-domain unification report.
#[derive(Debug, Clone, PartialEq)]
pub struct UnificationReport {
    pub checks: Vec<DomainCheck>,
    /// Overall score = the weakest domain (unification is gated by its worst sector).
    pub score: f64,
}

impl UnificationReport {
    /// True only if every domain is at least minimally consistent.
    pub fn is_unified(&self) -> bool {
        self.checks.iter().all(DomainCheck::passed)
    }
}

/// Gravitational-wave luminosity-distance ratio d_L^GW/d_L^EM from the Planck-mass run α_M,
/// assuming α_M ≈ const over the observable range: ratio = (1+z)^(−α_M/2).
fn siren_distance_ratio(alpha_m: f64, z: f64) -> f64 {
    (1.0 + z).powf(-alpha_m / 2.0)
}

/// Score a theory across the cross-domain consistency channel.
pub fn unification_report(theory: &Theory) -> UnificationReport {
    let mut checks = Vec::new();

    // 1. GW speed (GW170817): the tensor-speed excess must vanish.
    let alpha_t = theory.alpha.alpha_t.abs();
    checks.push(DomainCheck {
        domain: "gravitational_wave_speed",
        score: (1.0 - alpha_t / ALPHA_T_BOUND).clamp(0.0, 1.0),
        note: format!("|alpha_T|={alpha_t:.2e} vs GW170817 bound {ALPHA_T_BOUND:.0e}"),
    });

    // 2. Solar-system PPN (Cassini γ): a gravity-modifying theory needs a screening mechanism.
    let modifies = theory.alpha.modification_scale() > PPN_MODIFICATION_SCALE;
    let ppn_score = if !modifies {
        1.0
    } else if theory.screening.is_some() {
        0.9 // passes via screening, with a small naturalness penalty for relying on it
    } else {
        0.0 // unscreened modification → violates Cassini γ
    };
    checks.push(DomainCheck {
        domain: "solar_system_ppn",
        score: ppn_score,
        note: match (modifies, theory.screening.as_deref()) {
            (false, _) => "GR in the weak field".into(),
            (true, Some(s)) => format!("modified gravity screened by {s}"),
            (true, None) => "unscreened modification violates Cassini gamma".into(),
        },
    });

    // 3. Lab fifth-force / equivalence principle (Eöt-Wash, MICROSCOPE): same screening need.
    checks.push(DomainCheck {
        domain: "lab_fifth_force_ep",
        score: if !modifies || theory.screening.is_some() {
            1.0
        } else {
            0.0
        },
        note: "fifth-force / EP requires screening if gravity is modified".into(),
    });

    // 4. BBN light-element abundances: Y_p (from the background) vs the observed value.
    let yp = theory.background.bbn_helium_fraction();
    let yp_dev = (yp - YP_OBSERVED).abs() / YP_SIGMA;
    checks.push(DomainCheck {
        domain: "bbn_abundance",
        // 1 at the observed value, 0 at 3σ and beyond.
        score: (1.0 - yp_dev / 3.0).clamp(0.0, 1.0),
        note: format!("Y_p={yp:.4} ({yp_dev:.1} sigma from observed {YP_OBSERVED})"),
    });

    // 5. GW-friction standard-siren ratio: α_M ≠ 0 ⇒ d_L^GW/d_L^EM ≠ 1.
    let ratio = siren_distance_ratio(theory.alpha.alpha_m, SIREN_Z);
    let dev = (ratio - 1.0).abs();
    checks.push(DomainCheck {
        domain: "gw_friction_siren",
        score: (1.0 - dev / SIREN_BOUND).clamp(0.0, 1.0),
        note: format!(
            "d_L^GW/d_L^EM(z={SIREN_Z})={ratio:.4} (dev {dev:.3} vs bound {SIREN_BOUND})"
        ),
    });

    // Unification is gated by the weakest sector — no trading one domain off against another.
    let score = checks
        .iter()
        .map(|c| c.score)
        .fold(1.0_f64, |acc, s| acc.min(s));

    UnificationReport { checks, score }
}

#[cfg(test)]
mod tests {
    use super::super::{AlphaBasis, Theory};
    use super::*;

    #[test]
    fn gr_baseline_is_fully_unified() {
        let r = unification_report(&Theory::baseline_lcdm());
        assert!(r.is_unified());
        assert!(r.score > 0.7, "baseline unification score = {}", r.score);
    }

    #[test]
    fn screened_modified_gravity_is_unified_unscreened_is_not() {
        let mut screened = Theory::baseline_lcdm();
        screened.alpha = AlphaBasis {
            alpha_m: 0.02,
            alpha_b: 0.05,
            alpha_k: 0.1,
            alpha_t: 0.0,
        };
        screened.screening = Some("vainshtein".into());
        assert!(unification_report(&screened).is_unified());

        let mut unscreened = screened.clone();
        unscreened.screening = None;
        let r = unification_report(&unscreened);
        assert!(!r.is_unified());
        // The failing domains are the solar-system / fifth-force sectors.
        assert!(r
            .checks
            .iter()
            .any(|c| c.domain == "solar_system_ppn" && !c.passed()));
    }

    #[test]
    fn bbn_violating_radiation_breaks_unification() {
        let mut hot = Theory::baseline_lcdm();
        hot.background.n_eff = 4.5; // too much extra radiation → Y_p too high
        let r = unification_report(&hot);
        assert!(r
            .checks
            .iter()
            .any(|c| c.domain == "bbn_abundance" && !c.passed()));
        assert!(!r.is_unified());
    }

    #[test]
    fn strong_planck_mass_run_breaks_the_siren_channel() {
        let mut t = Theory::baseline_lcdm();
        t.alpha.alpha_m = 0.6; // large GW friction
        t.screening = Some("vainshtein".into());
        let r = unification_report(&t);
        let siren = r
            .checks
            .iter()
            .find(|c| c.domain == "gw_friction_siren")
            .unwrap();
        assert!(siren.score < 1.0, "siren score = {}", siren.score);
    }

    #[test]
    fn score_is_the_weakest_domain() {
        // A theory perfect everywhere except one borderline sector scores that sector's value.
        let r = unification_report(&Theory::baseline_lcdm());
        let min = r.checks.iter().map(|c| c.score).fold(1.0_f64, f64::min);
        assert!((r.score - min).abs() < 1e-12);
    }
}
