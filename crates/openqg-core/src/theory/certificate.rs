//! M1: value-level derivation certificates — make "derived, not fit" bind the *value*, not just
//! the `Provenance::Derived` text label.
//!
//! Today a `Provenance::Derived { mechanism }` is accepted on the strength of a non-empty mechanism
//! string (see [`super::Provenance::is_whitebox`]): the *number* it carries can be anything. That is
//! exactly the gray-box trap the project forbids (`zyal-next-level-design.md` §1.3): a fitted value
//! wearing a "derived" label. A [`DerivedCertificate`] closes the hole — it names a closed-form
//! relation from a small, *cited* registry, supplies the named inputs, and the deterministic
//! [`DerivedCertificate::verify`] oracle recomputes the value and checks it against the claimed
//! `expected` to within `tolerance`. A `Derived` value with a certificate that *fails* this check is
//! demoted to `Free` (and killed) by the veto cascade — see [`super::vetoes`].
//!
//! The registry holds only standard, citable, closed-form modified-gravity relations. Every entry
//! cites its literature source in a comment. No relation is invented here.

use serde::{Deserialize, Serialize};

/// A machine-checkable certificate that a `Derived` parameter's value equals a closed-form function
/// of its declared inputs. The oracle ([`Self::verify`]) is deterministic and self-contained — it
/// never consults wall-clock, RNG, or external state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DerivedCertificate {
    /// Name of the closed-form relation in the registry (see [`relation_registry`]).
    pub relation: String,
    /// Named inputs to the relation `(name, value)`. Order is not significant; the relation looks
    /// up its inputs by name so a missing/misnamed input fails verification rather than mis-binding.
    pub inputs: Vec<(String, f64)>,
    /// The value the parameter claims to take (i.e. the number the theory actually uses).
    pub expected: f64,
    /// Absolute tolerance |computed − expected| ≤ tolerance for the certificate to verify. A
    /// non-finite or negative tolerance never verifies (guards against a "tolerance = ∞" cheat).
    pub tolerance: f64,
}

/// Outcome of running a certificate through the oracle, with enough detail for a receipt/diagnostic.
#[derive(Debug, Clone, PartialEq)]
pub enum CertificateOutcome {
    /// The relation recomputed the value and it matched `expected` within `tolerance`.
    Verified { computed: f64, residual: f64 },
    /// `expected` did not match the recomputed value within `tolerance`.
    ValueMismatch { computed: f64, residual: f64 },
    /// The named relation is not in the registry.
    UnknownRelation,
    /// A required input was missing, or an input was outside the relation's physical domain
    /// (e.g. a non-positive crossover scale), so the closed form is undefined.
    MissingOrInvalidInput { detail: String },
    /// The declared tolerance is not a usable non-negative finite number.
    InvalidTolerance,
}

impl DerivedCertificate {
    /// Look up a named input by exact name. Returns `None` if absent (caller treats as invalid).
    fn input(&self, name: &str) -> Option<f64> {
        // V7 (P1.1): case-insensitive input lookup — 'A_drag' vs 'a_drag' spelling killed ~100
        // otherwise-valid dark-scattering certificates in the V6 campaign. The canonical key is
        // still the lowercase form; matching is forgiving, semantics are unchanged.
        self.inputs
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| *v)
    }

    /// Look up a *required* named input, returning an `Err(detail)` if absent. Factors the common
    /// "missing input" extraction so each relation reads as just its closed form.
    fn required(&self, name: &str, what: &str) -> Result<f64, String> {
        self.input(name)
            .ok_or_else(|| format!("missing input '{name}' ({what})"))
    }

    /// Run the deterministic oracle: recompute `expected` from `inputs` via the named registry
    /// relation and report a detailed [`CertificateOutcome`].
    pub fn check(&self) -> CertificateOutcome {
        if !(self.tolerance.is_finite() && self.tolerance >= 0.0) {
            return CertificateOutcome::InvalidTolerance;
        }
        let relation = match relation_registry(&self.relation) {
            Some(r) => r,
            None => return CertificateOutcome::UnknownRelation,
        };
        match relation(self) {
            Ok(computed) => {
                let residual = (computed - self.expected).abs();
                if residual <= self.tolerance {
                    CertificateOutcome::Verified { computed, residual }
                } else {
                    CertificateOutcome::ValueMismatch { computed, residual }
                }
            }
            Err(detail) => CertificateOutcome::MissingOrInvalidInput { detail },
        }
    }

    /// Boolean oracle: `true` iff the certificate verifies. Convenience over [`Self::check`].
    pub fn verify(&self) -> bool {
        matches!(self.check(), CertificateOutcome::Verified { .. })
    }
}

/// Signature of a registry relation: a pure closed form from a certificate's inputs to the value it
/// should produce, or an `Err(detail)` when an input is missing or outside the physical domain.
type Relation = fn(&DerivedCertificate) -> Result<f64, String>;

/// The registry of named, closed-form, *cited* relations. Returns `None` for an unknown name.
///
/// Keep these simple and correct. Each maps a small set of fundamental/background inputs to the
/// derived quantity; the literature source is cited inline at each relation.
pub fn relation_registry(name: &str) -> Option<Relation> {
    match name {
        "ndgp_geff_over_g" => Some(ndgp_geff_over_g),
        "ndgp_beta_from_omega_rc" => Some(ndgp_beta_from_omega_rc),
        "planck_mu0_geff" => Some(planck_mu0_geff),
        "dark_scattering_growth_drag" => Some(dark_scattering_growth_drag),
        "fr_largescale_geff_over_g" => Some(fr_largescale_geff_over_g),
        "fr_alpha_m" => Some(fr_alpha_m),
        "coupled_de_geff_over_g" => Some(coupled_de_geff_over_g),
        "h0_from_h" => Some(h0_from_h),
        "flat_universe_omega_lambda" => Some(flat_universe_omega_lambda),
        _ => None,
    }
}

/// All registered relation names (stable, sorted) — useful for diagnostics and tests.
pub fn registered_relations() -> Vec<&'static str> {
    vec![
        "coupled_de_geff_over_g",
        "dark_scattering_growth_drag",
        "flat_universe_omega_lambda",
        "fr_alpha_m",
        "fr_largescale_geff_over_g",
        "h0_from_h",
        "ndgp_beta_from_omega_rc",
        "ndgp_geff_over_g",
        "planck_mu0_geff",
    ]
}

/// How much *derivation* a verified certificate on this relation represents, in `[0,1]`.
///
/// This is the fix for the "definition wearing a derived label" gaming surface: recomputing a
/// definition or an algebraic closure is a tautology, not a derivation. `h0_from_h` (H0 = 100·h, a
/// unit definition) and `flat_universe_omega_lambda` (Ω_Λ = 1 − ΣΩ, flat closure) therefore earn
/// **zero** rigor — they keep their value-binding/veto role (a fitted H0 inconsistent with h is
/// still caught) but contribute nothing to derivation_rigor. The genuine closed-form
/// modified-gravity relations earn full weight. An unknown name earns zero (it is not a known,
/// machine-checkable derivation).
pub fn relation_rigor_weight(name: &str) -> f64 {
    match name {
        "h0_from_h" | "flat_universe_omega_lambda" => 0.0,
        "ndgp_geff_over_g"
        | "ndgp_beta_from_omega_rc"
        | "fr_largescale_geff_over_g"
        | "fr_alpha_m"
        | "coupled_de_geff_over_g" => 1.0,
        // Honest accounting: μ(a=1) = 1 + μ0 is a *cited phenomenological parametrization*
        // (Planck 2018), not a mechanism — it is near-definitional, so it earns only partial
        // rigor. It still binds the value (a fitted G_eff/G inconsistent with μ0 is caught).
        "planck_mu0_geff" => 0.3,
        // A real mechanism (DE–DM momentum exchange) with cited equations the engine integrates;
        // 0.8 until the full Boltzmann treatment exists.
        "dark_scattering_growth_drag" => 0.8,
        _ => 0.0,
    }
}

/// The GR/ΛCDM-limit value of a *modification* relation — the value the derived quantity takes when
/// the modification vanishes — or `None` for trivial definitions that have no GR limit. Used to
/// decide whether a certified-derived parameter actually *departs* from GR (i.e. is physically
/// distinct from ΛCDM) rather than sitting at the GR point.
pub fn relation_gr_value(name: &str) -> Option<f64> {
    Some(match name {
        "ndgp_geff_over_g" => 1.0,            // β → ∞
        "coupled_de_geff_over_g" => 1.0,      // β → 0
        "fr_largescale_geff_over_g" => 1.0,   // outside the Compton wavelength
        "fr_alpha_m" => 0.0,                  // f_R → 0
        "planck_mu0_geff" => 1.0,             // μ0 = 0 ⇒ G_eff/G = 1 (GR)
        "dark_scattering_growth_drag" => 0.0, // Γ0 = 0 ⇒ no drag (GR)
        // ndgp_beta_from_omega_rc has *no finite* GR value: the GR limit is β → ∞ (Ω_rc → 0),
        // not a number β can sit at. A certified β therefore cannot establish GR-distinctness by
        // itself — pair it with ndgp_geff_over_g (whose GR value is 1) for that.
        _ => return None, // also h0_from_h / flat_universe_omega_lambda: no GR limit
    })
}

/// A one-line input-signature hint per relation (for proposer prompts / diagnostics).
pub fn relation_signature(name: &str) -> Option<&'static str> {
    Some(match name {
        "ndgp_geff_over_g" => "inputs {beta}; G_eff/G = 1 + 1/(3·beta)",
        "ndgp_beta_from_omega_rc" => {
            "inputs {omega_rc, omega_m, [omega_r, omega_k, w0]}; beta(a=1) = 1 + (1+D1/3)/sqrt(omega_rc)"
        }
        "planck_mu0_geff" => {
            "inputs {mu0 > -1, negative allowed}; G_eff/G = 1 + mu0 (Planck 2018 MG parametrization)"
        }
        "dark_scattering_growth_drag" => {
            "inputs {a_drag >= 0, w0 >= -1, omega_de0 in [0,1]}; Gamma0 = a_drag·(1+w0)·omega_de0 (Simpson 2010)"
        }
        "fr_largescale_geff_over_g" => {
            "inputs {regime: 1.0 inside / 0.0 outside Compton}; 4/3 or 1"
        }
        "fr_alpha_m" => "inputs {f_R, a_f_R_prime}; alpha_M = a_f_R_prime/(1+f_R)",
        "coupled_de_geff_over_g" => "inputs {beta}; G_eff/G = 1 + 2·beta²",
        "h0_from_h" => "inputs {h}; H0 = 100·h (km/s/Mpc)",
        "flat_universe_omega_lambda" => {
            "inputs {omega_m, [omega_r], [omega_k]}; Omega_Lambda = 1 − omega_m − omega_r − omega_k"
        }
        _ => return None,
    })
}

// --- Registry relations (each cited) ----------------------------------------------------------

/// nDGP normal-branch linear effective gravitational coupling:
///     G_eff / G = 1 + 1 / (3 β)
/// with the (dimensionless) braneworld function
///     β = 1 + 2 H r_c (1 + Ḣ / (3 H²)).
/// Reference: Koyama & Maartens, JCAP 0601:016 (2006), arXiv:astro-ph/0511634; Schmidt, Phys. Rev.
/// D 80, 043001 (2009), arXiv:0905.0858. The crossover scale r_c controls leakage of gravity into
/// the 5D bulk; β → ∞ (large H r_c) recovers GR (G_eff/G → 1).
///
/// Inputs: `beta` (the precomputed β). We take β directly rather than re-deriving it from
/// (H, r_c, Ḣ) so the certificate is a clean closed form over a single fundamental input; the
/// β(a) background relation is the forward model's job. β must be non-zero.
fn ndgp_geff_over_g(c: &DerivedCertificate) -> Result<f64, String> {
    let beta = c.required("beta", "braneworld β")?;
    if beta == 0.0 || !beta.is_finite() {
        return Err(format!("nDGP β must be finite and non-zero, got {beta}"));
    }
    Ok(1.0 + 1.0 / (3.0 * beta))
}

/// nDGP braneworld function β at a = 1 (today), closed form from the background budget (V5):
///     Ω_de = 1 − Ω_m − Ω_r − Ω_k                                   (flat-closure dark energy)
///     D1   = dlnE/dN |_{a=1} = −½ (3Ω_m + 4Ω_r + 2Ω_k + 3(1+w0)Ω_de)   (E(a=1) = 1)
///     β    = 1 + (1/√Ω_rc) (1 + D1/3),
/// which is exactly `NdgpParams::beta` (sectors/ndgp.rs), β = 1 + (E/√Ω_rc)(1 + (dlnE/dN)/3),
/// evaluated at a = 1 where E = 1. Reference: Koyama & Maartens, JCAP 0601:016 (2006),
/// arXiv:astro-ph/0511634; Bose et al. 2018, arXiv:1606.02520 (Ω_rc ≡ 1/(4H₀²r_c²) convention,
/// so 2 H₀ r_c = 1/√Ω_rc).
///
/// Inputs: `omega_rc` (required, finite, > 0), `omega_m` (required, finite, ≥ 0); optional
/// `omega_r`, `omega_k` (default 0) and `w0` (default −1). Domain errors for Ω_rc ≤ 0,
/// any non-finite input, or Ω_de < 0.
fn ndgp_beta_from_omega_rc(c: &DerivedCertificate) -> Result<f64, String> {
    let omega_rc = c.required("omega_rc", "dimensionless crossover Ω_rc = 1/(4H₀²r_c²)")?;
    let omega_m = c.required("omega_m", "matter density parameter today")?;
    let omega_r = c.input("omega_r").unwrap_or(0.0);
    let omega_k = c.input("omega_k").unwrap_or(0.0);
    let w0 = c.input("w0").unwrap_or(-1.0);
    if !omega_rc.is_finite() || omega_rc <= 0.0 {
        return Err(format!(
            "nDGP Ω_rc must be finite and positive, got {omega_rc}"
        ));
    }
    if !omega_m.is_finite() || omega_m < 0.0 {
        return Err(format!(
            "Ω_m must be finite and non-negative, got {omega_m}"
        ));
    }
    if !(omega_r.is_finite() && omega_k.is_finite() && w0.is_finite()) {
        return Err(format!(
            "Ω_r, Ω_k, w0 must all be finite, got ({omega_r}, {omega_k}, {w0})"
        ));
    }
    let omega_de = 1.0 - omega_m - omega_r - omega_k;
    if !omega_de.is_finite() || omega_de < 0.0 {
        return Err(format!(
            "flat-closure Ω_de = 1 − Ω_m − Ω_r − Ω_k must be non-negative, got {omega_de}"
        ));
    }
    // dlnE/dN at a = 1 (E = 1): −½ Σ_i (3(1+w_i)) Ω_i with w_m = 0, w_r = 1/3, w_k = −1/3.
    let d1 = -0.5 * (3.0 * omega_m + 4.0 * omega_r + 2.0 * omega_k + 3.0 * (1.0 + w0) * omega_de);
    Ok(1.0 + (1.0 / omega_rc.sqrt()) * (1.0 + d1 / 3.0))
}

/// Planck-2018 modified-gravity μ0 parametrization evaluated today (V5):
///     G_eff/G (a=1) = μ(a=1) = 1 + μ0,
/// where the engine's growth code implements the time dependence μ(a) = 1 + μ0·Ω_DE(a)/Ω_DE0 (so
/// at a = 1 the ratio is exactly 1 and G_eff/G = 1 + μ0). Reference: Planck 2018 results VI,
/// arXiv:1807.06209 (and Planck 2015 XIV, arXiv:1502.01590), modified-gravity parametrization.
///
/// Input: `mu0` (required, finite, > −1 so G_eff stays positive). NEGATIVE μ0 is explicitly
/// allowed — that is the *suppressed-growth* direction (G_eff/G < 1), which the parametrization
/// covers symmetrically with the enhanced-growth direction. μ0 = 0 is GR.
fn planck_mu0_geff(c: &DerivedCertificate) -> Result<f64, String> {
    let mu0 = c.required("mu0", "Planck 2018 μ0 (G_eff/G − 1 today)")?;
    if !mu0.is_finite() || mu0 <= -1.0 {
        return Err(format!(
            "μ0 must be finite and > −1 (G_eff must stay positive), got {mu0}"
        ));
    }
    Ok(1.0 + mu0)
}

/// V6 dark-scattering growth drag (Simpson 2010, PRD 82, 083505: "Scattering of dark matter
/// and dark energy"; Pourtsidou, Skordis & Copeland 2013): a DE–DM momentum-exchange cross
/// section adds a friction term Γ(a) = A_drag·(1+w(a))·Ω_de(a) to the linear growth equation.
/// This relation certifies the TODAY amplitude:
///     Γ₀ = A_drag · (1 + w0) · Ω_de0,
/// the value the growth code's `growth_drag_gamma(a=1)` computes. Domain: A_drag ≥ 0 and
/// w0 ≥ −1 (no momentum exchange with a cosmological constant; phantom drag is unphysical).
/// GR limit: A_drag → 0 ⇒ Γ₀ = 0.
fn dark_scattering_growth_drag(c: &DerivedCertificate) -> Result<f64, String> {
    let a_drag = c.required("a_drag", "dimensionless drag amplitude A_drag")?;
    let w0 = c.required("w0", "dark-energy equation of state today")?;
    let omega_de0 = c.required("omega_de0", "dark-energy density fraction today")?;
    if !a_drag.is_finite() || a_drag < 0.0 {
        return Err(format!("A_drag must be finite and >= 0, got {a_drag}"));
    }
    if !w0.is_finite() || w0 < -1.0 {
        return Err(format!(
            "w0 must be finite and >= -1 for a physical drag, got {w0}"
        ));
    }
    if !(0.0..=1.0).contains(&omega_de0) {
        return Err(format!("omega_de0 must be in [0,1], got {omega_de0}"));
    }
    Ok(a_drag * (1.0 + w0) * omega_de0)
}

/// f(R) effective gravitational coupling in the small-scale / large-k quasi-static limit, i.e. for
/// modes well *inside* the scalaron Compton wavelength (k ≫ a m_fR):
///     G_eff / G = 4/3.
/// On scales well *outside* it (k ≪ a m_fR) one recovers GR (G_eff/G → 1); the chameleon mechanism
/// restores GR in high-density (solar-system) environments. Reference: Hu & Sawicki, Phys. Rev. D
/// 76, 064004 (2007), arXiv:0705.1158; Pogosian & Silvestri, Phys. Rev. D 77, 023503 (2008),
/// arXiv:0709.0296 (the 4/3 = 1 + 1/3 inside-Compton-wavelength enhancement).
///
/// Inputs: `regime` — a flag (1.0 = inside Compton wavelength → 4/3; 0.0 = outside → 1.0). Encoding
/// the two analytic limits keeps this a pure closed form without a Boltzmann solve.
fn fr_largescale_geff_over_g(c: &DerivedCertificate) -> Result<f64, String> {
    let regime = c.required("regime", "1.0 inside / 0.0 outside Compton wavelength")?;
    if regime == 1.0 {
        Ok(4.0 / 3.0)
    } else if regime == 0.0 {
        Ok(1.0)
    } else {
        Err(format!(
            "f(R) regime flag must be 1.0 (inside) or 0.0 (outside Compton wavelength), got {regime}"
        ))
    }
}

/// f(R) effective-Planck-mass run rate α_M in the α-basis (Bellini & Sawicki 2014):
///     M*² = 1 + f_R,   α_M = d ln M*² / d ln a = (a f_R')/(1 + f_R),
/// where f_R = df/dR and f_R' = df_R/da. Reference: Bellini & Sawicki, JCAP 1407:050 (2014),
/// arXiv:1404.3713 (α-basis definitions); Hu & Sawicki 2007 for f(R). |f_R| ≪ 1 for viable models,
/// so to leading order α_M ≈ a f_R'.
///
/// Inputs: `f_R` (= df/dR, value of the scalaron today) and `a_f_R_prime` (= a · df_R/da). The
/// closed form is α_M = a_f_R_prime / (1 + f_R); 1 + f_R must be non-zero.
fn fr_alpha_m(c: &DerivedCertificate) -> Result<f64, String> {
    let f_r = c.required("f_R", "df/dR today")?;
    let a_f_r_prime = c.required("a_f_R_prime", "a·df_R/da")?;
    let denom = 1.0 + f_r;
    if denom == 0.0 || !denom.is_finite() {
        return Err(format!(
            "f(R) (1 + f_R) must be finite and non-zero, got {denom}"
        ));
    }
    Ok(a_f_r_prime / denom)
}

/// Coupled dark energy (coupled quintessence) fifth-force enhancement of the dark-matter Newton
/// constant:
///     G_eff / G = 1 + 2 β²,
/// where β is the dimensionless DE–DM coupling (baryons uncoupled). Reference: Amendola, Phys. Rev.
/// D 62, 043511 (2000), arXiv:astro-ph/9908023; Amendola, Phys. Rev. D 69, 103524 (2004),
/// arXiv:astro-ph/0311175. β → 0 recovers GR for the dark sector.
///
/// Inputs: `beta` (the coupling). The relation is even in β.
fn coupled_de_geff_over_g(c: &DerivedCertificate) -> Result<f64, String> {
    let beta = c.required("beta", "DE–DM coupling")?;
    if !beta.is_finite() {
        return Err(format!("coupled-DE β must be finite, got {beta}"));
    }
    Ok(1.0 + 2.0 * beta * beta)
}

/// Reduced-Hubble definition: H0 = 100 h (km/s/Mpc), with h the dimensionless reduced Hubble
/// parameter. This is a definition, not a fit. Input: `h` (must be finite, positive).
fn h0_from_h(c: &DerivedCertificate) -> Result<f64, String> {
    let h = c.required("h", "reduced Hubble parameter")?;
    if !h.is_finite() || h <= 0.0 {
        return Err(format!(
            "reduced Hubble h must be finite and positive, got {h}"
        ));
    }
    Ok(100.0 * h)
}

/// Flat-universe (FRW) energy-budget closure: Ω_Λ = 1 − Ω_m − Ω_r − Ω_k, i.e. the densities sum to
/// one for k = 0. Reference: any standard FRW cosmology text (Σ Ω_i = 1 for a spatially flat
/// universe). Inputs: `omega_m` (required); `omega_r`, `omega_k` (optional, default 0).
fn flat_universe_omega_lambda(c: &DerivedCertificate) -> Result<f64, String> {
    let omega_m = c.required("omega_m", "matter density parameter today")?;
    let omega_r = c.input("omega_r").unwrap_or(0.0);
    let omega_k = c.input("omega_k").unwrap_or(0.0);
    let lambda = 1.0 - omega_m - omega_r - omega_k;
    if !lambda.is_finite() {
        return Err("non-finite Ω_Λ from flat closure".into());
    }
    Ok(lambda)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cert(relation: &str, inputs: &[(&str, f64)], expected: f64, tol: f64) -> DerivedCertificate {
        DerivedCertificate {
            relation: relation.into(),
            inputs: inputs.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
            expected,
            tolerance: tol,
        }
    }

    #[test]
    fn ndgp_correct_value_verifies() {
        // β = 2 ⇒ G_eff/G = 1 + 1/6 = 1.16666...
        let c = cert("ndgp_geff_over_g", &[("beta", 2.0)], 1.0 + 1.0 / 6.0, 1e-9);
        assert!(c.verify(), "{:?}", c.check());
    }

    #[test]
    fn relation_rigor_weight_zeros_trivial_definitions() {
        // Definitions / closure algebra earn no rigor; real modified-gravity relations earn full.
        assert_eq!(relation_rigor_weight("h0_from_h"), 0.0);
        assert_eq!(relation_rigor_weight("flat_universe_omega_lambda"), 0.0);
        assert_eq!(relation_rigor_weight("ndgp_geff_over_g"), 1.0);
        assert_eq!(relation_rigor_weight("coupled_de_geff_over_g"), 1.0);
        // V5: the β(a=1) closed form is a genuine mechanism (full weight); the Planck μ0
        // parametrization is cited but near-definitional (partial weight).
        assert_eq!(relation_rigor_weight("ndgp_beta_from_omega_rc"), 1.0);
        assert_eq!(relation_rigor_weight("planck_mu0_geff"), 0.3);
        assert_eq!(relation_rigor_weight("not_a_relation"), 0.0);
    }

    #[test]
    fn relation_gr_value_known_for_modifications_only() {
        assert_eq!(relation_gr_value("ndgp_geff_over_g"), Some(1.0));
        assert_eq!(relation_gr_value("fr_alpha_m"), Some(0.0));
        // V5: GR at μ0 = 0 ⇒ G_eff/G = 1.
        assert_eq!(relation_gr_value("planck_mu0_geff"), Some(1.0));
        // V5: β's GR limit is β → ∞, not a finite value — no GR point to compare against.
        assert_eq!(relation_gr_value("ndgp_beta_from_omega_rc"), None);
        // Trivial definitions have no GR limit.
        assert_eq!(relation_gr_value("h0_from_h"), None);
        assert_eq!(relation_gr_value("flat_universe_omega_lambda"), None);
    }

    #[test]
    fn h0_from_h_verifies_and_catches_a_fit() {
        // h = 0.674 ⇒ H0 = 67.4 (definition).
        assert!(cert("h0_from_h", &[("h", 0.674)], 67.4, 1e-9).verify());
        // A claimed H0 inconsistent with h is a mismatch (a fitted number wearing a derived label).
        assert!(!cert("h0_from_h", &[("h", 0.674)], 73.0, 1e-3).verify());
    }

    #[test]
    fn flat_closure_verifies_and_optional_inputs_default_to_zero() {
        // Ω_m = 0.315 (flat, no radiation/curvature) ⇒ Ω_Λ = 0.685.
        assert!(cert(
            "flat_universe_omega_lambda",
            &[("omega_m", 0.315)],
            0.685,
            1e-9
        )
        .verify());
        // With explicit radiation: Ω_Λ = 1 − 0.315 − 0.0001 = 0.6849.
        let c = cert(
            "flat_universe_omega_lambda",
            &[("omega_m", 0.315), ("omega_r", 0.0001)],
            0.6849,
            1e-9,
        );
        assert!(c.verify(), "{:?}", c.check());
        // A non-closing budget (claimed Ω_Λ that does not sum to 1) is a mismatch.
        assert!(!cert(
            "flat_universe_omega_lambda",
            &[("omega_m", 0.315)],
            0.5,
            1e-3
        )
        .verify());
    }

    #[test]
    fn ndgp_wrong_value_fails() {
        // Claim G_eff/G = 1.0 (GR) but β = 2 gives 1.1666..., outside tolerance ⇒ mismatch.
        let c = cert("ndgp_geff_over_g", &[("beta", 2.0)], 1.0, 1e-3);
        assert!(!c.verify());
        assert!(matches!(
            c.check(),
            CertificateOutcome::ValueMismatch { .. }
        ));
    }

    #[test]
    fn ndgp_large_beta_approaches_gr() {
        // Large H r_c ⇒ β large ⇒ G_eff/G → 1.
        let c = cert("ndgp_geff_over_g", &[("beta", 1.0e6)], 1.0, 1e-5);
        assert!(c.verify(), "{:?}", c.check());
    }

    #[test]
    fn ndgp_zero_beta_is_invalid_input() {
        let c = cert("ndgp_geff_over_g", &[("beta", 0.0)], 1.0, 1e-3);
        assert!(matches!(
            c.check(),
            CertificateOutcome::MissingOrInvalidInput { .. }
        ));
    }

    #[test]
    fn ndgp_missing_input_fails() {
        let c = cert("ndgp_geff_over_g", &[("notbeta", 2.0)], 1.166, 1e-2);
        assert!(matches!(
            c.check(),
            CertificateOutcome::MissingOrInvalidInput { .. }
        ));
    }

    #[test]
    fn fr_largescale_inside_compton_is_four_thirds() {
        let c = cert(
            "fr_largescale_geff_over_g",
            &[("regime", 1.0)],
            4.0 / 3.0,
            1e-12,
        );
        assert!(c.verify(), "{:?}", c.check());
        let outside = cert("fr_largescale_geff_over_g", &[("regime", 0.0)], 1.0, 1e-12);
        assert!(outside.verify(), "{:?}", outside.check());
    }

    #[test]
    fn fr_largescale_wrong_value_fails() {
        // Inside the Compton wavelength is 4/3, not 1.0.
        let c = cert("fr_largescale_geff_over_g", &[("regime", 1.0)], 1.0, 1e-3);
        assert!(!c.verify());
    }

    #[test]
    fn fr_alpha_m_closed_form() {
        // f_R = -1e-4, a·f_R' = 2e-4 ⇒ α_M = 2e-4 / (1 - 1e-4) ≈ 2.0002e-4.
        let computed = 2.0e-4 / (1.0 - 1.0e-4);
        let c = cert(
            "fr_alpha_m",
            &[("f_R", -1.0e-4), ("a_f_R_prime", 2.0e-4)],
            computed,
            1e-12,
        );
        assert!(c.verify(), "{:?}", c.check());
    }

    #[test]
    fn coupled_de_correct_value_verifies() {
        // β = 0.1 ⇒ G_eff/G = 1 + 2·0.01 = 1.02.
        let c = cert("coupled_de_geff_over_g", &[("beta", 0.1)], 1.02, 1e-12);
        assert!(c.verify(), "{:?}", c.check());
    }

    #[test]
    fn coupled_de_wrong_value_fails() {
        let c = cert("coupled_de_geff_over_g", &[("beta", 0.1)], 1.5, 1e-3);
        assert!(!c.verify());
        assert!(matches!(
            c.check(),
            CertificateOutcome::ValueMismatch { .. }
        ));
    }

    #[test]
    fn unknown_relation_is_reported() {
        let c = cert("not_a_real_relation", &[("x", 1.0)], 1.0, 1e-3);
        assert!(matches!(c.check(), CertificateOutcome::UnknownRelation));
        assert!(!c.verify());
    }

    #[test]
    fn negative_or_nonfinite_tolerance_never_verifies() {
        let neg = cert("coupled_de_geff_over_g", &[("beta", 0.1)], 1.02, -1.0);
        assert!(matches!(neg.check(), CertificateOutcome::InvalidTolerance));
        let nan = cert("coupled_de_geff_over_g", &[("beta", 0.1)], 1.02, f64::NAN);
        assert!(matches!(nan.check(), CertificateOutcome::InvalidTolerance));
    }

    #[test]
    fn certificate_serde_round_trips() {
        let c = cert("ndgp_geff_over_g", &[("beta", 2.0)], 1.0 + 1.0 / 6.0, 1e-9);
        let json = serde_json::to_string(&c).expect("serialize");
        let back: DerivedCertificate = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c, back);
        assert!(back.verify());
    }

    #[test]
    fn registry_names_all_resolve() {
        for name in registered_relations() {
            assert!(
                relation_registry(name).is_some(),
                "registered relation {name} must resolve"
            );
        }
        // V5: both new relations are registered, and the list stays sorted.
        let names = registered_relations();
        assert!(names.contains(&"ndgp_beta_from_omega_rc"));
        assert!(names.contains(&"planck_mu0_geff"));
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "registered_relations must stay sorted");
    }

    /// exploit-class: specification — V8 Wave 0.1. The V6→V7 era shipped a relation
    /// (`dark_scattering_growth_drag`) that was registered and verifiable but missing from
    /// `relation_signature`, so proposer prompts/diagnostics silently lacked its input contract
    /// (and ~100 proposals died on input-key spelling). Every registered relation must expose a
    /// signature hint, forever.
    #[test]
    fn registered_relations_have_signatures() {
        for name in registered_relations() {
            assert!(
                relation_signature(name).is_some(),
                "registered relation {name} must have a relation_signature entry"
            );
        }
    }

    #[test]
    fn ndgp_beta_from_omega_rc_roundtrips_planck_background() {
        // Planck-ish flat background: Ω_m = 0.315, Ω_r = Ω_k = 0, w0 = −1 ⇒ Ω_de = 0.685,
        // D1 = −½·(3·0.315 + 3·0·0.685) = −0.4725, 1 + D1/3 = 0.8425. For β = 2 we need
        // √Ω_rc = 0.8425 ⇒ Ω_rc = 0.70980625; the relation must recompute β ≈ 2 within 1e-9.
        // (Cross-checked against NdgpParams::beta = 1 + (E/√Ω_rc)(1 + dlnE_dN/3) at E(a=1) = 1.)
        let c = cert(
            "ndgp_beta_from_omega_rc",
            &[("omega_rc", 0.70980625), ("omega_m", 0.315)],
            2.0,
            1e-9,
        );
        assert!(c.verify(), "{:?}", c.check());
        // The same inputs cannot certify a different β (a fitted β wearing a derived label).
        let wrong = cert(
            "ndgp_beta_from_omega_rc",
            &[("omega_rc", 0.70980625), ("omega_m", 0.315)],
            2.5,
            1e-3,
        );
        assert!(!wrong.verify());
        assert!(matches!(
            wrong.check(),
            CertificateOutcome::ValueMismatch { .. }
        ));
    }

    #[test]
    fn ndgp_beta_optional_inputs_default_and_w0_enters() {
        // Explicit defaults (Ω_r = Ω_k = 0, w0 = −1) must agree with the omitted-input form.
        let implicit = cert(
            "ndgp_beta_from_omega_rc",
            &[("omega_rc", 0.70980625), ("omega_m", 0.315)],
            2.0,
            1e-9,
        );
        let explicit = cert(
            "ndgp_beta_from_omega_rc",
            &[
                ("omega_rc", 0.70980625),
                ("omega_m", 0.315),
                ("omega_r", 0.0),
                ("omega_k", 0.0),
                ("w0", -1.0),
            ],
            2.0,
            1e-9,
        );
        assert!(implicit.verify(), "{:?}", implicit.check());
        assert!(explicit.verify(), "{:?}", explicit.check());
        // w0 ≠ −1 shifts D1: w0 = −0.9 ⇒ D1 = −½(0.945 + 0.3·0.685) = −0.57525,
        // β = 1 + (1 − 0.19175)/0.8425.
        let beta_w = 1.0 + (1.0 - 0.57525 / 3.0) / 0.8425;
        let c = cert(
            "ndgp_beta_from_omega_rc",
            &[("omega_rc", 0.70980625), ("omega_m", 0.315), ("w0", -0.9)],
            beta_w,
            1e-9,
        );
        assert!(c.verify(), "{:?}", c.check());
    }

    #[test]
    fn ndgp_beta_domain_errors() {
        // Ω_rc = 0 is the GR limit (β → ∞), not a finite closed form ⇒ domain error.
        let zero = cert(
            "ndgp_beta_from_omega_rc",
            &[("omega_rc", 0.0), ("omega_m", 0.315)],
            2.0,
            1e-3,
        );
        assert!(matches!(
            zero.check(),
            CertificateOutcome::MissingOrInvalidInput { .. }
        ));
        // Negative Ω_rc is unphysical.
        let neg = cert(
            "ndgp_beta_from_omega_rc",
            &[("omega_rc", -0.5), ("omega_m", 0.315)],
            2.0,
            1e-3,
        );
        assert!(matches!(
            neg.check(),
            CertificateOutcome::MissingOrInvalidInput { .. }
        ));
        // Ω_de < 0 (over-full budget) breaks the flat closure.
        let overfull = cert(
            "ndgp_beta_from_omega_rc",
            &[("omega_rc", 0.71), ("omega_m", 1.5)],
            2.0,
            1e-3,
        );
        assert!(matches!(
            overfull.check(),
            CertificateOutcome::MissingOrInvalidInput { .. }
        ));
        // Non-finite input never produces a closed form.
        let nan = cert(
            "ndgp_beta_from_omega_rc",
            &[("omega_rc", 0.71), ("omega_m", f64::NAN)],
            2.0,
            1e-3,
        );
        assert!(matches!(
            nan.check(),
            CertificateOutcome::MissingOrInvalidInput { .. }
        ));
    }

    #[test]
    fn planck_mu0_verifies_both_signs() {
        // Suppressed-growth direction (negative μ0 explicitly allowed): μ0 = −0.1 ⇒ G_eff/G = 0.9.
        let neg = cert("planck_mu0_geff", &[("mu0", -0.1)], 0.9, 1e-12);
        assert!(neg.verify(), "{:?}", neg.check());
        // Enhanced direction: μ0 = 0.2 ⇒ G_eff/G = 1.2.
        let pos = cert("planck_mu0_geff", &[("mu0", 0.2)], 1.2, 1e-12);
        assert!(pos.verify(), "{:?}", pos.check());
        // A claimed G_eff/G inconsistent with μ0 is a mismatch.
        assert!(!cert("planck_mu0_geff", &[("mu0", 0.2)], 1.0, 1e-3).verify());
    }

    #[test]
    fn planck_mu0_below_minus_one_is_domain_error() {
        // μ0 ≤ −1 ⇒ G_eff ≤ 0: outside the physical domain.
        let c = cert("planck_mu0_geff", &[("mu0", -1.2)], -0.2, 1e-3);
        assert!(matches!(
            c.check(),
            CertificateOutcome::MissingOrInvalidInput { .. }
        ));
        assert!(!c.verify());
    }
}
