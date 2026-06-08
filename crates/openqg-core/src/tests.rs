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

    fn obs(observable_id: &str, value: f64, uncertainty: f64, unit: &str) -> ObservableRecord {
        ObservableRecord {
            observable_id: observable_id.into(),
            kind: "cosmology".into(),
            value,
            uncertainty,
            unit: unit.into(),
            source: None,
        }
    }

    fn pred(observable_id: &str, value: f64, uncertainty: f64, unit: &str) -> PredictionRecord {
        PredictionRecord {
            observable_id: observable_id.into(),
            value,
            uncertainty,
            unit: unit.into(),
            theory_id: None,
        }
    }

    // Phase 0 trust gate: the real scoring engine must discriminate a robust theory from a
    // broken one on a tension-bearing fixture where the baseline is NOT a perfect fit.
    // This is the foundation the whole adversarial-robustness redesign stands on: if a
    // known-better prediction set does not outrank a known-worse one, nothing downstream
    // (islands, judges, 1000 generations) can be trusted.
    #[test]
    fn tension_fixture_discriminates_robust_from_broken() {
        // Measured values: Planck CMB anchors + SH0ES local H0 + weak-lensing S8 + BBN.
        let observables = vec![
            obs("h0", 67.4, 0.5, "km s^-1 Mpc^-1"),
            obs("omega_m", 0.315, 0.007, "dimensionless"),
            obs("sum_mnu", 0.06, 0.02, "eV"),
            obs("h0_local", 73.04, 1.04, "km s^-1 Mpc^-1"),
            obs("s8", 0.776, 0.017, "dimensionless"),
            obs("n_eff", 2.99, 0.17, "dimensionless"),
            obs("omega_b_h2", 0.02237, 0.00015, "dimensionless"),
        ];

        // Baseline LambdaCDM has one H0, so it misses local H0 and S8 (the real tensions).
        let baseline_preds = vec![
            pred("h0", 67.4, 0.5, "km s^-1 Mpc^-1"),
            pred("omega_m", 0.315, 0.007, "dimensionless"),
            pred("sum_mnu", 0.06, 0.02, "eV"),
            pred("h0_local", 67.4, 0.5, "km s^-1 Mpc^-1"),
            pred("s8", 0.83, 0.013, "dimensionless"),
            pred("n_eff", 3.046, 0.17, "dimensionless"),
            pred("omega_b_h2", 0.02237, 0.00015, "dimensionless"),
        ];
        let (baseline, _) = score_metrics(&observables, &baseline_preds, 3, 0.0);
        assert!(
            baseline.log_likelihood < 0.0,
            "baseline must leave headroom (no perfect fit); LL = {}",
            baseline.log_likelihood
        );
        let baseline_ll = baseline.log_likelihood;

        // A robust candidate that resolves the H0 + S8 tensions (5 declared parameters).
        let resolver_preds = vec![
            pred("h0", 67.4, 0.5, "km s^-1 Mpc^-1"),
            pred("omega_m", 0.315, 0.007, "dimensionless"),
            pred("sum_mnu", 0.06, 0.02, "eV"),
            pred("h0_local", 73.0, 1.0, "km s^-1 Mpc^-1"),
            pred("s8", 0.78, 0.017, "dimensionless"),
            pred("n_eff", 3.0, 0.17, "dimensionless"),
            pred("omega_b_h2", 0.02237, 0.00015, "dimensionless"),
        ];
        let (resolver, _) = score_metrics(&observables, &resolver_preds, 5, baseline_ll);
        assert!(
            resolver.delta_log_likelihood > 0.0,
            "robust candidate must beat the baseline; delta = {}",
            resolver.delta_log_likelihood
        );
        assert!((resolver.coverage - 1.0).abs() < 1e-9);

        // A broken candidate (H0 = 100 and friends) must score far worse than the baseline.
        let junk_preds = vec![
            pred("h0", 100.0, 0.5, "km s^-1 Mpc^-1"),
            pred("omega_m", 0.6, 0.007, "dimensionless"),
            pred("sum_mnu", 1.0, 0.02, "eV"),
            pred("h0_local", 100.0, 0.5, "km s^-1 Mpc^-1"),
            pred("s8", 1.2, 0.017, "dimensionless"),
            pred("n_eff", 6.0, 0.17, "dimensionless"),
            pred("omega_b_h2", 0.05, 0.00015, "dimensionless"),
        ];
        let (junk, _) = score_metrics(&observables, &junk_preds, 3, baseline_ll);
        assert!(
            junk.delta_log_likelihood < 0.0,
            "broken candidate must not beat the baseline"
        );
        assert!(
            junk.log_likelihood < baseline_ll - 100.0,
            "broken candidate must be clearly bad; LL = {}",
            junk.log_likelihood
        );

        // An overfit echo (one free parameter per observable) fits perfectly but BIC must
        // rank it below the parsimonious resolver: parsimony, not raw fit, is rewarded.
        let overfit_preds = vec![
            pred("h0", 67.4, 0.5, "km s^-1 Mpc^-1"),
            pred("omega_m", 0.315, 0.007, "dimensionless"),
            pred("sum_mnu", 0.06, 0.02, "eV"),
            pred("h0_local", 73.04, 1.04, "km s^-1 Mpc^-1"),
            pred("s8", 0.776, 0.017, "dimensionless"),
            pred("n_eff", 2.99, 0.17, "dimensionless"),
            pred("omega_b_h2", 0.02237, 0.00015, "dimensionless"),
        ];
        let (overfit, _) = score_metrics(&observables, &overfit_preds, 7, baseline_ll);
        assert!(
            resolver.bic < overfit.bic,
            "BIC must reward parsimony over overfitting ({} vs {})",
            resolver.bic,
            overfit.bic
        );
        assert!(
            resolver.bic < baseline.bic,
            "robust parsimonious candidate must beat the baseline by BIC ({} vs {})",
            resolver.bic,
            baseline.bic
        );
    }
}
