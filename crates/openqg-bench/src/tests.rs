#[cfg(test)]
mod tests {
    use openqg_core::*;
    use std::collections::BTreeMap;

    #[test]
    fn scorecard_rewards_perfect_coverage() {
        let observables = vec![ObservableRecord {
            observable_id: "h0".into(),
            kind: "cosmology".into(),
            value: 67.4,
            uncertainty: 0.5,
            unit: "km s^-1 Mpc^-1".into(),
            source: None,
        }];
        let predictions = vec![PredictionRecord {
            observable_id: "h0".into(),
            value: 67.4,
            uncertainty: 0.5,
            unit: "km s^-1 Mpc^-1".into(),
            theory_id: Some("sm-gr-lcdm-mnu".into()),
        }];
        let scorecard = scorecard_from_predictions(
            "benchmark-v0.1.0".into(),
            "smoke".into(),
            "sm-gr-lcdm-mnu".into(),
            "sm-gr-lcdm-mnu".into(),
            4,
            &observables,
            &predictions,
            0.0,
            "0".into(),
        );
        let (score, status) = repo_score_from_scorecard(&scorecard);
        assert!(score >= 85);
        assert_eq!(status, "pass");
    }

    #[test]
    fn validate_release_manifest_requires_candidates() {
        let manifest = ReleaseManifest {
            version: "v1".into(),
            benchmark_version: "benchmark-v0.1.0".into(),
            scorecard_hash: "abc".into(),
            data_lock_hash: "def".into(),
            candidate_hashes: BTreeMap::new(),
            scorecard_path: "scorecard.json".into(),
            approved: false,
        };
        assert!(validate_release_manifest(&manifest).is_err());
    }

    // V8 Wave 0.3: decoy fixture regression tests — each fixture in tests-fixtures/decoy/
    // must be rejected (disqualified/vetoed). These lock the S03/S06 jailgun gaps.

    #[test]
    fn decoy_right_named_no_action_is_vetoed() {
        // decoy-right-named-no-action.json: correct term name/dimension, but mechanism = "".
        // proposal_into_theory demotes mu0 to Free → FreeParameter veto kills it.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests-fixtures/decoy/decoy-right-named-no-action.json");
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let receipt = openqg_core::proposal_receipt(&json, "fixture:decoy-right-named-no-action");
        assert!(
            receipt.vetoed,
            "decoy-right-named-no-action must be vetoed; reasons={:?}",
            receipt.veto_reasons
        );
        assert!(
            receipt.demoted_parameters.contains(&"mu0".to_string()),
            "mu0 (empty mechanism) must be demoted: {:?}",
            receipt.demoted_parameters
        );
    }

    #[test]
    fn decoy_fit_only_claim_is_vetoed() {
        // decoy-fit-only-claim.json: w0 and wa both have provenance "fit" (unrecognised →
        // Free) → FreeParameter veto kills the theory before reaching the claims linter.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests-fixtures/decoy/decoy-fit-only-claim.json");
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let receipt = openqg_core::proposal_receipt(&json, "fixture:decoy-fit-only-claim");
        assert!(
            receipt.vetoed,
            "decoy-fit-only-claim must be vetoed; reasons={:?}",
            receipt.veto_reasons
        );
    }

    #[test]
    fn claims_linter_rejects_fit_only_claim_graph() {
        // The claims linter (Wave 0.8) catches BEATS_LCDM_WITHOUT_TRIALS_CORRECTION +
        // NO_EVIDENCE_HASH for a claim asserting fit-set BIC superiority.
        let stmt = "This theory beats ΛCDM with ΔBIC = -6.2 on the fit-set BAO+CMB data";
        let report = openqg_core::validation::lint_claims(&openqg_core::ClaimGraph {
            claims: vec![openqg_core::Claim {
                id: "claim-beats-lcdm-fit-only".into(),
                sector: openqg_core::Sector::Background,
                kind: openqg_core::ClaimKind::Physics,
                statement: stmt.into(),
                evidence: vec![],
                obligations: vec!["ob".into()],
                depends_on: vec![],
            }],
        });
        assert!(
            report.fatal,
            "fit-only-claim must be flagged by the claims linter"
        );
        assert!(
            report.findings.iter().any(|f| f.rule == "NO_EVIDENCE_HASH"),
            "expected NO_EVIDENCE_HASH; got {:?}",
            report.findings
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.rule == "BEATS_LCDM_WITHOUT_TRIALS_CORRECTION"),
            "expected BEATS_LCDM_WITHOUT_TRIALS_CORRECTION"
        );
    }

    // V8 Phase 1 integration tests

    #[test]
    fn forward_tier_background_model_is_t1_not_promotion_grade() {
        use openqg_core::cosmology::BackgroundForwardModel;
        use openqg_core::cosmology::ForwardModel;
        let m = BackgroundForwardModel.manifest();
        assert_eq!(m.tier, openqg_core::cosmology::ForwardTier::T1Emulator);
        assert!(!m.tier.is_promotion_grade());
        assert!(!m.tier.is_publication_grade());
    }

    #[test]
    fn data_tier_sealed_entries_recognized() {
        use openqg_core::validation::{DataTier, DataTierManifest};
        let m = DataTierManifest::new("desi-dr3-bao-v1", DataTier::FutureSealed);
        assert!(m.tier.is_sealed());
        assert!(m.tier.earns_novelty_credit());
        let m2 = DataTierManifest::new("planck-pr4-tt", DataTier::OpenFit);
        assert!(!m2.tier.is_sealed());
        assert!(!m2.tier.earns_novelty_credit());
    }

    #[test]
    fn value_firewall_blocks_sealed_dataset_reference() {
        use openqg_core::validation::check_value_firewall;
        use std::collections::BTreeSet;
        let sealed = BTreeSet::from(["future-euclid-wl".to_string()]);
        let packet = serde_json::json!({
            "theory_id": "ndgp",
            "training_data": "future-euclid-wl"
        });
        let report = check_value_firewall(&sealed, &packet);
        assert!(
            report.blocked,
            "firewall must block sealed dataset reference"
        );
    }

    #[test]
    fn search_ledger_trials_gate_grows_with_n() {
        use openqg_core::theory::SearchLedger;
        let mut l1 = SearchLedger::new();
        l1.record_evaluation();
        let mut l100 = SearchLedger::new();
        for _ in 0..100 {
            l100.record_evaluation();
        }
        assert!(l100.standard_gate().threshold() > l1.standard_gate().threshold());
    }

    #[test]
    fn engine_kpis_healthy_check() {
        use openqg_core::validation::EngineKpis;
        let healthy = EngineKpis::new(200, 200, 120, 12);
        assert!(healthy.is_healthy());
        let not_enough_replays = EngineKpis::new(200, 150, 120, 12);
        assert!(!not_enough_replays.is_healthy());
    }

    // Phase 0: the on-disk tension fixture must parse, leave real headroom over the
    // baseline, and correctly calibrate the anchor/decoy set via the real scoring engine.
    #[test]
    fn tension_fixture_files_have_headroom_and_calibrate_anchors() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let observables: Vec<ObservableRecord> =
            crate::util::read_jsonl(&root.join("data/fixtures/tension/observables.jsonl")).unwrap();
        let baseline_preds: Vec<PredictionRecord> =
            crate::util::read_jsonl(&root.join("data/fixtures/tension/baseline-predictions.jsonl"))
                .unwrap();
        assert!(observables.len() >= 5, "tension suite must be informative");
        assert!(
            observables.len() > 3,
            "observable_count must exceed the baseline free-param count so BIC bites"
        );

        let (baseline, _) = score_metrics(&observables, &baseline_preds, 3, 0.0);
        assert!(
            baseline.log_likelihood < 0.0,
            "baseline must leave headroom (LL = {})",
            baseline.log_likelihood
        );
        let baseline_ll = baseline.log_likelihood;

        let mut checked = 0;
        for entry in std::fs::read_dir(root.join("ZYAL/anchors")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let v: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let outcome = v["expected_anchor_outcome"].as_str().unwrap_or("");
            let preds: Vec<PredictionRecord> =
                serde_json::from_value(v["predictions"].clone()).unwrap();
            let k = v["parameter_count"].as_u64().unwrap_or(3) as usize;
            let (m, _) = score_metrics(&observables, &preds, k, baseline_ll);
            let artifact: crate::zyal_robustness::TheoryArtifact =
                serde_json::from_value(v.clone()).unwrap();
            match outcome {
                "die" => assert!(
                    m.delta_log_likelihood < 0.0 || !artifact.whitebox_violations().is_empty(),
                    "decoy {:?} must be killed by physics or the whitebox gate (delta = {})",
                    path.file_name().unwrap(),
                    m.delta_log_likelihood
                ),
                "survive" => assert!(
                    m.delta_log_likelihood >= 0.0,
                    "good anchor {:?} must not be worse than the baseline (delta = {})",
                    path.file_name().unwrap(),
                    m.delta_log_likelihood
                ),
                _ => {}
            }
            checked += 1;
        }
        assert!(
            checked >= 4,
            "expected at least 4 anchor artifacts, found {checked}"
        );
    }
}
