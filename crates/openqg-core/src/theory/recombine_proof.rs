//! V4 M2 — recombination compatibility proof.
//!
//! Before the ZYAL `05-compatibility` stage allows two parent theories to be recombined into a
//! child, it demands a *typed, machine-checkable proof* that the two are physically and
//! dimensionally mergeable — not a vibe, a structured artifact whose every clause is a named
//! [`CompatCheck`]. A recombination is permitted iff every check passes; otherwise the proof
//! carries the human-readable `reasons` the merge was refused, and the stage blocks.
//!
//! The checks are deliberately conservative: when in doubt we *fail closed*, because a silently
//! incoherent recombination (two fundamental constants disagreeing, contradictory screening on the
//! same sector, opposite tensor-speed sectors) would corrupt the whole child lineage.

use serde::{Deserialize, Serialize};

/// One clause of the compatibility proof: a named predicate, whether it held, and a human-readable
/// detail (the explanation, and — when `passed == false` — the reason the merge is refused).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompatCheck {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

impl CompatCheck {
    fn pass(name: &str, detail: impl Into<String>) -> Self {
        CompatCheck {
            name: name.into(),
            passed: true,
            detail: detail.into(),
        }
    }

    fn fail(name: &str, detail: impl Into<String>) -> Self {
        CompatCheck {
            name: name.into(),
            passed: false,
            detail: detail.into(),
        }
    }
}

/// The assembled proof. `compatible` is true iff every check passed; `reasons` collects the
/// `detail` of every failed check (empty when compatible).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompatibilityProof {
    pub compatible: bool,
    pub checks: Vec<CompatCheck>,
    pub reasons: Vec<String>,
}

impl CompatibilityProof {
    /// Assemble a proof from its checks, deriving `compatible` and `reasons` so the two can never
    /// drift out of sync with `checks`.
    fn from_checks(checks: Vec<CompatCheck>) -> Self {
        let compatible = checks.iter().all(|c| c.passed);
        let reasons = checks
            .iter()
            .filter(|c| !c.passed)
            .map(|c| c.detail.clone())
            .collect();
        CompatibilityProof {
            compatible,
            checks,
            reasons,
        }
    }
}

/// True when this provenance names a value the theory claims to *fix* (not freely fit): a
/// fundamental constant or a certificate-backed derived constant. Used to detect a "knob vs fixed
/// constant" collision when the two parents disagree about the same symbol.
fn is_pinned_constant(p: &super::Provenance) -> bool {
    match p {
        super::Provenance::Fundamental => true,
        super::Provenance::Derived { certificate, .. } => certificate.is_some(),
        super::Provenance::Free => false,
    }
}

fn is_fundamental(p: &super::Provenance) -> bool {
    matches!(p, super::Provenance::Fundamental)
}

fn is_free(p: &super::Provenance) -> bool {
    matches!(p, super::Provenance::Free)
}

/// `"screening_mergeable"`: contradictory screening mechanisms on the same sector need a declared
/// bridge we do not have. If both parents declare screening and the strings differ, fail; otherwise
/// (at most one declares, or both declare the same) pass.
fn check_screening_mergeable(a: &super::Theory, b: &super::Theory) -> CompatCheck {
    const NAME: &str = "screening_mergeable";
    match (a.screening.as_deref(), b.screening.as_deref()) {
        (Some(sa), Some(sb)) if sa != sb => CompatCheck::fail(
            NAME,
            format!(
                "contradictory screening mechanisms on the same sector: \"{sa}\" vs \"{sb}\" \
                 — recombination needs a declared bridge, which is absent"
            ),
        ),
        (Some(sa), Some(_)) => CompatCheck::pass(
            NAME,
            format!("both declare the same screening mechanism \"{sa}\""),
        ),
        (Some(s), None) | (None, Some(s)) => CompatCheck::pass(
            NAME,
            format!("only one parent declares screening (\"{s}\"); mergeable"),
        ),
        (None, None) => CompatCheck::pass(NAME, "neither parent declares screening; mergeable"),
    }
}

/// `"parameter_provenance_consistent"`: for every symbol present in BOTH parents, a conflict is
/// either (a) the two values differ by > 1e-9 while at least one side is `Fundamental` (a
/// fundamental constant cannot take two values), or (b) one side is `Free` while the other pins the
/// symbol as `Fundamental` or certified-`Derived` (a fitting knob colliding with a fixed constant).
fn check_parameter_provenance_consistent(a: &super::Theory, b: &super::Theory) -> CompatCheck {
    const NAME: &str = "parameter_provenance_consistent";
    let mut offenders: Vec<String> = Vec::new();

    for pa in &a.parameters {
        for pb in &b.parameters {
            if pa.symbol != pb.symbol {
                continue;
            }
            let value_conflict = (pa.value - pb.value).abs() > 1e-9
                && (is_fundamental(&pa.provenance) || is_fundamental(&pb.provenance));
            let knob_vs_constant = (is_free(&pa.provenance) && is_pinned_constant(&pb.provenance))
                || (is_free(&pb.provenance) && is_pinned_constant(&pa.provenance));
            if value_conflict || knob_vs_constant {
                offenders.push(pa.symbol.clone());
            }
        }
    }

    if offenders.is_empty() {
        CompatCheck::pass(NAME, "no shared symbol has a value/provenance conflict")
    } else {
        CompatCheck::fail(
            NAME,
            format!(
                "conflicting provenance/value for shared symbol(s): {}",
                offenders.join(", ")
            ),
        )
    }
}

/// `"alpha_basis_finite"`: the recombined α-basis is a blend of the two; every one of the eight
/// components (four per parent) must be finite or the blend cannot stay finite.
fn check_alpha_basis_finite(a: &super::Theory, b: &super::Theory) -> CompatCheck {
    const NAME: &str = "alpha_basis_finite";
    let components = [
        ("a.alpha_m", a.alpha.alpha_m),
        ("a.alpha_b", a.alpha.alpha_b),
        ("a.alpha_k", a.alpha.alpha_k),
        ("a.alpha_t", a.alpha.alpha_t),
        ("b.alpha_m", b.alpha.alpha_m),
        ("b.alpha_b", b.alpha.alpha_b),
        ("b.alpha_k", b.alpha.alpha_k),
        ("b.alpha_t", b.alpha.alpha_t),
    ];
    let nonfinite: Vec<&str> = components
        .iter()
        .filter(|(_, v)| !v.is_finite())
        .map(|(name, _)| *name)
        .collect();
    if nonfinite.is_empty() {
        CompatCheck::pass(NAME, "all eight α-components are finite")
    } else {
        CompatCheck::fail(
            NAME,
            format!("non-finite α-component(s): {}", nonfinite.join(", ")),
        )
    }
}

/// `"tensor_sector_consistent"`: if both parents have non-negligible `alpha_t` (|·| > 1e-9) with
/// opposite signs, a single recombination cannot satisfy two incompatible tensor-speed sectors.
fn check_tensor_sector_consistent(a: &super::Theory, b: &super::Theory) -> CompatCheck {
    const NAME: &str = "tensor_sector_consistent";
    let ta = a.alpha.alpha_t;
    let tb = b.alpha.alpha_t;
    let both_significant = ta.abs() > 1e-9 && tb.abs() > 1e-9;
    if both_significant && ta.signum() != tb.signum() {
        CompatCheck::fail(
            NAME,
            format!("incompatible tensor-speed sectors: alpha_t = {ta} vs {tb} (opposite signs)"),
        )
    } else {
        CompatCheck::pass(
            NAME,
            "tensor sectors are compatible (not both significant with opposite signs)",
        )
    }
}

/// `"background_combinable"`: the recombined background is blended by the forward model; here we
/// confirm only that both parents' backgrounds serialize cleanly via `serde_json` (a lightweight
/// structural sanity gate — a background that cannot be serialized cannot be persisted or blended).
fn check_background_combinable(a: &super::Theory, b: &super::Theory) -> CompatCheck {
    const NAME: &str = "background_combinable";
    let sa = serde_json::to_string(&a.background);
    let sb = serde_json::to_string(&b.background);
    match (sa, sb) {
        (Ok(_), Ok(_)) => CompatCheck::pass(
            NAME,
            "both backgrounds serialize cleanly; blended by the forward model",
        ),
        (Err(e), _) => CompatCheck::fail(
            NAME,
            format!("parent A background failed to serialize: {e}"),
        ),
        (_, Err(e)) => CompatCheck::fail(
            NAME,
            format!("parent B background failed to serialize: {e}"),
        ),
    }
}

/// Run the full recombination compatibility proof for parents `a` and `b`.
///
/// Assembles the five named checks (`screening_mergeable`, `parameter_provenance_consistent`,
/// `alpha_basis_finite`, `tensor_sector_consistent`, `background_combinable`) into a
/// [`CompatibilityProof`]. The recombination is permitted iff `proof.compatible`.
pub fn recombination_compatible(a: &super::Theory, b: &super::Theory) -> CompatibilityProof {
    let checks = vec![
        check_screening_mergeable(a, b),
        check_parameter_provenance_consistent(a, b),
        check_alpha_basis_finite(a, b),
        check_tensor_sector_consistent(a, b),
        check_background_combinable(a, b),
    ];
    CompatibilityProof::from_checks(checks)
}

/// One-line human summary of a proof, e.g. `"compatible: 5/5 checks"` or
/// `"INCOMPATIBLE: screening_mergeable, parameter_provenance_consistent"`.
pub fn proof_summary(p: &CompatibilityProof) -> String {
    let total = p.checks.len();
    if p.compatible {
        let passed = p.checks.iter().filter(|c| c.passed).count();
        format!("compatible: {passed}/{total} checks")
    } else {
        let failed: Vec<&str> = p
            .checks
            .iter()
            .filter(|c| !c.passed)
            .map(|c| c.name.as_str())
            .collect();
        format!("INCOMPATIBLE: {}", failed.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::{Parameter, Provenance, Theory};

    #[test]
    fn identical_baselines_are_compatible() {
        let a = Theory::baseline_lcdm();
        let b = Theory::baseline_lcdm();
        let proof = recombination_compatible(&a, &b);
        assert!(proof.compatible, "two baseline clones must be compatible");
        assert_eq!(proof.checks.len(), 5);
        assert!(proof.checks.iter().all(|c| c.passed));
        assert!(proof.reasons.is_empty());
    }

    #[test]
    fn contradictory_screening_fails() {
        let mut a = Theory::baseline_lcdm();
        let mut b = Theory::baseline_lcdm();
        a.screening = Some("chameleon".into());
        b.screening = Some("vainshtein".into());
        let proof = recombination_compatible(&a, &b);
        assert!(!proof.compatible);
        let c = proof
            .checks
            .iter()
            .find(|c| c.name == "screening_mergeable")
            .expect("screening_mergeable check present");
        assert!(!c.passed);
        assert!(c.detail.contains("chameleon") && c.detail.contains("vainshtein"));
        assert!(proof.reasons.iter().any(|r| r == &c.detail));
    }

    #[test]
    fn same_screening_passes() {
        let mut a = Theory::baseline_lcdm();
        let mut b = Theory::baseline_lcdm();
        a.screening = Some("chameleon".into());
        b.screening = Some("chameleon".into());
        let proof = recombination_compatible(&a, &b);
        assert!(proof.compatible);
    }

    #[test]
    fn fundamental_value_conflict_fails() {
        let mut a = Theory::baseline_lcdm();
        let mut b = Theory::baseline_lcdm();
        // Both declare H0 as a fundamental constant but with different values.
        a.parameters = vec![Parameter {
            symbol: "H0".into(),
            value: 67.4,
            physical_meaning: "present-day expansion rate".into(),
            provenance: Provenance::Fundamental,
        }];
        b.parameters = vec![Parameter {
            symbol: "H0".into(),
            value: 70.0,
            physical_meaning: "present-day expansion rate".into(),
            provenance: Provenance::Fundamental,
        }];
        let proof = recombination_compatible(&a, &b);
        assert!(!proof.compatible);
        let c = proof
            .checks
            .iter()
            .find(|c| c.name == "parameter_provenance_consistent")
            .expect("provenance check present");
        assert!(!c.passed);
        assert!(c.detail.contains("H0"));
    }

    #[test]
    fn free_vs_fundamental_same_symbol_fails() {
        let mut a = Theory::baseline_lcdm();
        let mut b = Theory::baseline_lcdm();
        // Same value, but one side fits it freely while the other claims it fundamental.
        a.parameters = vec![Parameter {
            symbol: "H0".into(),
            value: 67.4,
            physical_meaning: "knob".into(),
            provenance: Provenance::Free,
        }];
        b.parameters = vec![Parameter {
            symbol: "H0".into(),
            value: 67.4,
            physical_meaning: "fixed".into(),
            provenance: Provenance::Fundamental,
        }];
        let proof = recombination_compatible(&a, &b);
        assert!(!proof.compatible);
        let c = proof
            .checks
            .iter()
            .find(|c| c.name == "parameter_provenance_consistent")
            .unwrap();
        assert!(!c.passed);
    }

    #[test]
    fn opposite_sign_tensor_speed_fails() {
        let mut a = Theory::baseline_lcdm();
        let mut b = Theory::baseline_lcdm();
        a.alpha.alpha_t = 0.01;
        b.alpha.alpha_t = -0.01;
        let proof = recombination_compatible(&a, &b);
        assert!(!proof.compatible);
        let c = proof
            .checks
            .iter()
            .find(|c| c.name == "tensor_sector_consistent")
            .unwrap();
        assert!(!c.passed);
    }

    #[test]
    fn one_negligible_tensor_speed_passes() {
        let mut a = Theory::baseline_lcdm();
        let mut b = Theory::baseline_lcdm();
        a.alpha.alpha_t = 0.01;
        b.alpha.alpha_t = 0.0; // negligible — only one significant sector
        let proof = recombination_compatible(&a, &b);
        let c = proof
            .checks
            .iter()
            .find(|c| c.name == "tensor_sector_consistent")
            .unwrap();
        assert!(c.passed);
    }

    #[test]
    fn alpha_basis_finiteness_detected() {
        let mut a = Theory::baseline_lcdm();
        let b = Theory::baseline_lcdm();
        a.alpha.alpha_m = f64::NAN;
        let proof = recombination_compatible(&a, &b);
        assert!(!proof.compatible);
        let c = proof
            .checks
            .iter()
            .find(|c| c.name == "alpha_basis_finite")
            .unwrap();
        assert!(!c.passed);
    }

    #[test]
    fn proof_summary_formats_both_cases() {
        let a = Theory::baseline_lcdm();
        let b = Theory::baseline_lcdm();
        let ok = recombination_compatible(&a, &b);
        assert_eq!(proof_summary(&ok), "compatible: 5/5 checks");

        let mut x = Theory::baseline_lcdm();
        let mut y = Theory::baseline_lcdm();
        x.screening = Some("chameleon".into());
        y.screening = Some("vainshtein".into());
        let bad = recombination_compatible(&x, &y);
        let summary = proof_summary(&bad);
        assert!(summary.starts_with("INCOMPATIBLE:"));
        assert!(summary.contains("screening_mergeable"));
    }
}
