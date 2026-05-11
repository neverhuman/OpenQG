use openqg_core::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn prediction_records_with_finite_values_validate(
        value in -1.0e6f64..1.0e6,
        uncertainty in 0.0f64..1.0e6,
    ) {
        let record = PredictionRecord {
            observable_id: "h0".into(),
            value,
            uncertainty,
            unit: "km s^-1 Mpc^-1".into(),
            theory_id: Some("sm-gr-lcdm-mnu".into()),
        };

        prop_assert!(validate_prediction_record(&record).is_ok());
    }

    #[test]
    fn gaussian_log_likelihood_is_finite_for_positive_sigma(
        observed in -1.0e6f64..1.0e6,
        predicted in -1.0e6f64..1.0e6,
        sigma in 1.0e-6f64..1.0e6,
    ) {
        let value = gaussian_log_likelihood(observed, predicted, sigma);

        prop_assert!(value.is_finite());
    }
}
