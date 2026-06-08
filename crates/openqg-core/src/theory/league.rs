//! Fair model selection: profile-fit a model class to the data and rank by information criteria.
//!
//! The evolution loop (`theory/evolve.rs`) is a *search heuristic* — it tunes a candidate's
//! parameters with a squashed diagonal likelihood and reports the most-fit elite. That is fine for
//! discovery but it is **not** a defensible model-selection statement: it compares a fitted
//! candidate against a *fixed* baseline under a diagonal likelihood, with no complexity penalty
//! (see `docs/zyal-next-level-design.md` §1.4). This module is the rigorous adjudicator that turns
//! "champion beats ΛCDM by +36.7" into a number a referee would accept:
//!
//! 1. Each model is a [`ModelClass`] — a fixed structural form plus a list of *free* parameters.
//! 2. Every model (the baseline included) is **profile-fit** to the *same* data: its free
//!    parameters are optimized to the maximum likelihood by deterministic Nelder–Mead, against the
//!    covariance-aware likelihood ([`crate::scoring::score_metrics_cov`]).
//! 3. Models are compared at their *best fit* with a complexity penalty — AIC, BIC, and the
//!    Schwarz/Laplace evidence approximation `ln Z ≈ −½ BIC` — so extra parameters must earn their
//!    keep. ΔAIC / Δln Z vs ΛCDM is the reported headline, not a raw Δlog-likelihood.
//!
//! Determinism: the simplex starts from each free parameter's `init`/step, Nelder–Mead is
//! coefficient-fixed, and the forward model is pure — so a fit reproduces bit-for-bit.

use crate::cosmology::{CosmologyParams, ForwardModel};
use crate::scoring::{score_metrics_cov, LikelihoodData};
use serde::Serialize;

/// A free parameter of a model class: the name of a [`CosmologyParams`] field, a starting value,
/// and inclusive bounds the optimizer is clamped to (physical priors).
#[derive(Debug, Clone)]
pub struct FreeParam {
    pub name: &'static str,
    pub init: f64,
    pub lo: f64,
    pub hi: f64,
}

impl FreeParam {
    pub fn new(name: &'static str, init: f64, lo: f64, hi: f64) -> Self {
        FreeParam {
            name,
            init,
            lo,
            hi,
        }
    }
}

/// Set a named [`CosmologyParams`] field. Returns false for an unknown name (a programming error
/// in a model definition, surfaced rather than silently ignored).
fn set_param(c: &mut CosmologyParams, name: &str, v: f64) -> bool {
    match name {
        "h" => c.h = v,
        "omega_m" => c.omega_m = v,
        "omega_b_h2" => c.omega_b_h2 = v,
        "n_eff" => c.n_eff = v,
        "sum_mnu" => c.sum_mnu = v,
        "w0" => c.w0 = v,
        "wa" => c.wa = v,
        "omega_k" => c.omega_k = v,
        _ => return false,
    }
    true
}

/// A model class: a fixed structural form (`base` cosmology + any extra metadata) and the
/// parameters left free to fit. ΛCDM fixes `w0=-1, wa=0`; w0waCDM frees them; etc.
#[derive(Debug, Clone)]
pub struct ModelClass {
    pub id: String,
    /// Human-readable physical description (for the league artifact).
    pub description: String,
    /// Fixed structural form; free parameters overwrite their fields during the fit.
    pub base: CosmologyParams,
    /// Parameters optimized to the data.
    pub free: Vec<FreeParam>,
}

impl ModelClass {
    /// ΛCDM: GR + cosmological constant. Free: H0, Ω_m (w fixed at −1).
    pub fn lcdm() -> Self {
        ModelClass {
            id: "lcdm".into(),
            description: "GR + cosmological constant (w = -1)".into(),
            base: CosmologyParams::planck_lcdm(),
            free: vec![
                FreeParam::new("h", 0.674, 0.55, 0.80),
                FreeParam::new("omega_m", 0.315, 0.20, 0.45),
            ],
        }
    }

    /// w0waCDM: CPL evolving dark energy. Free: H0, Ω_m, w0, wa (the DESI DR2 headline class).
    pub fn w0wa_cdm() -> Self {
        let mut base = CosmologyParams::planck_lcdm();
        base.w0 = -0.9;
        base.wa = -0.3;
        ModelClass {
            id: "w0wacdm".into(),
            description: "CPL evolving dark energy w(a) = w0 + wa(1-a)".into(),
            base,
            free: vec![
                FreeParam::new("h", 0.674, 0.55, 0.80),
                FreeParam::new("omega_m", 0.315, 0.20, 0.45),
                FreeParam::new("w0", -0.9, -2.0, -0.3),
                FreeParam::new("wa", -0.3, -3.0, 1.0),
            ],
        }
    }

    /// wCDM: constant equation of state (one extra parameter over ΛCDM). Free: H0, Ω_m, w0.
    pub fn w_cdm() -> Self {
        let mut base = CosmologyParams::planck_lcdm();
        base.w0 = -1.0;
        ModelClass {
            id: "wcdm".into(),
            description: "constant-w dark energy (wa = 0)".into(),
            base,
            free: vec![
                FreeParam::new("h", 0.674, 0.55, 0.80),
                FreeParam::new("omega_m", 0.315, 0.20, 0.45),
                FreeParam::new("w0", -1.0, -2.0, -0.3),
            ],
        }
    }

    /// Build the [`CosmologyParams`] for a given free-parameter vector (clamped to bounds).
    fn params_for(&self, x: &[f64]) -> CosmologyParams {
        let mut c = self.base.clone();
        for (p, &v) in self.free.iter().zip(x) {
            set_param(&mut c, p.name, v.clamp(p.lo, p.hi));
        }
        c
    }
}

/// The result of profile-fitting one model to one dataset.
#[derive(Debug, Clone, Serialize)]
pub struct FitResult {
    pub model_id: String,
    pub description: String,
    /// Best-fit free-parameter values (in `free` order).
    pub best_params: Vec<(String, f64)>,
    /// Maximum log-likelihood found.
    pub log_likelihood: f64,
    /// χ² = −2 ln L at the best fit.
    pub chi2: f64,
    /// Number of free (fitted) parameters.
    pub k: usize,
    /// Number of data points scored.
    pub n_data: usize,
    /// Akaike information criterion 2k − 2 ln L (lower is better).
    pub aic: f64,
    /// Bayesian/Schwarz information criterion k ln n − 2 ln L (lower is better).
    pub bic: f64,
    /// Coverage (fraction of observables the model could predict).
    pub coverage: f64,
}

/// Profile-fit a model class to the data: optimize its free parameters to the maximum
/// (covariance-aware) likelihood with deterministic Nelder–Mead, and report the information
/// criteria at the best fit.
pub fn fit_model<M>(model: &ModelClass, data: &LikelihoodData, fwd: &M) -> FitResult
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    let ids: Vec<String> = data
        .observables
        .iter()
        .map(|o| o.observable_id.clone())
        .collect();
    let k = model.free.len();

    // Objective: χ² = −2 ln L (Nelder–Mead minimizes). A forward-model failure is +∞.
    let neg2_loglik = |x: &[f64]| -> f64 {
        let c = model.params_for(x);
        match fwd.predict(&c, &ids) {
            Ok(preds) => {
                let (m, _) = score_metrics_cov(data, &preds, k.max(1), 0.0);
                -2.0 * m.log_likelihood
            }
            Err(_) => f64::INFINITY,
        }
    };

    let x0: Vec<f64> = model.free.iter().map(|p| p.init).collect();
    let steps: Vec<f64> = model
        .free
        .iter()
        .map(|p| 0.05 * (p.hi - p.lo).abs().max(1e-3))
        .collect();
    let best_x = if k == 0 {
        x0
    } else {
        nelder_mead(&neg2_loglik, &x0, &steps, 4000, 1e-10)
    };
    let best = model.params_for(&best_x);

    let preds = fwd.predict(&best, &ids).unwrap_or_default();
    let (metrics, _) = score_metrics_cov(data, &preds, k.max(1), 0.0);
    let log_likelihood = metrics.log_likelihood;
    let n_data = data.observables.len();
    let kf = k as f64;
    let nf = n_data.max(1) as f64;

    FitResult {
        model_id: model.id.clone(),
        description: model.description.clone(),
        best_params: model
            .free
            .iter()
            .zip(&best_x)
            .map(|(p, &v)| (p.name.to_string(), v.clamp(p.lo, p.hi)))
            .collect(),
        log_likelihood,
        chi2: -2.0 * log_likelihood,
        k,
        n_data,
        aic: 2.0 * kf - 2.0 * log_likelihood,
        bic: kf * nf.ln() - 2.0 * log_likelihood,
        coverage: metrics.coverage,
    }
}

/// One row of the model-selection league table: a fitted model compared to the reference.
#[derive(Debug, Clone, Serialize)]
pub struct LeagueRow {
    pub fit: FitResult,
    /// ΔAIC vs the reference model (negative ⇒ better than reference).
    pub delta_aic: f64,
    /// ΔBIC vs the reference model (negative ⇒ better than reference).
    pub delta_bic: f64,
    /// Schwarz/Laplace log-evidence difference Δln Z ≈ −½ ΔBIC (positive ⇒ favored).
    pub delta_ln_evidence: f64,
}

/// Fit every model to the data and rank them against a reference model (by id). The reference is
/// the null every challenger must beat; ΔAIC / Δln Z are reported relative to it. Rows are sorted
/// by AIC ascending (best first).
pub fn model_league<M>(
    models: &[ModelClass],
    data: &LikelihoodData,
    fwd: &M,
    reference_id: &str,
) -> Vec<LeagueRow>
where
    M: ForwardModel<Theory = CosmologyParams>,
{
    let fits: Vec<FitResult> = models.iter().map(|m| fit_model(m, data, fwd)).collect();
    let ref_aic = fits
        .iter()
        .find(|f| f.model_id == reference_id)
        .map(|f| f.aic)
        .unwrap_or(0.0);
    let ref_bic = fits
        .iter()
        .find(|f| f.model_id == reference_id)
        .map(|f| f.bic)
        .unwrap_or(0.0);
    let mut rows: Vec<LeagueRow> = fits
        .into_iter()
        .map(|f| {
            let delta_aic = f.aic - ref_aic;
            let delta_bic = f.bic - ref_bic;
            LeagueRow {
                fit: f,
                delta_aic,
                delta_bic,
                delta_ln_evidence: -0.5 * delta_bic,
            }
        })
        .collect();
    rows.sort_by(|a, b| a.fit.aic.partial_cmp(&b.fit.aic).unwrap_or(std::cmp::Ordering::Equal));
    rows
}

/// Deterministic Nelder–Mead simplex minimizer. Standard coefficients (reflection 1, expansion 2,
/// contraction 0.5, shrink 0.5). No randomness, so a fit is reproducible. Stops at `max_iter`
/// evaluations or when the simplex spread falls below `tol`.
fn nelder_mead<F: Fn(&[f64]) -> f64>(
    f: &F,
    x0: &[f64],
    steps: &[f64],
    max_iter: usize,
    tol: f64,
) -> Vec<f64> {
    let n = x0.len();
    // Build the initial simplex: x0 and x0 + step·e_i.
    let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(n + 1);
    simplex.push(x0.to_vec());
    for i in 0..n {
        let mut v = x0.to_vec();
        v[i] += steps[i];
        simplex.push(v);
    }
    let mut fvals: Vec<f64> = simplex.iter().map(|v| f(v)).collect();
    let mut iters = 0usize;

    while iters < max_iter {
        // Order by function value (best first).
        let mut order: Vec<usize> = (0..=n).collect();
        order.sort_by(|&a, &b| fvals[a].partial_cmp(&fvals[b]).unwrap_or(std::cmp::Ordering::Equal));
        let best = order[0];
        let worst = order[n];
        let second_worst = order[n - 1];

        // Convergence: spread of function values.
        if (fvals[worst] - fvals[best]).abs() <= tol * (1.0 + fvals[best].abs()) {
            break;
        }

        // Centroid of all but the worst.
        let mut centroid = vec![0.0; n];
        for (i, s) in simplex.iter().enumerate() {
            if i == worst {
                continue;
            }
            for d in 0..n {
                centroid[d] += s[d] / n as f64;
            }
        }
        let reflect = |coef: f64| -> Vec<f64> {
            (0..n)
                .map(|d| centroid[d] + coef * (centroid[d] - simplex[worst][d]))
                .collect()
        };

        let xr = reflect(1.0);
        let fr = f(&xr);
        iters += 1;

        if fr < fvals[best] {
            // Expansion.
            let xe = reflect(2.0);
            let fe = f(&xe);
            iters += 1;
            if fe < fr {
                simplex[worst] = xe;
                fvals[worst] = fe;
            } else {
                simplex[worst] = xr;
                fvals[worst] = fr;
            }
        } else if fr < fvals[second_worst] {
            simplex[worst] = xr;
            fvals[worst] = fr;
        } else {
            // Contraction.
            let xc = reflect(0.5);
            let fc = f(&xc);
            iters += 1;
            if fc < fvals[worst] {
                simplex[worst] = xc;
                fvals[worst] = fc;
            } else {
                // Shrink toward the best vertex.
                let xb = simplex[best].clone();
                for (i, s) in simplex.iter_mut().enumerate() {
                    if i == best {
                        continue;
                    }
                    for d in 0..n {
                        s[d] = xb[d] + 0.5 * (s[d] - xb[d]);
                    }
                    fvals[i] = f(s);
                    iters += 1;
                }
            }
        }
    }

    // Return the best vertex.
    let mut best = 0usize;
    for i in 1..=n {
        if fvals[i] < fvals[best] {
            best = i;
        }
    }
    simplex[best].clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cosmology::BackgroundForwardModel;
    use crate::types::ObservableRecord;

    fn tier0() -> LikelihoodData {
        // A handful of DESI DR1 BAO points + BBN + the CMB priors (diagonal here; covariance
        // blocks are exercised in the covariance module's tests).
        let rows = [
            ("dm_over_rd@0.510", 13.62, 0.25),
            ("dh_over_rd@0.510", 20.98, 0.61),
            ("dm_over_rd@0.930", 21.71, 0.28),
            ("dh_over_rd@0.930", 17.88, 0.35),
            ("dm_over_rd@2.330", 39.71, 0.94),
            ("dh_over_rd@2.330", 8.52, 0.17),
            ("bbn_yp", 0.2453, 0.0034),
            ("cmb_R", 1.7502, 0.0046),
            ("cmb_lA", 301.471, 0.090),
        ];
        let observables = rows
            .iter()
            .map(|(id, v, s)| ObservableRecord {
                observable_id: (*id).into(),
                kind: "cosmo".into(),
                value: *v,
                uncertainty: *s,
                unit: "dimensionless".into(),
                source: None,
            })
            .collect();
        LikelihoodData::diagonal(observables)
    }

    #[test]
    fn nelder_mead_minimizes_a_quadratic() {
        // f(x) = (x0-3)^2 + (x1+1)^2, min at (3,-1).
        let f = |x: &[f64]| (x[0] - 3.0).powi(2) + (x[1] + 1.0).powi(2);
        let x = nelder_mead(&f, &[0.0, 0.0], &[1.0, 1.0], 2000, 1e-12);
        assert!((x[0] - 3.0).abs() < 1e-4, "x0={}", x[0]);
        assert!((x[1] + 1.0).abs() < 1e-4, "x1={}", x[1]);
    }

    #[test]
    fn fit_lcdm_recovers_a_reasonable_h_and_omega_m() {
        let data = tier0();
        let fit = fit_model(&ModelClass::lcdm(), &data, &BackgroundForwardModel);
        let h = fit.best_params.iter().find(|(n, _)| n == "h").unwrap().1;
        let om = fit.best_params.iter().find(|(n, _)| n == "omega_m").unwrap().1;
        // DESI+CMB-prior best-fit ΛCDM sits near Planck/DESI values.
        assert!(h > 0.64 && h < 0.71, "h = {h}");
        assert!(om > 0.27 && om < 0.34, "omega_m = {om}");
        assert!(fit.log_likelihood.is_finite());
        assert_eq!(fit.k, 2);
    }

    #[test]
    fn fitting_lcdm_beats_a_detuned_start() {
        // The fitted ΛCDM log-L must be at least as good as the fixed Planck baseline's.
        let data = tier0();
        let fit = fit_model(&ModelClass::lcdm(), &data, &BackgroundForwardModel);
        let ids: Vec<String> = data
            .observables
            .iter()
            .map(|o| o.observable_id.clone())
            .collect();
        let preds = BackgroundForwardModel
            .predict(&CosmologyParams::planck_lcdm(), &ids)
            .unwrap();
        let (fixed, _) = score_metrics_cov(&data, &preds, 2, 0.0);
        assert!(
            fit.log_likelihood >= fixed.log_likelihood - 1e-6,
            "fit {} should beat fixed {}",
            fit.log_likelihood,
            fixed.log_likelihood
        );
    }

    #[test]
    fn league_ranks_models_and_penalizes_free_parameters() {
        let data = tier0();
        let models = vec![
            ModelClass::lcdm(),
            ModelClass::w_cdm(),
            ModelClass::w0wa_cdm(),
        ];
        let rows = model_league(&models, &data, &BackgroundForwardModel, "lcdm");
        // Reference row has ΔAIC = 0.
        let lcdm = rows.iter().find(|r| r.fit.model_id == "lcdm").unwrap();
        assert!(lcdm.delta_aic.abs() < 1e-9);
        // More-flexible models cannot have a worse maximum likelihood (nested).
        let w0wa = rows.iter().find(|r| r.fit.model_id == "w0wacdm").unwrap();
        assert!(
            w0wa.fit.log_likelihood >= lcdm.fit.log_likelihood - 1e-3,
            "w0wa logL {} vs lcdm {}",
            w0wa.fit.log_likelihood,
            lcdm.fit.log_likelihood
        );
        // Rows are AIC-sorted.
        for w in rows.windows(2) {
            assert!(w[0].fit.aic <= w[1].fit.aic + 1e-9);
        }
    }
}

/// Regression guard pinning the *honest* Tier-0 model-selection result, which contradicts the old
/// "+36.7 log-likelihood, beats ΛCDM" headline (`docs/production-run-1000.md`). When ΛCDM is itself
/// re-fit to the same 15 observables (not held at a fixed Planck baseline) and CPL's two extra
/// parameters are penalized, evolving dark energy is *disfavored* on DESI DR1 + CMB-prior + BBN:
/// the raw χ² improvement is only ~a few, and ΔAIC > 0 / Δln Z < 0. This is the defensible number.
#[cfg(test)]
mod tier0_honest_number {
    use super::*;
    use crate::cosmology::BackgroundForwardModel;
    use crate::types::ObservableRecord;
    use std::io::BufRead;

    fn real_tier0() -> LikelihoodData {
        let f = std::fs::File::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/fixtures/cosmology/tier0-combined.jsonl"
        ))
        .expect("tier0-combined.jsonl");
        let observables: Vec<ObservableRecord> = std::io::BufReader::new(f)
            .lines()
            .map_while(Result::ok)
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(&l).unwrap())
            .collect();
        LikelihoodData::diagonal(observables)
    }

    #[test]
    fn evolving_de_is_not_favored_once_lcdm_is_refit_and_params_penalized() {
        let data = real_tier0();
        assert_eq!(data.observables.len(), 15);
        let models = vec![ModelClass::lcdm(), ModelClass::w_cdm(), ModelClass::w0wa_cdm()];
        let rows = model_league(&models, &data, &BackgroundForwardModel, "lcdm");

        let lcdm = rows.iter().find(|r| r.fit.model_id == "lcdm").unwrap();
        let w0wa = rows.iter().find(|r| r.fit.model_id == "w0wacdm").unwrap();

        // The raw χ² improvement of CPL over a RE-FIT ΛCDM is small (~a few), NOT ~73 (=2·36.7).
        let raw_chi2_gain = lcdm.fit.chi2 - w0wa.fit.chi2;
        assert!(
            raw_chi2_gain > 0.0 && raw_chi2_gain < 12.0,
            "raw Δχ² = {raw_chi2_gain} (expected a few, never ~73)"
        );
        // With the 2-parameter penalty, evolving DE is DISFAVORED on this data: ΔAIC > 0.
        assert!(
            w0wa.delta_aic > 0.0,
            "w0waCDM ΔAIC = {} should be > 0 (disfavored)",
            w0wa.delta_aic
        );
        // And the Schwarz evidence prefers ΛCDM.
        assert!(
            w0wa.delta_ln_evidence < 0.0,
            "w0waCDM Δln Z = {} should be < 0",
            w0wa.delta_ln_evidence
        );
        // Best-ranked (lowest AIC) model is ΛCDM.
        assert_eq!(rows[0].fit.model_id, "lcdm");
    }
}
