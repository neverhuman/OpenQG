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

/// V8 Phase 10 (SYNTHESIS #8): CMB lensing amplitude verdict.
///
/// A_lens = 1 in ΛCDM. Modified gravity predicts A_lens ≠ 1. This verdict captures the
/// comparison of theory vs the ACT DR6 or Planck lensing amplitude measurement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CmbLensingVerdict {
    /// Observed lensing amplitude A_lens.
    pub a_lens_observed: f64,
    /// 1-sigma uncertainty on A_lens.
    pub a_lens_sigma: f64,
    /// Theory-predicted A_lens from the forward model.
    pub a_lens_predicted: f64,
    /// Pull = (predicted - observed) / sigma.
    pub pull: f64,
    /// Dataset identifier (e.g. "ACT-DR6", "Planck-2018-lensing").
    pub dataset: String,
}

impl CmbLensingVerdict {
    pub fn new(
        a_lens_observed: f64,
        a_lens_sigma: f64,
        a_lens_predicted: f64,
        dataset: impl Into<String>,
    ) -> Self {
        let pull = if a_lens_sigma == 0.0 {
            0.0
        } else {
            (a_lens_predicted - a_lens_observed) / a_lens_sigma
        };
        CmbLensingVerdict {
            a_lens_observed,
            a_lens_sigma,
            a_lens_predicted,
            pull,
            dataset: dataset.into(),
        }
    }

    /// True when the pull exceeds 2σ — moderate tension.
    pub fn is_tension(&self) -> bool {
        self.pull.abs() > 2.0
    }
}

/// V8 Phase 10 (SYNTHESIS #8): per-dataset survival record for the pre-scoring table.
///
/// Before any theory earns a score in the growth sector, the engine must confirm each mandatory
/// dataset's likelihood reproduces at the ΛCDM fiducial. A failed reproduction is a blocker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetSurvivalRecord {
    /// Dataset identifier (e.g. "eBOSS-DR16-fσ8", "KiDS-Legacy-S8", "ACT-DR6-Alens").
    pub dataset_id: String,
    /// Fiducial chi²/dof at ΛCDM parameters. Should be ≈1.0 if the likelihood is calibrated.
    pub fiducial_chi2_per_dof: Option<f64>,
    /// True when the fiducial check passed (chi²/dof ≤ 2.0 at ΛCDM).
    pub fiducial_ok: bool,
    /// True when this dataset was actually used in scoring (not just checked).
    pub used_in_scoring: bool,
}

impl DatasetSurvivalRecord {
    pub fn new(
        dataset_id: impl Into<String>,
        fiducial_chi2_per_dof: Option<f64>,
        used: bool,
    ) -> Self {
        let fiducial_ok = fiducial_chi2_per_dof.is_none_or(|c| c <= 2.0);
        DatasetSurvivalRecord {
            dataset_id: dataset_id.into(),
            fiducial_chi2_per_dof,
            fiducial_ok,
            used_in_scoring: used,
        }
    }
}

/// V8 Phase 10 (SYNTHESIS #8): coverage gate for the mandatory growth-sector dataset list.
///
/// No suppressed-growth claim is reportable without all five conditions satisfied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrowthCoverageGate {
    /// Number of independent full-shape RSD datasets evaluated.
    pub n_rsd_datasets: u32,
    /// Number of independent weak-lensing surveys (KiDS/DES/HSC lineage).
    pub n_wl_surveys: u32,
    /// True when at least one CMB lensing measurement is included (ACT DR6 or Planck).
    pub has_cmb_lensing: bool,
    /// True when at least one BAO dataset is included (DESI DR2 or equivalent).
    pub has_bao: bool,
    /// True when Pantheon+ full-covariance SNe dataset is included.
    pub has_sne_pantheon_plus: bool,
}

impl GrowthCoverageGate {
    /// Coverage is satisfied when:
    /// - ≥1 full-shape RSD dataset
    /// - ≥2 independent WL surveys
    /// - ≥1 CMB lensing dataset
    /// - BAO present
    /// - Pantheon+ SNe present
    ///
    /// All five must hold before a suppressed-growth claim can be scored.
    pub fn coverage_satisfied(&self) -> bool {
        self.n_rsd_datasets >= 1
            && self.n_wl_surveys >= 2
            && self.has_cmb_lensing
            && self.has_bao
            && self.has_sne_pantheon_plus
    }

    /// Human-readable list of unmet coverage requirements.
    pub fn missing_coverage(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.n_rsd_datasets < 1 {
            missing.push("full-shape RSD (≥1 dataset required)");
        }
        if self.n_wl_surveys < 2 {
            missing.push("independent WL surveys (≥2 required: KiDS/DES/HSC)");
        }
        if !self.has_cmb_lensing {
            missing.push("CMB lensing dataset (ACT DR6 or Planck required)");
        }
        if !self.has_bao {
            missing.push("BAO dataset (DESI DR2 or equivalent required)");
        }
        if !self.has_sne_pantheon_plus {
            missing.push("Pantheon+ full-covariance SNe dataset required");
        }
        missing
    }
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

// ---- DatasetSurvivalTable helper ----

/// A per-dataset survival table — must be checked before scoring.
pub type DatasetSurvivalTable = Vec<DatasetSurvivalRecord>;

/// True when all datasets in the table passed their fiducial check.
pub fn all_fiducials_pass(table: &DatasetSurvivalTable) -> bool {
    table.iter().all(|r| r.fiducial_ok)
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

    // ---- CmbLensingVerdict ----

    #[test]
    fn cmb_lensing_pull_computed_correctly() {
        let v = CmbLensingVerdict::new(1.013, 0.025, 0.998, "ACT-DR6");
        let expected = (0.998 - 1.013) / 0.025;
        assert!((v.pull - expected).abs() < 1e-12);
        assert!(!v.is_tension()); // pull ≈ 0.6
    }

    #[test]
    fn cmb_lensing_tension_detected_above_2sigma() {
        let v = CmbLensingVerdict::new(1.013, 0.025, 0.960, "ACT-DR6");
        assert!(v.is_tension()); // pull ≈ 2.12
    }

    #[test]
    fn cmb_lensing_zero_sigma_gives_zero_pull() {
        let v = CmbLensingVerdict::new(1.0, 0.0, 1.1, "test");
        assert_eq!(v.pull, 0.0);
    }

    // ---- GrowthCoverageGate ----

    #[test]
    fn coverage_satisfied_when_all_five_conditions_met() {
        let gate = GrowthCoverageGate {
            n_rsd_datasets: 2,
            n_wl_surveys: 3,
            has_cmb_lensing: true,
            has_bao: true,
            has_sne_pantheon_plus: true,
        };
        assert!(gate.coverage_satisfied());
        assert!(gate.missing_coverage().is_empty());
    }

    #[test]
    fn coverage_fails_when_only_one_wl_survey() {
        let gate = GrowthCoverageGate {
            n_rsd_datasets: 1,
            n_wl_surveys: 1,
            has_cmb_lensing: true,
            has_bao: true,
            has_sne_pantheon_plus: true,
        };
        assert!(!gate.coverage_satisfied());
        let missing = gate.missing_coverage();
        assert!(missing.iter().any(|m| m.contains("WL surveys")));
    }

    #[test]
    fn coverage_fails_when_cmb_lensing_absent() {
        let gate = GrowthCoverageGate {
            n_rsd_datasets: 1,
            n_wl_surveys: 2,
            has_cmb_lensing: false,
            has_bao: true,
            has_sne_pantheon_plus: true,
        };
        assert!(!gate.coverage_satisfied());
        let missing = gate.missing_coverage();
        assert!(missing.iter().any(|m| m.contains("CMB lensing")));
    }

    #[test]
    fn coverage_missing_returns_all_unmet_requirements() {
        let gate = GrowthCoverageGate {
            n_rsd_datasets: 0,
            n_wl_surveys: 0,
            has_cmb_lensing: false,
            has_bao: false,
            has_sne_pantheon_plus: false,
        };
        assert!(!gate.coverage_satisfied());
        assert_eq!(gate.missing_coverage().len(), 5);
    }

    // ---- DatasetSurvivalTable ----

    #[test]
    fn all_fiducials_pass_when_all_ok() {
        let table = vec![
            DatasetSurvivalRecord::new("eBOSS-DR16", Some(1.1), true),
            DatasetSurvivalRecord::new("KiDS-Legacy", Some(1.3), true),
            DatasetSurvivalRecord::new("ACT-DR6", Some(0.9), true),
        ];
        assert!(all_fiducials_pass(&table));
    }

    #[test]
    fn all_fiducials_fail_when_one_has_high_chi2() {
        let table = vec![
            DatasetSurvivalRecord::new("eBOSS-DR16", Some(1.1), true),
            DatasetSurvivalRecord::new("broken-dataset", Some(3.5), true),
        ];
        assert!(!all_fiducials_pass(&table));
    }

    #[test]
    fn survival_record_without_chi2_is_ok() {
        let rec = DatasetSurvivalRecord::new("DESI-DR2", None, true);
        assert!(rec.fiducial_ok); // None means "not checked yet" → pass
    }
}
