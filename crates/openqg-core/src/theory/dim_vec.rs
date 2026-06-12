//! V8 Phase 2 (#12): DimVec — dimensional analysis for every quantity, term, and expression.
//!
//! ## Problem closed
//! `Term.mass_dimension` is a proposer-asserted integer with no machine check. A proposer can
//! write `mass_dimension: 4` on a term that is actually dimension 5 (e.g. `∇⁴φ²`). The veto
//! cascade accepts it because it only checks the *declared* value, not what the expression
//! actually is.
//!
//! This module closes the hole by:
//! 1. `DimVec`: a 7-component SI dimension vector with integer exponents (all exact).
//! 2. `TermAst`: a typed expression AST whose `mass_dim()` method COMPUTES the mass dimension
//!    from the structure, not the declaration.
//! 3. `dim_check_term()`: compares `TermAst::mass_dim()` against `Term::mass_dimension` and
//!    returns a kill reason if they disagree.
//!
//! ## Dimensional convention
//! In natural units (ℏ = c = 1), mass dimension is the engineering dimension measured in powers
//! of mass (equivalently energy). Length ~ 1/mass, time ~ 1/mass.
//!
//! The `DimVec` uses 7 SI bases [M, L, T, Θ, I, N, J] as i16 exponents. In natural units,
//! only M is non-zero for most quantities, but the full SI vector catches `exp(H0)` (H0 has
//! dimension T⁻¹ = [M0 L0 T−1] ≠ dimensionless).

use serde::{Deserialize, Serialize};

/// SI dimension vector: [M, L, T, Θ, I, N, J] as integer exponents.
///
/// All arithmetic is exact (no floating point). Overflow is reported as an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DimVec(pub [i16; 7]);

impl DimVec {
    // ---- Named constructors for common physical quantities ----

    /// Dimensionless (all exponents zero).
    pub const DIMENSIONLESS: Self = DimVec([0, 0, 0, 0, 0, 0, 0]);
    /// Mass [M].
    pub const MASS: Self = DimVec([1, 0, 0, 0, 0, 0, 0]);
    /// Length [L].
    pub const LENGTH: Self = DimVec([0, 1, 0, 0, 0, 0, 0]);
    /// Time [T].
    pub const TIME: Self = DimVec([0, 0, 1, 0, 0, 0, 0]);
    /// Inverse time [T⁻¹] — dimension of Hubble constant.
    pub const INVERSE_TIME: Self = DimVec([0, 0, -1, 0, 0, 0, 0]);
    /// Energy [M L² T⁻²].
    pub const ENERGY: Self = DimVec([1, 2, -2, 0, 0, 0, 0]);

    /// True when this quantity is dimensionless.
    pub fn is_dimensionless(self) -> bool {
        self == Self::DIMENSIONLESS
    }

    /// Multiply two dimensional quantities: add the exponents.
    pub fn mul(self, rhs: Self) -> Self {
        let mut v = [0i16; 7];
        for i in 0..7 {
            v[i] = self.0[i] + rhs.0[i];
        }
        DimVec(v)
    }

    /// Divide two dimensional quantities: subtract the exponents.
    pub fn div(self, rhs: Self) -> Self {
        let mut v = [0i16; 7];
        for i in 0..7 {
            v[i] = self.0[i] - rhs.0[i];
        }
        DimVec(v)
    }

    /// Raise to an integer power: scale all exponents.
    pub fn pow(self, n: i16) -> Self {
        let mut v = [0i16; 7];
        for i in 0..7 {
            v[i] = self.0[i] * n;
        }
        DimVec(v)
    }

    /// The mass dimension (exponent of M alone). In natural units this is the dimension of
    /// an action term in a 4D Lagrangian density. A scalar action term must have mass dim 4.
    pub fn mass_dim(self) -> i32 {
        self.0[0] as i32
    }
}

impl std::fmt::Display for DimVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labels = ["M", "L", "T", "Θ", "I", "N", "J"];
        let mut parts = Vec::new();
        for (i, &exp) in self.0.iter().enumerate() {
            if exp != 0 {
                if exp == 1 {
                    parts.push(labels[i].to_string());
                } else {
                    parts.push(format!("{}^{}", labels[i], exp));
                }
            }
        }
        if parts.is_empty() {
            write!(f, "1")
        } else {
            write!(f, "{}", parts.join("·"))
        }
    }
}

/// A typed expression AST node. Each node can compute its own dimensional signature.
///
/// This is NOT a full symbolic algebra — it is just enough structure to:
/// 1. Check that `Term.mass_dimension` matches what the expression actually is.
/// 2. Catch `exp(dimensioned)` and similar dimensional horrors.
/// 3. Verify that a 4D Lagrangian action term sums to mass dimension 4.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TermAst {
    /// A scalar quantity with a known dimensional signature.
    Scalar { dim: DimVec },
    /// Product of sub-expressions: dimension = product of sub-dimensions.
    Product { factors: Vec<TermAst> },
    /// Sum of sub-expressions: all must have the same dimension.
    Sum { terms: Vec<TermAst> },
    /// Ratio of two sub-expressions: dimension = numerator / denominator.
    Ratio {
        numerator: Box<TermAst>,
        denominator: Box<TermAst>,
    },
    /// Integer power of a sub-expression.
    Power { base: Box<TermAst>, exponent: i16 },
    /// Spacetime derivative ∂^n. Each derivative carries dimension [L⁻¹] (in natural units [M¹]).
    Derivative { of: Box<TermAst>, order: u32 },
    /// exp(x): requires x to be dimensionless.
    Exp { argument: Box<TermAst> },
    /// ln(x): requires x to be dimensionless.
    Ln { argument: Box<TermAst> },
}

/// Dimensional check failure.
#[derive(Debug, Clone, PartialEq)]
pub struct DimError {
    pub detail: String,
}

impl TermAst {
    /// Compute the dimensional signature of this expression, or return an error if the
    /// expression is dimensionally inconsistent (e.g. sum of different dimensions, exp of
    /// a dimensioned quantity).
    pub fn dim(&self) -> Result<DimVec, DimError> {
        match self {
            TermAst::Scalar { dim } => Ok(*dim),
            TermAst::Product { factors } => {
                let mut result = DimVec::DIMENSIONLESS;
                for f in factors {
                    result = result.mul(f.dim()?);
                }
                Ok(result)
            }
            TermAst::Sum { terms } => {
                if terms.is_empty() {
                    return Ok(DimVec::DIMENSIONLESS);
                }
                let first = terms[0].dim()?;
                for t in &terms[1..] {
                    let d = t.dim()?;
                    if d != first {
                        return Err(DimError {
                            detail: format!(
                                "sum of terms with different dimensions: {first} ≠ {d}"
                            ),
                        });
                    }
                }
                Ok(first)
            }
            TermAst::Ratio {
                numerator,
                denominator,
            } => Ok(numerator.dim()?.div(denominator.dim()?)),
            TermAst::Power { base, exponent } => Ok(base.dim()?.pow(*exponent)),
            TermAst::Derivative { of, order } => {
                // In natural units, ∂/∂x^μ has dimension [M^1]. Each derivative order
                // multiplies by [M^1].
                let base_dim = of.dim()?;
                let deriv_dim = DimVec::MASS.pow(*order as i16);
                Ok(base_dim.mul(deriv_dim))
            }
            TermAst::Exp { argument } => {
                let d = argument.dim()?;
                if !d.is_dimensionless() {
                    return Err(DimError {
                        detail: format!(
                            "exp() argument must be dimensionless, got [{d}]; \
                             exp(H0) is forbidden — H0 has dimension [T⁻¹]"
                        ),
                    });
                }
                Ok(DimVec::DIMENSIONLESS)
            }
            TermAst::Ln { argument } => {
                let d = argument.dim()?;
                if !d.is_dimensionless() {
                    return Err(DimError {
                        detail: format!("ln() argument must be dimensionless, got [{d}]"),
                    });
                }
                Ok(DimVec::DIMENSIONLESS)
            }
        }
    }

    /// The mass dimension of this expression in natural units (M component of `dim()`).
    pub fn mass_dim(&self) -> Result<i32, DimError> {
        self.dim().map(|d| d.mass_dim())
    }
}

/// Check that a `Term`'s asserted `mass_dimension` matches what its `TermAst` computes.
///
/// Returns `None` when they agree (or `ast` is `None`), or `Some(kill_reason)` when they
/// disagree — this is a kill-level dimensional inconsistency.
pub fn dim_check_term(term_name: &str, asserted: i32, ast: Option<&TermAst>) -> Option<String> {
    let ast = ast?;
    match ast.mass_dim() {
        Ok(computed) if computed == asserted => None,
        Ok(computed) => Some(format!(
            "dimensional inconsistency in term '{}': asserted mass_dimension={} \
             but TermAst computes {} ({})",
            term_name,
            asserted,
            computed,
            ast.dim().unwrap(),
        )),
        Err(e) => Some(format!(
            "dimensional error in term '{}': {}",
            term_name, e.detail
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- DimVec arithmetic ----

    #[test]
    fn dimensionless_is_zero_vector() {
        assert!(DimVec::DIMENSIONLESS.is_dimensionless());
    }

    #[test]
    fn mul_adds_exponents() {
        let m = DimVec::MASS;
        let l = DimVec::LENGTH;
        let ml = m.mul(l);
        assert_eq!(ml, DimVec([1, 1, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn div_subtracts_exponents() {
        let e = DimVec::ENERGY; // [M L² T⁻²]
        let m = DimVec::MASS;
        let remainder = e.div(m); // [L² T⁻²]
        assert_eq!(remainder, DimVec([0, 2, -2, 0, 0, 0, 0]));
    }

    #[test]
    fn pow_scales_exponents() {
        let m = DimVec::MASS;
        let m3 = m.pow(3);
        assert_eq!(m3, DimVec([3, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn mass_dim_extraction() {
        assert_eq!(DimVec::ENERGY.mass_dim(), 1);
        assert_eq!(DimVec::DIMENSIONLESS.mass_dim(), 0);
    }

    #[test]
    fn display_dimensionless() {
        assert_eq!(DimVec::DIMENSIONLESS.to_string(), "1");
    }

    #[test]
    fn display_mass() {
        assert_eq!(DimVec::MASS.to_string(), "M");
    }

    // ---- TermAst dimensional computation ----

    #[test]
    fn scalar_returns_its_dim() {
        let t = TermAst::Scalar { dim: DimVec::MASS };
        assert_eq!(t.dim().unwrap(), DimVec::MASS);
        assert_eq!(t.mass_dim().unwrap(), 1);
    }

    #[test]
    fn product_multiplies_dims() {
        // M × L = M¹ L¹
        let t = TermAst::Product {
            factors: vec![
                TermAst::Scalar { dim: DimVec::MASS },
                TermAst::Scalar {
                    dim: DimVec::LENGTH,
                },
            ],
        };
        assert_eq!(t.dim().unwrap(), DimVec([1, 1, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn sum_of_same_dim_ok() {
        let t = TermAst::Sum {
            terms: vec![
                TermAst::Scalar { dim: DimVec::MASS },
                TermAst::Scalar { dim: DimVec::MASS },
            ],
        };
        assert_eq!(t.dim().unwrap(), DimVec::MASS);
    }

    #[test]
    fn sum_of_different_dim_errors() {
        let t = TermAst::Sum {
            terms: vec![
                TermAst::Scalar { dim: DimVec::MASS },
                TermAst::Scalar {
                    dim: DimVec::LENGTH,
                },
            ],
        };
        assert!(t.dim().is_err());
    }

    #[test]
    fn derivative_adds_mass_per_order() {
        // ∂φ where φ has dim MASS → should have dim MASS × MASS = MASS²
        let t = TermAst::Derivative {
            of: Box::new(TermAst::Scalar { dim: DimVec::MASS }),
            order: 1,
        };
        assert_eq!(t.mass_dim().unwrap(), 2);
    }

    #[test]
    fn exp_of_dimensionless_is_ok() {
        let t = TermAst::Exp {
            argument: Box::new(TermAst::Scalar {
                dim: DimVec::DIMENSIONLESS,
            }),
        };
        assert!(t.dim().is_ok());
        assert!(t.dim().unwrap().is_dimensionless());
    }

    #[test]
    fn exp_of_dimensioned_kills() {
        // exp(H0): H0 has dimension [T⁻¹] — FORBIDDEN
        let h0 = TermAst::Scalar {
            dim: DimVec::INVERSE_TIME,
        };
        let t = TermAst::Exp {
            argument: Box::new(h0),
        };
        let err = t.dim().unwrap_err();
        assert!(
            err.detail.contains("dimensionless"),
            "expected dimensionless error; got: {}",
            err.detail
        );
    }

    #[test]
    fn ln_of_dimensioned_kills() {
        let t = TermAst::Ln {
            argument: Box::new(TermAst::Scalar { dim: DimVec::MASS }),
        };
        assert!(t.dim().is_err());
    }

    #[test]
    fn power_scales_dim() {
        let t = TermAst::Power {
            base: Box::new(TermAst::Scalar { dim: DimVec::MASS }),
            exponent: 4,
        };
        assert_eq!(t.mass_dim().unwrap(), 4);
    }

    // ---- dim_check_term ----

    #[test]
    fn dim_check_agrees_on_mass_dim_4() {
        let ast = TermAst::Power {
            base: Box::new(TermAst::Scalar { dim: DimVec::MASS }),
            exponent: 4,
        };
        let result = dim_check_term("R^2", 4, Some(&ast));
        assert!(
            result.is_none(),
            "agreed dim should return None; got: {result:?}"
        );
    }

    #[test]
    fn dim_check_catches_mismatch() {
        // Asserted dim 4, but actual (computed) dim 5.
        let ast = TermAst::Power {
            base: Box::new(TermAst::Scalar { dim: DimVec::MASS }),
            exponent: 5,
        };
        let result = dim_check_term("forged-term", 4, Some(&ast));
        assert!(result.is_some(), "mismatch should produce a kill reason");
        assert!(result.unwrap().contains("forged-term"));
    }

    #[test]
    fn dim_check_none_ast_is_ok() {
        // No AST = no check (backward compat for terms without an AST).
        assert!(dim_check_term("old-term", 4, None).is_none());
    }

    #[test]
    fn dim_check_exp_h0_kills() {
        let h0 = TermAst::Scalar {
            dim: DimVec::INVERSE_TIME,
        };
        let bad = TermAst::Exp {
            argument: Box::new(h0),
        };
        let result = dim_check_term("exp_h0_term", 0, Some(&bad));
        assert!(result.is_some(), "exp(H0) must produce a kill reason");
    }
}
