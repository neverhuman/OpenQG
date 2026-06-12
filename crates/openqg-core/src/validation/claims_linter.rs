//! V8 Wave 0.8: claim-level lint rules for the V4 claim graph.
//!
//! Three rules enforced before scoring:
//! - `NO_EVIDENCE_HASH`                      — physics claim carries no content-bound evidence ref
//! - `BEATS_LCDM_WITHOUT_TRIALS_CORRECTION`  — statement asserts beating the baseline; no trials correction
//! - `FIT_SET_NOVELTY_CREDIT`                — novelty asserted from fit-set evidence only (no out-of-sample ref)
//!
//! Any fatal finding drives `disqualified = true` in the scorecard.

use crate::theory::{Claim, ClaimGraph};

/// One lint finding on a single claim.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimFinding {
    pub claim_id: String,
    pub rule: &'static str,
    pub message: String,
}

/// The aggregate lint result for a claim graph.
#[derive(Debug, Clone, PartialEq)]
pub struct ClaimLintReport {
    pub findings: Vec<ClaimFinding>,
    /// True when any finding exists (all current rules are fatal).
    pub fatal: bool,
}

/// Run all three lint rules over every physics claim in `cg`.
pub fn lint_claims(cg: &ClaimGraph) -> ClaimLintReport {
    let mut findings = Vec::new();

    for claim in cg.physics_claims() {
        check_no_evidence_hash(claim, &mut findings);
        check_beats_lcdm_without_trials_correction(claim, &mut findings);
        check_fit_set_novelty_credit(claim, &mut findings);
    }

    let fatal = !findings.is_empty();
    ClaimLintReport { findings, fatal }
}

/// Rule NO_EVIDENCE_HASH: a physics claim with no evidence refs can only pass content-binding
/// by accident — it has nothing to bind.
fn check_no_evidence_hash(claim: &Claim, findings: &mut Vec<ClaimFinding>) {
    if claim.evidence.is_empty() {
        findings.push(ClaimFinding {
            claim_id: claim.id.clone(),
            rule: "NO_EVIDENCE_HASH",
            message: format!(
                "physics claim '{}' carries no content-bound evidence references; \
                 add an EvidenceRef with a sha256 digest",
                claim.id
            ),
        });
    }
}

/// Rule BEATS_LCDM_WITHOUT_TRIALS_CORRECTION: a claim that asserts superiority over the baseline
/// must declare the number of hypotheses tested, otherwise the comparison is a look-elsewhere
/// artifact. Check: statement contains "beats"/"better than" near "cdm"/"baseline", with no
/// mention of a trials correction.
fn check_beats_lcdm_without_trials_correction(claim: &Claim, findings: &mut Vec<ClaimFinding>) {
    let s = claim.statement.to_ascii_lowercase();
    let asserts_superiority = (s.contains("beats") || s.contains("better than"))
        && (s.contains("cdm") || s.contains("baseline") || s.contains("reference model"));
    if !asserts_superiority {
        return;
    }
    let has_correction =
        s.contains("trials") || s.contains("bonferroni") || s.contains("look-elsewhere");
    if !has_correction {
        findings.push(ClaimFinding {
            claim_id: claim.id.clone(),
            rule: "BEATS_LCDM_WITHOUT_TRIALS_CORRECTION",
            message: format!(
                "claim '{}' asserts beating the baseline without a trials correction; \
                 cite the number of hypotheses tested or add a Bonferroni correction",
                claim.id
            ),
        });
    }
}

/// Rule FIT_SET_NOVELTY_CREDIT: a claim that earns novelty credit from evidence that is wholly
/// fit-set (BIC/AIC/chi-squared paths, no out-of-sample reference). An empty evidence list is
/// already caught by NO_EVIDENCE_HASH and skipped here to avoid double-reporting.
fn check_fit_set_novelty_credit(claim: &Claim, findings: &mut Vec<ClaimFinding>) {
    if claim.evidence.is_empty() {
        return;
    }

    let s = claim.statement.to_ascii_lowercase();
    let novelty_asserted = s.contains("beats")
        || s.contains("novel")
        || s.contains("better than")
        || s.contains("improves")
        || s.contains("new prediction");
    if !novelty_asserted {
        return;
    }

    let fit_set_only = claim.evidence.iter().all(|e| {
        let p = e.path.to_ascii_lowercase();
        p.contains("bic") || p.contains("aic") || p.contains("chi") || p.contains("fit-set")
    });
    let has_out_of_sample = claim.evidence.iter().any(|e| {
        let p = e.path.to_ascii_lowercase();
        p.contains("forecast") || p.contains("holdout") || p.contains("split")
    });

    if fit_set_only && !has_out_of_sample {
        findings.push(ClaimFinding {
            claim_id: claim.id.clone(),
            rule: "FIT_SET_NOVELTY_CREDIT",
            message: format!(
                "claim '{}' asserts novelty from fit-set evidence only (no out-of-sample or \
                 pre-registered forecast reference); fit-set superiority earns no novelty credit",
                claim.id
            ),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::claim_graph::{Claim, ClaimKind, Sector};
    use crate::theory::evidence::{EvidenceRef, EvidenceTier};

    fn physics_claim(id: &str, statement: &str, evidence: Vec<EvidenceRef>) -> Claim {
        Claim {
            id: id.into(),
            sector: Sector::Background,
            kind: ClaimKind::Physics,
            statement: statement.into(),
            evidence,
            obligations: vec!["ob".into()],
            depends_on: vec![],
        }
    }

    fn bound_ref(path: &str) -> EvidenceRef {
        EvidenceRef::new(path, "a".repeat(64), EvidenceTier::T2)
    }

    fn single_claim_graph(claim: Claim) -> ClaimGraph {
        ClaimGraph {
            claims: vec![claim],
        }
    }

    // ---- Rule 1: NO_EVIDENCE_HASH ----

    #[test]
    fn no_evidence_hash_fires_on_empty_evidence() {
        let cg = single_claim_graph(physics_claim("c1", "a physics result", vec![]));
        let report = lint_claims(&cg);
        assert!(report.fatal);
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule, "NO_EVIDENCE_HASH");
    }

    #[test]
    fn no_evidence_hash_clean_when_evidence_present() {
        let cg = single_claim_graph(physics_claim(
            "c1",
            "a physics result",
            vec![bound_ref("bg.json")],
        ));
        let report = lint_claims(&cg);
        let no_ev: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule == "NO_EVIDENCE_HASH")
            .collect();
        assert!(no_ev.is_empty(), "{no_ev:?}");
    }

    // ---- Rule 2: BEATS_LCDM_WITHOUT_TRIALS_CORRECTION ----

    #[test]
    fn beats_lcdm_fires_without_trials_correction() {
        let stmt = "This theory beats ΛCDM with ΔBIC = -6.2 on the fit-set BAO+CMB data";
        let cg = single_claim_graph(physics_claim("c1", stmt, vec![bound_ref("bg.json")]));
        let report = lint_claims(&cg);
        assert!(report.fatal);
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == "BEATS_LCDM_WITHOUT_TRIALS_CORRECTION"));
    }

    #[test]
    fn beats_lcdm_clean_when_trials_cited() {
        let stmt = "This theory beats ΛCDM; trials correction applied over N=50 hypotheses";
        let cg = single_claim_graph(physics_claim("c1", stmt, vec![bound_ref("bg.json")]));
        let report = lint_claims(&cg);
        let found: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule == "BEATS_LCDM_WITHOUT_TRIALS_CORRECTION")
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn neutral_statement_does_not_trigger_beats_lcdm_rule() {
        let stmt = "G_eff/G is derived from the nDGP braneworld function";
        let cg = single_claim_graph(physics_claim("c1", stmt, vec![bound_ref("growth.json")]));
        let report = lint_claims(&cg);
        assert!(!report.fatal, "{:?}", report.findings);
    }

    // ---- Rule 3: FIT_SET_NOVELTY_CREDIT ----

    #[test]
    fn fit_set_novelty_fires_for_bic_only_novelty_claim() {
        let stmt = "This model beats the baseline on the training set";
        let cg = single_claim_graph(physics_claim(
            "c1",
            stmt,
            vec![bound_ref("bic-improvement.json")],
        ));
        let report = lint_claims(&cg);
        assert!(report.fatal);
        assert!(report
            .findings
            .iter()
            .any(|f| f.rule == "FIT_SET_NOVELTY_CREDIT"));
    }

    #[test]
    fn fit_set_novelty_clean_when_forecast_evidence_present() {
        let stmt = "This model beats the baseline; see the pre-registered forecast";
        let cg = single_claim_graph(physics_claim(
            "c1",
            stmt,
            vec![bound_ref("bic.json"), bound_ref("forecast-desi-dr3.json")],
        ));
        let report = lint_claims(&cg);
        let found: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.rule == "FIT_SET_NOVELTY_CREDIT")
            .collect();
        assert!(found.is_empty(), "{found:?}");
    }

    // ---- Integration: decoy-fit-only-claim scenario ----

    #[test]
    fn decoy_fit_only_claim_is_caught() {
        // Represents the scenario in tests-fixtures/decoy/decoy-fit-only-claim.json:
        // a claim that "beats ΛCDM" with BIC evidence only, no content-bound evidence ref.
        let stmt = "This theory beats ΛCDM with ΔBIC = -6.2 on the fit-set BAO+CMB data";
        let cg = single_claim_graph(physics_claim(
            "claim-beats-lcdm-fit-only",
            stmt,
            vec![], // no EvidenceRef: the BIC improvement is not a content-bound file
        ));
        let report = lint_claims(&cg);
        assert!(
            report.fatal,
            "fit-only-claim decoy must produce fatal lint findings"
        );
        assert!(
            report.findings.iter().any(|f| f.rule == "NO_EVIDENCE_HASH"),
            "expected NO_EVIDENCE_HASH finding"
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.rule == "BEATS_LCDM_WITHOUT_TRIALS_CORRECTION"),
            "expected BEATS_LCDM_WITHOUT_TRIALS_CORRECTION finding"
        );
    }

    // ---- Engineering claims are not linted ----

    #[test]
    fn engineering_claims_are_skipped() {
        use crate::theory::claim_graph::ClaimKind;
        let cg = ClaimGraph {
            claims: vec![Claim {
                id: "eng-c".into(),
                sector: Sector::Background,
                kind: ClaimKind::Engineering,
                statement: "pipeline reproduces".into(),
                evidence: vec![],
                obligations: vec![],
                depends_on: vec![],
            }],
        };
        let report = lint_claims(&cg);
        assert!(
            !report.fatal,
            "engineering claims must not trigger lint rules"
        );
    }
}
