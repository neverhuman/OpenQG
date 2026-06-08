//! f(R) Hu–Sawicki `{n, f_R0}` — a *derived* scale-dependent modified-gravity sector.
//!
//! The action adds a function of the Ricci scalar to Einstein–Hilbert, `S = ∫ d⁴x √−g [R + f(R)]/2κ`,
//! with the Hu & Sawicki 2007 form (arXiv:0705.1158, eq. 2)
//!
//! ```text
//!   f(R) = −m² · c₁ (R/m²)ⁿ / [ c₂ (R/m²)ⁿ + 1 ],     n > 0.
//! ```
//!
//! In the high-curvature regime relevant to large-scale structure (`R ≫ m²`, with `m² ∝ Ω_m H₀²`),
//! the only physical handle is the present-day scalaron field amplitude `f_R0 ≡ df/dR |_{R₀} < 0`
//! (Hu & Sawicki §IV.C); the genome is the *fundamental* pair `{n, f_R0}`, NOT a fitted `mu`. The
//! field then tracks the background curvature as (De Felice & Tsujikawa 2010 Living Review,
//! arXiv:1002.4928 eq. 13.31; Hu & Sawicki eq. 25 in the large-curvature limit)
//!
//! ```text
//!   f_R(a) = f_R0 · (R₀ / R(a))^{n+1},     f_R0 < 0,
//! ```
//!
//! where `R(a)` is the background Ricci scalar. For a ΛCDM-like background (Hu & Sawicki use a ΛCDM
//! expansion history to high accuracy),
//!
//! ```text
//!   R(a) = 3 H₀² [ Ω_m a⁻³ + 4 Ω_Λ ]                  (De Felice & Tsujikawa eq. 4.64),
//! ```
//!
//! i.e. `R = 3 H₀²(Ω_m a⁻³ + 4 Ω_Λ)` — the trace of the Einstein equations for matter + Λ.
//!
//! The scalaron mass squared is the curvature of the effective potential
//! (De Felice & Tsujikawa eq. 13.4; Pogosian & Silvestri 2008, arXiv:0709.0888 eq. 9):
//!
//! ```text
//!   m²(a) = (1/3) [ (1 + f_R) / f_{RR} − R ] ≈ 1/(3 f_{RR})   (high curvature, |f_R| ≪ 1),
//! ```
//!
//! with `f_{RR} ≡ d²f/dR² = −(n+1) f_R / R` from the tracking solution above. The Compton wavelength
//! of the scalaron is `λ_C = 1/m`; perturbations with `k/a ≳ m` feel the fifth force, those with
//! `k/a ≪ m` do not. The quasi-static effective gravitational coupling for matter is
//! (Pogosian & Silvestri 2008 eq. 26; Tsujikawa 2007, arXiv:0705.1032; standard MG-growth QSA)
//!
//! ```text
//!   G_eff(a,k)/G = 1/(1 + f_R) · [ 1 + (1/3) · (k²/a²) / (k²/a² + a² m²(a)) ] = μ(a,k),
//! ```
//!
//! and the Weyl/lensing coupling is (De Felice & Tsujikawa eq. 13.40; Σ has the *complementary* 1/3)
//!
//! ```text
//!   Σ(a,k) = 1/(1 + f_R) · [ 1 + (1/3) · (a² m²(a)) / (k²/a² + a² m²(a)) ] · ... → 1/(1+f_R) on small
//!   scales (light deflection is NOT enhanced in f(R): the extra force is exactly cancelled in Φ+Ψ).
//! ```
//!
//! Limits: as `|f_R0| → 0` the scalaron mass `m → ∞`, the scale-dependent bracket → 1, and
//! `μ, Σ → 1` — exact GR. On small scales (`k/a ≫ m`) the matter coupling saturates at the famous
//! `4/3` enhancement, the hallmark of f(R) growth. Chameleon screening (the field becoming heavy in
//! deep potentials) restores GR in the Solar System; that nonlinear restoration is the adjudication
//! veto's job, not this linear-QSA function.

use super::MgResponse;

/// Reduced Hubble `H₀` in units of `h/Mpc` (so the scalaron mass `m` comes out in `h/Mpc`, the same
/// units as a comoving `k`): `H₀ = 100 h km/s/Mpc` and `c = 299792.458 km/s` ⇒ `H₀/c = 100/c` per
/// unit `h`, i.e. `1/2997.92458` in `h/Mpc`. We work per-`h`, so curvature `R` is in `(h/Mpc)²`.
const H0_OVER_C_PER_H: f64 = 100.0 / crate::cosmology::C_KM_S; // h/Mpc, per unit h

/// Fundamental f(R) Hu–Sawicki parameters: the power-law index `n` and the present-day scalaron
/// field amplitude `f_R0` (the magnitude is what matters physically; sign is fixed negative).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrParams {
    /// Hu–Sawicki index `n > 0` (typically 1–4). Sets how fast the scalaron decouples at high z.
    pub n: f64,
    /// Present-day scalaron amplitude `|f_R0|` (≥ 0). `f_R0 = 0` ⇒ exact GR. Solar-system + cluster
    /// bounds put `|f_R0| ≲ 10⁻⁶`–`10⁻⁴`; cosmological growth probes `|f_R0| ~ 10⁻⁵`–`10⁻⁴`.
    pub f_r0_abs: f64,
}

impl FrParams {
    /// Construct from the fundamental `{n, |f_R0|}`. `|f_R0|` is clamped to be non-negative.
    pub fn new(n: f64, f_r0_abs: f64) -> Self {
        FrParams {
            n,
            f_r0_abs: f_r0_abs.max(0.0),
        }
    }

    /// Background Ricci scalar `R(a) = 3 H₀² (Ω_m a⁻³ + 4 Ω_Λ)` in `(h/Mpc)²` (De Felice &
    /// Tsujikawa 2010 eq. 4.64). `omega_lambda` is the effective dark-energy density today (ΛCDM
    /// background); for a near-ΛCDM Hu–Sawicki model `Ω_Λ ≈ 1 − Ω_m`.
    fn ricci(&self, a: f64, omega_m: f64, omega_lambda: f64) -> f64 {
        let h0_sq = H0_OVER_C_PER_H * H0_OVER_C_PER_H; // (h/Mpc)²
        3.0 * h0_sq * (omega_m * a.powi(-3) + 4.0 * omega_lambda)
    }

    /// The scalaron field `f_R(a) = f_R0 (R₀/R(a))^{n+1}`, with `f_R0 = −|f_R0|` (Hu & Sawicki
    /// large-curvature limit). Returns the *signed* value (≤ 0).
    fn f_r(&self, a: f64, omega_m: f64, omega_lambda: f64) -> f64 {
        if self.f_r0_abs <= 0.0 {
            return 0.0;
        }
        let r_now = self.ricci(1.0, omega_m, omega_lambda);
        let r_a = self.ricci(a, omega_m, omega_lambda);
        -self.f_r0_abs * (r_now / r_a).powf(self.n + 1.0)
    }

    /// Scalaron mass squared `m²(a)` in `(h/Mpc)²` (so `m` is a comoving inverse length in `h/Mpc`).
    /// High-curvature QSA: `m² ≈ (1/3)(1+f_R)/f_{RR}` with `f_{RR} = −(n+1) f_R / R`. As
    /// `|f_R0| → 0`, `f_R → 0` ⇒ `f_{RR} → 0` ⇒ `m² → ∞` (the scalaron decouples and we recover GR).
    pub fn scalaron_mass_sq(&self, a: f64, omega_m: f64, omega_lambda: f64) -> f64 {
        let f_r = self.f_r(a, omega_m, omega_lambda);
        if f_r == 0.0 {
            return f64::INFINITY;
        }
        let r_a = self.ricci(a, omega_m, omega_lambda);
        // f_{RR} = −(n+1) f_R / R  (differentiate f_R = f_R0 (R₀/R)^{n+1} w.r.t. R). f_R < 0 ⇒
        // f_{RR} > 0, so m² > 0 as required for a stable (non-tachyonic) scalaron.
        let f_rr = -(self.n + 1.0) * f_r / r_a;
        ((1.0 + f_r) / f_rr - r_a) / 3.0
    }

    /// Derived quasi-static response `μ(a,k) = G_eff/G` and `Σ(a,k)` at comoving wavenumber `k`
    /// (in `h/Mpc`) and scale factor `a`. GR is recovered as `|f_R0| → 0` (and at `a→0`, deep in
    /// matter domination where the scalaron is heavy).
    ///
    /// `μ = 1/(1+f_R) · [1 + (1/3) g]`, `g = (k/a)² / [(k/a)² + m²]` ∈ [0, 1]. Small scales (`g→1`)
    /// give the `4/3` matter enhancement; large scales (`g→0`) give GR. Lensing `Σ` does NOT pick up
    /// the fifth force (the Weyl potential `Φ+Ψ` is unmodified up to the `1/(1+f_R)` Planck-mass
    /// rescaling): `Σ = 1/(1+f_R)` (De Felice & Tsujikawa eq. 13.40 — the `μΣ` "no extra lensing"
    /// signature of f(R)).
    pub fn response(&self, a: f64, k: f64, omega_m: f64, omega_lambda: f64) -> MgResponse {
        let f_r = self.f_r(a, omega_m, omega_lambda);
        if f_r == 0.0 {
            return MgResponse::GR;
        }
        let m2 = self.scalaron_mass_sq(a, omega_m, omega_lambda);
        let k_phys_sq = (k / a) * (k / a); // (k/a)² in (h/Mpc)²
        let g = if m2.is_finite() {
            k_phys_sq / (k_phys_sq + m2)
        } else {
            0.0
        };
        let planck_rescale = 1.0 / (1.0 + f_r); // ≥ 1 since f_R < 0
        MgResponse {
            mu: planck_rescale * (1.0 + g / 3.0),
            sigma: planck_rescale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A Planck-ish background for the tests.
    const OM: f64 = 0.31;
    const OL: f64 = 0.69;

    #[test]
    fn recovers_gr_as_f_r0_goes_to_zero() {
        let k = 0.1; // h/Mpc
        let a = 1.0;
        let strong = FrParams::new(1.0, 1e-4).response(a, k, OM, OL);
        let weak = FrParams::new(1.0, 1e-8).response(a, k, OM, OL);
        let gr = FrParams::new(1.0, 0.0).response(a, k, OM, OL);
        assert_eq!(gr, MgResponse::GR);
        // Vanishing f_R0 ⇒ μ → 1; the response is monotone in |f_R0| near the GR point.
        assert!((weak.mu - 1.0).abs() < (strong.mu - 1.0).abs());
        assert!((weak.mu - 1.0).abs() < 1e-3, "weak μ = {}", weak.mu);
    }

    #[test]
    fn enhances_matter_coupling_on_small_scales() {
        // Larger |f_R0| ⇒ lighter scalaron ⇒ more scales feel the fifth force ⇒ larger μ.
        let a = 1.0;
        let k = 0.2; // h/Mpc, mildly nonlinear / small scale
        let mu_small = FrParams::new(1.0, 1e-6).response(a, k, OM, OL).mu;
        let mu_big = FrParams::new(1.0, 1e-4).response(a, k, OM, OL).mu;
        assert!(
            mu_big > mu_small,
            "μ(1e-4)={mu_big} should exceed μ(1e-6)={mu_small}"
        );
        assert!(mu_big > 1.0, "f(R) must enhance gravity: μ = {mu_big}");
    }

    #[test]
    fn matter_coupling_saturates_below_four_thirds() {
        // On very small scales (k/a ≫ m) the bracket → 4/3; with the 1/(1+f_R) ≥ 1 prefactor the
        // matter coupling is ≥ 4/3 but bounded for sane |f_R0|.
        let mu = FrParams::new(1.0, 1e-4).response(1.0, 5.0, OM, OL).mu;
        assert!(mu > 1.30 && mu < 1.40, "small-scale μ = {mu} (expect ≈4/3)");
    }

    #[test]
    fn lensing_is_not_force_enhanced() {
        // f(R) signature: Σ carries only the Planck-mass rescaling, no scale-dependent fifth force.
        let r = FrParams::new(1.0, 1e-4).response(1.0, 5.0, OM, OL);
        let rescale = 1.0 / (1.0 - 1e-4 * 1.0_f64.powf(2.0)); // crude, just to bound
        assert!(
            r.sigma > 1.0 && r.sigma < 1.001,
            "Σ = {} ({rescale})",
            r.sigma
        );
        assert!(
            r.mu > r.sigma,
            "matter coupling exceeds lensing coupling in f(R)"
        );
    }

    #[test]
    fn scalaron_is_heavier_in_the_past() {
        // R(a) grows toward early times (∝ a⁻³), the field decouples, m² rises: μ → GR at high z.
        let p = FrParams::new(1.0, 1e-4);
        let m2_now = p.scalaron_mass_sq(1.0, OM, OL);
        let m2_past = p.scalaron_mass_sq(0.3, OM, OL);
        assert!(
            m2_past > m2_now,
            "m²(a=0.3)={m2_past} should exceed m²(a=1)={m2_now}"
        );
        // At fixed k, a heavier scalaron ⇒ μ closer to GR.
        let k = 0.1;
        let mu_now = p.response(1.0, k, OM, OL).mu;
        let mu_past = p.response(0.3, k, OM, OL).mu;
        assert!(
            (mu_past - 1.0).abs() < (mu_now - 1.0).abs(),
            "high-z μ={mu_past} should be closer to GR than low-z μ={mu_now}"
        );
    }
}
