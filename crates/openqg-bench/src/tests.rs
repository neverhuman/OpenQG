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
            "benchmark-v0.1.1".into(),
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
            benchmark_version: "benchmark-v0.1.1".into(),
            scorecard_hash: "abc".into(),
            data_lock_hash: "def".into(),
            candidate_hashes: BTreeMap::new(),
            scorecard_path: "scorecard.json".into(),
            approved: false,
        };
        assert!(validate_release_manifest(&manifest).is_err());
    }
}
