//! nDGP `{r_c}` — the normal-branch Dvali–Gabadadze–Porrati braneworld, a *derived* modified-gravity
//! sector with a single fundamental scale: the crossover length `r_c` (the radius at which gravity
//! leaks from the 4D brane into the 5D bulk).
//!
//! On linear sub-horizon scales the brane-bending scalar mediates a fifth force whose strength is set
//! by the quasi-static effective gravitational coupling (Koyama & Maartens 2006, astro-ph/0511634;
//! Koyama 2016 review, arXiv:1504.04623 §4.2; Schmidt 2009, arXiv:0905.0858)
//!
//! ```text
//!   G_eff(a)/G = μ(a) = 1 + 1/(3 β(a)),
//!   β(a) = 1 + 2 H r_c (1 + Ḣ/(3 H²))   (normal branch: the "+" sign).
//! ```
//!
//! This is **scale-free** on linear scales (no `k` dependence — unlike f(R) the brane scalar is
//! massless), so `μ(a,k) = μ(a)`. The dimensionless crossover is conventionally written
//! `Ω_rc ≡ 1/(4 H₀² r_c²)` (Bose et al. 2018, arXiv:1606.02520; Euclid prep., arXiv:2512.09748), so
//! `2 H r_c = E(a)/√Ω_rc` with `E = H/H₀`. We carry the fundamental scale as `Ω_rc` (≥ 0); `Ω_rc → 0`
//! ⇔ `r_c → ∞` is the GR limit.
//!
//! Sign / limit (PHYSICS NOTE, verified against the literature — see the crate-level note in
//! `theory/sectors`): the normal branch has `β > 0`, hence `μ = 1 + 1/(3β) > 1`. nDGP normal branch
//! therefore **ENHANCES** linear growth (Koyama & Maartens 2006; Schmidt 2009; Falck et al. 2015,
//! arXiv:1404.5469: "the growth of structure is always enhanced in this model"). The branch that
//! *suppresses* growth is the self-accelerating (sDGP) branch (`β < 0`), which is ghost-unstable and
//! not a viable family — so we implement the healthy, growth-enhancing normal branch. As `r_c → ∞`
//! (`Ω_rc → 0`), `β → ∞` and `μ → 1`: exact GR.
//!
//! Lensing: the brane-bending mode contributes equally to the two metric potentials in the QSA, so
//! `Σ(a) = μ(a)` on linear scales (Koyama 2016 §4.2; no anisotropic-stress slip from the scalar in
//! the linear regime). Vainshtein screening restores `μ → 1` inside the Vainshtein radius of bound
//! objects — a nonlinear effect handled by the adjudication veto, not this linear-QSA function.

use super::MgResponse;

/// Fundamental nDGP parameter: the dimensionless crossover `Ω_rc ≡ 1/(4 H₀² r_c²)`. `Ω_rc = 0`
/// (`r_c → ∞`) is exact GR. Cosmological growth probes `Ω_rc ~ 0.1`–`1` (`H₀ r_c ~ 0.5`–`1.6`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NdgpParams {
    /// `Ω_rc = 1/(4 H₀² r_c²)` (≥ 0).
    pub omega_rc: f64,
}

impl NdgpParams {
    /// Construct from the dimensionless crossover `Ω_rc` (clamped non-negative).
    pub fn from_omega_rc(omega_rc: f64) -> Self {
        NdgpParams {
            omega_rc: omega_rc.max(0.0),
        }
    }

    /// Construct from the physical crossover scale `r_c` in units of the Hubble length `c/H₀`
    /// (i.e. the dimensionless `H₀ r_c`): `Ω_rc = 1/(4 (H₀ r_c)²)`. `H₀ r_c → ∞` ⇒ `Ω_rc → 0` ⇒ GR.
    pub fn from_h0_rc(h0_rc: f64) -> Self {
        if h0_rc <= 0.0 {
            // r_c ≤ 0 is unphysical; treat as the maximally-modified limit's complement → GR-safe 0.
            return NdgpParams { omega_rc: 0.0 };
        }
        NdgpParams {
            omega_rc: 1.0 / (4.0 * h0_rc * h0_rc),
        }
    }

    /// The braiding function `β(a) = 1 + (E(a)/√Ω_rc)(1 + Ḣ/3H²)` (normal branch). `e` is `E(a)=H/H₀`
    /// and `dln_e_dn` is `dlnE/dN = Ḣ/H²` (with `N = ln a`). As `Ω_rc → 0`, `β → +∞`. β diverging is
    /// the GR limit; β finite-positive gives an enhanced coupling.
    fn beta(&self, e: f64, dln_e_dn: f64) -> f64 {
        if self.omega_rc <= 0.0 {
            return f64::INFINITY;
        }
        // 2 H r_c = E / √Ω_rc  (from Ω_rc = 1/(4 H₀² r_c²) ⇒ 2 H₀ r_c = 1/√Ω_rc, times E = H/H₀).
        1.0 + (e / self.omega_rc.sqrt()) * (1.0 + dln_e_dn / 3.0)
    }

    /// Derived quasi-static response `μ(a) = 1 + 1/(3β)` and `Σ(a) = μ(a)` (scale-free: no `k`).
    /// Inputs are the background `E(a) = H/H₀` and `Ḣ/H² = dlnE/dN`, which the growth integrator
    /// already computes from the (DGP-modified or ΛCDM-background) expansion history. GR is recovered
    /// as `Ω_rc → 0`. Normal branch ⇒ `μ > 1` (growth enhanced).
    pub fn response(&self, e: f64, dln_e_dn: f64) -> MgResponse {
        let beta = self.beta(e, dln_e_dn);
        if !beta.is_finite() || beta == 0.0 {
            return MgResponse::GR;
        }
        let mu = 1.0 + 1.0 / (3.0 * beta);
        MgResponse { mu, sigma: mu }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_gr_as_rc_goes_to_infinity() {
        // r_c → ∞ ⇔ Ω_rc → 0: μ → 1.
        let huge_rc = NdgpParams::from_h0_rc(1.0e6);
        let r = huge_rc.response(1.0, -0.5);
        assert!(
            (r.mu - 1.0).abs() < 1e-6,
            "μ = {} should → 1 as r_c → ∞",
            r.mu
        );
        assert_eq!(
            NdgpParams::from_omega_rc(0.0).response(1.0, -0.5),
            MgResponse::GR
        );
    }

    #[test]
    fn normal_branch_enhances_gravity() {
        // PHYSICS: normal-branch nDGP has β > 0 ⇒ μ = 1 + 1/(3β) > 1 (Koyama & Maartens 2006).
        let p = NdgpParams::from_omega_rc(0.25); // H₀ r_c = 1
                                                 // A representative late-time background: E ≈ 1, Ḣ/H² ≈ −0.5 (matter+Λ at a~1).
        let r = p.response(1.0, -0.5);
        assert!(
            r.mu > 1.0,
            "normal-branch nDGP must enhance gravity: μ = {}",
            r.mu
        );
        // Σ = μ (no linear slip).
        assert_eq!(r.sigma, r.mu);
    }

    #[test]
    fn smaller_rc_means_stronger_modification() {
        // Smaller r_c (larger Ω_rc) ⇒ smaller β ⇒ larger μ.
        let e = 1.0;
        let dh = -0.5;
        let weak = NdgpParams::from_omega_rc(0.1).response(e, dh).mu; // larger r_c
        let strong = NdgpParams::from_omega_rc(1.0).response(e, dh).mu; // smaller r_c
        assert!(
            strong > weak,
            "Ω_rc=1 μ={strong} should exceed Ω_rc=0.1 μ={weak}"
        );
        assert!(weak > 1.0);
    }

    #[test]
    fn modification_grows_toward_late_times() {
        // β ∝ E(a) = H/H₀, which falls toward a=1 (H decreases), so β shrinks and μ grows late.
        // At high z (large E) the fifth force is more suppressed (β large ⇒ μ → 1).
        let p = NdgpParams::from_omega_rc(0.25);
        let mu_late = p.response(1.0, -0.5).mu; // a ≈ 1, E ≈ 1
        let mu_early = p.response(3.0, 0.0).mu; // high z, large E
        assert!(
            mu_late > mu_early,
            "late μ={mu_late} should exceed early μ={mu_early}"
        );
        assert!((mu_early - 1.0) < (mu_late - 1.0));
    }
}
