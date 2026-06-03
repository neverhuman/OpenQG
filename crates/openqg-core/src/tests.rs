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
}
