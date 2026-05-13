#[cfg(test)]
mod tests {
    use crate::*;
    use std::collections::BTreeMap;

    fn sample_theory_manifest(command: &str) -> TheoryManifest {
        TheoryManifest {
            id: "example".into(),
            name: "Example".into(),
            status: "scaffolded".into(),
            benchmark_suite: "smoke".into(),
            description: "conservative".into(),
            citations: vec![Citation {
                title: "Paper".into(),
                authors: vec!["A".into()],
                year: 2024,
                url: "https://example.com".into(),
            }],
            observables: vec!["h0".into()],
            parameters: vec![ParameterSpec {
                name: "alpha".into(),
                symbol: "a".into(),
                unit: "dimensionless".into(),
                physical_meaning: "effective coupling".into(),
                prior_or_fixed_value: "fixed".into(),
                source_or_free_reason: "derived from literature".into(),
            }],
            adapter: TheoryAdapter {
                command: command.into(),
                predictions_path: None,
            },
        }
    }

    fn sample_dataset_manifest(local_cache_path: Option<&str>) -> DatasetManifest {
        DatasetManifest {
            id: "nist".into(),
            title: "NIST".into(),
            source_url: "https://example.com".into(),
            license: "CC0".into(),
            citation: "cite".into(),
            version: "1".into(),
            checksum_strategy: "sha256".into(),
            access_method: "manual".into(),
            expected_columns: vec!["name".into()],
            unit_map: BTreeMap::from([("name".into(), "1".into())]),
            suite: "constants".into(),
            local_cache_path: local_cache_path.map(ToOwned::to_owned),
        }
    }

    #[test]
    fn rejects_black_box_parameter_text() {
        let mut manifest = sample_theory_manifest("openqg-theory predict");
        manifest.parameters = vec![ParameterSpec {
            name: "latent knob".into(),
            symbol: "z".into(),
            unit: "dimensionless".into(),
            physical_meaning: "hidden latent fit parameter".into(),
            prior_or_fixed_value: "fixed".into(),
            source_or_free_reason: "because".into(),
        }];

        assert!(validate_theory_manifest(&manifest).is_err());
    }

    #[test]
    fn rejects_missing_units_for_dataset_manifest() {
        let mut manifest = sample_dataset_manifest(None);
        manifest.unit_map = BTreeMap::new();

        assert!(validate_dataset_manifest(&manifest).is_err());
    }

    #[test]
    fn rejects_non_cli_adapter_command() {
        let manifest = sample_theory_manifest("echo not a cli");

        assert!(validate_theory_manifest(&manifest).is_err());
    }

    #[test]
    fn accepts_prediction_records_with_uncertainty() {
        let prediction = PredictionRecord {
            observable_id: "h0".into(),
            value: 67.4,
            uncertainty: 0.5,
            unit: "km s^-1 Mpc^-1".into(),
            theory_id: Some("sm-gr-lcdm-mnu".into()),
        };

        assert!(validate_prediction_record(&prediction).is_ok());
    }

    #[test]
    fn validates_paper_question_records_with_route_metadata() {
        let route = RouteMetadata {
            request_id: Some("request".into()),
            route_mode: Some("fast".into()),
            primary_model_id: Some("primary".into()),
            backup_model_ids: Vec::new(),
            fusion_model_id: None,
            winner_model_id: Some("primary".into()),
            confidence: Some(0.9),
            provider: Some("provider".into()),
            model: Some("model".into()),
            agent_role: "auditor".into(),
            zyal_run_id: "run".into(),
            zyal_lane_id: "lane".into(),
        };
        let challenge = ChallengeRecord {
            challenge_hash: "6f6f4e70c5591c0a54d5d7062ed8d924479b12ab9c26bec78f8485291d54d4f0"
                .into(),
            publication_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .into(),
            rubric_version: "v1".into(),
            question: "What is alpha?".into(),
            answer_key: "one".into(),
            support_sections: vec!["s1".into()],
            context_pack: ContextPackSettings {
                strategy: "hard".into(),
                target_fill_ratio: 0.82,
                output_reserve_tokens: 4096,
                safe_window_tokens: None,
            },
            generator_agents: Vec::new(),
            blind_answer_attempts: Vec::new(),
            critic_attempts: Vec::new(),
            audit_attempts: vec![AgentAttemptRecord {
                agent_id: "a1".into(),
                role: "auditor".into(),
                answer: None,
                score: Some(1.0),
                route_metadata: route,
            }],
            acceptance: AcceptanceDecision {
                accepted: true,
                reason: "ok".into(),
            },
        };
        assert!(validate_challenge_record(&challenge).is_ok());
    }
}
