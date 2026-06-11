//! The `ForwardModel` seam: the single abstraction the evolution loop talks to, so that *how*
//! a theory's observables are computed (parametric stand-in today, in-repo derivation engine,
//! or an external Boltzmann backend tomorrow) can change without touching selection.
//!
//! A model maps a theory representation to `PredictionRecord`s and reports a `ForwardManifest`
//! describing exactly which code + data produced them, so a score is reproducible and the
//! provenance is auditable.

use crate::types::PredictionRecord;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// What a forward model fundamentally is — used to flag the fidelity of a prediction set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardKind {
    /// Transparent parametric stand-in (e.g. an identity/heuristic map). NOT a solver.
    Parametric,
    /// In-repo numeric derivation (ODE / quadrature integration from physical parameters).
    Derivation,
    /// External Boltzmann solver backend (CLASS / hi_class / cobaya).
    Boltzmann,
}

/// Reproducibility receipt for a forward model: stamped into every score so two runs with the
/// same manifest hash are provably comparable, and a different code/data version is visible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForwardManifest {
    /// Stable identifier of the model implementation.
    pub model_id: String,
    /// Version of the model / code path.
    pub version: String,
    /// What class of computation produced the predictions.
    pub kind: ForwardKind,
    /// Content hash of the external code + data the predictions depend on. Empty for a pure,
    /// deterministic in-repo model that needs no external inputs.
    pub provenance_hash: String,
}

/// A forward model turns a theory representation into predicted observables.
///
/// The `Theory` associated type keeps this decoupled from any one representation: the
/// background engine below uses [`super::CosmologyParams`]; a future symbolic-derivation
/// engine will use the `Theory` AST without changing this trait or the selection loop.
pub trait ForwardModel {
    /// The theory representation this model maps to observables.
    type Theory;

    /// Predict the requested observables. A model returns a record only for the observables it
    /// can genuinely compute — an unpredictable observable (e.g. σ8 from a background-only
    /// model) is *omitted*, never faked, so coverage honestly reflects what the theory derives.
    fn predict(
        &self,
        theory: &Self::Theory,
        observable_ids: &[String],
    ) -> Result<Vec<PredictionRecord>>;

    /// Reproducibility receipt for this model.
    fn manifest(&self) -> ForwardManifest;
}

/// The pure-Rust background-cosmology forward model (Tier-0 observables). Deterministic and
/// self-contained: distances, BAO ratios, sound horizon, and BBN Y_p are integrated from the
/// parameters; observables that require linear growth or the full CMB are deliberately omitted
/// (they are the optional Boltzmann backend's job).
#[derive(Debug, Clone, Default)]
pub struct BackgroundForwardModel;

impl BackgroundForwardModel {
    pub const MODEL_ID: &'static str = "openqg-background";
    pub const VERSION: &'static str = "0.1.0";

    /// Compute a single observable id, or `None` if this background model cannot derive it.
    /// Redshift-dependent observables use the `name@<z>` convention, e.g. `dv_over_rd@0.51`.
    fn compute(&self, c: &super::CosmologyParams, id: &str) -> Option<(f64, f64, &'static str)> {
        // (value, default 1σ model uncertainty, unit). The uncertainty is a small numerical
        // floor; the data record's own uncertainty dominates the likelihood.
        let parse_z = |id: &str, prefix: &str| -> Option<f64> {
            id.strip_prefix(prefix)
                .and_then(|rest| rest.strip_prefix('@'))
                .and_then(|z| z.parse::<f64>().ok())
        };
        match id {
            "h0" | "h0_local" => Some((c.h0(), 0.001, "km s^-1 Mpc^-1")),
            "omega_m" => Some((c.omega_m, 1e-6, "dimensionless")),
            "sum_mnu" => Some((c.sum_mnu, 1e-6, "eV")),
            "n_eff" => Some((c.n_eff, 1e-6, "dimensionless")),
            "omega_b_h2" => Some((c.omega_b_h2, 1e-9, "dimensionless")),
            "r_drag" => Some((c.sound_horizon_drag(), 0.05, "Mpc")),
            "bbn_yp" | "yp" => Some((c.bbn_helium_fraction(), 1e-5, "dimensionless")),
            // Compressed CMB distance priors (computable from the background alone).
            // V6.1 (P0.11, CRITICAL): fitting-formula-grade predictions must never meet
            // Boltzmann-grade data raw — the engine's lA carried a +0.755 (8.4σ) bias at the
            // Planck anchor and the V6 campaign optimizer harvested ~35 nats of pure model
            // error by drifting h to shift lA. Anchor-calibrate at Planck-2018 best fit
            // (same precedent as the Aubourg r_drag treatment): planck_lcdm() now predicts the
            // published distance priors exactly; deviations measure PHYSICS, not formula bias.
            // Guard: `calibrated_anchor_matches_planck_distance_priors` below.
            "cmb_R" => Some((
                c.cmb_shift_r() + CMB_R_ANCHOR_CALIBRATION,
                0.001,
                "dimensionless",
            )),
            "cmb_lA" => Some((
                c.cmb_acoustic_scale() + CMB_LA_ANCHOR_CALIBRATION,
                0.01,
                "dimensionless",
            )),
            // The third Planck-2018 compressed-CMB prior (Chen, Huang & Wang 2019,
            // arXiv:1808.05724): the baryon density ω_b h². It is a background parameter, so the
            // model just reports it; the `cmb_` alias lets the 3×3 (R, ℓ_A, ω_b h²) covariance
            // block (`scoring/covariance.rs`) name a single ordered observable set.
            "cmb_omega_b_h2" => Some((c.omega_b_h2, 1e-9, "dimensionless")),
            // Growth-of-structure observables (Tier-1): linear growth integrated from the
            // background + the late-time μ0 modified-gravity handle.
            "s8" | "S8" => Some((c.s8(), 1e-4, "dimensionless")),
            "sigma8" => Some((c.sigma8, 1e-6, "dimensionless")),
            _ => {
                if let Some(z) = parse_z(id, "dm_over_rd") {
                    Some((c.bao_dm_over_rd(z), 0.01, "dimensionless"))
                } else if let Some(z) = parse_z(id, "dh_over_rd") {
                    Some((c.bao_dh_over_rd(z), 0.01, "dimensionless"))
                } else if let Some(z) = parse_z(id, "dv_over_rd") {
                    Some((c.bao_dv_over_rd(z), 0.01, "dimensionless"))
                } else if let Some(z) = parse_z(id, "mu") {
                    Some((c.distance_modulus(z), 1e-4, "mag"))
                } else if let Some(z) = parse_z(id, "fsigma8") {
                    // M4: a declared *derived* MG family (f(R)/nDGP) grows the reference k-mode with
                    // its computed scale-dependent μ(a,k); plain ΛCDM/μ0 keeps the scale-free path
                    // (growth_fsigma8_kref == growth_fsigma8 when mg_family == None — byte-identical).
                    let fs8 = if c.mg_family == super::MgFamily::None {
                        c.growth_fsigma8(z)
                    } else {
                        c.growth_fsigma8_kref(z)
                    };
                    Some((fs8, 1e-4, "dimensionless"))
                } else {
                    None
                }
            }
        }
    }
}

/// V6.1 anchor calibrations: published Planck-2018 distance priors (Chen, Huang & Wang 2019,
/// Table I) minus this engine's fitting-formula predictions at `CosmologyParams::planck_lcdm()`.
/// Measured 2026-06-11: raw lA = 302.225598 vs 301.471 published; raw R = 1.749003 vs 1.7502.
pub const CMB_LA_ANCHOR_CALIBRATION: f64 = 301.471 - 302.225_598;
pub const CMB_R_ANCHOR_CALIBRATION: f64 = 1.7502 - 1.749_003;

impl ForwardModel for BackgroundForwardModel {
    type Theory = super::CosmologyParams;

    fn predict(
        &self,
        theory: &Self::Theory,
        observable_ids: &[String],
    ) -> Result<Vec<PredictionRecord>> {
        let mut out = Vec::new();
        for id in observable_ids {
            if let Some((value, uncertainty, unit)) = self.compute(theory, id) {
                out.push(PredictionRecord {
                    observable_id: id.clone(),
                    value,
                    uncertainty,
                    unit: unit.to_string(),
                    theory_id: Some(Self::MODEL_ID.to_string()),
                });
            }
        }
        Ok(out)
    }

    fn manifest(&self) -> ForwardManifest {
        ForwardManifest {
            model_id: Self::MODEL_ID.to_string(),
            version: Self::VERSION.to_string(),
            kind: ForwardKind::Derivation,
            // Pure, deterministic, no external inputs ⇒ no external provenance to hash.
            provenance_hash: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::CosmologyParams;
    use super::*;

    #[test]
    fn predicts_derived_and_parameter_observables() {
        let model = BackgroundForwardModel;
        let c = CosmologyParams::planck_lcdm();
        let ids: Vec<String> = ["h0", "dv_over_rd@0.51", "bbn_yp", "r_drag", "mu@0.5"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let preds = model.predict(&c, &ids).unwrap();
        assert_eq!(preds.len(), 5);
        let h0 = preds.iter().find(|p| p.observable_id == "h0").unwrap();
        assert!((h0.value - 67.4).abs() < 1e-6);
        let dv = preds
            .iter()
            .find(|p| p.observable_id == "dv_over_rd@0.51")
            .unwrap();
        assert!(dv.value > 10.0 && dv.value < 16.0);
    }

    #[test]
    fn omits_observables_it_cannot_derive_rather_than_faking() {
        // The background+growth model must NOT invent a full-CMB-spectrum observable (a C_ℓ band
        // power needs the Boltzmann backend) — coverage stays honest.
        let model = BackgroundForwardModel;
        let c = CosmologyParams::planck_lcdm();
        let preds = model
            .predict(&c, &["cl_tt@220".to_string(), "h0".to_string()])
            .unwrap();
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].observable_id, "h0");
    }

    #[test]
    fn predicts_growth_observables() {
        let model = BackgroundForwardModel;
        let c = CosmologyParams::planck_lcdm();
        let ids: Vec<String> = ["fsigma8@0.5", "s8", "sigma8"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let preds = model.predict(&c, &ids).unwrap();
        assert_eq!(preds.len(), 3);
        let fs8 = preds
            .iter()
            .find(|p| p.observable_id == "fsigma8@0.5")
            .unwrap();
        assert!(fs8.value > 0.40 && fs8.value < 0.50, "fσ8 = {}", fs8.value);
    }

    #[test]
    fn manifest_is_stable_and_marks_derivation() {
        let m = BackgroundForwardModel.manifest();
        assert_eq!(m.kind, ForwardKind::Derivation);
        assert_eq!(m.model_id, "openqg-background");
        assert!(m.provenance_hash.is_empty());
    }
}

#[cfg(test)]
mod v61_calibration_guard {
    use super::*;
    use crate::cosmology::CosmologyParams;

    /// THE P0.11 guard: the ΛCDM baseline must hit the published Planck distance priors at the
    /// anchor — a drifting fitting formula re-opens the model-bias exploit and fails here first.
    #[test]
    fn calibrated_anchor_matches_planck_distance_priors() {
        let model = BackgroundForwardModel;
        let p = CosmologyParams::planck_lcdm();
        let ids = vec![
            "cmb_R".to_string(),
            "cmb_lA".to_string(),
            "cmb_omega_b_h2".to_string(),
        ];
        let preds = model.predict(&p, &ids).unwrap();
        let get = |id: &str| preds.iter().find(|x| x.observable_id == id).unwrap().value;
        // Published values + sigmas: R 1.7502±0.0046, lA 301.471±0.090, wb 0.02236±0.00015.
        assert!((get("cmb_R") - 1.7502).abs() < 0.2 * 0.0046, "R off anchor");
        assert!(
            (get("cmb_lA") - 301.471).abs() < 0.2 * 0.090,
            "lA off anchor"
        );
        assert!(
            (get("cmb_omega_b_h2") - 0.02236).abs() < 0.5 * 0.000_15,
            "omega_b_h2 off anchor"
        );
    }
}
