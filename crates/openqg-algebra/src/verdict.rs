//! Structural generation verdict: is a relation's generating action present?
//!
//! The core gate of #13 (term algebra): a `Derived` parameter claiming relation X must have
//! a corresponding `AlgebraReceipt` naming an action term that generates X, or it is
//! `StructurallyUngenerated`. `PhenomenologyOnly` relations (planck_mu0_geff, h0_from_h,
//! flat_universe_omega_lambda) are explicitly permitted without an action — they just earn
//! no derivation-rigor or mechanism-novelty credit.

use crate::action::AlgebraTheory;
use crate::catalog::{relation_action_term, relation_is_mechanism, ActionTermKind};
use serde::{Deserialize, Serialize};

/// Outcome of the structural generation check for one relation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum StructureVerdict {
    /// The relation has a generating action in the algebra theory and the action contains the
    /// required term (e.g. `BraneTerm::NormalDgp` for `ndgp_geff_over_g`).
    StructurallyGenerated {
        /// SHA-256 fingerprint of the `AlgebraTheory` that generates this relation.
        action_fingerprint: String,
        /// Which action term kind generates this relation.
        action_term_kind: String,
    },
    /// The relation is a mechanism relation but no generating action is present.
    /// This is a veto-eligible finding: a derived parameter cannot earn mechanism credit
    /// without a structural generator.
    StructurallyUngenerated { reason: String },
    /// The relation is explicitly phenomenological (or a background unit theorem) and earns
    /// no mechanism credit by design. This is NOT an error — it is the correct state for
    /// `planck_mu0_geff`, `flat_universe_omega_lambda`, and `h0_from_h`.
    PhenomenologyOnly { note: String },
    /// The relation is not in the theorem catalog at all — caller should treat as unregistered.
    UnknownRelation,
}

impl StructureVerdict {
    /// True when this verdict allows the relation to proceed without a veto.
    pub fn is_acceptable(&self) -> bool {
        matches!(
            self,
            StructureVerdict::StructurallyGenerated { .. }
                | StructureVerdict::PhenomenologyOnly { .. }
        )
    }

    /// True when this relation carries mechanism-level derivation rigor.
    pub fn is_mechanism_generated(&self) -> bool {
        matches!(self, StructureVerdict::StructurallyGenerated { .. })
    }
}

/// Check whether `algebra_theory` structurally generates `relation`.
///
/// Returns `StructureVerdict::StructurallyGenerated` when the algebra contains the action term
/// that the theorem catalog says produces the relation.
/// Returns `StructureVerdict::StructurallyUngenerated` when the relation is a mechanism
/// relation and no matching action term is present.
/// Returns `StructureVerdict::PhenomenologyOnly` for non-mechanism relations.
/// Returns `StructureVerdict::UnknownRelation` for relations not in the catalog.
pub fn check_structural_generation(
    relation: &str,
    algebra: Option<&AlgebraTheory>,
) -> StructureVerdict {
    let action_term = match relation_action_term(relation) {
        None => return StructureVerdict::UnknownRelation,
        Some(t) => t,
    };

    match &action_term {
        ActionTermKind::BackgroundConstraint { constraint } => {
            return StructureVerdict::PhenomenologyOnly {
                note: constraint.clone(),
            };
        }
        ActionTermKind::PhenomenologyOnly { note } => {
            return StructureVerdict::PhenomenologyOnly { note: note.clone() };
        }
        _ => {}
    }

    // Mechanism relation — needs a generating action.
    let theory = match algebra {
        None => {
            return StructureVerdict::StructurallyUngenerated {
                reason: format!(
                    "relation '{relation}' requires a generating action \
                     but no AlgebraTheory was provided; \
                     supply an AlgebraTheory with the appropriate action term"
                ),
            };
        }
        Some(t) => t,
    };

    // Check whether the correct action term is present.
    let present = action_term_present(theory, &action_term);
    if present {
        StructureVerdict::StructurallyGenerated {
            action_fingerprint: theory.action_fingerprint.clone(),
            action_term_kind: action_term_label(&action_term),
        }
    } else {
        StructureVerdict::StructurallyUngenerated {
            reason: format!(
                "relation '{relation}' requires {} but the AlgebraTheory \
                 does not contain the required action term",
                action_term_label(&action_term)
            ),
        }
    }
}

/// Check whether the algebra theory contains the action term required by an `ActionTermKind`.
fn action_term_present(theory: &AlgebraTheory, required: &ActionTermKind) -> bool {
    use crate::action::{BraneTerm, ScalarTensorTerm};
    match required {
        ActionTermKind::NormalDgpBrane { .. } => theory.has_ndgp_brane(),
        ActionTermKind::HuSawickiFR { .. } => theory.has_hu_sawicki_fr(),
        // HuSawickiFR is a specialization of G4 — either satisfies a G4-based theorem.
        ActionTermKind::HorndeskiG4 { .. } => theory.has_g4_term() || theory.has_hu_sawicki_fr(),
        ActionTermKind::ConformedDarkMatterCoupling { .. } => theory
            .matter_couplings
            .iter()
            .any(|c| matches!(c, crate::action::MatterCoupling::Conformal { .. })),
        ActionTermKind::DarkScatteringDrag { .. } => theory
            .matter_couplings
            .iter()
            .any(|c| matches!(c, crate::action::MatterCoupling::DarkScatteringDrag { .. })),
        ActionTermKind::BackgroundConstraint { .. } | ActionTermKind::PhenomenologyOnly { .. } => {
            true
        } // always acceptable
    }
}

fn action_term_label(kind: &ActionTermKind) -> String {
    match kind {
        ActionTermKind::NormalDgpBrane { computes, .. } => {
            format!("BraneTerm::NormalDgp (computes: {computes})")
        }
        ActionTermKind::HuSawickiFR { computes, .. } => {
            format!("ScalarTensorTerm::HuSawickiFR (computes: {computes})")
        }
        ActionTermKind::HorndeskiG4 { computes, .. } => {
            format!("ScalarTensorTerm::HorndeskiG4 (computes: {computes})")
        }
        ActionTermKind::ConformedDarkMatterCoupling { computes, .. } => {
            format!("MatterCoupling::Conformal (computes: {computes})")
        }
        ActionTermKind::DarkScatteringDrag { computes, .. } => {
            format!("MatterCoupling::DarkScatteringDrag (computes: {computes})")
        }
        ActionTermKind::BackgroundConstraint { .. } => "BackgroundConstraint".into(),
        ActionTermKind::PhenomenologyOnly { .. } => "PhenomenologyOnly".into(),
    }
}

/// Check all mechanism relations claimed by a theory against its algebra.
///
/// Returns a list of `(relation, StructureVerdict)` for every relation that is a mechanism
/// (not background / phenomonly). Callers should veto if any verdict is `StructurallyUngenerated`.
pub fn audit_structural_generation(
    claimed_relations: &[&str],
    algebra: Option<&AlgebraTheory>,
) -> Vec<(String, StructureVerdict)> {
    claimed_relations
        .iter()
        .filter(|r| relation_is_mechanism(r))
        .map(|r| (r.to_string(), check_structural_generation(r, algebra)))
        .collect()
}
