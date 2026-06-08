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
            "cmb_R" => Some((c.cmb_shift_r(), 0.001, "dimensionless")),
            "cmb_lA" => Some((c.cmb_acoustic_scale(), 0.01, "dimensionless")),
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
                    Some((c.growth_fsigma8(z), 1e-4, "dimensionless"))
                } else {
                    None
                }
            }
        }
    }
}

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
        let fs8 = preds.iter().find(|p| p.observable_id == "fsigma8@0.5").unwrap();
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
