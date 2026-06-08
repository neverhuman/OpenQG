//! Symbolic theory representation for the adversarial-robustness engine.
//!
//! A candidate is no longer a bag of fitted numbers: it is a structured physical theory whose
//! parameters each carry a machine-checkable *provenance* (a free fitting knob is a hard kill,
//! not a matter of text), expressed in the linear α-basis (Bellini & Sawicki 2014) on top of a
//! Horndeski scalar-tensor action, with the structural metadata the deterministic veto cascade
//! ([`vetoes`]) needs: term mass-dimensions and free Lorentz indices, the scalar-sector
//! stability coefficients, the tensor-speed excess constrained by GW170817, and any declared
//! screening mechanism that lets the model pass solar-system (PPN) tests.
//!
//! This is the true, structural enforcement of the repo's "whitebox theories only" rule and the
//! "derived, not fit" preference — see `docs/research/automated-theory-discovery.md` §2/§4.

pub mod evaluate;
pub mod mutation;
pub mod unification;
pub mod vetoes;

pub use evaluate::{derivation_score, evaluate, Evaluation};
pub use mutation::{flip_to_quintic_decoy, inject_free_parameter, mutate, recombine, Rng};
pub use unification::{unification_report, DomainCheck, UnificationReport};
pub use vetoes::{run_veto_cascade, VetoReason};

use crate::cosmology::CosmologyParams;

/// Where a parameter's value comes from. The engine trusts structure, not prose: only a
/// `Derived` parameter with a non-empty mechanism, or a `Fundamental` constant, is whitebox.
#[derive(Debug, Clone, PartialEq)]
pub enum Provenance {
    /// A fundamental constant fixed by a symmetry or first principle of the theory.
    Fundamental,
    /// Derived from a stated upstream mechanism/relation (the note must be non-empty and name
    /// the derivation — this is the machine-checkable replacement for self-asserted text).
    Derived { mechanism: String },
    /// A free fitting parameter — the gray-box trap. Always vetoed.
    Free,
}

impl Provenance {
    /// True when this provenance is acceptable for a whitebox theory.
    pub fn is_whitebox(&self) -> bool {
        match self {
            Provenance::Fundamental => true,
            Provenance::Derived { mechanism } => !mechanism.trim().is_empty(),
            Provenance::Free => false,
        }
    }
}

/// A named physical parameter with a meaning and a provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub symbol: String,
    pub value: f64,
    pub physical_meaning: String,
    pub provenance: Provenance,
}

/// One building block of the action, carrying just enough structure for the cheap symbolic
/// vetoes: its mass dimension (a 4D Lagrangian density must be dimension 4) and the number of
/// uncontracted Lorentz indices (a scalar action term must have zero).
#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub name: String,
    pub mass_dimension: i32,
    pub free_lorentz_indices: u32,
}

/// Linear α-basis deviation functions (evaluated today). GR ⇒ all zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlphaBasis {
    /// Planck-mass run rate α_M (modifies lensing + GW friction).
    pub alpha_m: f64,
    /// Braiding α_B (scalar–metric kinetic mixing → DE clustering, growth).
    pub alpha_b: f64,
    /// Kineticity α_K (scalar kinetic energy).
    pub alpha_k: f64,
    /// Tensor speed excess α_T, with c_GW² = c²(1 + α_T). GW170817 ⇒ α_T ≈ 0.
    pub alpha_t: f64,
}

impl AlphaBasis {
    /// The GR point: no deviation.
    pub fn gr() -> Self {
        AlphaBasis {
            alpha_m: 0.0,
            alpha_b: 0.0,
            alpha_k: 0.0,
            alpha_t: 0.0,
        }
    }

    /// Largest gravity-modifying deviation magnitude (α_T excluded — it has its own veto).
    pub fn modification_scale(&self) -> f64 {
        self.alpha_m
            .abs()
            .max(self.alpha_b.abs())
            .max(self.alpha_k.abs())
    }
}

/// Scalar-sector linear-stability coefficients. No-ghost ⇒ kinetic term and Q_s positive;
/// no gradient instability ⇒ sound speed squared non-negative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stability {
    /// Sign/coefficient of the scalar kinetic term (must be > 0: wrong sign ⇒ ghost).
    pub kinetic_coefficient: f64,
    /// No-ghost determinant Q_s (must be > 0).
    pub q_s: f64,
    /// Scalar sound speed squared c_s² (must be ≥ 0: negative ⇒ gradient instability).
    pub sound_speed_sq: f64,
    /// True if the action contains non-degenerate higher-than-first time derivatives of a
    /// field (Ostrogradsky ⇒ ghost). Horndeski/α-basis theories are degenerate ⇒ false.
    pub has_nondegenerate_higher_derivatives: bool,
}

impl Stability {
    /// A healthy GR-like scalar sector.
    pub fn healthy() -> Self {
        Stability {
            kinetic_coefficient: 1.0,
            q_s: 1.0,
            sound_speed_sq: 1.0,
            has_nondegenerate_higher_derivatives: false,
        }
    }
}

/// A full candidate theory.
#[derive(Debug, Clone, PartialEq)]
pub struct Theory {
    pub id: String,
    /// Named physical parameters with provenance.
    pub parameters: Vec<Parameter>,
    /// Action building blocks (for dimensional + Lorentz structural checks).
    pub terms: Vec<Term>,
    /// Linear α-basis deviations from GR.
    pub alpha: AlphaBasis,
    /// Scalar-sector stability coefficients.
    pub stability: Stability,
    /// Declared screening mechanism that recovers GR at solar-system densities (chameleon,
    /// Vainshtein, symmetron, k-mouflage…). `None` ⇒ none declared.
    pub screening: Option<String>,
    /// Background cosmology this theory predicts (drives the forward model).
    pub background: CosmologyParams,
}

impl Theory {
    /// The GR + ΛCDM baseline: a valid whitebox theory that passes every veto and sits exactly
    /// at the GR point. Used as the reference anchor the adversary must never kill.
    pub fn baseline_lcdm() -> Self {
        Theory {
            id: "gr-lcdm-baseline".into(),
            parameters: vec![
                Parameter {
                    symbol: "H0".into(),
                    value: 67.4,
                    physical_meaning: "present-day expansion rate".into(),
                    provenance: Provenance::Fundamental,
                },
                Parameter {
                    symbol: "Omega_m".into(),
                    value: 0.315,
                    physical_meaning: "total matter density today".into(),
                    provenance: Provenance::Fundamental,
                },
            ],
            terms: vec![
                Term {
                    name: "einstein_hilbert".into(),
                    mass_dimension: 4,
                    free_lorentz_indices: 0,
                },
                Term {
                    name: "cosmological_constant".into(),
                    mass_dimension: 4,
                    free_lorentz_indices: 0,
                },
            ],
            alpha: AlphaBasis::gr(),
            stability: Stability::healthy(),
            screening: None,
            background: CosmologyParams::planck_lcdm(),
        }
    }

    /// True if the theory modifies gravity on linear scales (non-GR α-functions).
    pub fn modifies_gravity(&self) -> bool {
        self.alpha.modification_scale() > 1e-6 || self.alpha.alpha_t.abs() > 1e-6
    }
}
