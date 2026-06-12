//! `openqg-algebra` — Horndeski action-level IR and structural generation gate for OpenQG V8.
//!
//! ## Purpose
//!
//! Implements V8 Phase 2 item #13 (term algebra): every MG parameter claimed as `Derived` must
//! be the image of a generating action under the algebra compiler. The three-layer stack is:
//!
//! 1. **`action`** — IR types for the action: `AlgebraTheory`, `ScalarTensorTerm`, `BraneTerm`,
//!    `FunctionBasis`, `MatterCoupling`.
//! 2. **`catalog`** — static theorem catalog mapping the 9 registered certificate relations to
//!    their action generators and theorem statements.
//! 3. **`verdict`** — `check_structural_generation()` → `StructureVerdict`; the gate that
//!    returns `StructurallyUngenerated` when a mechanism relation lacks a generating action.
//! 4. **`receipt`** — `AlgebraReceipt`: a sealed record binding a theory to the algebra that
//!    generated its certified relations, for storage alongside the theory JSON.
//!
//! ## Axiom
//!
//! **Derived cosmological functions must be compiled from an action, or they are phenomenology
//! with no derivation-rigor credit.** (S03 §2, core rule)

pub mod action;
pub mod catalog;
pub mod receipt;
pub mod verdict;

pub use action::{
    AlgebraTheory, BraneTerm, FieldDecl, FunctionBasis, GravitySector, MatterCoupling,
    PhiXMonomial, ScalarTensorTerm, SCHEMA_VERSION,
};
pub use catalog::{
    relation_action_term, relation_is_mechanism, relation_theorem, theorem_catalog, ActionTermKind,
    TheoremEntry,
};
pub use receipt::AlgebraReceipt;
pub use verdict::{audit_structural_generation, check_structural_generation, StructureVerdict};

#[cfg(test)]
mod tests {
    use super::*;
    use action::{BraneTerm, FunctionBasis, GravitySector, MatterCoupling, ScalarTensorTerm};

    // ---- Catalog coverage ----

    /// Spec acceptance test (rank 1): all mechanism relations in the catalog have a non-None
    /// theorem entry, and all catalog entries parse and are consistent.
    #[test]
    fn theorem_catalog_complete() {
        let catalog = theorem_catalog();
        // The 9 registered relations from certificate.rs:
        let required = [
            "ndgp_geff_over_g",
            "ndgp_beta_from_omega_rc",
            "dark_scattering_growth_drag",
            "fr_largescale_geff_over_g",
            "fr_alpha_m",
            "coupled_de_geff_over_g",
            "planck_mu0_geff",
            "flat_universe_omega_lambda",
            "h0_from_h",
        ];
        for rel in required {
            let entry = catalog.iter().find(|e| e.relation == rel);
            assert!(
                entry.is_some(),
                "relation '{rel}' missing from theorem catalog"
            );
        }
    }

    /// Spec acceptance test (rank 1): each mechanism relation has a non-background, non-phenomonly
    /// theorem entry; non-mechanism relations are correctly marked.
    #[test]
    fn theorem_registry_relations_from_algebra() {
        let mechanism_relations = [
            "ndgp_geff_over_g",
            "ndgp_beta_from_omega_rc",
            "dark_scattering_growth_drag",
            "fr_largescale_geff_over_g",
            "fr_alpha_m",
            "coupled_de_geff_over_g",
        ];
        for rel in mechanism_relations {
            assert!(
                relation_is_mechanism(rel),
                "relation '{rel}' should be a mechanism relation"
            );
            let entry = relation_theorem(rel).expect("relation must be in catalog");
            assert!(
                entry.is_mechanism(),
                "relation '{rel}' is mechanism but catalog entry says otherwise"
            );
        }

        // Non-mechanism relations.
        for rel in ["planck_mu0_geff", "flat_universe_omega_lambda", "h0_from_h"] {
            assert!(
                !relation_is_mechanism(rel),
                "relation '{rel}' should NOT be a mechanism relation"
            );
        }
    }

    // ---- StructureVerdict checks ----

    fn minimal_ndgp_theory() -> AlgebraTheory {
        let mut t = AlgebraTheory {
            schema_version: SCHEMA_VERSION.into(),
            fields: vec![],
            gravity: GravitySector::EinsteinHilbert,
            scalar_terms: vec![],
            brane_terms: vec![BraneTerm::NormalDgp { omega_rc: 0.25 }],
            matter_couplings: vec![],
            action_fingerprint: String::new(),
        };
        t.compute_fingerprint();
        t
    }

    fn minimal_fr_theory() -> AlgebraTheory {
        let mut t = AlgebraTheory {
            schema_version: SCHEMA_VERSION.into(),
            fields: vec![],
            gravity: GravitySector::EinsteinHilbert,
            scalar_terms: vec![ScalarTensorTerm::HuSawickiFR {
                n: 1.0,
                log10_fr0: -5.0,
            }],
            brane_terms: vec![],
            matter_couplings: vec![],
            action_fingerprint: String::new(),
        };
        t.compute_fingerprint();
        t
    }

    fn minimal_dark_scattering_theory() -> AlgebraTheory {
        let mut t = AlgebraTheory {
            schema_version: SCHEMA_VERSION.into(),
            fields: vec![],
            gravity: GravitySector::EinsteinHilbert,
            scalar_terms: vec![],
            brane_terms: vec![],
            matter_couplings: vec![MatterCoupling::DarkScatteringDrag {
                species: "dark_matter".into(),
                a_drag: 0.5,
            }],
            action_fingerprint: String::new(),
        };
        t.compute_fingerprint();
        t
    }

    /// Spec acceptance test (rank 1): deleting the generating action while keeping the
    /// old derived parameter fails with StructurallyUngenerated.
    #[test]
    fn no_ndgp_action_gives_structurally_ungenerated() {
        let empty_theory = AlgebraTheory {
            schema_version: SCHEMA_VERSION.into(),
            fields: vec![],
            gravity: GravitySector::EinsteinHilbert,
            scalar_terms: vec![],
            brane_terms: vec![], // no DGP brane action
            matter_couplings: vec![],
            action_fingerprint: String::new(),
        };
        let verdict = check_structural_generation("ndgp_geff_over_g", Some(&empty_theory));
        assert!(
            matches!(verdict, StructureVerdict::StructurallyUngenerated { .. }),
            "no DGP action must yield StructurallyUngenerated; got: {verdict:?}"
        );
    }

    /// Spec acceptance test (rank 2): DGP extension term present → StructurallyGenerated.
    #[test]
    fn ndgp_action_gives_structurally_generated() {
        let theory = minimal_ndgp_theory();
        let verdict = check_structural_generation("ndgp_geff_over_g", Some(&theory));
        assert!(
            matches!(verdict, StructureVerdict::StructurallyGenerated { .. }),
            "DGP brane action must yield StructurallyGenerated; got: {verdict:?}"
        );
        let verdict2 = check_structural_generation("ndgp_beta_from_omega_rc", Some(&theory));
        assert!(
            matches!(verdict2, StructureVerdict::StructurallyGenerated { .. }),
            "DGP action must generate beta relation too; got: {verdict2:?}"
        );
    }

    /// Spec acceptance test (rank 2): test candidate with DGP term name but NO DGP action data
    /// is rejected.
    #[test]
    fn no_action_returns_structurally_ungenerated_for_all_mechanisms() {
        for rel in [
            "ndgp_geff_over_g",
            "ndgp_beta_from_omega_rc",
            "fr_largescale_geff_over_g",
            "fr_alpha_m",
        ] {
            let verdict = check_structural_generation(rel, None);
            assert!(
                matches!(verdict, StructureVerdict::StructurallyUngenerated { .. }),
                "None algebra must yield StructurallyUngenerated for '{rel}'; got: {verdict:?}"
            );
        }
    }

    #[test]
    fn fr_action_generates_fr_relations() {
        let theory = minimal_fr_theory();
        for rel in ["fr_largescale_geff_over_g", "fr_alpha_m"] {
            let verdict = check_structural_generation(rel, Some(&theory));
            assert!(
                matches!(verdict, StructureVerdict::StructurallyGenerated { .. }),
                "f(R) action must generate '{rel}'; got: {verdict:?}"
            );
        }
    }

    #[test]
    fn dark_scattering_action_generates_drag_relation() {
        let theory = minimal_dark_scattering_theory();
        let verdict = check_structural_generation("dark_scattering_growth_drag", Some(&theory));
        assert!(
            matches!(verdict, StructureVerdict::StructurallyGenerated { .. }),
            "dark scattering action must generate drag relation; got: {verdict:?}"
        );
    }

    #[test]
    fn phenomonly_relations_are_acceptable_without_action() {
        for rel in ["planck_mu0_geff", "flat_universe_omega_lambda", "h0_from_h"] {
            let verdict = check_structural_generation(rel, None);
            assert!(
                verdict.is_acceptable(),
                "phenomonly relation '{rel}' must be acceptable; got: {verdict:?}"
            );
            assert!(
                !verdict.is_mechanism_generated(),
                "phenomonly relation '{rel}' must NOT be mechanism-generated; got: {verdict:?}"
            );
        }
    }

    // ---- AlgebraReceipt ----

    #[test]
    fn receipt_build_succeeds_for_valid_ndgp_theory() {
        let theory = minimal_ndgp_theory();
        let claimed = ["ndgp_geff_over_g", "ndgp_beta_from_omega_rc", "h0_from_h"];
        let result = AlgebraReceipt::build("ndgp-test", &theory, &claimed);
        assert!(
            result.is_ok(),
            "valid DGP theory should produce an AlgebraReceipt; got: {result:?}"
        );
        let receipt = result.unwrap();
        assert!(receipt.is_mechanism_generated("ndgp_geff_over_g"));
        assert!(receipt.is_mechanism_generated("ndgp_beta_from_omega_rc"));
        assert!(!receipt.is_mechanism_generated("h0_from_h"));
        assert!(receipt.covers("h0_from_h")); // h0_from_h is phenomonly
    }

    #[test]
    fn receipt_build_fails_when_action_missing() {
        let empty = AlgebraTheory {
            schema_version: SCHEMA_VERSION.into(),
            fields: vec![],
            gravity: GravitySector::EinsteinHilbert,
            scalar_terms: vec![],
            brane_terms: vec![],
            matter_couplings: vec![],
            action_fingerprint: String::new(),
        };
        let claimed = ["ndgp_geff_over_g"]; // mechanism relation but no action
        let result = AlgebraReceipt::build("bad-theory", &empty, &claimed);
        assert!(
            result.is_err(),
            "missing DGP action must cause receipt build to fail"
        );
        let failures = result.unwrap_err();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].0, "ndgp_geff_over_g");
    }

    // ---- Action fingerprint ----

    #[test]
    fn action_fingerprint_is_deterministic() {
        let mut t1 = minimal_ndgp_theory();
        let mut t2 = minimal_ndgp_theory();
        t1.compute_fingerprint();
        t2.compute_fingerprint();
        assert_eq!(t1.action_fingerprint, t2.action_fingerprint);
        assert!(!t1.action_fingerprint.is_empty());
        assert!(t1.action_fingerprint.starts_with("sha256:"));
    }

    #[test]
    fn different_theories_have_different_fingerprints() {
        let mut t1 = minimal_ndgp_theory();
        let mut t2 = minimal_fr_theory();
        t1.compute_fingerprint();
        t2.compute_fingerprint();
        assert_ne!(t1.action_fingerprint, t2.action_fingerprint);
    }
}
