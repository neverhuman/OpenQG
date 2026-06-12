//! Deterministic marginal log-evidence over a bounded parameter box.
//!
//! Bayesian model selection ranks models by the *marginal likelihood* (the evidence)
//!
//! ```text
//!     Z = ∫ L(θ) π(θ) dθ ,
//! ```
//!
//! the average of the likelihood over the prior, which automatically penalizes unused
//! flexibility (the Occam factor). The league's headline `Δln Z ≈ −½ ΔBIC` is the *Schwarz*
//! approximation (Schwarz 1978): asymptotically valid but it drops the Occam-factor prefactor and
//! is known to mis-rank near-degenerate extensions (Trotta 2008, *Bayes in the sky*, Contemp.
//! Phys. 49, 71, arXiv:0803.4089, §3–4). These estimators are **diagnostic approximations only**
//! — they supply fast cross-checks but do NOT replace nested sampling for any formal model
//! comparison. Promotion-grade ΔlnZ must come from an `EvidenceReceipt` produced by UltraNest or
//! Dynesty (see `validation::evidence_receipt`). The two estimators here are:
//!
//! 1. [`laplace_log_evidence`] — the **Laplace approximation** (Laplace 1774; see Trotta 2008
//!    eq. 12, MacKay 2003 *Information Theory, Inference & Learning Algorithms* §27):
//!
//!    ```text
//!      ln Z ≈ ln L_max + (k/2) ln(2π) − ½ ln|H| + ln π(θ_MAP)
//!    ```
//!
//!    with `H = −∂²ln L/∂θ²` the (positive-definite) Hessian of the *negative* log-likelihood at
//!    the maximum and a flat prior `π = 1/V_box` over the bounded box (so `ln π = −ln V_box`).
//!    The Hessian is formed by a deterministic central finite difference — no RNG, fully
//!    reproducible.
//!
//! 2. [`grid_log_evidence`] — a deterministic **Riemann/grid cross-check** of the same integral
//!    on a tensor grid over the box (exact in the limit of a fine grid). For the 1–2 dimensional
//!    cases the league actually adjudicates, this is a cheap, assumption-free check that the
//!    Gaussian Laplace estimate did not mislead. (For a true high-dimensional integral one would
//!    reach for nested sampling / SMC — out of scope here; the grid is the low-dim ground truth.)
//!
//! Both return a *log* evidence with the SAME prior normalization, so their difference is the
//! quantity to compare, and the analytic Gaussian test below pins both against a closed form.

/// Whether a Laplace approximation is expected to be reliable.
///
/// The Laplace approximation assumes a unimodal, near-Gaussian posterior. These flags encode when
/// that assumption is known to be violated, so a caller can treat the estimate as a rough bound
/// rather than a precision measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaplaceValidity {
    /// Posterior is approximately Gaussian at the optimum and well-contained in the box.
    Valid,
    /// MAP sits within `tol` of a box boundary; the Gaussian tail is artificially truncated.
    BoundaryPinned,
    /// Evidence of a secondary mode (e.g., the Hessian eigenvalue ratio > 10); the Laplace
    /// estimate reflects only the dominant mode, not the full integral.
    MultimodalPosterior,
}

/// Diagnostic summary of a Laplace evidence approximation.
///
/// This struct is attached to an `EvidenceReceipt` as a cross-check, NOT as the primary
/// evidence. When `disagreement_with_nested` exceeds ~0.5 ln-units, the Laplace is unreliable
/// and the nested-sampling receipt value should be used exclusively.
#[derive(Debug, Clone)]
pub struct LaplaceDiagnostic {
    /// The Laplace estimate of ln Z.
    pub ln_z_estimate: f64,
    /// Whether the approximation is expected to be reliable.
    pub validity: LaplaceValidity,
    /// |ln Z_laplace − ln Z_nested|, if both estimates are available.
    /// `None` when only Laplace is available (no nested receipt to compare against).
    pub disagreement_with_nested: Option<f64>,
}

impl LaplaceDiagnostic {
    /// True when the Laplace estimate is trustworthy enough to cite in a diagnostic table.
    ///
    /// Criteria: validity is `Valid` AND either there is no nested estimate to compare against,
    /// or the disagreement is ≤ 0.5 ln-units.
    pub fn is_reliable(&self) -> bool {
        self.validity == LaplaceValidity::Valid
            && self.disagreement_with_nested.map_or(true, |d| d <= 0.5)
    }
}

/// Result of a Laplace evidence estimate, carrying the decomposition so a caller can audit it.
#[derive(Debug, Clone)]
pub struct LaplaceEvidence {
    /// ln Z ≈ ln L_max + (k/2) ln(2π) − ½ ln|H| − ln V_box.
    pub log_evidence: f64,
    /// The maximum log-likelihood ln L_max used.
    pub log_likelihood_max: f64,
    /// Number of free parameters k.
    pub k: usize,
    /// ln |H|, the log-determinant of the negative-log-likelihood Hessian at the optimum.
    pub log_det_hessian: f64,
    /// ln of the prior box volume (the flat-prior normalization, ln V_box).
    pub log_prior_volume: f64,
    /// True if the numerical Hessian came out symmetric positive-definite (a genuine maximum).
    pub hessian_positive_definite: bool,
}

/// Laplace marginal log-evidence over a bounded box with a flat prior.
///
/// - `neg_log_likelihood`: returns −ln L(θ) (the league's `χ²/2` objective works directly).
/// - `theta_map`: the maximum-likelihood (= MAP under a flat prior) point, in-box.
/// - `lower`, `upper`: the per-parameter box bounds defining the flat prior `π = 1/∏(upper−lower)`.
/// - `rel_step`: relative finite-difference step for the Hessian (a small fraction of each box
///   width). A central difference makes the leading error O(step²) and deterministic.
///
/// Returns `None` if the dimensions disagree or the box is degenerate.
pub fn laplace_log_evidence<F: Fn(&[f64]) -> f64>(
    neg_log_likelihood: &F,
    theta_map: &[f64],
    lower: &[f64],
    upper: &[f64],
    rel_step: f64,
) -> Option<LaplaceEvidence> {
    let k = theta_map.len();
    if k == 0 || lower.len() != k || upper.len() != k {
        return None;
    }
    let mut log_prior_volume = 0.0;
    let mut steps = vec![0.0; k];
    for i in 0..k {
        let width = upper[i] - lower[i];
        if !width.is_finite() || width <= 0.0 {
            return None;
        }
        log_prior_volume += width.ln();
        steps[i] = (rel_step * width).max(1e-9);
    }

    let nll_max = neg_log_likelihood(theta_map);
    let log_likelihood_max = -nll_max;

    // Hessian of the NEGATIVE log-likelihood by central finite differences. H = ∂²(−ln L)/∂θ².
    // Diagonal: (f(+e_i) − 2f(0) + f(−e_i)) / h_i².
    // Off-diagonal: the standard 4-point central stencil.
    let mut hessian = vec![vec![0.0_f64; k]; k];
    let eval_shift = |deltas: &[(usize, f64)]| -> f64 {
        let mut x = theta_map.to_vec();
        for &(i, d) in deltas {
            x[i] += d;
        }
        neg_log_likelihood(&x)
    };
    for i in 0..k {
        let hi = steps[i];
        let fpp = eval_shift(&[(i, hi)]);
        let fmm = eval_shift(&[(i, -hi)]);
        hessian[i][i] = (fpp - 2.0 * nll_max + fmm) / (hi * hi);
    }
    for i in 0..k {
        for j in (i + 1)..k {
            let hi = steps[i];
            let hj = steps[j];
            let fpp = eval_shift(&[(i, hi), (j, hj)]);
            let fpm = eval_shift(&[(i, hi), (j, -hj)]);
            let fmp = eval_shift(&[(i, -hi), (j, hj)]);
            let fmm = eval_shift(&[(i, -hi), (j, -hj)]);
            let hij = (fpp - fpm - fmp + fmm) / (4.0 * hi * hj);
            hessian[i][j] = hij;
            hessian[j][i] = hij;
        }
    }

    // ln|H| via Cholesky if PD (the well-behaved maximum case), else via an LU log-determinant of
    // |H| with a sign flag.
    let (log_det_hessian, hessian_positive_definite) = match cholesky_lower(&hessian) {
        Some(l) => {
            // |H| = ∏ l_ii², so ln|H| = 2 Σ ln l_ii.
            let logdet = 2.0 * (0..k).map(|i| l[i][i].ln()).sum::<f64>();
            (logdet, true)
        }
        None => (log_abs_det(&hessian), false),
    };

    // ln Z = ln L_max + (k/2) ln(2π) − ½ ln|H| − ln V_box.
    let log_evidence = log_likelihood_max + (k as f64 / 2.0) * (2.0 * std::f64::consts::PI).ln()
        - 0.5 * log_det_hessian
        - log_prior_volume;

    Some(LaplaceEvidence {
        log_evidence,
        log_likelihood_max,
        k,
        log_det_hessian,
        log_prior_volume,
        hessian_positive_definite,
    })
}

/// Deterministic grid (Riemann mid-point) estimate of `ln Z = ln ∫ L π dθ` over the box, with the
/// same flat prior `π = 1/V_box`. `points_per_dim` cells per axis; the total work is
/// `points_per_dim^k`, so this is the low-dimensional cross-check (k ≤ ~3). Uses the
/// log-sum-exp trick for numerical stability. `neg_log_likelihood` returns −ln L(θ).
// The odometer reads and writes `idx[d]` with an early break; an index loop is the clearest form.
#[allow(clippy::needless_range_loop)]
pub fn grid_log_evidence<F: Fn(&[f64]) -> f64>(
    neg_log_likelihood: &F,
    lower: &[f64],
    upper: &[f64],
    points_per_dim: usize,
) -> Option<f64> {
    let k = lower.len();
    if k == 0 || upper.len() != k || points_per_dim == 0 {
        return None;
    }
    let mut log_cell = 0.0; // ln of the cell volume in θ-space.
    let mut log_prior_volume = 0.0;
    let mut widths = vec![0.0; k];
    for i in 0..k {
        let w = upper[i] - lower[i];
        if !w.is_finite() || w <= 0.0 {
            return None;
        }
        widths[i] = w;
        log_cell += (w / points_per_dim as f64).ln();
        log_prior_volume += w.ln();
    }

    // Iterate the tensor grid by an odometer over per-axis indices; sample the cell mid-points.
    let total: usize = points_per_dim.checked_pow(k as u32)?;
    let mut idx = vec![0usize; k];
    let mut log_terms: Vec<f64> = Vec::with_capacity(total.min(1 << 24));
    for _ in 0..total {
        let mut theta = vec![0.0; k];
        for d in 0..k {
            theta[d] = lower[d] + widths[d] * (idx[d] as f64 + 0.5) / points_per_dim as f64;
        }
        // ln(L π ΔV) = ln L − ln V_box + ln ΔV  (the −ln V_box pulled out of the sum below).
        log_terms.push(-neg_log_likelihood(&theta));
        // Increment the odometer.
        for d in 0..k {
            idx[d] += 1;
            if idx[d] < points_per_dim {
                break;
            }
            idx[d] = 0;
        }
    }
    let lse = log_sum_exp(&log_terms);
    // ln Z = ln Σ L_i + ln ΔV_cell − ln V_box.
    Some(lse + log_cell - log_prior_volume)
}

/// log-sum-exp of a slice (returns −∞ for an empty slice), stable against overflow.
fn log_sum_exp(values: &[f64]) -> f64 {
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max.is_finite() {
        return max;
    }
    let sum: f64 = values.iter().map(|v| (v - max).exp()).sum();
    max + sum.ln()
}

/// In-place Cholesky `A = L Lᵀ` (lower). `None` if `A` is not positive-definite. (A local copy so
/// the evidence module is self-contained; mirrors `covariance::cholesky`.)
#[allow(clippy::needless_range_loop)] // coupled L/A indexing; index loops keep the kernel clear.
fn cholesky_lower(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = a.len();
    let mut l = vec![vec![0.0_f64; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i][j];
            for kk in 0..j {
                sum -= l[i][kk] * l[j][kk];
            }
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

/// ln|det A| for a general square matrix via Gaussian elimination with partial pivoting. Used as
/// the fallback log-determinant when the Hessian is not positive-definite (so the Laplace estimate
/// still reports a value, flagged by `hessian_positive_definite = false`).
#[allow(clippy::needless_range_loop)] // pivot/eliminate over m by index; the kernel is clearest so.
fn log_abs_det(a: &[Vec<f64>]) -> f64 {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = a.to_vec();
    let mut log_det = 0.0;
    for col in 0..n {
        // Partial pivot.
        let mut pivot = col;
        for r in (col + 1)..n {
            if m[r][col].abs() > m[pivot][col].abs() {
                pivot = r;
            }
        }
        if m[pivot][col] == 0.0 {
            return f64::NEG_INFINITY;
        }
        if pivot != col {
            m.swap(pivot, col);
        }
        log_det += m[col][col].abs().ln();
        for r in (col + 1)..n {
            let factor = m[r][col] / m[col][col];
            for c in col..n {
                m[r][c] -= factor * m[col][c];
            }
        }
    }
    log_det
}

#[cfg(test)]
mod tests {
    use super::*;

    /// For a 1-D Gaussian likelihood L(θ) = exp(−(θ−μ)²/(2s²)) with a flat prior over a box
    /// [a,b] wide enough that the Gaussian is fully enclosed, the exact evidence is
    ///   Z = (1/(b−a)) ∫ L dθ ≈ (1/(b−a)) · s √(2π),
    /// so ln Z ≈ ln(s) + ½ ln(2π) − ln(b−a). The Laplace estimate is EXACT here (the negative
    /// log-likelihood is exactly quadratic), which is the cleanest analytic check.
    #[test]
    fn laplace_reproduces_a_known_1d_gaussian_analytically() {
        let mu = 0.3;
        let s = 0.05;
        let nll = |x: &[f64]| {
            let d = x[0] - mu;
            0.5 * d * d / (s * s)
        };
        let lo = [mu - 1.0];
        let hi = [mu + 1.0];
        let est = laplace_log_evidence(&nll, &[mu], &lo, &hi, 1e-3).unwrap();
        let analytic = s.ln() + 0.5 * (2.0 * std::f64::consts::PI).ln() - (hi[0] - lo[0]).ln();
        assert!(est.hessian_positive_definite);
        assert!(
            (est.log_evidence - analytic).abs() < 1e-6,
            "laplace {} vs analytic {}",
            est.log_evidence,
            analytic
        );
    }

    /// 2-D isotropic Gaussian: ln Z = ln(s1) + ln(s2) + ln(2π) − ln V_box. With a correlation the
    /// determinant changes; test the diagonal-covariance case where the closed form is clean.
    #[test]
    fn laplace_reproduces_a_known_2d_gaussian() {
        let mu = [0.2, -0.1];
        let s = [0.04, 0.07];
        let nll = |x: &[f64]| {
            let d0 = x[0] - mu[0];
            let d1 = x[1] - mu[1];
            0.5 * (d0 * d0 / (s[0] * s[0]) + d1 * d1 / (s[1] * s[1]))
        };
        let lo = [mu[0] - 1.0, mu[1] - 1.0];
        let hi = [mu[0] + 1.0, mu[1] + 1.0];
        let est = laplace_log_evidence(&nll, &mu, &lo, &hi, 1e-3).unwrap();
        let analytic = s[0].ln() + s[1].ln() + (2.0 * std::f64::consts::PI).ln()
            - (hi[0] - lo[0]).ln()
            - (hi[1] - lo[1]).ln();
        assert!(
            (est.log_evidence - analytic).abs() < 1e-5,
            "laplace {} vs analytic {}",
            est.log_evidence,
            analytic
        );
    }

    /// The deterministic grid cross-check must agree with both the analytic value and the Laplace
    /// estimate for the Gaussian case (on a fine-enough grid).
    #[test]
    fn grid_cross_check_agrees_with_laplace_on_a_gaussian() {
        let mu = 0.3;
        let s = 0.06;
        let nll = |x: &[f64]| {
            let d = x[0] - mu;
            0.5 * d * d / (s * s)
        };
        // A tight box (±10σ) so a modest grid resolves the peak well.
        let lo = [mu - 0.6];
        let hi = [mu + 0.6];
        let analytic = s.ln() + 0.5 * (2.0 * std::f64::consts::PI).ln() - (hi[0] - lo[0]).ln();
        let grid = grid_log_evidence(&nll, &lo, &hi, 4001).unwrap();
        let lap = laplace_log_evidence(&nll, &[mu], &lo, &hi, 1e-3).unwrap();
        assert!(
            (grid - analytic).abs() < 1e-3,
            "grid {grid} vs analytic {analytic}"
        );
        assert!(
            (grid - lap.log_evidence).abs() < 1e-3,
            "grid {grid} vs laplace {}",
            lap.log_evidence
        );
    }

    /// Evidence is deterministic: identical inputs give bit-identical output.
    #[test]
    fn evidence_is_deterministic() {
        let nll = |x: &[f64]| 0.5 * (x[0] * x[0] + x[1] * x[1]) / 0.01;
        let lo = [-1.0, -1.0];
        let hi = [1.0, 1.0];
        let a = laplace_log_evidence(&nll, &[0.0, 0.0], &lo, &hi, 1e-3).unwrap();
        let b = laplace_log_evidence(&nll, &[0.0, 0.0], &lo, &hi, 1e-3).unwrap();
        assert_eq!(a.log_evidence.to_bits(), b.log_evidence.to_bits());
    }

    /// Occam factor: at equal best-fit likelihood, a model with a wider unconstrained prior box
    /// has *lower* evidence (it spread its prior mass over regions the data rule out). This is the
    /// mechanism BIC only approximates.
    #[test]
    fn wider_prior_box_lowers_the_evidence_at_equal_fit() {
        let mu = 0.0;
        let s = 0.05;
        let nll = |x: &[f64]| {
            let d = x[0] - mu;
            0.5 * d * d / (s * s)
        };
        let narrow = laplace_log_evidence(&nll, &[mu], &[-0.5], &[0.5], 1e-3).unwrap();
        let wide = laplace_log_evidence(&nll, &[mu], &[-5.0], &[5.0], 1e-3).unwrap();
        assert!(
            wide.log_evidence < narrow.log_evidence,
            "wide {} should be < narrow {}",
            wide.log_evidence,
            narrow.log_evidence
        );
        // The gap is exactly ln(wide width / narrow width) = ln(10/1) = ln 10.
        assert!(((narrow.log_evidence - wide.log_evidence) - 10.0_f64.ln()).abs() < 1e-6);
    }

    #[test]
    fn laplace_diagnostic_valid_no_disagreement_is_reliable() {
        let d = LaplaceDiagnostic {
            ln_z_estimate: -12.3,
            validity: LaplaceValidity::Valid,
            disagreement_with_nested: None,
        };
        assert!(d.is_reliable());
    }

    #[test]
    fn laplace_diagnostic_boundary_pinned_is_not_reliable() {
        let d = LaplaceDiagnostic {
            ln_z_estimate: -12.3,
            validity: LaplaceValidity::BoundaryPinned,
            disagreement_with_nested: None,
        };
        assert!(!d.is_reliable());
    }

    #[test]
    fn laplace_diagnostic_multimodal_is_not_reliable() {
        let d = LaplaceDiagnostic {
            ln_z_estimate: -12.3,
            validity: LaplaceValidity::MultimodalPosterior,
            disagreement_with_nested: Some(0.1),
        };
        assert!(!d.is_reliable());
    }

    #[test]
    fn laplace_diagnostic_large_disagreement_is_not_reliable() {
        let d = LaplaceDiagnostic {
            ln_z_estimate: -12.3,
            validity: LaplaceValidity::Valid,
            disagreement_with_nested: Some(0.6),
        };
        assert!(!d.is_reliable(), "disagreement > 0.5 must be unreliable");
    }

    #[test]
    fn laplace_diagnostic_small_disagreement_is_reliable() {
        let d = LaplaceDiagnostic {
            ln_z_estimate: -12.3,
            validity: LaplaceValidity::Valid,
            disagreement_with_nested: Some(0.4),
        };
        assert!(d.is_reliable());
    }
}
