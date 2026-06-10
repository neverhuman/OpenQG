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

pub mod adversary;
pub mod anchors;
pub mod assessment;
pub mod certificate;
pub mod claim_graph;
pub mod contenders;
pub mod evaluate;
pub mod evidence;
pub mod evolve;
pub mod fingerprint;
pub mod holdout;
pub mod league;
pub mod mutation;
pub mod obligation;
pub mod pareto;
pub mod proposal;
pub mod recombine_proof;
pub mod robustness;
pub mod scorecard;
pub mod sectors;
pub mod unification;
pub mod unification_data;
pub mod vetoes;

pub use adversary::{Adversary, Decoy, DecoyKind};
pub use anchors::{anchor_set, miscalibrated, Anchor, AnchorKind};
pub use assessment::{assess, CandidateAssessment};
pub use certificate::{
    registered_relations, relation_gr_value, relation_registry, relation_rigor_weight,
    relation_signature, CertificateOutcome, DerivedCertificate,
};
pub use claim_graph::{
    claim_graph_digest, Claim, ClaimGraph, ClaimId, ClaimKind, Sector, SharedParam,
    UnificationClaim,
};
pub use contenders::{
    decoy_contenders, decoy_false_positive_rate, human_contenders, score_contender, Contender,
    ContenderSuite, ExpectedVerdict, MapEvidenceStore,
};
pub use evaluate::{derivation_score, evaluate, Evaluation};
pub use evidence::{
    audit_bytes, EvidenceRef, EvidenceStore, EvidenceTier, MaterializedEvidenceAudit,
};
pub use evolve::{evolve, evolve_run, Cell, Champion, EvolutionResult, GenerationReport};
pub use fingerprint::claim_fingerprint;
pub use holdout::{alternating_holdout, held_out_evaluate, HeldOutScore};
pub use league::{fit_model, model_league, FitResult, FreeParam, LeagueRow, ModelClass};
pub use mutation::{flip_to_quintic_decoy, inject_free_parameter, mutate, recombine, Rng};
pub use obligation::{
    dimensional_consistency, falsifier, limit_recovers_gr, DerivationObligation,
    DerivationObligationKind, LimitWitness, NovelPredictionWitness, ObligationOutcome,
};
pub use pareto::{dominates, objectives, pareto_front, Objectives};
pub use proposal::{
    parse_proposal, proposal_into_theory, proposal_receipt, proposal_to_theory, ProposalReceipt,
    TheoryProposal,
};
pub use recombine_proof::{
    proof_summary, recombination_compatible, CompatCheck, CompatibilityProof,
};
pub use robustness::perturbation_robustness;
pub use scorecard::{
    score, scorecard_receipt, DataFitOutcome, RubricComponent, RubricV4, ScorecardReceipt,
    ScorecardV4,
};
pub use unification::{unification_report, DomainCheck, UnificationReport};
pub use vetoes::{
    adjudicate, is_adjudicated_out, obligation_vetoes, run_veto_cascade, run_veto_cascade_full,
    tensor_speed_excess_at, VetoReason,
};

use crate::cosmology::CosmologyParams;
use serde::{Deserialize, Serialize};

/// Where a parameter's value comes from. The engine trusts structure, not prose: only a
/// `Derived` parameter with a non-empty mechanism, or a `Fundamental` constant, is whitebox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// A fundamental constant fixed by a symmetry or first principle of the theory.
    Fundamental,
    /// Derived from a stated upstream mechanism/relation. The `mechanism` note must be non-empty
    /// and name the derivation (the text-level machine-check). `certificate` optionally binds the
    /// *value*: a [`DerivedCertificate`] that recomputes the number from cited closed-form inputs.
    /// A certificate that FAILS verification demotes the parameter to `Free` (killed) in the veto
    /// cascade; `None` keeps today's text-only behavior but is flagged "uncertified". `#[serde(default)]`
    /// keeps back-compat: pre-M1 JSON (no `certificate` key) deserializes with `certificate = None`.
    Derived {
        mechanism: String,
        #[serde(default)]
        certificate: Option<DerivedCertificate>,
    },
    /// A free fitting parameter — the gray-box trap. Always vetoed.
    Free,
}

impl Provenance {
    /// True when this provenance is acceptable for a whitebox theory.
    ///
    /// NOTE: this is the cheap text-level gate (non-empty mechanism). It does **not** run the
    /// value-level certificate oracle — that lives in the veto cascade ([`vetoes`]), which demotes a
    /// `Derived` parameter whose attached certificate fails verification. Keeping the certificate
    /// check out of `is_whitebox` preserves the existing semantics of every caller.
    pub fn is_whitebox(&self) -> bool {
        match self {
            Provenance::Fundamental => true,
            Provenance::Derived { mechanism, .. } => !mechanism.trim().is_empty(),
            Provenance::Free => false,
        }
    }

    /// Convenience constructor for a text-only (uncertified) derived provenance — equivalent to the
    /// pre-M1 `Provenance::Derived { mechanism }`. Use [`Self::derived_certified`] to bind a value.
    pub fn derived(mechanism: impl Into<String>) -> Self {
        Provenance::Derived {
            mechanism: mechanism.into(),
            certificate: None,
        }
    }

    /// Convenience constructor for a value-level certified derived provenance.
    pub fn derived_certified(
        mechanism: impl Into<String>,
        certificate: DerivedCertificate,
    ) -> Self {
        Provenance::Derived {
            mechanism: mechanism.into(),
            certificate: Some(certificate),
        }
    }
}

/// A named physical parameter with a meaning and a provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub symbol: String,
    pub value: f64,
    pub physical_meaning: String,
    pub provenance: Provenance,
}

/// Taxonomy of the *role* a numeric degree of freedom plays in a model, for class-aware scoring and
/// complexity penalties (M1 §5 / plan "Parameter registry + taxonomy"). This is the type + a helper
/// now; scoring wires it in a later milestone. The roles distinguish a genuinely *derived* constant
/// (no complexity cost: it is fixed by the theory) from a *physically free* fitted knob (the
/// gray-box cost the engine penalizes) and from book-keeping roles (nuisance, calibration) and the
/// adversary traps (a post-search choice, a forbidden fixture value).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterRole {
    /// A constant fixed by the theory's structure and recomputable from inputs (a verified
    /// `Provenance::Derived` certificate). Carries no complexity penalty.
    DerivedConstant,
    /// A value pinned to an external measurement/convention (e.g. a fixed CMB temperature). Fixed,
    /// not fitted; no complexity penalty.
    FixedExternal,
    /// A genuine physical free parameter that the optimizer varies (the gray-box cost). Penalized.
    PhysicalFree,
    /// A nuisance parameter marginalized over (e.g. a survey calibration offset). Penalized, but as
    /// a nuisance, not as a physics claim.
    Nuisance,
    /// A calibration/standardization parameter (e.g. a supernova light-curve standardization term).
    Calibration,
    /// A choice made *after* seeing the search data (a multiplicity/look-elsewhere hazard). Flagged.
    PostSearchChoice,
    /// A value matching a frozen calibration fixture/decoy it must never reproduce (an adversary
    /// trap: copying the fixture answer instead of deriving it). A hard red flag.
    ForbiddenFixtureValue,
}

impl ParameterRole {
    /// Whether this role contributes to the model's complexity penalty (i.e. it is an effective
    /// fitted degree of freedom rather than something fixed or derived). Derived constants and
    /// fixed-external values cost nothing; free/nuisance/calibration knobs do. The two adversary
    /// traps are counted as costed (they are illegitimate fitted freedom).
    pub fn counts_as_free_dof(&self) -> bool {
        match self {
            ParameterRole::DerivedConstant | ParameterRole::FixedExternal => false,
            ParameterRole::PhysicalFree
            | ParameterRole::Nuisance
            | ParameterRole::Calibration
            | ParameterRole::PostSearchChoice
            | ParameterRole::ForbiddenFixtureValue => true,
        }
    }
}

/// One building block of the action, carrying just enough structure for the cheap symbolic
/// vetoes: its mass dimension (a 4D Lagrangian density must be dimension 4) and the number of
/// uncontracted Lorentz indices (a scalar action term must have zero).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Term {
    pub name: String,
    pub mass_dimension: i32,
    pub free_lorentz_indices: u32,
}

/// Linear α-basis deviation functions (evaluated today). GR ⇒ all zero.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Numeric screening *recovery efficiency* in `[0, 1]`: the fraction by which the linear
    /// gravity modification is suppressed inside the screened region (the solar system). 1.0 ⇒
    /// gravity is fully restored to GR (PPN γ → 1, no residual fifth force); 0.0 ⇒ no screening
    /// at all. This is the machine-checkable replacement for a bare `screening` *string*: M3
    /// adjudication recomputes the residual PPN deviation `(γ−1)_pred = modification_scale ·
    /// (1 − recovery)` and tests it against the Cassini bound (Bertotti, Iess & Tortora 2003,
    /// Nature 425, 374: γ−1 = (2.1 ± 2.3)×10⁻⁵). `None` ⇒ no numeric recovery was supplied, so a
    /// theory that *claims* screening but does not quantify it fails adjudication. Additive,
    /// serde-default, so theories written before M3 deserialize unchanged.
    #[serde(default)]
    pub screening_recovery: Option<f64>,
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
            screening_recovery: None,
            background: CosmologyParams::planck_lcdm(),
        }
    }

    /// True if the theory modifies gravity on linear scales (non-GR α-functions).
    pub fn modifies_gravity(&self) -> bool {
        self.alpha.modification_scale() > 1e-6 || self.alpha.alpha_t.abs() > 1e-6
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;

    #[test]
    fn theory_serde_round_trips() {
        let t = Theory::baseline_lcdm();
        let json = serde_json::to_string(&t).expect("serialize");
        let back: Theory = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(t, back);
    }

    #[test]
    fn provenance_serializes_snake_case() {
        let json = serde_json::to_string(&Provenance::Fundamental).unwrap();
        assert_eq!(json, "\"fundamental\"");
        let d = serde_json::to_string(&Provenance::Derived {
            mechanism: "m".into(),
            certificate: None,
        })
        .unwrap();
        assert!(d.contains("derived"));
    }

    #[test]
    fn legacy_derived_json_without_certificate_round_trips() {
        // Back-compat: pre-M1 JSON has no `certificate` key. `#[serde(default)]` fills it with None.
        let legacy = r#"{"derived":{"mechanism":"conformal coupling"}}"#;
        let p: Provenance = serde_json::from_str(legacy).expect("legacy derived deserializes");
        assert_eq!(p, Provenance::derived("conformal coupling"));
        assert!(p.is_whitebox());
    }

    #[test]
    fn certified_derived_round_trips() {
        let cert = crate::theory::DerivedCertificate {
            relation: "coupled_de_geff_over_g".into(),
            inputs: vec![("beta".into(), 0.1)],
            expected: 1.02,
            tolerance: 1e-9,
        };
        let p = Provenance::derived_certified("coupled-DE fifth force", cert.clone());
        let json = serde_json::to_string(&p).unwrap();
        let back: Provenance = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
        match back {
            Provenance::Derived {
                certificate: Some(c),
                ..
            } => assert!(c.verify()),
            _ => panic!("expected certified derived"),
        }
    }

    #[test]
    fn parameter_role_complexity_accounting() {
        assert!(!ParameterRole::DerivedConstant.counts_as_free_dof());
        assert!(!ParameterRole::FixedExternal.counts_as_free_dof());
        assert!(ParameterRole::PhysicalFree.counts_as_free_dof());
        assert!(ParameterRole::Nuisance.counts_as_free_dof());
        assert!(ParameterRole::Calibration.counts_as_free_dof());
        assert!(ParameterRole::PostSearchChoice.counts_as_free_dof());
        assert!(ParameterRole::ForbiddenFixtureValue.counts_as_free_dof());
        // serde uses snake_case (matches the registry-role strings in the plan).
        let j = serde_json::to_string(&ParameterRole::DerivedConstant).unwrap();
        assert_eq!(j, "\"derived_constant\"");
    }
}
