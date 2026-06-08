//! Derived modified-gravity sectors: families whose growth modification is *computed* from one
//! fundamental, action-level parameter — not a fitted, constant late-time `mu(a)` knob.
//!
//! The reviewers' demand (`docs/zyal-next-level-design.md`; plan M4 item 13–14): "a modified-gravity
//! theorist will not accept a constant late-time mu(a) as the genome." So each sector here exposes a
//! deterministic `mu(a,k)` (the matter-perturbation effective gravitational coupling G_eff/G in the
//! Poisson equation `k² Ψ = −4πG a² μ ρ δ`) and `Sigma(a,k)` (the lensing/Weyl coupling), derived
//! from the *fundamental* parameters of a covariant action:
//!
//! - [`fr`]: f(R) Hu–Sawicki `{n, f_R0}` — a scalaron of mass `m(a)` set by the background curvature;
//!   `G_eff(a,k)` is scale-dependent (the fifth force turns on for `k/a ≳ m`) and chameleon-screened.
//! - [`ndgp`]: nDGP `{r_c}` — a brane-crossover scale; `G_eff(a)` is scale-free on linear scales and
//!   Vainshtein-screened on small scales.
//!
//! Both reduce to GR (`mu = Sigma = 1`) in their decoupling limit (`f_R0 → 0`; `r_c → ∞`), so a
//! family with a vanishing fundamental parameter is *literally* ΛCDM, not a penalized near-miss.
//!
//! These are quasi-static-approximation (QSA) expressions, standard for sub-horizon linear growth
//! (Pogosian & Silvestri 2008, arXiv:0709.0888; De Felice & Tsujikawa 2010 Living Review,
//! arXiv:1002.4928 §13; Koyama 2016 review, arXiv:1504.04623). The full scale-and-time `G_eff(a,k)`
//! beyond QSA is the Boltzmann backend's job (v3.1).

pub mod fr;
pub mod ndgp;

/// A derived scale-dependent modified-gravity response at a point `(a, k)`: the matter coupling
/// `mu = G_eff/G` and the lensing coupling `Sigma = G_lens/G`. GR is `mu = Sigma = 1`.
///
/// `k` is a *comoving* wavenumber in `h/Mpc`; the QSA expressions depend on the physical scale
/// `k/a`, so callers pass comoving `k` and the scale factor `a` separately. The physical-to-comoving
/// `h` factor is folded in by the sector that needs it (f(R) compares `k/a` against the scalaron
/// mass `m`, which the implementation carries in the same `h/Mpc` units).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MgResponse {
    /// Matter-perturbation effective gravitational coupling `G_eff/G` (the Poisson-equation `mu`).
    pub mu: f64,
    /// Weyl/lensing effective coupling `Sigma = (Φ+Ψ) response /G` (deflection of light).
    pub sigma: f64,
}

impl MgResponse {
    /// The GR point: no modification.
    pub const GR: MgResponse = MgResponse {
        mu: 1.0,
        sigma: 1.0,
    };
}
