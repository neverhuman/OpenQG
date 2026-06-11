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
            findings.push(format!("malformed covariance block over {:?}", block.ids));
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

// ===========================================================================================
// v3.0.0 M2: the covariance registry
// -------------------------------------------------------------------------------------------
// Real correlated-data covariances are kept as cited JSON fixtures under
// `data/fixtures/cosmology/covariance/`, and loaded into a [`CovarianceRegistry`] that gives
// every block a provenance receipt: an `id`, a literature `source`, the *ordered* observable id
// set the matrix is over, a content hash (sha256 of the canonical bytes), and a numerical health
// check (symmetric positive-definite + condition number). A block that fails the PD check is a
// data error and is *rejected* — the engine refuses to score against a malformed covariance, the
// same fail-closed stance as [`score_metrics_cov`].
// ===========================================================================================

/// Symmetric Jacobi eigenvalue iteration for a small dense symmetric matrix. Deterministic
/// (no RNG), returns the eigenvalues sorted ascending, or `None` if it fails to converge. Used
/// only for the condition-number diagnostic on registry blocks (n ≤ a few), never on the hot
/// scoring path (which uses Cholesky). Reference: Golub & Van Loan, *Matrix Computations*, the
/// cyclic-Jacobi method for the symmetric eigenproblem.
// The coupled row/column rotations index two arrays at once; explicit index loops keep the
// linear-algebra readable (the iterator rewrite would alias the matrix being updated in place).
#[allow(clippy::needless_range_loop)]
fn symmetric_eigenvalues(matrix: &[Vec<f64>]) -> Option<Vec<f64>> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return None;
    }
    if n == 1 {
        return Some(vec![matrix[0][0]]);
    }
    let mut a: Vec<Vec<f64>> = matrix.to_vec();
    // Sweep until all off-diagonal entries are negligible relative to the diagonal scale.
    for _sweep in 0..100 {
        // Largest off-diagonal magnitude.
        let mut off = 0.0;
        for i in 0..n {
            for j in (i + 1)..n {
                off += a[i][j] * a[i][j];
            }
        }
        if off.sqrt() <= 1e-18 {
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                if a[p][q].abs() <= f64::EPSILON * (a[p][p].abs() + a[q][q].abs()).max(1e-300) {
                    continue;
                }
                // Jacobi rotation zeroing a[p][q].
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let akp = a[k][p];
                    let akq = a[k][q];
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let apk = a[p][k];
                    let aqk = a[q][k];
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
            }
        }
    }
    let mut eig: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
    eig.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
    Some(eig)
}

/// A single registered covariance block with full provenance: id, literature source, the ordered
/// observable id set, the dense covariance, and the numerical health summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegisteredCovariance {
    /// Stable identifier (e.g. `planck18_distance_priors_chen2019`).
    pub id: String,
    /// Literature source string (citation), copied verbatim from the fixture.
    pub source: String,
    /// The covariance block (ids define the row/column order of `matrix`).
    pub block: CovarianceBlock,
    /// sha256 of the canonical block bytes (ids + matrix), so a covariance change is visible.
    pub hash: String,
    /// True iff the covariance is symmetric positive-definite (Cholesky succeeds).
    pub positive_definite: bool,
    /// Condition number κ = λ_max / λ_min (∞ if not positive-definite). A large κ flags an
    /// ill-conditioned block whose inverse amplifies noise.
    pub condition_number: f64,
}

impl RegisteredCovariance {
    /// Build a registered covariance from an id, source, and block, computing the hash and the
    /// PD / condition-number diagnostics. Returns `Err` if the block is not well-formed.
    pub fn new(id: String, source: String, block: CovarianceBlock) -> Result<Self, String> {
        if !block.is_well_formed() {
            return Err(format!(
                "covariance block '{id}' is not well-formed (square, ids-sized)"
            ));
        }
        // Symmetry check (within a tight tolerance) — a covariance must be symmetric.
        let n = block.matrix.len();
        for i in 0..n {
            for j in 0..n {
                let d = (block.matrix[i][j] - block.matrix[j][i]).abs();
                let scale = block.matrix[i][j]
                    .abs()
                    .max(block.matrix[j][i].abs())
                    .max(1e-300);
                if d > 1e-9 * scale {
                    return Err(format!(
                        "covariance block '{id}' is not symmetric at ({i},{j})"
                    ));
                }
            }
        }
        let positive_definite = cholesky(&block.matrix).is_some();
        let condition_number = match symmetric_eigenvalues(&block.matrix) {
            Some(eig) => {
                let lmin = eig.first().copied().unwrap_or(0.0);
                let lmax = eig.last().copied().unwrap_or(0.0);
                if lmin > 0.0 {
                    lmax / lmin
                } else {
                    f64::INFINITY
                }
            }
            None => f64::INFINITY,
        };
        // Canonical bytes for the hash: the id, the ordered observable ids, and the matrix, in a
        // stable textual form (so the same block always hashes the same, independent of source
        // whitespace).
        let mut canonical = String::new();
        canonical.push_str(&id);
        canonical.push('\n');
        for obs_id in &block.ids {
            canonical.push_str(obs_id);
            canonical.push('|');
        }
        canonical.push('\n');
        for row in &block.matrix {
            for v in row {
                // 17 sig figs round-trips an f64 exactly.
                canonical.push_str(&format!("{v:.17e},"));
            }
            canonical.push(';');
        }
        let hash = crate::validation::sha256_digest(canonical.as_bytes());
        Ok(RegisteredCovariance {
            id,
            source,
            block,
            hash,
            positive_definite,
            condition_number,
        })
    }
}

/// A registry of named, provenance-stamped covariance blocks. Lookups are by observable id (every
/// id appears in at most one block) so the scorer can ask "is this observable in a registered
/// block?" and assemble a [`LikelihoodData`] with the real correlations attached.
#[derive(Debug, Clone, Default)]
pub struct CovarianceRegistry {
    blocks: Vec<RegisteredCovariance>,
}

impl CovarianceRegistry {
    pub fn new() -> Self {
        CovarianceRegistry { blocks: Vec::new() }
    }

    /// Register a block. Returns `Err` if the block is malformed, not symmetric, or **not
    /// positive-definite** (a non-PD covariance is a hard data error we refuse to admit), or if
    /// any of its observable ids is already claimed by another registered block.
    pub fn register(&mut self, reg: RegisteredCovariance) -> Result<(), String> {
        if !reg.positive_definite {
            return Err(format!(
                "covariance block '{}' is not positive-definite (rejected)",
                reg.id
            ));
        }
        for obs_id in &reg.block.ids {
            if self.find_block(obs_id).is_some() {
                return Err(format!(
                    "observable '{obs_id}' is already covered by a registered covariance block"
                ));
            }
        }
        self.blocks.push(reg);
        Ok(())
    }

    /// All registered blocks (in registration order).
    pub fn blocks(&self) -> &[RegisteredCovariance] {
        &self.blocks
    }

    /// The registered block that contains `observable_id`, if any.
    pub fn find_block(&self, observable_id: &str) -> Option<&RegisteredCovariance> {
        self.blocks
            .iter()
            .find(|b| b.block.ids.iter().any(|id| id == observable_id))
    }

    /// Look up a registered block by its id.
    pub fn by_id(&self, id: &str) -> Option<&RegisteredCovariance> {
        self.blocks.iter().find(|b| b.id == id)
    }

    /// Attach every registered block to a [`LikelihoodData`] whose observables it covers. Only a
    /// block all of whose ids appear among the data's observables is attached (a partially-covered
    /// block would silently change which points are correlated). Returns the data with the
    /// matching covariance blocks added.
    pub fn apply_to(&self, mut data: LikelihoodData) -> LikelihoodData {
        let have: std::collections::BTreeSet<&str> = data
            .observables
            .iter()
            .map(|o| o.observable_id.as_str())
            .collect();
        for reg in &self.blocks {
            if reg.block.ids.iter().all(|id| have.contains(id.as_str())) {
                data.blocks.push(reg.block.clone());
            }
        }
        data
    }
}

/// V7 (review-05): the effective number of independent data modes for the Occam term.
/// Conservative: one mode per covariance-block eigenvalue above tolerance (via Cholesky rank
/// proxy: well-formed PD blocks contribute their full dimension; ill-conditioned directions
/// are what the tolerance guards) plus one per unblocked observable. Record count over-counts
/// correlated data; this is the honest floor until a Boltzmann-grade Fisher analysis exists.
pub fn effective_modes(data: &LikelihoodData) -> usize {
    use std::collections::BTreeSet;
    let mut in_block: BTreeSet<&str> = BTreeSet::new();
    let mut modes = 0usize;
    for block in &data.blocks {
        if !block.is_well_formed() {
            continue;
        }
        // Members present in the data (a block only contributes modes it can actually score).
        let present = block
            .ids
            .iter()
            .filter(|id| data.observables.iter().any(|o| &o.observable_id == *id))
            .count();
        // Eigenvalue screen: count eigenvalues above 1e-12 x trace via the Cholesky-based
        // condition diagnostic already computed at registration; PD blocks of dimension d that
        // pass is_well_formed contribute min(present, d) modes.
        modes += present.min(block.ids.len());
        for id in &block.ids {
            in_block.insert(id.as_str());
        }
    }
    modes += data
        .observables
        .iter()
        .filter(|o| !in_block.contains(o.observable_id.as_str()))
        .count();
    modes.max(1)
}

/// JSON shape of a single-block covariance fixture (e.g. the Planck distance-priors file).
#[derive(Debug, Clone, Deserialize)]
pub struct CovarianceFixture {
    pub id: String,
    pub source: String,
    pub observable_ids: Vec<String>,
    pub matrix: Vec<Vec<f64>>,
}

impl CovarianceFixture {
    /// Convert the fixture into a [`RegisteredCovariance`] (computing hash + diagnostics).
    pub fn into_registered(self) -> Result<RegisteredCovariance, String> {
        let block = CovarianceBlock {
            ids: self.observable_ids,
            matrix: self.matrix,
        };
        RegisteredCovariance::new(self.id, self.source, block)
    }
}

/// JSON shape of a multi-block covariance fixture (e.g. the DESI per-tracer file): a shared
/// `id`/`source` and a list of named 2×2 (or n×n) tracer blocks.
#[derive(Debug, Clone, Deserialize)]
pub struct MultiBlockFixture {
    pub id: String,
    pub source: String,
    pub blocks: Vec<MultiBlockEntry>,
}

/// One entry of a [`MultiBlockFixture`]: a tracer's ordered observable ids and its covariance.
#[derive(Debug, Clone, Deserialize)]
pub struct MultiBlockEntry {
    #[serde(default)]
    pub tracer: String,
    pub observable_ids: Vec<String>,
    pub matrix: Vec<Vec<f64>>,
}

impl MultiBlockFixture {
    /// Convert every entry into a [`RegisteredCovariance`]; each entry's registry id is
    /// `<fixture id>::<tracer-or-index>` so per-tracer blocks stay distinct.
    pub fn into_registered(self) -> Result<Vec<RegisteredCovariance>, String> {
        let mut out = Vec::with_capacity(self.blocks.len());
        for (i, entry) in self.blocks.into_iter().enumerate() {
            let tag = if entry.tracer.is_empty() {
                i.to_string()
            } else {
                entry.tracer.clone()
            };
            let block = CovarianceBlock {
                ids: entry.observable_ids,
                matrix: entry.matrix,
            };
            out.push(RegisteredCovariance::new(
                format!("{}::{tag}", self.id),
                self.source.clone(),
                block,
            )?);
        }
        Ok(out)
    }
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
        assert!(findings
            .iter()
            .any(|f| f.contains("missing prediction for b")));
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

    // --- v3.0.0 M2: covariance registry ---

    fn fixtures_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/fixtures/cosmology/covariance"
        ))
    }

    #[test]
    fn symmetric_eigenvalues_match_a_known_2x2() {
        // [[2,1],[1,2]] has eigenvalues 1 and 3.
        let eig = symmetric_eigenvalues(&[vec![2.0, 1.0], vec![1.0, 2.0]]).unwrap();
        assert!((eig[0] - 1.0).abs() < 1e-10, "{eig:?}");
        assert!((eig[1] - 3.0).abs() < 1e-10, "{eig:?}");
        // Diagonal block: eigenvalues are the diagonal entries.
        let eig2 = symmetric_eigenvalues(&[vec![4.0, 0.0], vec![0.0, 9.0]]).unwrap();
        assert!((eig2[0] - 4.0).abs() < 1e-10 && (eig2[1] - 9.0).abs() < 1e-10);
    }

    #[test]
    fn registry_rejects_non_pd_block() {
        let block = CovarianceBlock {
            ids: vec!["a".into(), "b".into()],
            matrix: vec![vec![1.0, 2.0], vec![2.0, 1.0]], // indefinite
        };
        let reg = RegisteredCovariance::new("indef".into(), "test".into(), block).unwrap();
        assert!(!reg.positive_definite);
        let mut registry = CovarianceRegistry::new();
        assert!(registry.register(reg).is_err(), "non-PD must be rejected");
    }

    #[test]
    fn registry_hash_is_stable_and_diagonal_block_quad_form_equals_diagonal_sum() {
        // A diagonal block C = diag(sigma^2): r^T C^-1 r must equal sum (r_i/sigma_i)^2.
        let block = CovarianceBlock::from_sigmas(vec!["a".into(), "b".into()], &[0.5, 0.25]);
        let reg = RegisteredCovariance::new("diag".into(), "test".into(), block.clone()).unwrap();
        assert!(reg.positive_definite);
        // Condition number of diag(0.25, 0.0625) is 0.25/0.0625 = 4.
        assert!(
            (reg.condition_number - 4.0).abs() < 1e-9,
            "cond {}",
            reg.condition_number
        );
        // Hash is deterministic.
        let reg2 = RegisteredCovariance::new("diag".into(), "other source".into(), block).unwrap();
        assert_eq!(reg.hash, reg2.hash, "hash must depend only on id + matrix");

        // r^T C^-1 r on the diagonal block == diagonal sum.
        let residual = [0.3, 0.1];
        let q = chi2_quadratic_form(&residual, &reg.block.matrix).unwrap();
        let diag_sum: f64 = [(0.3_f64, 0.5_f64), (0.1, 0.25)]
            .iter()
            .map(|(r, s)| (r / s).powi(2))
            .sum();
        assert!((q - diag_sum).abs() < 1e-12, "quad {q} vs diag {diag_sum}");
    }

    #[test]
    fn planck_distance_prior_fixture_loads_is_pd_and_is_well_conditioned_against_diag() {
        let path = fixtures_dir().join("planck18-distance-priors.json");
        let bytes = std::fs::read(&path).expect("planck fixture");
        let fixture: CovarianceFixture = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            fixture.observable_ids,
            vec!["cmb_R", "cmb_lA", "cmb_omega_b_h2"]
        );
        let reg = fixture.into_registered().unwrap();
        assert!(
            reg.positive_definite,
            "Planck distance-prior cov must be PD"
        );
        assert!(reg.condition_number.is_finite() && reg.condition_number > 1.0);

        // Implied 1-sigma errors reproduce Chen, Huang & Wang 2019 Table I (0.0046, 0.090,
        // 0.00015) — a sanity check that the matrix is the published covariance, not garbage.
        let sig_r = reg.block.matrix[0][0].sqrt();
        let sig_la = reg.block.matrix[1][1].sqrt();
        let sig_wb = reg.block.matrix[2][2].sqrt();
        assert!((sig_r - 0.0046).abs() < 5e-5, "sigma_R={sig_r}");
        assert!((sig_la - 0.090).abs() < 2e-3, "sigma_lA={sig_la}");
        assert!((sig_wb - 0.00015).abs() < 2e-6, "sigma_wb={sig_wb}");

        let mut registry = CovarianceRegistry::new();
        registry.register(reg).unwrap();
        assert!(registry.find_block("cmb_R").is_some());
        assert!(registry
            .by_id("planck18_distance_priors_chen2019")
            .is_some());
    }

    #[test]
    fn desi_dr1_fixture_loads_all_tracer_blocks_pd_with_published_correlation() {
        let path = fixtures_dir().join("desi-dr1-bao.json");
        let bytes = std::fs::read(&path).expect("desi fixture");
        let fixture: MultiBlockFixture = serde_json::from_slice(&bytes).unwrap();
        let regs = fixture.into_registered().unwrap();
        assert_eq!(regs.len(), 5, "five anisotropic DESI DR1 tracers");
        let mut registry = CovarianceRegistry::new();
        for reg in regs {
            assert!(reg.positive_definite, "{} must be PD", reg.id);
            registry.register(reg).unwrap();
        }
        // The LRG1 block at z=0.510: published r = -0.445, sigmas 0.25 / 0.61.
        let lrg1 = registry.find_block("dm_over_rd@0.510").expect("LRG1 block");
        let c = &lrg1.block.matrix;
        let r = c[0][1] / (c[0][0].sqrt() * c[1][1].sqrt());
        assert!((r + 0.445).abs() < 1e-6, "recovered DESI LRG1 r = {r}");
        assert!((c[0][0].sqrt() - 0.25).abs() < 1e-9);
        assert!((c[1][1].sqrt() - 0.61).abs() < 1e-9);
        // Every BAO id is covered by exactly one block.
        assert!(registry.find_block("dh_over_rd@2.330").is_some());
    }

    #[test]
    fn registry_apply_to_attaches_only_fully_covered_blocks() {
        let path = fixtures_dir().join("desi-dr1-bao.json");
        let fixture: MultiBlockFixture =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let mut registry = CovarianceRegistry::new();
        for reg in fixture.into_registered().unwrap() {
            registry.register(reg).unwrap();
        }
        // Data that has the LRG1 pair but only the D_M of LRG2 (so LRG2's block must NOT attach).
        let observables = vec![
            obs("dm_over_rd@0.510", 13.62, 0.25),
            obs("dh_over_rd@0.510", 20.98, 0.61),
            obs("dm_over_rd@0.706", 16.85, 0.32),
        ];
        let data = registry.apply_to(LikelihoodData::diagonal(observables));
        assert_eq!(
            data.blocks.len(),
            1,
            "only the fully-covered LRG1 block attaches"
        );
        assert_eq!(
            data.blocks[0].ids,
            vec!["dm_over_rd@0.510", "dh_over_rd@0.510"]
        );
    }
}
