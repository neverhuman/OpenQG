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
        assert!(checked >= 4, "expected at least 4 anchor artifacts, found {checked}");
    }
}
