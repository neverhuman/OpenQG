//! V8 Phase 1 (#8): GrowthVerdictPack — structured verdict from the growth-of-structure sector.
//!
//! The growth sector includes RSD (redshift-space distortion) likelihoods, weak lensing (WL)
//! shear correlations, and CMB lensing convergence. These are the most constraining post-fit-set
//! observables for modified gravity theories: a theory that fits background data can still be
//! killed by growth data.
//!
//! This module defines the typed verdict structure. Computing the actual RSD/lensing likelihoods
//! requires external data files and (for lensing) an angular power spectrum from a Boltzmann
//! solver. The `GrowthVerdictPack` is filled by the engine's scoring pipeline and stamped into
//! the scorecard alongside the standard DataFitOutcome.

use serde::{Deserialize, Serialize};

/// A single RSD (redshift-space distortion) measurement: fσ8(z) observed vs predicted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RsdMeasurement {
    /// Redshift bin center.
    pub z: f64,
    /// Observed fσ8(z) value.
    pub observed: f64,
    /// 1-sigma uncertainty on observed fσ8.
    pub sigma: f64,
    /// Theory prediction for fσ8(z) from the forward model.
    pub predicted: f64,
    /// Pull = (predicted - observed) / sigma. |pull| > 3 is a 3σ tension.
    pub pull: f64,
    /// Survey or dataset identifier (e.g. "BOSS-DR12-z0.57").
    pub dataset: String,
}

impl RsdMeasurement {
    pub fn new(
        z: f64,
        observed: f64,
        sigma: f64,
        predicted: f64,
        dataset: impl Into<String>,
    ) -> Self {
        let pull = if sigma == 0.0 {
            0.0
        } else {
            (predicted - observed) / sigma
        };
        RsdMeasurement {
            z,
            observed,
            sigma,
            predicted,
            pull,
            dataset: dataset.into(),
        }
    }

    /// True when the pull exceeds 3σ — the theory is in tension with this measurement.
    pub fn is_tension(&self) -> bool {
        self.pull.abs() > 3.0
    }
}

/// Weak lensing shear summary: the compressed S8 = σ8 * (Ω_m / 0.3)^0.5 constraint.
///
/// Full tomographic lensing likelihoods require a Boltzmann backend; this struct captures the
/// compressed constraint that can be compared against the theory's background prediction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensingS8Verdict {
    /// Observed S8 value from the lensing survey.
    pub s8_observed: f64,
    /// 1-sigma uncertainty on S8.
    pub s8_sigma: f64,
    /// Theory prediction for S8.
    pub s8_predicted: f64,
    /// Pull = (predicted - observed) / sigma.
    pub pull: f64,
    /// Survey identifier (e.g. "KiDS-1000", "DES-Y3", "HSC-Y3").
    pub dataset: String,
}

impl LensingS8Verdict {
    pub fn new(
        s8_observed: f64,
        s8_sigma: f64,
        s8_predicted: f64,
        dataset: impl Into<String>,
    ) -> Self {
        let pull = if s8_sigma == 0.0 {
            0.0
        } else {
            (s8_predicted - s8_observed) / s8_sigma
        };
        LensingS8Verdict {
            s8_observed,
            s8_sigma,
            s8_predicted,
            pull,
            dataset: dataset.into(),
        }
    }

    /// True when the pull exceeds 2σ — moderate tension.
    pub fn is_tension(&self) -> bool {
        self.pull.abs() > 2.0
    }
}

/// Overall growth verdict: the aggregate result of comparing a theory against all growth
/// sector constraints (RSD + weak lensing + CMB lensing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrowthVerdictPack {
    /// Theory identifier.
    pub theory_id: String,

    /// Per-redshift RSD measurements with pulls.
    pub rsd_measurements: Vec<RsdMeasurement>,

    /// S8 lensing verdicts from WL surveys.
    pub lensing_verdicts: Vec<LensingS8Verdict>,

    /// χ²/dof for the full growth sector (RSD + lensing combined).
    /// `None` if not enough data to compute a meaningful reduced chi-squared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chi2_per_dof: Option<f64>,

    /// True when ANY RSD measurement has |pull| > 3 OR any lensing verdict has |pull| > 2.
    pub growth_tension: bool,

    /// True when the growth sector rejects the theory (growth_tension + chi2_per_dof > 2.0).
    /// This is a kill-level verdict: theories rejected here are disqualified regardless of
    /// their background fit quality.
    pub growth_killed: bool,
}

impl GrowthVerdictPack {
    /// Build a verdict pack from RSD and lensing inputs.
    ///
    /// `chi2_per_dof`: pass `Some(value)` if you have computed it from the combined likelihood.
    pub fn build(
        theory_id: impl Into<String>,
        rsd_measurements: Vec<RsdMeasurement>,
        lensing_verdicts: Vec<LensingS8Verdict>,
        chi2_per_dof: Option<f64>,
    ) -> Self {
        let growth_tension = rsd_measurements.iter().any(|r| r.is_tension())
            || lensing_verdicts.iter().any(|l| l.is_tension());
        let growth_killed = growth_tension && chi2_per_dof.is_some_and(|c| c > 2.0);
        GrowthVerdictPack {
            theory_id: theory_id.into(),
            rsd_measurements,
            lensing_verdicts,
            chi2_per_dof,
            growth_tension,
            growth_killed,
        }
    }

    /// Compute the combined χ² from all available pulls (RSD + lensing).
    /// Returns `(chi2, dof)`.
    pub fn compute_chi2(&self) -> (f64, usize) {
        let dof = self.rsd_measurements.len() + self.lensing_verdicts.len();
        let chi2: f64 = self
            .rsd_measurements
            .iter()
            .map(|r| r.pull.powi(2))
            .chain(self.lensing_verdicts.iter().map(|l| l.pull.powi(2)))
            .sum();
        (chi2, dof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_rsd(z: f64) -> RsdMeasurement {
        RsdMeasurement::new(z, 0.45, 0.02, 0.44, "BOSS")
    }

    fn ok_lensing() -> LensingS8Verdict {
        LensingS8Verdict::new(0.759, 0.021, 0.762, "KiDS-1000")
    }

    #[test]
    fn rsd_pull_computed_correctly() {
        let m = RsdMeasurement::new(0.5, 0.450, 0.020, 0.460, "BOSS");
        let expected = (0.460 - 0.450) / 0.020;
        assert!((m.pull - expected).abs() < 1e-12);
        assert!(!m.is_tension());
    }

    #[test]
    fn rsd_tension_detected_above_3sigma() {
        let m = RsdMeasurement::new(0.5, 0.450, 0.010, 0.520, "BOSS");
        assert!(m.is_tension()); // pull = 7.0
    }

    #[test]
    fn lensing_pull_computed_correctly() {
        let l = LensingS8Verdict::new(0.759, 0.021, 0.762, "KiDS-1000");
        let expected = (0.762 - 0.759) / 0.021;
        assert!((l.pull - expected).abs() < 1e-10);
        assert!(!l.is_tension());
    }

    #[test]
    fn lensing_tension_detected_above_2sigma() {
        let l = LensingS8Verdict::new(0.759, 0.021, 0.820, "KiDS-1000");
        assert!(l.is_tension()); // pull ≈ 2.9
    }

    #[test]
    fn no_tension_when_all_within_tolerance() {
        let pack = GrowthVerdictPack::build(
            "gr-lcdm",
            vec![ok_rsd(0.5), ok_rsd(0.8)],
            vec![ok_lensing()],
            Some(1.1),
        );
        assert!(!pack.growth_tension);
        assert!(!pack.growth_killed);
    }

    #[test]
    fn tension_from_rsd_sets_growth_tension() {
        let bad_rsd = RsdMeasurement::new(0.5, 0.450, 0.010, 0.520, "BOSS"); // 7σ
        let pack =
            GrowthVerdictPack::build("bad-theory", vec![bad_rsd], vec![ok_lensing()], Some(1.5));
        assert!(pack.growth_tension);
    }

    #[test]
    fn growth_killed_requires_tension_and_high_chi2() {
        let bad_rsd = RsdMeasurement::new(0.5, 0.450, 0.010, 0.520, "BOSS"); // 7σ
        let pack_low_chi2 = GrowthVerdictPack::build("t", vec![bad_rsd.clone()], vec![], Some(1.5));
        assert!(pack_low_chi2.growth_tension);
        assert!(!pack_low_chi2.growth_killed); // chi2/dof = 1.5 < 2.0

        let pack_high_chi2 = GrowthVerdictPack::build("t", vec![bad_rsd], vec![], Some(2.1));
        assert!(pack_high_chi2.growth_killed);
    }

    #[test]
    fn chi2_computation_is_sum_of_squared_pulls() {
        let rsd = RsdMeasurement::new(0.5, 0.450, 0.020, 0.460, "BOSS"); // pull = 0.5
        let lensing = LensingS8Verdict::new(0.759, 0.021, 0.762, "KiDS"); // pull ≈ 0.143
        let pack = GrowthVerdictPack::build("t", vec![rsd.clone()], vec![lensing.clone()], None);
        let (chi2, dof) = pack.compute_chi2();
        let expected = rsd.pull.powi(2) + lensing.pull.powi(2);
        assert!((chi2 - expected).abs() < 1e-10);
        assert_eq!(dof, 2);
    }
}
