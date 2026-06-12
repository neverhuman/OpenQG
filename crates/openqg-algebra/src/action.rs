//! Action-level IR for scalar-tensor theories.
//!
//! Every parameter claimed as `Derived` in a whitebox theory must be the image of a generating
//! action under the algebra compiler. Bare relations without an action source are
//! `StructurallyUngenerated` and earn no mechanism-novelty or derivation-rigor credit.
//!
//! The IR covers the Horndeski class plus the DGP brane extension (kept for continuity with
//! the V7 scoring lane). Beyond-Horndeski / DHOST and non-scalar-tensor theories are rejected
//! with `UnsupportedActionClass` until algebraic degeneracy certificates exist.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Semantic version string for the algebra IR schema.
pub const SCHEMA_VERSION: &str = "v8.0.0";

/// The complete action-level specification of a scalar-tensor theory.
///
/// This is the canonical input to the algebra compiler. All MG parameters must be the output of
/// compiling this struct, not proposer-asserted floats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlgebraTheory {
    /// Schema version for forward-compatibility checks.
    pub schema_version: String,
    /// The scalar fields declared in this theory.
    pub fields: Vec<FieldDecl>,
    /// Gravity sector: GR + optional conformal factor.
    pub gravity: GravitySector,
    /// Horndeski action terms (G2–G5 and potential).
    pub scalar_terms: Vec<ScalarTensorTerm>,
    /// Brane extensions (DGP, Galileon decoupling limit).
    pub brane_terms: Vec<BraneTerm>,
    /// Non-minimal matter couplings beyond minimal GR.
    pub matter_couplings: Vec<MatterCoupling>,
    /// SHA-256 fingerprint of the canonical JSON representation (self-computed via `fingerprint()`).
    #[serde(default)]
    pub action_fingerprint: String,
}

impl AlgebraTheory {
    /// Compute and return the SHA-256 fingerprint of the canonical (sorted, fingerprint-zeroed)
    /// JSON representation. Also sets `self.action_fingerprint`.
    pub fn compute_fingerprint(&mut self) -> &str {
        let mut copy = self.clone();
        copy.action_fingerprint = String::new();
        let json = serde_json::to_string(&copy).unwrap_or_default();
        let hash = Sha256::digest(json.as_bytes());
        self.action_fingerprint = format!("sha256:{hash:x}");
        &self.action_fingerprint
    }

    /// Return true when this theory includes a normal-DGP brane extension.
    pub fn has_ndgp_brane(&self) -> bool {
        self.brane_terms
            .iter()
            .any(|t| matches!(t, BraneTerm::NormalDgp { .. }))
    }

    /// Return true when this theory includes a Hu-Sawicki f(R) term.
    pub fn has_hu_sawicki_fr(&self) -> bool {
        self.scalar_terms
            .iter()
            .any(|t| matches!(t, ScalarTensorTerm::HuSawickiFR { .. }))
    }

    /// Return true when any Horndeski G4 term is present (f(R) enters via G4).
    pub fn has_g4_term(&self) -> bool {
        self.scalar_terms
            .iter()
            .any(|t| matches!(t, ScalarTensorTerm::HorndeskiG4 { .. }))
    }

    /// Return the NormalDgp brane term if present.
    pub fn ndgp_brane(&self) -> Option<&BraneTerm> {
        self.brane_terms
            .iter()
            .find(|t| matches!(t, BraneTerm::NormalDgp { .. }))
    }
}

/// A scalar field declared as a degree of freedom in the action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDecl {
    /// Symbol (e.g. "phi", "chi").
    pub symbol: String,
    /// Whether this field is the dominant dark-energy scalar.
    pub is_de_scalar: bool,
    /// Species labels for non-universal couplings.
    pub coupled_species: Vec<String>,
}

/// The gravity sector: pure GR (M_pl^2/2 * R) or with a running Planck mass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GravitySector {
    /// Pure Einstein-Hilbert: S_g = M_pl^2/2 * ∫ d^4x sqrt(-g) R.
    EinsteinHilbert,
    /// Running Planck mass: S_g = M*^2(phi)/2 * ∫ d^4x sqrt(-g) R (Brans-Dicke / scalar-tensor).
    RunningPlanckMass { mstar2_basis: FunctionBasis },
}

/// Horndeski action terms, plus the Hu-Sawicki f(R) shortcut.
///
/// The Horndeski Lagrangian density is (Bellini & Sawicki, JCAP 2014):
/// `L = G2 - G3 Box(phi) + G4 R + G4_X[...] + G5 G_mu_nu phi^;mu_nu - G5_X[...]/6`
///
/// where Gi = Gi(phi, X), X = -1/2 (∂phi)^2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScalarTensorTerm {
    /// G2(phi, X) — kinetic and potential mixing term. Quintessence: G2 = X - V(phi).
    HorndeskiG2 { basis: FunctionBasis },
    /// G3(phi, X) — kinetic braiding / Galileon term. Cubic Galileon: G3 = c3.
    HorndeskiG3 { basis: FunctionBasis },
    /// G4(phi, X) — non-minimal coupling to R. f(R) enters as G4 = f(phi)/2 with phi=R.
    HorndeskiG4 { basis: FunctionBasis },
    /// G5(phi, X) — non-minimal coupling to G_mu_nu phi^;mu_nu.
    HorndeskiG5 { basis: FunctionBasis },
    /// Standalone scalar potential V(phi) (equivalent to G2 = X - V with G2_phi ignored).
    Potential { basis: FunctionBasis },
    /// Hu-Sawicki f(R) gravity shortcut: f(R) = -m^2 c1(R/m^2)^n / [c2(R/m^2)^n + 1].
    /// This is structurally equivalent to a G4 term but is given its own constructor for
    /// theorem-catalog lookup and compiler short-circuit.
    HuSawickiFR {
        /// Hu-Sawicki shape index n (typically n=1).
        n: f64,
        /// Background value f_R0 = df/dR|_{R=R0}.
        log10_fr0: f64,
    },
    /// Quintessence shortcut: G2 = X - V, all other Gi = 0, G4 = M_pl^2/2.
    Quintessence { potential_basis: FunctionBasis },
    /// K-essence shortcut: G2 = K(X) - V(phi), G4 = M_pl^2/2.
    KEssence { k_basis: FunctionBasis },
}

/// Brane extension terms beyond the 4D scalar-tensor action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BraneTerm {
    /// Normal-branch DGP: adds the 5D Gibbons-Hawking term 1/(2 r_c) ∫ d^4x sqrt(-g) K.
    /// `omega_rc = H0^2 r_c^2 / 4` is the observational parameterization.
    NormalDgp { omega_rc: f64 },
}

/// Non-minimal matter couplings beyond minimal GR.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MatterCoupling {
    /// Conformal coupling: species i sees an effective metric A_i^2(phi) g_mu_nu.
    Conformal {
        species: String,
        a_of_phi: FunctionBasis,
    },
    /// Disformal coupling: species sees C(phi,X) g_mu_nu + D(phi,X) ∂_mu phi ∂_nu phi.
    Disformal {
        species: String,
        c_basis: FunctionBasis,
        d_basis: FunctionBasis,
    },
    /// Momentum-exchange / dark-scattering drag between dark matter and dark energy.
    DarkScatteringDrag {
        /// Species receiving the drag (typically dark matter).
        species: String,
        /// Drag amplitude parameter a_drag.
        a_drag: f64,
    },
}

/// A basis for an action function G_i(phi, X) or V(phi).
///
/// All proposer-asserted alpha splines default to `Phenomenological` and do not earn
/// derivation-rigor credit unless an `inverse_witness` action exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FunctionBasis {
    /// Monomial expansion: sum_ij c_ij phi^i X^j.
    PolynomialPhiX { terms: Vec<PhiXMonomial> },
    /// Exponential basis: sum_i A_i exp(lambda_i phi / M_pl).
    ExponentialPhi {
        amplitudes: Vec<f64>,
        slopes: Vec<f64>,
    },
    /// Constant: G_i = constant (e.g. G4 = M_pl^2/2 for GR).
    Constant { value: f64 },
    /// Phenomenological: proposer-asserted functional form without a derivation witness.
    /// Earns no mechanism-novelty or derivation-rigor credit.
    Phenomenological,
}

/// A single monomial c * phi^p * X^q * M^r in the polynomial expansion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhiXMonomial {
    /// Coefficient (may carry a mass-scale power via `mass_power`).
    pub coefficient: f64,
    /// Power of phi.
    pub phi_power: u8,
    /// Power of X = -(∂phi)^2/2.
    pub x_power: u8,
    /// Power of reference mass scale M (for dimensional completion in natural units).
    pub mass_power: i8,
}
