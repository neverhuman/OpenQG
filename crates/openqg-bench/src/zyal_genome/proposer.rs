//! V4 M6 (deterministic core): the "LLM proposes, oracle disposes" contract.
//!
//! A proposer (the live jailgun/jnoccio LLM, or a deterministic fixture) emits a [`ProposalDoc`]:
//! a real `Theory` plus a `ClaimGraph`, typed `DerivationObligation`s, a `UnificationClaim`, and the
//! evidence *content* its claims cite. This module turns that document into the typed objects,
//! **content-binds the evidence** (it hashes the bytes the proposer actually supplied, so a claim
//! that cites evidence it does not provide is laundering and is disqualified), and adjudicates it
//! through the deterministic [`score_candidate`] — the LLM never scores. A derivation-rich proposal
//! earns the `derivation_rigor` + `unification` credit that pure parameter evolution scores 0 on;
//! a malformed, laundering, or hidden-knob proposal is disqualified by the same gates.
//!
//! The actual LLM call (the live jailgun MCP round-trip) is the only non-deterministic step and is
//! kept behind the [`Proposer`] trait, so everything here is unit-testable with a fixture proposer.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use openqg_core::theory::{
    Claim, ClaimGraph, ClaimKind, DerivationObligation, DerivationObligationKind,
    DerivedCertificate, EvidenceRef, EvidenceTier, MapEvidenceStore, Parameter, Provenance,
    ScorecardV4, Sector, SharedParam, Theory, UnificationClaim,
};
use openqg_core::ObservableRecord;

use super::physics_score::{baseline_log_likelihood, score_candidate};

const EVIDENCE_SCHEMA: &str = "proposer-evidence.v1";

fn default_unification() -> UnificationClaim {
    UnificationClaim { shared: vec![] }
}

/// The document a proposer emits. `evidence` maps a cited path → its exact content; the scorer
/// binds each [`EvidenceRef`]'s hash to those bytes, so the proposer cannot cite evidence it does
/// not supply (that is the anti-laundering contract).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProposalDoc {
    pub theory: Theory,
    #[serde(default)]
    pub claims: Vec<Claim>,
    #[serde(default)]
    pub obligations: Vec<DerivationObligation>,
    #[serde(default = "default_unification")]
    pub unification: UnificationClaim,
    /// Cited evidence: path → exact content bytes (as a string).
    #[serde(default)]
    pub evidence: BTreeMap<String, String>,
}

/// Parse a proposal from raw JSON (the LLM's output). A parse error is returned as `Err` so the
/// caller can record it as a vetoed verdict rather than crash (mirrors `proposal_receipt`).
pub(crate) fn parse_proposal_doc(json: &str) -> Result<ProposalDoc> {
    serde_json::from_str(json).context("parse proposal document")
}

/// Adjudicate a proposal veto-first through the real oracle. Builds the evidence store from the
/// supplied content, content-binds each claim's `EvidenceRef` to those bytes (an empty/absent
/// content leaves the ref unbound ⇒ disqualifying), then runs [`score_candidate`].
pub(crate) fn score_proposal(
    doc: &ProposalDoc,
    observables: &[ObservableRecord],
    baseline_ll: f64,
) -> ScorecardV4 {
    let mut store = MapEvidenceStore::default();
    for (path, content) in &doc.evidence {
        store.0.insert(path.clone(), content.clone().into_bytes());
    }

    // Content-bind the cited evidence: fill each ref's hash from the bytes the proposer supplied.
    // A ref whose path has no supplied content stays unbound ⇒ the evidence gate disqualifies it.
    let mut claims = doc.claims.clone();
    for c in &mut claims {
        for r in &mut c.evidence {
            if r.sha256.is_empty() {
                if let Some(bytes) = store.0.get(&r.path) {
                    r.sha256 = openqg_core::sha256_digest(bytes);
                }
            }
        }
    }
    let cg = ClaimGraph { claims };

    score_candidate(
        &doc.theory,
        observables,
        baseline_ll,
        &cg,
        &doc.obligations,
        &doc.unification,
        &store,
        EVIDENCE_SCHEMA,
    )
}

/// Convenience: parse + score in one call, computing the baseline internally.
pub(crate) fn parse_and_score(json: &str, observables: &[ObservableRecord]) -> Result<ScorecardV4> {
    let doc = parse_proposal_doc(json)?;
    let baseline_ll = baseline_log_likelihood(observables);
    Ok(score_proposal(&doc, observables, baseline_ll))
}

/// A source of proposals. The deterministic [`FixtureProposer`] is used in tests and offline; the
/// live jailgun/jnoccio adapter (which performs the MCP round-trip and returns the JSON) implements
/// the same trait, so the engine consumes either identically.
pub(crate) trait Proposer {
    fn propose(&self) -> Result<ProposalDoc>;
}

/// A deterministic proposer that emits a derivation-rich candidate: an nDGP-style theory whose
/// effective-gravity parameter is value-certified (`ndgp_geff_over_g`), with a verifying
/// `NumericWitness` obligation behind a content-bound physics claim, and a genuine shared-parameter
/// unification claim with no hidden knob. It is what a *good* LLM proposal looks like — and it earns
/// the derivation + unification credit that pure parameter evolution cannot.
pub(crate) struct FixtureProposer;

impl Proposer for FixtureProposer {
    fn propose(&self) -> Result<ProposalDoc> {
        Ok(fixture_proposal())
    }
}

/// Rate-limits an inner (e.g. live) proposer so a long deterministic campaign only spends a live
/// call on the first generation and every `every`-th generation thereafter. Off-generations return
/// `Err`, which the engine treats as "no proposal this generation" — so the bulk evolves
/// deterministically and the live (jailgun) budget is bounded. Single-threaded (the engine is
/// sequential), so a `Cell` counter is sufficient.
pub(crate) struct BudgetedProposer<'a> {
    inner: &'a dyn Proposer,
    every: usize,
    count: std::cell::Cell<usize>,
}

impl<'a> BudgetedProposer<'a> {
    pub(crate) fn new(inner: &'a dyn Proposer, every: usize) -> Self {
        Self {
            inner,
            every: every.max(1),
            count: std::cell::Cell::new(0),
        }
    }
}

impl Proposer for BudgetedProposer<'_> {
    fn propose(&self) -> Result<ProposalDoc> {
        let n = self.count.get() + 1;
        self.count.set(n);
        if n == 1 || n % self.every == 0 {
            self.inner.propose()
        } else {
            anyhow::bail!(
                "budgeted proposer: skip generation {n} (live every {})",
                self.every
            )
        }
    }
}

/// The reference derivation-rich proposal (also used as the canonical test vector).
pub(crate) fn fixture_proposal() -> ProposalDoc {
    // β = 2 ⇒ G_eff/G = 1 + 1/(3·2) = 7/6, recomputed by the cited closed form.
    let geff = 1.0 + 1.0 / 6.0;
    let cert = DerivedCertificate {
        relation: "ndgp_geff_over_g".into(),
        inputs: vec![("beta".into(), 2.0)],
        expected: geff,
        tolerance: 1e-6,
    };

    let mut theory = Theory::baseline_lcdm();
    theory.id = "ndgp-proposed".into();
    theory.parameters.push(Parameter {
        symbol: "geff_over_g".into(),
        value: geff,
        physical_meaning: "linear effective gravitational coupling (nDGP normal branch)".into(),
        provenance: Provenance::derived_certified("nDGP braneworld linear coupling", cert.clone()),
    });

    let evidence_path = "derivations/ndgp-geff.txt";
    let evidence_body = "nDGP normal branch: G_eff/G = 1 + 1/(3 beta); beta = 2 => 7/6.\n\
        Ref: Koyama & Maartens 2006; Schmidt 2009.\n";
    let mut evidence = BTreeMap::new();
    evidence.insert(evidence_path.to_string(), evidence_body.to_string());

    let claim = Claim {
        id: "claim-geff".into(),
        sector: Sector::Growth,
        kind: ClaimKind::Physics,
        statement: "G_eff/G is derived from the nDGP braneworld function, not fitted".into(),
        // sha256 left empty ⇒ the scorer binds it from the supplied content.
        evidence: vec![EvidenceRef::new(evidence_path, "", EvidenceTier::T2)],
        obligations: vec!["ob-geff".into()],
        depends_on: vec![],
    };

    let obligation = DerivationObligation {
        claim_id: "ob-geff".into(),
        kind: DerivationObligationKind::NumericWitness,
        detail: "recompute G_eff/G from the cited closed form".into(),
        certificate: Some(cert),
        limit: None,
        citation: Some("Koyama & Maartens 2006".into()),
    };

    // H0 is shared across Background and Growth (both fundamental) — a genuine, hidden-knob-free
    // unification claim.
    let unification = UnificationClaim {
        shared: vec![SharedParam {
            symbol: "H0".into(),
            value: 67.4,
            sectors: vec![Sector::Background, Sector::Growth],
        }],
    };

    ProposalDoc {
        theory,
        claims: vec![claim],
        obligations: vec![obligation],
        unification,
        evidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs() -> Vec<ObservableRecord> {
        ["bao_dv_z038", "bao_dv_z051", "fsigma8_z038", "fsigma8_z051"]
            .iter()
            .enumerate()
            .map(|(i, id)| ObservableRecord {
                observable_id: (*id).into(),
                kind: "cosmology".into(),
                value: 1.0 + i as f64 * 0.1,
                uncertainty: 0.05,
                unit: "x".into(),
                source: None,
            })
            .collect()
    }

    #[test]
    fn a_derivation_rich_proposal_earns_rigor_and_unification() {
        let observables = obs();
        let baseline_ll = baseline_log_likelihood(&observables);
        let sc = score_proposal(&fixture_proposal(), &observables, baseline_ll);
        assert!(
            !sc.disqualified,
            "fixture proposal should survive: {:?}",
            sc.kill_reasons
        );
        let pts = |n: &str| {
            sc.components
                .iter()
                .find(|c| c.name == n)
                .map(|c| c.points)
                .unwrap_or(0.0)
        };
        assert!(
            pts("derivation_rigor") > 0.0,
            "must earn derivation rigor (verified obligation)"
        );
        assert!(
            pts("unification") > 0.0,
            "must earn unification (shared param, no hidden knob)"
        );
        assert!(
            sc.total > 40.0,
            "a derivation-rich proposal should score well: {}",
            sc.total
        );
    }

    #[test]
    fn the_proposal_round_trips_through_json_and_the_oracle() {
        let doc = fixture_proposal();
        let json = serde_json::to_string(&doc).unwrap();
        let observables = obs();
        let sc = parse_and_score(&json, &observables).unwrap();
        assert!(!sc.disqualified);
        // The fixture proposer satisfies the Proposer trait too.
        let from_trait = FixtureProposer.propose().unwrap();
        let sc2 = score_proposal(
            &from_trait,
            &observables,
            baseline_log_likelihood(&observables),
        );
        assert_eq!(sc.total, sc2.total);
    }

    #[test]
    fn evidence_laundering_in_a_proposal_is_disqualified() {
        // Cite evidence but supply NO content for it ⇒ unbound ⇒ disqualified.
        let mut doc = fixture_proposal();
        doc.evidence.clear();
        let observables = obs();
        let sc = score_proposal(&doc, &observables, baseline_log_likelihood(&observables));
        assert!(
            sc.disqualified,
            "a proposal citing unsupplied evidence must be disqualified"
        );
        assert!(sc.kill_reasons.iter().any(|r| r.contains("evidence")));
    }

    #[test]
    fn a_hidden_knob_proposal_is_disqualified() {
        let mut doc = fixture_proposal();
        // add an uncertified derived knob NOT in the shared set, while still claiming unification.
        doc.theory.parameters.push(Parameter {
            symbol: "g_secret".into(),
            value: 0.7,
            physical_meaning: "sector-private knob".into(),
            provenance: Provenance::derived("hand-wave"),
        });
        let observables = obs();
        let sc = score_proposal(&doc, &observables, baseline_log_likelihood(&observables));
        assert!(sc.disqualified);
        assert!(sc
            .kill_reasons
            .iter()
            .any(|r| r.contains("no_hidden_knob") || r.contains("hidden")));
    }

    #[test]
    fn a_free_parameter_proposal_is_disqualified() {
        let mut doc = fixture_proposal();
        doc.theory.parameters.push(Parameter {
            symbol: "w_fit".into(),
            value: -0.9,
            physical_meaning: "fitted knob".into(),
            provenance: Provenance::Free,
        });
        let observables = obs();
        let sc = score_proposal(&doc, &observables, baseline_log_likelihood(&observables));
        assert!(sc.disqualified);
    }
}
