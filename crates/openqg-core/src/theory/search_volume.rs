//! V8 Phase 19 (#20): SearchVolumeSpec — Rust companion to `data/search-volume.yml`.
//!
//! Closes the SYNTHESIS #20 gap: the search volume is declared in YAML but the engine has no
//! machine-checkable type that can (a) represent a frozen parameter box and (b) assert whether a
//! theory's parameters fall inside the preregistered exclusion volume.
//!
//! The canonical V8 class is `C_growth-suppression(V8)`:
//!   μ0 ∈ [−0.30, 0], A_drag ∈ [0, 10], w0 ∈ [−0.99, −0.80],
//!   plus profiled background parameters (h, Ω_m, σ₈).
//! GW170817-safe and scale-independent gates are structural (not parameter-box checks).

use serde::{Deserialize, Serialize};

/// A single preregistered parameter bound from the search-volume YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchParamBox {
    /// Parameter symbol (e.g. `"mu0"`, `"A_drag"`).
    pub symbol: String,
    /// Inclusive lower bound.
    pub lo: f64,
    /// Inclusive upper bound.
    pub hi: f64,
    /// The mechanism route this parameter belongs to, or `None` for a background parameter.
    pub mechanism_route: Option<String>,
}

impl SearchParamBox {
    /// True when `value` is within `[lo, hi]` (inclusive, NaN-safe — NaN is out of scope).
    pub fn contains(&self, value: f64) -> bool {
        value.is_finite() && value >= self.lo && value <= self.hi
    }
}

/// A parameter value that falls outside its registered box — returned as an out-of-scope signal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutOfScopeViolation {
    /// Symbol of the out-of-scope parameter.
    pub symbol: String,
    /// The value that was checked.
    pub value: f64,
    /// The registered lower bound.
    pub lo: f64,
    /// The registered upper bound.
    pub hi: f64,
}

impl OutOfScopeViolation {
    /// Human-readable description of the violation.
    pub fn describe(&self) -> String {
        format!(
            "parameter '{}' = {} is outside the registered box [{}, {}]",
            self.symbol, self.value, self.lo, self.hi
        )
    }
}

/// Frozen search-volume specification for an exclusion campaign.
///
/// Corresponds to the structure in `data/search-volume.yml`. The `sealed` flag controls whether
/// post-freeze edits are detected — seal logic is enforced externally (via `ops/seal-forecast.sh`
/// and the associated git tag); this type provides the runtime check gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchVolumeSpec {
    /// Schema version string (e.g. `"v8.0.0"`).
    pub schema_version: String,
    /// Human-readable name for the exclusion class (e.g. `"C_growth-suppression(V8)"`).
    pub exclusion_class_name: String,
    /// Preregistered parameter boxes (mechanism-specific and background).
    pub parameter_boxes: Vec<SearchParamBox>,
    /// True when all theories in scope must be GW170817-safe (δc_T = 0).
    pub gw170717_safe: bool,
    /// True when scale-dependent growth modifications are excluded.
    pub scale_independent: bool,
    /// Upper redshift bound (theories affecting z > max_redshift are excluded from scope).
    pub max_redshift: f64,
}

impl SearchVolumeSpec {
    /// The canonical V8 growth-suppression search volume (hardcoded from `data/search-volume.yml`).
    ///
    /// Use when loading the YAML at runtime is not available (e.g. in unit tests).
    pub fn v8_growth_suppression() -> Self {
        SearchVolumeSpec {
            schema_version: "v8.0.0".into(),
            exclusion_class_name: "C_growth-suppression(V8)".into(),
            parameter_boxes: vec![
                SearchParamBox {
                    symbol: "mu0".into(),
                    lo: -0.30,
                    hi: 0.00,
                    mechanism_route: Some("planck_mu0".into()),
                },
                SearchParamBox {
                    symbol: "A_drag".into(),
                    lo: 0.0,
                    hi: 10.0,
                    mechanism_route: Some("dark_scattering_drag".into()),
                },
                SearchParamBox {
                    symbol: "w0".into(),
                    lo: -0.99,
                    hi: -0.80,
                    mechanism_route: Some("dark_scattering_drag".into()),
                },
                SearchParamBox {
                    symbol: "h".into(),
                    lo: 0.60,
                    hi: 0.80,
                    mechanism_route: None,
                },
                SearchParamBox {
                    symbol: "Omega_m".into(),
                    lo: 0.20,
                    hi: 0.45,
                    mechanism_route: None,
                },
                SearchParamBox {
                    symbol: "sigma8".into(),
                    lo: 0.60,
                    hi: 1.00,
                    mechanism_route: None,
                },
            ],
            gw170717_safe: true,
            scale_independent: true,
            max_redshift: 2.5,
        }
    }

    /// Look up the parameter box for `symbol`. Returns `None` if that symbol is not registered.
    pub fn box_for(&self, symbol: &str) -> Option<&SearchParamBox> {
        self.parameter_boxes.iter().find(|b| b.symbol == symbol)
    }

    /// True when a single `(symbol, value)` pair is inside its registered box (or if the symbol
    /// is not registered, no constraint applies — the theory is considered not-scoped for that
    /// parameter rather than out-of-scope).
    pub fn contains_param(&self, symbol: &str, value: f64) -> bool {
        self.box_for(symbol).map_or(true, |b| b.contains(value))
    }

    /// Check all registered parameter boxes against the supplied `(symbol, value)` pairs.
    ///
    /// Returns violations for every registered box whose symbol appears in `params` AND whose
    /// value falls outside `[lo, hi]`. Registered boxes not present in `params` are skipped
    /// (the theory does not claim that parameter).
    pub fn check_params(&self, params: &[(&str, f64)]) -> Vec<OutOfScopeViolation> {
        let mut violations = Vec::new();
        for &(symbol, value) in params {
            if let Some(b) = self.box_for(symbol) {
                if !b.contains(value) {
                    violations.push(OutOfScopeViolation {
                        symbol: symbol.into(),
                        value,
                        lo: b.lo,
                        hi: b.hi,
                    });
                }
            }
        }
        violations
    }

    /// True when every supplied parameter is within its registered box.
    pub fn in_scope(&self, params: &[(&str, f64)]) -> bool {
        self.check_params(params).is_empty()
    }

    /// All mechanism-specific parameter boxes (i.e. boxes with a non-None `mechanism_route`).
    pub fn mechanism_boxes(&self) -> Vec<&SearchParamBox> {
        self.parameter_boxes
            .iter()
            .filter(|b| b.mechanism_route.is_some())
            .collect()
    }

    /// All background parameter boxes (i.e. boxes with `mechanism_route = None`).
    pub fn background_boxes(&self) -> Vec<&SearchParamBox> {
        self.parameter_boxes
            .iter()
            .filter(|b| b.mechanism_route.is_none())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> SearchVolumeSpec {
        SearchVolumeSpec::v8_growth_suppression()
    }

    #[test]
    fn v8_spec_has_correct_class_name() {
        assert_eq!(spec().exclusion_class_name, "C_growth-suppression(V8)");
    }

    #[test]
    fn v8_spec_has_six_parameter_boxes() {
        assert_eq!(spec().parameter_boxes.len(), 6);
    }

    #[test]
    fn gw170817_safe_and_scale_independent_are_set() {
        let s = spec();
        assert!(s.gw170717_safe);
        assert!(s.scale_independent);
    }

    #[test]
    fn max_redshift_is_2p5() {
        assert!((spec().max_redshift - 2.5).abs() < 1e-12);
    }

    // ---- SearchParamBox::contains ----

    #[test]
    fn mu0_box_contains_minus_015() {
        let s = spec();
        let b = s.box_for("mu0").unwrap();
        assert!(b.contains(-0.15));
    }

    #[test]
    fn mu0_box_does_not_contain_positive() {
        let s = spec();
        let b = s.box_for("mu0").unwrap();
        assert!(
            !b.contains(0.01),
            "positive mu0 is out of scope (enhanced growth)"
        );
    }

    #[test]
    fn mu0_box_contains_boundary_values() {
        let s = spec();
        let b = s.box_for("mu0").unwrap();
        assert!(b.contains(-0.30));
        assert!(b.contains(0.00));
    }

    #[test]
    fn nan_is_always_out_of_scope() {
        let s = spec();
        let b = s.box_for("mu0").unwrap();
        assert!(!b.contains(f64::NAN));
    }

    #[test]
    fn a_drag_box_contains_zero_to_ten() {
        let s = spec();
        let b = s.box_for("A_drag").unwrap();
        assert!(b.contains(0.0));
        assert!(b.contains(5.5));
        assert!(b.contains(10.0));
        assert!(!b.contains(-0.1));
        assert!(!b.contains(10.1));
    }

    #[test]
    fn w0_box_rejects_values_near_minus_one() {
        let s = spec();
        let b = s.box_for("w0").unwrap();
        assert!(!b.contains(-1.0), "phantom w0 < -0.99 is out of scope");
        assert!(b.contains(-0.99));
        assert!(b.contains(-0.80));
        assert!(!b.contains(-0.79));
    }

    // ---- check_params / in_scope ----

    #[test]
    fn in_scope_with_valid_mu0() {
        assert!(spec().in_scope(&[("mu0", -0.10)]));
    }

    #[test]
    fn out_of_scope_with_positive_mu0() {
        let violations = spec().check_params(&[("mu0", 0.10)]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].symbol, "mu0");
        assert!(!spec().in_scope(&[("mu0", 0.10)]));
    }

    #[test]
    fn unregistered_symbol_is_not_a_violation() {
        let violations = spec().check_params(&[("alpha_t", 0.5)]);
        assert!(violations.is_empty(), "unregistered symbol should pass");
    }

    #[test]
    fn multiple_params_one_violation() {
        let violations = spec().check_params(&[("mu0", -0.10), ("A_drag", 15.0)]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].symbol, "A_drag");
    }

    #[test]
    fn all_in_scope_params_produce_no_violations() {
        let violations = spec().check_params(&[
            ("mu0", -0.15),
            ("h", 0.70),
            ("Omega_m", 0.30),
            ("sigma8", 0.80),
        ]);
        assert!(violations.is_empty());
    }

    #[test]
    fn violation_describe_is_informative() {
        let violations = spec().check_params(&[("mu0", 0.05)]);
        let msg = violations[0].describe();
        assert!(msg.contains("mu0") && msg.contains("0.05"), "{msg}");
    }

    #[test]
    fn mechanism_boxes_are_three() {
        assert_eq!(spec().mechanism_boxes().len(), 3);
    }

    #[test]
    fn background_boxes_are_three() {
        assert_eq!(spec().background_boxes().len(), 3);
    }

    #[test]
    fn serde_round_trip() {
        let s = spec();
        let json = serde_json::to_string(&s).unwrap();
        let back: SearchVolumeSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn out_of_scope_violation_serde_round_trip() {
        let v = OutOfScopeViolation {
            symbol: "mu0".into(),
            value: 0.05,
            lo: -0.30,
            hi: 0.00,
        };
        let json = serde_json::to_string(&v).unwrap();
        let back: OutOfScopeViolation = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }
}
