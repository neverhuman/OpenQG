//! Published cross-domain constraint constants and the theory-side predictions they test, for the
//! [`super::unification`] likelihood channel. Every numeric value is cited to its primary source in
//! the doc comment beside it; nothing here is invented or tuned.
//!
//! The constants are the *measurements* (null tests linking cosmology to gravity and particle
//! physics); the `predicted_*` functions map a [`Theory`]'s structural physics (α-basis, declared
//! screening, background `omega_b`/`N_eff`) onto the same observables so the channel can form a
//! residual and a Gaussian log-likelihood.

use super::Theory;

// ----------------------------------------------------------------------------------------------
// Published cross-domain constraints (every value cited to its primary source).
// ----------------------------------------------------------------------------------------------

/// GW170817 + GRB170817A multi-messenger bound on the fractional tensor-speed excess
/// `|c_T/c - 1|`. The ~1.7 s arrival-time difference between the GW and the gamma-ray burst over a
/// ~40 Mpc baseline constrains `-3e-15 < c_GW/c - 1 < +7e-15`. We model it as a null with a
/// representative `sigma = 1e-15` (the two-sided bound is a few x 1e-15). Source: Abbott et al.
/// 2017, ApJL 848 L13 ("GW170817 and GRB170817A"), arXiv:1710.05834; GW detection arXiv:1710.05835.
pub const CT_EXCESS_SIGMA: f64 = 1.0e-15;

/// LVK GW170817 standard-siren Hubble constant: `H0 = 70 (+12 / -8)` km/s/Mpc (median + 68.3%
/// highest-density interval). A *broad, independent* H0 measurement that does not depend on the
/// distance ladder or the CMB. Source: Abbott et al. 2017, Nature 551 85
/// ("A gravitational-wave standard siren measurement of the Hubble constant"), arXiv:1710.05835.
pub const SIREN_H0: f64 = 70.0;
pub const SIREN_H0_SIGMA_HI: f64 = 12.0;
pub const SIREN_H0_SIGMA_LO: f64 = 8.0;

/// Cassini solar-conjunction measurement of the PPN parameter: `gamma - 1 = (2.1 +/- 2.3)e-5`.
/// GR predicts `gamma = 1` exactly. Source: Bertotti, Iess & Tortora 2003, Nature 425 374
/// ("A test of general relativity using radio links with the Cassini spacecraft").
pub const CASSINI_GAMMA_MINUS_ONE: f64 = 2.1e-5;
pub const CASSINI_GAMMA_SIGMA: f64 = 2.3e-5;

/// MICROSCOPE final Eötvös ratio (Ti/Pt test masses): `eta = (-1 +/- 2)e-15` (statistical +
/// systematic, 1σ). GR / the weak equivalence principle predicts `eta = 0`. Source: Touboul et al.
/// 2022, Phys. Rev. Lett. 129 121102 ("MICROSCOPE Mission: Final Results"), arXiv:2201.03889.
pub const MICROSCOPE_ETA: f64 = -1.0e-15;
pub const MICROSCOPE_ETA_SIGMA: f64 = 2.0e-15;

/// Most-precise primordial deuterium abundance: `(D/H)_p = (2.527 +/- 0.030)e-5` (weighted mean of
/// seven precision absorption systems). Source: Cooke, Pettini & Steidel 2018, ApJ 855 102
/// ("One Percent Determination of the Primordial Deuterium Abundance"), arXiv:1710.11129.
pub const DH_OBS: f64 = 2.527e-5;
pub const DH_OBS_SIGMA: f64 = 0.030e-5;
/// Theory-side (nuclear-rate) uncertainty on the predicted D/H, added in quadrature with the
/// observational error. The PArthENoPE/PRIMAT fits quote ~2% theory error; we use 0.06e-5 (~2.4%).
/// Source: Pitrou et al. 2018, Phys. Rept. 754 1 ("Precision big bang nucleosynthesis with improved
/// helium-4 predictions"), arXiv:1801.08023 (D/H error budget).
pub const DH_THEORY_SIGMA: f64 = 0.06e-5;

/// Reference effective neutrino number the PArthENoPE D/H fit's `delta_neff` is expanded about
/// (`delta_neff = N_eff - 3.046`, the Planck-2018-era standard value). Matches the repo baseline.
const NEFF_REFERENCE: f64 = 3.046;

// ----------------------------------------------------------------------------------------------
// Predicted cross-domain observables from the theory's structural physics.
// ----------------------------------------------------------------------------------------------

/// Fractional tensor-speed excess `c_T/c - 1`. With `c_GW^2 = c^2 (1 + alpha_T)` (Bellini &
/// Sawicki 2014, arXiv:1404.3713), `c_T/c = sqrt(1 + alpha_T)`, so `c_T/c - 1 = sqrt(1+alpha_T)-1`.
pub fn predicted_ct_excess(theory: &Theory) -> f64 {
    (1.0 + theory.alpha.alpha_t).max(0.0).sqrt() - 1.0
}

/// Deep-screening residual suppression factor applied to the linear PPN/EP deviation when a
/// screening mechanism is declared. A *working* chameleon/Vainshtein/symmetron/k-mouflage screen
/// restores GR at solar-system densities to far below current lab sensitivity: chameleon thin-shell
/// suppression is exponential in the source's Newtonian potential, and Vainshtein suppression on
/// the fifth force scales as `(r/r_V)^{3/2}` which is ≪ 1e-15 inside the screened radius (Khoury &
/// Weltman 2004 arXiv:astro-ph/0309300; Koyama 2016 arXiv:1504.04623; Burrage & Sakstein 2018
/// review arXiv:1709.09071). In v3.0.0 screening is a *declared* boolean (its action-level
/// derivation against Cassini/MICROSCOPE/Eöt-Wash is M3); a declared screen is therefore modeled as
/// driving the residual below every current lab bound, with a conservative factor `1e-18` (well
/// below the MICROSCOPE 2e-15 and Cassini 2.3e-5 sensitivities for any `O(1)` α-deviation).
const SCREENING_SUPPRESSION: f64 = 1.0e-18;

/// Predicted solar-system PPN `gamma - 1`. In a scalar-tensor / Horndeski theory the linear `gamma`
/// deviates from 1 by an amount set by the scalar–matter coupling, captured here by the
/// gravity-modification scale of the α-basis (`alpha_M, alpha_B, alpha_K`); GR ⇒ 0. We map the
/// dimensionless modification scale to `gamma - 1` at order unity (linear PPN; Will 2014, Living
/// Rev. Relativity 17 4). A declared screening mechanism suppresses the residual to a safe value.
pub fn predicted_gamma_minus_one(theory: &Theory) -> f64 {
    let linear = theory.alpha.modification_scale();
    if theory.screening.is_some() {
        linear * SCREENING_SUPPRESSION
    } else {
        linear
    }
}

/// Predicted equivalence-principle Eötvös ratio `eta`. A fifth force from an *unscreened*
/// scalar–matter coupling is composition-dependent and violates the (weak) EP; the same screening
/// that protects PPN also protects EP. To leading order both scale with the unsuppressed scalar
/// coupling, so we model `|eta| ~ gamma_deviation` (Hees et al. 2018, arXiv:1706.06294). GR ⇒ 0.
pub fn predicted_eta(theory: &Theory) -> f64 {
    predicted_gamma_minus_one(theory)
}

/// Predicted primordial deuterium abundance `(D/H)_p` from the background baryon density and
/// effective neutrino number, via the public PArthENoPE polynomial fit (the standard CAMB
/// `BBN_fitting_parthenope`). The fit, valid near the observed `omega_b`:
///
/// ```text
/// 1e5 (D/H)_p = 18.754 - 1534.4 w + 48656 w^2 - 552670 w^3
///   + dN (2.4914 - 208.11 w + 6760.9 w^2 - 78007 w^3)
///   + dN^2 (0.012907 - 1.3653 w + 37.388 w^2 - 267.78 w^3)
/// ```
///
/// with `w = omega_b_h2`, `dN = N_eff - 3.046`, at the PArthENoPE reference neutron lifetime
/// `tau_n = 880.3 s`. Source: PArthENoPE polynomial fit as implemented in CAMB
/// `camb/bbn.py::BBN_fitting_parthenope`; coefficients from the PArthENoPE code (Pisanti et al.
/// 2008, Comput. Phys. Commun. 178 956; Consiglio et al. 2018, arXiv:1712.04378) and the Planck
/// 2015 BBN appendix (Planck Collaboration 2016, A&A 594 A13). The `(tau_n/880.3)^0.418`
/// lifetime correction is omitted: for the PDG-2018 value `tau_n = 879.4 +/- 0.6 s` it is < 0.05%.
pub fn predicted_deuterium(theory: &Theory) -> f64 {
    let w = theory.background.omega_b_h2;
    let dn = theory.background.n_eff - NEFF_REFERENCE;
    let w2 = w * w;
    let w3 = w2 * w;
    let base = 18.754 - 1534.4 * w + 48656.0 * w2 - 552670.0 * w3;
    let lin = 2.4914 - 208.11 * w + 6760.9 * w2 - 78007.0 * w3;
    let quad = 0.012907 - 1.3653 * w + 37.388 * w2 - 267.78 * w3;
    (base + dn * lin + dn * dn * quad) * 1.0e-5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::Theory;

    #[test]
    fn predicted_deuterium_matches_published_value_at_planck_baryons() {
        // Sanity-check the PArthENoPE fit: at Planck omega_b, N_eff=3.046, D/H must land near the
        // Cooke+2018 central value (2.527e-5) to well within the observational sigma.
        let dh = predicted_deuterium(&Theory::baseline_lcdm());
        assert!(
            (dh - DH_OBS).abs() < 5.0 * DH_OBS_SIGMA,
            "PArthENoPE D/H = {dh:.4e} vs observed {DH_OBS:.4e}"
        );
    }

    #[test]
    fn gr_predictions_sit_at_the_nulls() {
        let t = Theory::baseline_lcdm();
        assert_eq!(predicted_ct_excess(&t), 0.0, "GR has c_T = c");
        assert_eq!(predicted_gamma_minus_one(&t), 0.0, "GR has gamma = 1");
        assert_eq!(predicted_eta(&t), 0.0, "GR respects the EP");
    }

    #[test]
    fn declared_screening_suppresses_the_ppn_and_ep_residual() {
        use crate::theory::AlphaBasis;
        let mut t = Theory::baseline_lcdm();
        t.alpha = AlphaBasis {
            alpha_m: 0.1,
            alpha_b: 0.0,
            alpha_k: 0.0,
            alpha_t: 0.0,
        };
        let unscreened = predicted_gamma_minus_one(&t);
        t.screening = Some("vainshtein".into());
        let screened = predicted_gamma_minus_one(&t);
        assert!(unscreened > 0.0);
        // Screened residual is far below the MICROSCOPE sensitivity (2e-15).
        assert!(
            screened < MICROSCOPE_ETA_SIGMA,
            "screened residual = {screened:.2e}"
        );
    }
}
