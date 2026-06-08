//! Covariance-aware Gaussian likelihood for correlated observables.
//!
//! The default [`super::score_metrics`] sums *independent* Gaussians — fine for uncorrelated
//! points, but wrong for the data sets this engine actually adjudicates: the DESI BAO `D_M`/`D_H`
//! at one tracer are correlated, the Planck compressed CMB priors `(R, ℓ_A, ω_b)` come with a 3×3
//! covariance, and the Pantheon+ SNe carry a full magnitude covariance. Treating those diagonal
//! mis-estimates χ² and inflates Δlog-likelihood — see `docs/zyal-next-level-design.md` §1.4.
//!
//! This module adds a **block covariance** likelihood: a candidate's residual vector over a
//! correlated block is scored as `−½ rᵀ C⁻¹ r` (the multivariate-Gaussian exponent, dropping the
//! `−½ ln|2πC|` normalization, which is identical across models on fixed data and cancels in every
//! Δ we report). Observables not named in any block fall back to the independent diagonal term, so
//! this is a strict generalization of the diagonal likelihood — with no blocks it reproduces it.
//!
//! Missing predictions are handled by **marginalization**: the marginal of a multivariate Gaussian
//! over a subset of components is just the covariance submatrix of those components, so if a model
//! cannot predict one member of a block we score the present members against the corresponding
//! sub-covariance (and record the omission), never faking the missing one.

use crate::{ObservableRecord, PredictionRecord, ScoreMetrics};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A dense covariance block over a named, ordered set of observables. `matrix` is the symmetric
/// positive-definite covariance of the observables listed in `ids`, in that order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CovarianceBlock {
    /// Observable ids, defining the row/column order of `matrix`.
    pub ids: Vec<String>,
    /// Symmetric positive-definite covariance, `ids.len()` square.
    pub matrix: Vec<Vec<f64>>,
}

impl CovarianceBlock {
    /// Build a block from a diagonal of 1σ uncertainties (a convenience for testing / degenerate
    /// blocks): C = diag(σ²).
    pub fn from_sigmas(ids: Vec<String>, sigmas: &[f64]) -> Self {
        let n = ids.len();
        let mut matrix = vec![vec![0.0; n]; n];
        for (i, s) in sigmas.iter().enumerate().take(n) {
            matrix[i][i] = s * s;
        }
        CovarianceBlock { ids, matrix }
    }

    /// Validate the block is square and matches `ids`.
    pub fn is_well_formed(&self) -> bool {
        let n = self.ids.len();
        n > 0 && self.matrix.len() == n && self.matrix.iter().all(|row| row.len() == n)
    }
}

/// A likelihood dataset: the observables plus any covariance blocks linking correlated subsets.
/// Observables not covered by a block are scored independently with their own `uncertainty`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LikelihoodData {
    pub observables: Vec<ObservableRecord>,
    #[serde(default)]
    pub blocks: Vec<CovarianceBlock>,
}

impl LikelihoodData {
    /// A dataset with no correlations (every observable independent).
    pub fn diagonal(observables: Vec<ObservableRecord>) -> Self {
        LikelihoodData {
            observables,
            blocks: Vec::new(),
        }
    }

    /// Attach a covariance block.
    pub fn with_block(mut self, block: CovarianceBlock) -> Self {
        self.blocks.push(block);
        self
    }
}

/// In-place Cholesky factorization `A = L Lᵀ` (lower triangular). Returns `None` if `A` is not
/// positive-definite (a non-PD covariance is a data error we refuse to score against).
fn cholesky(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut l = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i][j];
            sum -= l[i][..j]
                .iter()
                .zip(&l[j][..j])
                .map(|(x, y)| x * y)
                .sum::<f64>();
            if i == j {
                if sum <= 0.0 || !sum.is_finite() {
                    return None;
                }
                l[i][j] = sum.sqrt();
            } else {
                l[i][j] = sum / l[j][j];
            }
        }
    }
    Some(l)
}

/// Solve `A x = b` for symmetric positive-definite `A` via its Cholesky factor `L` (forward then
/// back substitution). Returns `None` if `A` is not PD.
fn cholesky_solve(a: &[Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = a.len();
    let l = cholesky(a)?;
    // Forward solve L y = b.
    let mut y = vec![0.0_f64; n];
    for i in 0..n {
        let mut sum = b[i];
        for k in 0..i {
            sum -= l[i][k] * y[k];
        }
        y[i] = sum / l[i][i];
    }
    // Back solve Lᵀ x = y.
    let mut x = vec![0.0_f64; n];
    for i in (0..n).rev() {
        let mut sum = y[i];
        for k in (i + 1)..n {
            sum -= l[k][i] * x[k];
        }
        x[i] = sum / l[i][i];
    }
    Some(x)
}

/// The χ² quadratic form `rᵀ C⁻¹ r` for a residual vector against a covariance, via Cholesky.
/// Returns `None` if `C` is not positive-definite.
pub fn chi2_quadratic_form(residual: &[f64], covariance: &[Vec<f64>]) -> Option<f64> {
    let x = cholesky_solve(covariance, residual)?;
    Some(residual.iter().zip(&x).map(|(r, xi)| r * xi).sum())
}

/// Extract the principal submatrix of `matrix` at the given index set (preserving order).
fn submatrix(matrix: &[Vec<f64>], idx: &[usize]) -> Vec<Vec<f64>> {
    idx.iter()
        .map(|&i| idx.iter().map(|&j| matrix[i][j]).collect())
        .collect()
}

/// Covariance-aware scoring. For each covariance block, the present (predicted) members are scored
/// jointly as `−½ rᵀ C⁻¹ r` against the corresponding sub-covariance (marginalizing over any
/// member the model could not predict). Observables in no block are scored with the independent
/// diagonal term `−½ (r/σ)²` using `σ = max(obs_σ, pred_σ)`. `delta_log_likelihood` is relative to
/// `baseline_log_likelihood`. A non-PD block covariance is a hard error: that block contributes
/// `−∞` (the candidate cannot be scored against malformed data).
pub fn score_metrics_cov(
    data: &LikelihoodData,
    predictions: &[PredictionRecord],
    parameter_count: usize,
    baseline_log_likelihood: f64,
) -> (ScoreMetrics, Vec<String>) {
    let mut pred_map: BTreeMap<&str, &PredictionRecord> = BTreeMap::new();
    for p in predictions {
        pred_map.insert(p.observable_id.as_str(), p);
    }
    let obs_map: BTreeMap<&str, &ObservableRecord> = data
        .observables
        .iter()
        .map(|o| (o.observable_id.as_str(), o))
        .collect();

    let mut log_likelihood = 0.0;
    let mut matched = 0usize;
    let mut findings = Vec::new();
    // Track which observables a block consumed, so the diagonal pass skips them.
    let mut in_block: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();

    for block in &data.blocks {
        if !block.is_well_formed() {
            findings.push(format!(
                "malformed covariance block over {:?}",
                block.ids
            ));
            continue;
        }
        // Members present in BOTH the data and the predictions; marginalize over the rest.
        let mut present_idx = Vec::new();
        let mut residual = Vec::new();
        for (i, id) in block.ids.iter().enumerate() {
            in_block.insert(id.as_str());
            match (obs_map.get(id.as_str()), pred_map.get(id.as_str())) {
                (Some(o), Some(p)) => {
                    present_idx.push(i);
                    residual.push(o.value - p.value);
                    matched += 1;
                    if o.unit.trim() != p.unit.trim() {
                        findings.push(format!(
                            "unit mismatch for {}: {} vs {}",
                            id, o.unit, p.unit
                        ));
                    }
                }
                (Some(_), None) => {
                    findings.push(format!("missing prediction for {id}"));
                }
                (None, _) => {
                    findings.push(format!("covariance block names unknown observable {id}"));
                }
            }
        }
        if present_idx.is_empty() {
            continue;
        }
        let sub = submatrix(&block.matrix, &present_idx);
        match chi2_quadratic_form(&residual, &sub) {
            Some(chi2) => log_likelihood += -0.5 * chi2,
            None => {
                findings.push(format!(
                    "non-positive-definite covariance block over {:?}",
                    block.ids
                ));
                log_likelihood = f64::NEG_INFINITY;
            }
        }
    }

    // Diagonal pass for everything not consumed by a block.
    for o in &data.observables {
        if in_block.contains(o.observable_id.as_str()) {
            continue;
        }
        match pred_map.get(o.observable_id.as_str()) {
            Some(p) => {
                matched += 1;
                let sigma = o.uncertainty.max(p.uncertainty);
                log_likelihood += super::gaussian_log_likelihood(o.value, p.value, sigma);
                if o.unit.trim() != p.unit.trim() {
                    findings.push(format!(
                        "unit mismatch for {}: {} vs {}",
                        o.observable_id, o.unit, p.unit
                    ));
                }
            }
            None => findings.push(format!("missing prediction for {}", o.observable_id)),
        }
    }

    let observable_count = data.observables.len().max(1) as f64;
    let coverage = matched as f64 / observable_count;
    let k = parameter_count.max(1) as f64;
    let n = data.observables.len().max(1) as f64;
    let aic = 2.0 * k - 2.0 * log_likelihood;
    let bic = k * n.ln() - 2.0 * log_likelihood;
    let mdl = bic + (1.0 - coverage) * 10.0 + findings.len() as f64;
    let parameter_count_penalty = k.log10().max(0.0);

    let metrics = ScoreMetrics {
        log_likelihood,
        delta_log_likelihood: log_likelihood - baseline_log_likelihood,
        aic,
        bic,
        mdl,
        coverage,
        parameter_count_penalty,
        invalid_prediction_count: findings.len(),
    };
    (metrics, findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(id: &str, value: f64, sigma: f64) -> ObservableRecord {
        ObservableRecord {
            observable_id: id.into(),
            kind: "test".into(),
            value,
            uncertainty: sigma,
            unit: "dimensionless".into(),
            source: None,
        }
    }
    fn pred(id: &str, value: f64) -> PredictionRecord {
        PredictionRecord {
            observable_id: id.into(),
            value,
            uncertainty: 0.0,
            unit: "dimensionless".into(),
            theory_id: None,
        }
    }

    #[test]
    fn cholesky_solves_a_known_system() {
        // A = [[4,2],[2,3]], b = [1,1] -> x = A^-1 b. det = 8. x = (1/8)[[3,-2],[-2,4]][1,1] =
        // (1/8)[1, 2] = [0.125, 0.25].
        let a = vec![vec![4.0, 2.0], vec![2.0, 3.0]];
        let x = cholesky_solve(&a, &[1.0, 1.0]).unwrap();
        assert!((x[0] - 0.125).abs() < 1e-12);
        assert!((x[1] - 0.25).abs() < 1e-12);
    }

    #[test]
    fn non_pd_matrix_is_rejected() {
        let a = vec![vec![1.0, 2.0], vec![2.0, 1.0]]; // indefinite
        assert!(cholesky_solve(&a, &[1.0, 1.0]).is_none());
        assert!(chi2_quadratic_form(&[1.0, 1.0], &a).is_none());
    }

    #[test]
    fn diagonal_block_reproduces_independent_gaussian() {
        // A block with a diagonal covariance must give the same log-L as the independent diagonal.
        let observables = vec![obs("a", 1.0, 0.5), obs("b", 2.0, 0.25)];
        let preds = vec![pred("a", 1.3), pred("b", 1.8)];

        let diag = LikelihoodData::diagonal(observables.clone());
        let (m_diag, _) = score_metrics_cov(&diag, &preds, 2, 0.0);

        let block = CovarianceBlock::from_sigmas(vec!["a".into(), "b".into()], &[0.5, 0.25]);
        let with_block = LikelihoodData {
            observables,
            blocks: vec![block],
        };
        let (m_block, _) = score_metrics_cov(&with_block, &preds, 2, 0.0);

        assert!((m_diag.log_likelihood - m_block.log_likelihood).abs() < 1e-12);
    }

    #[test]
    fn correlation_changes_the_chi2_vs_diagonal() {
        // Positively correlated residuals in the same direction are LESS surprising under a
        // correlated covariance than under a diagonal one (the model can be "off" coherently).
        let observables = vec![obs("a", 1.0, 1.0), obs("b", 2.0, 1.0)];
        let preds = vec![pred("a", 1.5), pred("b", 2.5)]; // both +0.5

        let diag = LikelihoodData::diagonal(observables.clone());
        let (m_diag, _) = score_metrics_cov(&diag, &preds, 2, 0.0);

        // Strong positive correlation rho=0.9.
        let block = CovarianceBlock {
            ids: vec!["a".into(), "b".into()],
            matrix: vec![vec![1.0, 0.9], vec![0.9, 1.0]],
        };
        let corr = LikelihoodData {
            observables,
            blocks: vec![block],
        };
        let (m_corr, _) = score_metrics_cov(&corr, &preds, 2, 0.0);

        // Higher log-L (smaller chi2) under the correlated model for a coherent residual.
        assert!(
            m_corr.log_likelihood > m_diag.log_likelihood,
            "corr {} vs diag {}",
            m_corr.log_likelihood,
            m_diag.log_likelihood
        );
    }

    #[test]
    fn marginalizes_over_a_missing_block_member() {
        // If the model cannot predict "b", the block is scored on "a" alone against C[a,a].
        let observables = vec![obs("a", 1.0, 1.0), obs("b", 2.0, 1.0)];
        let preds = vec![pred("a", 1.5)]; // no "b"
        let block = CovarianceBlock {
            ids: vec!["a".into(), "b".into()],
            matrix: vec![vec![1.0, 0.5], vec![0.5, 1.0]],
        };
        let data = LikelihoodData {
            observables,
            blocks: vec![block],
        };
        let (m, findings) = score_metrics_cov(&data, &preds, 2, 0.0);
        // chi2 on "a" alone: (0.5)^2 / 1.0 = 0.25 -> log-L = -0.125.
        assert!((m.log_likelihood + 0.125).abs() < 1e-12);
        assert!(findings.iter().any(|f| f.contains("missing prediction for b")));
        assert!((m.coverage - 0.5).abs() < 1e-12);
    }

    #[test]
    fn non_pd_block_makes_the_candidate_unscorable() {
        let observables = vec![obs("a", 1.0, 1.0), obs("b", 2.0, 1.0)];
        let preds = vec![pred("a", 1.0), pred("b", 2.0)];
        let block = CovarianceBlock {
            ids: vec!["a".into(), "b".into()],
            matrix: vec![vec![1.0, 2.0], vec![2.0, 1.0]], // indefinite
        };
        let data = LikelihoodData {
            observables,
            blocks: vec![block],
        };
        let (m, _) = score_metrics_cov(&data, &preds, 2, 0.0);
        assert!(m.log_likelihood.is_infinite() && m.log_likelihood < 0.0);
    }
}
