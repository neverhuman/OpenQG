//! V4 M4: the human-baseline + decoy calibration league.
//!
//! Trust in the score requires two demonstrations, both run through the *identical* [`score`] path:
//! 1. **Decoys must die.** A suite of deliberately-flawed candidates (gray-box free parameter,
//!    ghost, unobligated physics claim, hidden-knob "unification", path-only evidence laundering,
//!    failed value certificate) must ALL be disqualified. [`decoy_false_positive_rate`] must be 0 —
//!    a non-zero rate means the rubric is gameable and the trust gate fails.
//! 2. **Legitimate programs must survive.** A set of physically-sane, GR-recovering human-baseline
//!    contenders must produce finite, non-disqualified scorecards (they tie ΛCDM on data fit and are
//!    separated by derivation rigor / unification — the dimensions on which we intend to *rise above*
//!    them).
//!
//! Encoding caveat (honest): the top human programs (string/M-theory, LQG, asymptotic safety, causal
//! sets) make no *distinct low-energy cosmological* prediction — they reduce to GR/ΛCDM — so here
//! they are encoded as physically-sane ΛCDM-recovering baselines that exercise the gates and the
//! rubric. The point of this league is calibration (decoys die, legitimate physics survives, scoring
//! is reproducible), not a faithful quantum-gravity simulation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::claim_graph::{Claim, ClaimKind, Sector};
use super::scorecard::{score, DataFitOutcome, ScorecardV4};
use super::{
    audit_bytes, ClaimGraph, DerivationObligation, DerivationObligationKind, EvidenceRef,
    EvidenceStore, EvidenceTier, LimitWitness, MaterializedEvidenceAudit, Parameter, Provenance,
    SharedParam, Stability, Theory, UnificationClaim,
};

const EVIDENCE_SCHEMA: &str = "contender-evidence.v1";

/// What we expect the score path to do to a contender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedVerdict {
    /// A legitimate, physically-sane theory that must NOT be disqualified.
    Survives,
    /// A deliberately-flawed decoy that MUST be disqualified.
    Disqualified,
}

/// A fully-specified contender ready to run through [`score`].
#[derive(Debug, Clone)]
pub struct Contender {
    pub name: String,
    pub theory: Theory,
    pub claim_graph: ClaimGraph,
    pub obligations: Vec<DerivationObligation>,
    pub unification: UnificationClaim,
    pub data_fit: Option<DataFitOutcome>,
}

/// An in-memory, content-addressed evidence store: path → exact bytes. The contender builders insert
/// each piece of evidence here and reference it with a hash-bound [`EvidenceRef`].
#[derive(Debug, Default, Clone)]
pub struct MapEvidenceStore(pub BTreeMap<String, Vec<u8>>);

impl EvidenceStore for MapEvidenceStore {
    fn materialize(&self, r: &EvidenceRef, schema_id: &str) -> MaterializedEvidenceAudit {
        match self.0.get(&r.path) {
            Some(bytes) => audit_bytes(r, bytes, schema_id),
            None => MaterializedEvidenceAudit {
                path: r.path.clone(),
                sha256: String::new(),
                byte_len: 0,
                jsonl_count: 0,
                schema_id: schema_id.to_string(),
                tier: r.tier,
                ok: false,
                detail: "evidence path not present in store".into(),
            },
        }
    }
}

/// A suite of contenders sharing one evidence store.
pub struct ContenderSuite {
    pub entries: Vec<(Contender, ExpectedVerdict)>,
    pub store: MapEvidenceStore,
}

/// Insert `bytes` at `path` into `store` and return a hash-bound reference to them.
fn bind(store: &mut MapEvidenceStore, path: &str, bytes: &[u8], tier: EvidenceTier) -> EvidenceRef {
    store.0.insert(path.to_string(), bytes.to_vec());
    EvidenceRef::new(path, crate::sha256_digest(bytes), tier)
}

/// A ΛCDM-tie data fit (the human baselines neither beat nor lose to ΛCDM on cosmology).
fn tie_fit() -> DataFitOutcome {
    DataFitOutcome {
        likelihood_mode: super::scorecard::LikelihoodMode::Diagonal,
        covariance_block_count: 0,
        delta_aic: 0.0,
        delta_lnz: 0.0,
        generalization_gap: 0.01,
        coverage: 1.0,
        boundary_hit: false,
    }
}

/// A verifying GR-limit obligation for a claim id.
fn gr_limit_obligation(claim_id: &str) -> DerivationObligation {
    DerivationObligation {
        claim_id: claim_id.to_string(),
        kind: DerivationObligationKind::Limit,
        detail: "recovers GR/ΛCDM in the appropriate limit".into(),
        certificate: None,
        limit: Some(LimitWitness {
            name: "gr_limit".into(),
            residual: 0.0,
            bound: 1e-6,
        }),
        citation: None,
        novel: None,
    }
}

fn physics_claim(
    id: &str,
    sector: Sector,
    evidence: Vec<EvidenceRef>,
    obligation_id: &str,
) -> Claim {
    Claim {
        id: id.to_string(),
        sector,
        kind: ClaimKind::Physics,
        statement: format!("{id}: physical claim with a verified obligation"),
        evidence,
        obligations: vec![obligation_id.to_string()],
        depends_on: vec![],
    }
}

/// The human-baseline contenders — physically sane, GR-recovering, must all SURVIVE.
pub fn human_contenders() -> ContenderSuite {
    let mut store = MapEvidenceStore::default();
    let mut entries = Vec::new();

    // 1. GR + ΛCDM itself (the reference floor).
    {
        let ev = bind(
            &mut store,
            "lcdm/bg.json",
            b"{\"E2\":1.0}\n",
            EvidenceTier::T2,
        );
        let cg = ClaimGraph {
            claims: vec![physics_claim(
                "lcdm-bg",
                Sector::Background,
                vec![ev],
                "lcdm-ob",
            )],
        };
        entries.push((
            Contender {
                name: "gr_lcdm".into(),
                theory: Theory::baseline_lcdm(),
                claim_graph: cg,
                obligations: vec![gr_limit_obligation("lcdm-ob")],
                unification: UnificationClaim { shared: vec![] },
                data_fit: Some(tie_fit()),
            },
            ExpectedVerdict::Survives,
        ));
    }

    // 2. A unification-claiming, GR-recovering program with a genuinely shared parameter and NO
    //    hidden knob (the "rises-above" target: same data fit, more unification credit).
    {
        let ev_bg = bind(
            &mut store,
            "uni/bg.json",
            b"{\"E2\":1.0}\n",
            EvidenceTier::T2,
        );
        let ev_gr = bind(
            &mut store,
            "uni/growth.jsonl",
            b"{\"fs8\":0.45}\n",
            EvidenceTier::T3,
        );
        let cg = ClaimGraph {
            claims: vec![
                physics_claim("uni-bg", Sector::Background, vec![ev_bg], "uni-ob-bg"),
                physics_claim("uni-gr", Sector::Growth, vec![ev_gr], "uni-ob-gr"),
            ],
        };
        // H0 is shared across Background and Growth, both fundamental ⇒ no hidden knob.
        let uni = UnificationClaim {
            shared: vec![SharedParam {
                symbol: "H0".into(),
                value: 67.4,
                sectors: vec![Sector::Background, Sector::Growth],
            }],
        };
        entries.push((
            Contender {
                name: "shared_parameter_program".into(),
                theory: Theory::baseline_lcdm(),
                claim_graph: cg,
                obligations: vec![
                    gr_limit_obligation("uni-ob-bg"),
                    gr_limit_obligation("uni-ob-gr"),
                ],
                unification: uni,
                data_fit: Some(tie_fit()),
            },
            ExpectedVerdict::Survives,
        ));
    }

    // 3. A REAL modified-gravity program (V4.1 reference, not a ΛCDM strawman): nDGP linear coupling
    //    G_eff/G = 7/6, value-certified on a non-trivial relation, plus a verified falsifiable fσ8
    //    prediction. The rubric must rank this *above* the ΛCDM-recovering baselines — it is the
    //    "distinct physics beats rediscovery" exemplar.
    {
        let ev_bg = bind(
            &mut store,
            "ndgp/bg.json",
            b"{\"E2\":1.0}\n",
            EvidenceTier::T2,
        );
        let ev_gr = bind(
            &mut store,
            "ndgp/growth.jsonl",
            b"{\"fs8\":0.42}\n",
            EvidenceTier::T3,
        );
        let cert = super::DerivedCertificate {
            relation: "ndgp_geff_over_g".into(),
            inputs: vec![("beta".into(), 2.0)],
            expected: 1.0 + 1.0 / 6.0,
            tolerance: 1e-9,
        };
        let mut theory = Theory::baseline_lcdm();
        theory.id = "real_modification_program".into();
        theory.parameters.push(Parameter {
            symbol: "geff_over_g".into(),
            value: 1.0 + 1.0 / 6.0,
            physical_meaning: "nDGP normal-branch linear effective gravitational coupling".into(),
            provenance: Provenance::derived_certified(
                "nDGP braneworld linear coupling",
                cert.clone(),
            ),
        });
        let cg = ClaimGraph {
            claims: vec![Claim {
                id: "ndgp-geff".into(),
                sector: Sector::Growth,
                kind: ClaimKind::Physics,
                statement: "G_eff/G is derived from the nDGP braneworld function, not fitted"
                    .into(),
                evidence: vec![ev_bg, ev_gr],
                obligations: vec!["ndgp-ob-num".into(), "ndgp-ob-novel".into()],
                depends_on: vec![],
            }],
        };
        // V5: the witness numbers are ENGINE-COMPUTED, not invented — the truth-audit checks the
        // declared values against the model's prediction for the bound background, so the honest
        // reference contender derives its own falsifiable prediction from the physics (normal-
        // branch nDGP ENHANCES growth — the deviation is positive).
        let (predicted, baseline) = {
            use crate::cosmology::{BackgroundForwardModel, CosmologyParams, ForwardModel};
            let bound = super::binding::bind_modified_background(&theory).theory;
            let model = BackgroundForwardModel;
            let ids = vec!["fsigma8@0.51".to_string()];
            let p = model.predict(&bound.background, &ids).expect("predict")[0].value;
            let b = model
                .predict(&CosmologyParams::planck_lcdm(), &ids)
                .expect("predict")[0]
                .value;
            (p, b)
        };
        let obligations = vec![
            DerivationObligation {
                claim_id: "ndgp-ob-num".into(),
                kind: DerivationObligationKind::NumericWitness,
                detail: "recompute G_eff/G from the nDGP closed form".into(),
                certificate: Some(cert),
                limit: None,
                citation: Some("Koyama & Maartens 2006".into()),
                novel: None,
            },
            DerivationObligation {
                claim_id: "ndgp-ob-novel".into(),
                kind: DerivationObligationKind::NovelPrediction,
                detail: "enhanced fσ8 relative to ΛCDM (normal-branch nDGP)".into(),
                certificate: None,
                limit: None,
                citation: None,
                novel: Some(super::NovelPredictionWitness {
                    refreshed_by_engine: false,
                    observable: "fsigma8_z051".into(),
                    predicted,
                    baseline,
                    min_detectable: 0.01,
                    falsifier: "DESI/Euclid RSD fσ8 at z=0.51".into(),
                }),
            },
        ];
        entries.push((
            Contender {
                name: "real_modification_program".into(),
                theory,
                claim_graph: cg,
                obligations,
                unification: UnificationClaim { shared: vec![] },
                data_fit: Some(tie_fit()),
            },
            ExpectedVerdict::Survives,
        ));
    }

    ContenderSuite { entries, store }
}

/// The decoy suite — every entry MUST be disqualified by [`score`].
pub fn decoy_contenders() -> ContenderSuite {
    let mut store = MapEvidenceStore::default();
    let mut entries = Vec::new();

    let good_ev =
        |store: &mut MapEvidenceStore, p: &str| bind(store, p, b"{\"x\":1}\n", EvidenceTier::T2);

    // 1. Gray-box: a free fitting parameter (FreeParameter veto).
    {
        let ev = good_ev(&mut store, "decoy/gb.json");
        let mut t = Theory::baseline_lcdm();
        t.id = "decoy_gray_box".into();
        t.parameters.push(Parameter {
            symbol: "w_fit".into(),
            value: -0.9,
            physical_meaning: "fitted EoS knob".into(),
            provenance: Provenance::Free,
        });
        let cg = ClaimGraph {
            claims: vec![physics_claim("gb", Sector::Background, vec![ev], "gb-ob")],
        };
        entries.push((
            Contender {
                name: "decoy_gray_box".into(),
                theory: t,
                claim_graph: cg,
                obligations: vec![gr_limit_obligation("gb-ob")],
                unification: UnificationClaim { shared: vec![] },
                data_fit: Some(tie_fit()),
            },
            ExpectedVerdict::Disqualified,
        ));
    }

    // 2. Ghost: wrong-sign kinetic term (Ghost veto).
    {
        let ev = good_ev(&mut store, "decoy/ghost.json");
        let mut t = Theory::baseline_lcdm();
        t.id = "decoy_ghost".into();
        t.stability = Stability {
            kinetic_coefficient: -1.0,
            ..Stability::healthy()
        };
        let cg = ClaimGraph {
            claims: vec![physics_claim("gh", Sector::Background, vec![ev], "gh-ob")],
        };
        entries.push((
            Contender {
                name: "decoy_ghost".into(),
                theory: t,
                claim_graph: cg,
                obligations: vec![gr_limit_obligation("gh-ob")],
                unification: UnificationClaim { shared: vec![] },
                data_fit: Some(tie_fit()),
            },
            ExpectedVerdict::Disqualified,
        ));
    }

    // 3. Unobligated physics claim (no derivation behind a physics claim).
    {
        let ev = good_ev(&mut store, "decoy/unob.json");
        let mut c = physics_claim("unob", Sector::Growth, vec![ev], "none");
        c.obligations.clear();
        entries.push((
            Contender {
                name: "decoy_unobligated_claim".into(),
                theory: Theory::baseline_lcdm(),
                claim_graph: ClaimGraph { claims: vec![c] },
                obligations: vec![],
                unification: UnificationClaim { shared: vec![] },
                data_fit: Some(tie_fit()),
            },
            ExpectedVerdict::Disqualified,
        ));
    }

    // 4. Hidden knob: claims unification but hides a sector-private uncertified derived parameter.
    {
        let ev = good_ev(&mut store, "decoy/hk.json");
        let mut t = Theory::baseline_lcdm();
        t.id = "decoy_hidden_knob".into();
        t.parameters.push(Parameter {
            symbol: "g_dark".into(),
            value: 0.42,
            physical_meaning: "sector-private coupling".into(),
            provenance: Provenance::derived("hand-wave, uncertified"),
        });
        let uni = UnificationClaim {
            shared: vec![SharedParam {
                symbol: "H0".into(),
                value: 67.4,
                sectors: vec![Sector::Background, Sector::Growth],
            }],
        };
        entries.push((
            Contender {
                name: "decoy_hidden_knob".into(),
                theory: t,
                claim_graph: ClaimGraph {
                    claims: vec![physics_claim("hk", Sector::Background, vec![ev], "hk-ob")],
                },
                obligations: vec![gr_limit_obligation("hk-ob")],
                unification: uni,
                data_fit: Some(tie_fit()),
            },
            ExpectedVerdict::Disqualified,
        ));
    }

    // 5. Evidence laundering: a physics claim cites a path-only (unbound) reference.
    {
        let laundered = EvidenceRef::new("decoy/launder.json", "", EvidenceTier::T2); // empty sha256
        let cg = ClaimGraph {
            claims: vec![physics_claim(
                "ln",
                Sector::Background,
                vec![laundered],
                "ln-ob",
            )],
        };
        entries.push((
            Contender {
                name: "decoy_evidence_laundering".into(),
                theory: Theory::baseline_lcdm(),
                claim_graph: cg,
                obligations: vec![gr_limit_obligation("ln-ob")],
                unification: UnificationClaim { shared: vec![] },
                data_fit: Some(tie_fit()),
            },
            ExpectedVerdict::Disqualified,
        ));
    }

    // 6. Failed value certificate: a derived parameter whose certificate does not recompute.
    {
        let ev = good_ev(&mut store, "decoy/fc.json");
        let mut t = Theory::baseline_lcdm();
        t.id = "decoy_failed_certificate".into();
        // ndgp_geff_over_g with beta=2 computes 1.1666…; claim 1.0 ⇒ certificate fails ⇒ killed.
        let bad_cert = super::DerivedCertificate {
            relation: "ndgp_geff_over_g".into(),
            inputs: vec![("beta".into(), 2.0)],
            expected: 1.0,
            tolerance: 1e-6,
        };
        t.parameters.push(Parameter {
            symbol: "geff".into(),
            value: 1.0,
            physical_meaning: "claimed G_eff/G".into(),
            provenance: Provenance::derived_certified("nDGP linear coupling", bad_cert),
        });
        let cg = ClaimGraph {
            claims: vec![physics_claim("fc", Sector::Growth, vec![ev], "fc-ob")],
        };
        entries.push((
            Contender {
                name: "decoy_failed_certificate".into(),
                theory: t,
                claim_graph: cg,
                obligations: vec![gr_limit_obligation("fc-ob")],
                unification: UnificationClaim { shared: vec![] },
                data_fit: Some(tie_fit()),
            },
            ExpectedVerdict::Disqualified,
        ));
    }

    ContenderSuite { entries, store }
}

/// Score one contender through the identical [`score`] path.
pub fn score_contender(c: &Contender, store: &dyn EvidenceStore) -> ScorecardV4 {
    score(
        &c.theory,
        &c.claim_graph,
        &c.obligations,
        &c.unification,
        store,
        EVIDENCE_SCHEMA,
        c.data_fit,
    )
}

/// Fraction of decoys that were NOT disqualified — the calibration metric. MUST be 0 for the trust
/// gate to pass. `results` are `(scorecard, expected_verdict)` pairs; only `Disqualified`-expected
/// entries (the decoys) are counted.
pub fn decoy_false_positive_rate(results: &[(ScorecardV4, ExpectedVerdict)]) -> f64 {
    let decoys: Vec<&ScorecardV4> = results
        .iter()
        .filter(|(_, v)| *v == ExpectedVerdict::Disqualified)
        .map(|(s, _)| s)
        .collect();
    if decoys.is_empty() {
        return 0.0;
    }
    let survived = decoys.iter().filter(|s| !s.disqualified).count();
    survived as f64 / decoys.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_decoy_is_disqualified_zero_false_positive_rate() {
        let suite = decoy_contenders();
        let results: Vec<(ScorecardV4, ExpectedVerdict)> = suite
            .entries
            .iter()
            .map(|(c, v)| (score_contender(c, &suite.store), *v))
            .collect();
        for (c, _) in &suite.entries {
            let sc = score_contender(c, &suite.store);
            assert!(
                sc.disqualified,
                "decoy {} must be disqualified; kill_reasons={:?}",
                c.name, sc.kill_reasons
            );
            assert_eq!(sc.total, 0.0, "decoy {} must score 0", c.name);
        }
        assert_eq!(decoy_false_positive_rate(&results), 0.0);
    }

    #[test]
    fn human_contenders_survive_with_finite_scorecards() {
        let suite = human_contenders();
        for (c, v) in &suite.entries {
            assert_eq!(*v, ExpectedVerdict::Survives);
            let sc = score_contender(c, &suite.store);
            assert!(
                !sc.disqualified,
                "human contender {} should survive; kill_reasons={:?}",
                c.name, sc.kill_reasons
            );
            assert!(
                sc.total > 0.0 && sc.total <= 100.0,
                "{} total {}",
                c.name,
                sc.total
            );
            assert!(sc.total.is_finite());
        }
    }

    #[test]
    fn unification_program_outscores_bare_lcdm_on_unification() {
        let suite = human_contenders();
        let score_of = |name: &str| -> ScorecardV4 {
            let (c, _) = suite.entries.iter().find(|(c, _)| c.name == name).unwrap();
            score_contender(c, &suite.store)
        };
        let lcdm = score_of("gr_lcdm");
        let uni = score_of("shared_parameter_program");
        let uni_pts = |sc: &ScorecardV4| {
            sc.components
                .iter()
                .find(|x| x.name == "unification")
                .map(|x| x.points)
                .unwrap_or(0.0)
        };
        assert!(
            uni_pts(&uni) > uni_pts(&lcdm),
            "shared-parameter program must earn more unification credit than bare ΛCDM"
        );
    }

    #[test]
    fn real_modification_is_distinct_and_outscores_lcdm_baselines() {
        let suite = human_contenders();
        let score_of = |name: &str| -> ScorecardV4 {
            let (c, _) = suite.entries.iter().find(|(c, _)| c.name == name).unwrap();
            score_contender(c, &suite.store)
        };
        let real = score_of("real_modification_program");
        let lcdm = score_of("gr_lcdm");
        let uni = score_of("shared_parameter_program");
        assert!(
            real.distinct_from_baseline,
            "the nDGP program must be physically distinct from ΛCDM"
        );
        assert!(
            !lcdm.distinct_from_baseline && !uni.distinct_from_baseline,
            "the ΛCDM-recovering baselines must be flagged not-distinct"
        );
        assert!(
            real.total > lcdm.total && real.total > uni.total,
            "a real modification ({}) must outscore the ΛCDM baselines ({}, {})",
            real.total,
            lcdm.total,
            uni.total
        );
    }

    #[test]
    fn decoy_false_positive_rate_is_zero_when_all_disqualified() {
        // A hand-built results set: two disqualified decoys ⇒ rate 0.
        let suite = decoy_contenders();
        let results: Vec<(ScorecardV4, ExpectedVerdict)> = suite
            .entries
            .iter()
            .take(2)
            .map(|(c, v)| (score_contender(c, &suite.store), *v))
            .collect();
        assert_eq!(decoy_false_positive_rate(&results), 0.0);
    }
}
