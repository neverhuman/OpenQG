//! V5: **truth-binding** — make a theory's certified modified-gravity claims drive the background
//! the forward model actually integrates.
//!
//! The V4.1 gap: a theory could carry a *verified* certificate `geff_over_g = 7/6` (nDGP β=2) while
//! its `background` stayed plain ΛCDM (`mg_family = None`), so the forward model computed GR growth
//! — the claim had consequences for the rubric (distinctness, novelty) but none for the data. This
//! module closes the gap with a pure, idempotent binder: verified MG-relation certificates are
//! translated into the background fields the growth code consumes (`mg_family`, `ndgp_omega_rc`,
//! `fr_n`/`fr_log10_fr0`, `mu0`), with explicit precedence, conflict detection, and three new kill
//! classes:
//!
//! - **Unimplemented**: distinct via certificates, but the bound background still computes GR
//!   growth (an unbindable relation or a domain error) — claims must have computable consequences.
//! - **Unexplained**: a non-GR background field backed by *no* verified MG certificate — an
//!   uncertified modification is a fitting knob in disguise (FreeParameter severity).
//! - **Conflicting**: relation-derived and declared values for the same field disagree — the theory
//!   asserts two inconsistent values of one physical constant.
//!
//! The binder never touches the expansion history (MG here modifies growth, not E(a)), so binding
//! before prediction is exact and idempotent. It also computes the *novel-prediction truth audit*:
//! the witnesses' declared numbers are checked against the model-computed values for the bound
//! background, so distinctness and honesty are machine-computed, never declared.

use serde::{Deserialize, Serialize};

use super::obligation::{DerivationObligation, DerivationObligationKind};
use super::{Provenance, Theory, VetoReason};
use crate::cosmology::{
    canonicalize_observable_id, BackgroundForwardModel, CosmologyParams, ForwardModel, MgFamily,
};

/// Numeric agreement tolerance for reconciling two sources of one background field.
fn values_agree(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 + 1e-3 * a.abs()
}

/// One background field set by the binder, with its provenance and any fidelity caveat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldBinding {
    /// Background field name (`mu0`, `ndgp_omega_rc`, `fr_n`, `fr_log10_fr0`, `mg_family`).
    pub field: String,
    /// The bound numeric value (for `mg_family`, an encoding: 0 none / 1 fr / 2 ndgp).
    pub value: f64,
    /// Where it came from: `relation:<name>` or `param:<symbol>`.
    pub source: String,
    /// Honest caveat when the binding is an approximation (e.g. coupled-DE → μ0 amplitude-only).
    #[serde(default)]
    pub fidelity: Option<String>,
}

/// Serializable summary of one binding pass (embedded in the scorecard for the audit trail).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BindingReport {
    pub bindings: Vec<FieldBinding>,
    /// Human-readable mirrors of the typed binding vetoes (for the serialized record).
    pub veto_notes: Vec<String>,
    /// True iff the bound background computes non-GR growth.
    pub bound_non_gr: bool,
}

/// Full outcome: the bound theory, the serializable report, and the typed kill vetoes.
#[derive(Debug, Clone)]
pub struct BindingOutcome {
    pub theory: Theory,
    pub report: BindingReport,
    pub vetoes: Vec<VetoReason>,
}

/// Scan for a *verified* certificate on `relation` among the theory's derived parameters.
fn verified_cert<'a>(
    theory: &'a Theory,
    relation: &str,
) -> Option<(&'a str, &'a super::DerivedCertificate)> {
    theory.parameters.iter().find_map(|p| match &p.provenance {
        Provenance::Derived {
            certificate: Some(c),
            ..
        } if c.relation == relation && c.verify() => Some((p.symbol.as_str(), c)),
        _ => None,
    })
}

fn cert_input(cert: &super::DerivedCertificate, name: &str) -> Option<f64> {
    cert.inputs.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
}

/// Does this background compute non-GR growth? (the exact conditions the growth code branches on)
fn background_non_gr(bg: &CosmologyParams) -> bool {
    bg.mu0.abs() > 1e-12
        || (bg.mg_family == MgFamily::Ndgp && bg.ndgp_omega_rc > 0.0)
        || (bg.mg_family == MgFamily::FrHuSawicki && bg.fr_log10_fr0 > -20.0)
}

/// Bind a theory's certified modified-gravity claims into its background. Pure and idempotent:
/// binding a bound theory is a no-op (the relation re-derives the same values, which then agree
/// with the already-set fields).
pub fn bind_modified_background(theory: &Theory) -> BindingOutcome {
    let mut bound = theory.clone();
    let mut bindings: Vec<FieldBinding> = Vec::new();
    let mut vetoes: Vec<VetoReason> = Vec::new();
    // field → (value, source) set by a relation this pass (precedence over direct symbols).
    let mut relation_set: Vec<(&'static str, f64)> = Vec::new();
    // Snapshot the *declared* background before we mutate, for reconciliation at the end.
    let declared = theory.background.clone();

    // --- 1. Relation-certificate bindings (authoritative) -------------------------------------
    // nDGP: geff_over_g = 1 + 1/(3β) with β at a=1 from the theory's own expansion history.
    if let Some((symbol, cert)) = verified_cert(theory, "ndgp_geff_over_g") {
        match cert_input(cert, "beta") {
            Some(beta) if beta.is_finite() && beta > 1.0 => {
                // Invert β(a=1) = 1 + (E₁/√Ω_rc)(1 + D₁/3) — the exact form NdgpParams::beta
                // evaluates — using the theory's own background (E₁ = e_of_z(0) = 1 by
                // normalization; D₁ = dlnE/dN at a=1).
                let e1 = theory.background.e_of_z(0.0);
                let d1 = theory.background.dln_e_dn(1.0);
                let factor = e1 * (1.0 + d1 / 3.0);
                let omega_rc = (factor / (beta - 1.0)).powi(2);
                if omega_rc.is_finite() && omega_rc > 0.0 {
                    bound.background.mg_family = MgFamily::Ndgp;
                    bound.background.ndgp_omega_rc = omega_rc;
                    relation_set.push(("ndgp_omega_rc", omega_rc));
                    relation_set.push(("mg_family", 2.0));
                    bindings.push(FieldBinding {
                        field: "ndgp_omega_rc".into(),
                        value: omega_rc,
                        source: "relation:ndgp_geff_over_g".into(),
                        fidelity: None,
                    });
                } else {
                    vetoes.push(VetoReason::UnimplementedModification {
                        relation: "ndgp_geff_over_g".into(),
                        symbol: symbol.to_string(),
                        detail: format!("β={beta} inverts to unusable Ω_rc={omega_rc}"),
                    });
                }
            }
            Some(beta) => {
                vetoes.push(VetoReason::UnimplementedModification {
                    relation: "ndgp_geff_over_g".into(),
                    symbol: symbol.to_string(),
                    detail: format!(
                        "β={beta} is outside the normal-branch domain (require finite β > 1; \
                         β ≤ 1 is the ghost branch / unbindable)"
                    ),
                });
            }
            None => vetoes.push(VetoReason::UnimplementedModification {
                relation: "ndgp_geff_over_g".into(),
                symbol: symbol.to_string(),
                detail: "certificate carries no 'beta' input".into(),
            }),
        }
    }

    // nDGP direct: a verified ndgp_beta_from_omega_rc certificate carries Ω_rc explicitly.
    if let Some((_symbol, cert)) = verified_cert(theory, "ndgp_beta_from_omega_rc") {
        if let Some(omega_rc) = cert_input(cert, "omega_rc") {
            if omega_rc.is_finite() && omega_rc > 0.0 {
                if let Some((_, prev)) = relation_set.iter().find(|(f, _)| *f == "ndgp_omega_rc") {
                    if !values_agree(*prev, omega_rc) {
                        vetoes.push(VetoReason::ConflictingModification {
                            field: "ndgp_omega_rc".into(),
                            certificate_value: *prev,
                            declared_value: omega_rc,
                        });
                    }
                } else {
                    bound.background.mg_family = MgFamily::Ndgp;
                    bound.background.ndgp_omega_rc = omega_rc;
                    relation_set.push(("ndgp_omega_rc", omega_rc));
                    relation_set.push(("mg_family", 2.0));
                    bindings.push(FieldBinding {
                        field: "ndgp_omega_rc".into(),
                        value: omega_rc,
                        source: "relation:ndgp_beta_from_omega_rc".into(),
                        fidelity: None,
                    });
                }
            }
        }
    }

    // f(R) Hu–Sawicki: invert the tracking solution for (n, log10|f_R0|).
    if let Some((symbol, cert)) = verified_cert(theory, "fr_alpha_m") {
        let f_r = cert_input(cert, "f_R");
        let a_f_r_prime = cert_input(cert, "a_f_R_prime");
        match (f_r, a_f_r_prime) {
            (Some(f_r), Some(a_f_r_prime))
                if f_r < 0.0 && a_f_r_prime < 0.0 && f_r.abs() > 1e-20 =>
            {
                let om = theory.background.omega_m;
                let ode = (1.0 - om - theory.background.omega_k).max(1e-9);
                let n = (a_f_r_prime / f_r) * (om + 4.0 * ode) / (3.0 * om) - 1.0;
                if n.is_finite() && n > 0.0 {
                    let log10_fr0 = f_r.abs().log10();
                    bound.background.mg_family = MgFamily::FrHuSawicki;
                    bound.background.fr_n = n;
                    bound.background.fr_log10_fr0 = log10_fr0;
                    relation_set.push(("fr_n", n));
                    relation_set.push(("fr_log10_fr0", log10_fr0));
                    relation_set.push(("mg_family", 1.0));
                    bindings.push(FieldBinding {
                        field: "fr_log10_fr0".into(),
                        value: log10_fr0,
                        source: "relation:fr_alpha_m".into(),
                        fidelity: None,
                    });
                    bindings.push(FieldBinding {
                        field: "fr_n".into(),
                        value: n,
                        source: "relation:fr_alpha_m".into(),
                        fidelity: None,
                    });
                } else {
                    vetoes.push(VetoReason::UnimplementedModification {
                        relation: "fr_alpha_m".into(),
                        symbol: symbol.to_string(),
                        detail: format!("tracking inversion gives unusable n={n}"),
                    });
                }
            }
            _ => vetoes.push(VetoReason::UnimplementedModification {
                relation: "fr_alpha_m".into(),
                symbol: symbol.to_string(),
                detail: "inputs outside the Hu–Sawicki tracking domain (need f_R < 0, \
                         a_f_R_prime < 0, |f_R| > 1e-20)"
                    .into(),
            }),
        }
    }

    // Coupled DE: amplitude-today mapped onto the scale-free μ0 path (honest approximation).
    if let Some((_symbol, cert)) = verified_cert(theory, "coupled_de_geff_over_g") {
        if let Some(beta) = cert_input(cert, "beta") {
            if beta.is_finite() {
                let mu0 = 2.0 * beta * beta;
                bound.background.mu0 = mu0;
                relation_set.push(("mu0", mu0));
                bindings.push(FieldBinding {
                    field: "mu0".into(),
                    value: mu0,
                    source: "relation:coupled_de_geff_over_g".into(),
                    fidelity: Some(
                        "amplitude-only: G_eff/G(a=1) matched; time dependence approximated by \
                         the Planck-2018 μ0 parametrization"
                            .into(),
                    ),
                });
            }
        }
    }

    // Planck μ0 parametrization: direct (this IS the parametrization the growth code implements).
    if let Some((_symbol, cert)) = verified_cert(theory, "planck_mu0_geff") {
        if let Some(mu0) = cert_input(cert, "mu0") {
            if mu0.is_finite() && mu0 > -1.0 {
                if let Some((_, prev)) = relation_set.iter().find(|(f, _)| *f == "mu0") {
                    if !values_agree(*prev, mu0) {
                        vetoes.push(VetoReason::ConflictingModification {
                            field: "mu0".into(),
                            certificate_value: *prev,
                            declared_value: mu0,
                        });
                    }
                } else {
                    bound.background.mu0 = mu0;
                    relation_set.push(("mu0", mu0));
                    bindings.push(FieldBinding {
                        field: "mu0".into(),
                        value: mu0,
                        source: "relation:planck_mu0_geff".into(),
                        fidelity: None,
                    });
                }
            }
        }
    }

    // --- 2. Direct-symbol copies (only where no relation set the field) -----------------------
    // A Fundamental or certified-Derived parameter whose symbol exactly names an MG background
    // field is copied in; an uncertified Derived or Free parameter is never a binding source.
    for p in &theory.parameters {
        let eligible = match &p.provenance {
            Provenance::Fundamental => true,
            Provenance::Derived {
                certificate: Some(c),
                ..
            } => c.verify(),
            _ => false,
        };
        if !eligible || !p.value.is_finite() {
            continue;
        }
        let relation_owned = |f: &str| relation_set.iter().any(|(rf, _)| *rf == f);
        match p.symbol.as_str() {
            "mu0" => {
                if relation_owned("mu0") {
                    let prev = relation_set
                        .iter()
                        .find(|(f, _)| *f == "mu0")
                        .map(|(_, v)| *v)
                        .unwrap_or(0.0);
                    if !values_agree(prev, p.value) {
                        vetoes.push(VetoReason::ConflictingModification {
                            field: "mu0".into(),
                            certificate_value: prev,
                            declared_value: p.value,
                        });
                    }
                } else if p.value.abs() > 1e-12 {
                    bound.background.mu0 = p.value;
                    bindings.push(FieldBinding {
                        field: "mu0".into(),
                        value: p.value,
                        source: format!("param:{}", p.symbol),
                        fidelity: None,
                    });
                }
            }
            "ndgp_omega_rc" => {
                if relation_owned("ndgp_omega_rc") {
                    let prev = relation_set
                        .iter()
                        .find(|(f, _)| *f == "ndgp_omega_rc")
                        .map(|(_, v)| *v)
                        .unwrap_or(0.0);
                    if !values_agree(prev, p.value) {
                        vetoes.push(VetoReason::ConflictingModification {
                            field: "ndgp_omega_rc".into(),
                            certificate_value: prev,
                            declared_value: p.value,
                        });
                    }
                } else if p.value > 0.0 {
                    bound.background.mg_family = MgFamily::Ndgp;
                    bound.background.ndgp_omega_rc = p.value;
                    bindings.push(FieldBinding {
                        field: "ndgp_omega_rc".into(),
                        value: p.value,
                        source: format!("param:{}", p.symbol),
                        fidelity: None,
                    });
                }
            }
            "fr_log10_fr0" => {
                if !relation_owned("fr_log10_fr0") && p.value > -20.0 {
                    bound.background.mg_family = MgFamily::FrHuSawicki;
                    bound.background.fr_log10_fr0 = p.value;
                    bindings.push(FieldBinding {
                        field: "fr_log10_fr0".into(),
                        value: p.value,
                        source: format!("param:{}", p.symbol),
                        fidelity: None,
                    });
                }
            }
            "fr_n" => {
                if !relation_owned("fr_n") && (p.value - 1.0).abs() > 1e-12 {
                    bound.background.fr_n = p.value;
                    bindings.push(FieldBinding {
                        field: "fr_n".into(),
                        value: p.value,
                        source: format!("param:{}", p.symbol),
                        fidelity: None,
                    });
                }
            }
            _ => {}
        }
    }

    // --- 3. Reconcile the *declared* background against what binding produced ------------------
    // A declared non-GR field that no binding source explains is an uncertified modification.
    let explained = |field: &str| {
        relation_set.iter().any(|(f, _)| *f == field) || bindings.iter().any(|b| b.field == field)
    };
    if declared.mu0.abs() > 1e-12 {
        if explained("mu0") {
            if !values_agree(bound.background.mu0, declared.mu0) {
                vetoes.push(VetoReason::ConflictingModification {
                    field: "mu0".into(),
                    certificate_value: bound.background.mu0,
                    declared_value: declared.mu0,
                });
            }
        } else {
            vetoes.push(VetoReason::UnexplainedModification {
                field: "mu0".into(),
                value: declared.mu0,
            });
        }
    }
    if declared.mg_family == MgFamily::Ndgp && declared.ndgp_omega_rc > 0.0 {
        if explained("ndgp_omega_rc") {
            if !values_agree(bound.background.ndgp_omega_rc, declared.ndgp_omega_rc) {
                vetoes.push(VetoReason::ConflictingModification {
                    field: "ndgp_omega_rc".into(),
                    certificate_value: bound.background.ndgp_omega_rc,
                    declared_value: declared.ndgp_omega_rc,
                });
            }
        } else {
            vetoes.push(VetoReason::UnexplainedModification {
                field: "ndgp_omega_rc".into(),
                value: declared.ndgp_omega_rc,
            });
        }
    }
    if declared.mg_family == MgFamily::FrHuSawicki && declared.fr_log10_fr0 > -20.0 {
        if explained("fr_log10_fr0") {
            if !values_agree(bound.background.fr_log10_fr0, declared.fr_log10_fr0) {
                vetoes.push(VetoReason::ConflictingModification {
                    field: "fr_log10_fr0".into(),
                    certificate_value: bound.background.fr_log10_fr0,
                    declared_value: declared.fr_log10_fr0,
                });
            }
        } else {
            vetoes.push(VetoReason::UnexplainedModification {
                field: "fr_log10_fr0".into(),
                value: declared.fr_log10_fr0,
            });
        }
    }

    // --- 4. Unimplemented check: distinct via certs, but nothing non-GR was bound --------------
    let bound_non_gr = background_non_gr(&bound.background);
    if !bound_non_gr && vetoes.is_empty() {
        // Same cert-side distinctness test the scorecard uses: a verified certificate on a
        // modification relation whose value departs from that relation's GR-limit value.
        let distinct_via_cert = theory.parameters.iter().any(|p| {
            if let Provenance::Derived {
                certificate: Some(c),
                ..
            } = &p.provenance
            {
                if let Some(gr) = super::certificate::relation_gr_value(&c.relation) {
                    return c.verify() && (c.expected - gr).abs() > 1e-6;
                }
            }
            false
        });
        if distinct_via_cert {
            vetoes.push(VetoReason::UnimplementedModification {
                relation: "(unbindable)".into(),
                symbol: "(claims modification)".into(),
                detail: "theory is distinct via verified certificates but the bound background \
                         still computes GR growth — the claim has no computable consequence"
                    .into(),
            });
        }
    }

    let report = BindingReport {
        bindings,
        veto_notes: vetoes.iter().map(|v| format!("{v:?}")).collect(),
        bound_non_gr,
    };
    BindingOutcome {
        theory: bound,
        report,
        vetoes,
    }
}

/// One novel-prediction witness checked against the machine-computed truth: the model's prediction
/// for the *bound* theory vs the ΛCDM baseline. Distinctness and honesty are computed, not declared.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NovelPredictionAudit {
    pub claim_id: String,
    pub observable_raw: String,
    pub observable_canonical: Option<String>,
    pub declared_predicted: f64,
    pub declared_baseline: f64,
    pub computed_predicted: Option<f64>,
    pub computed_baseline: Option<f64>,
    pub min_detectable: f64,
    /// |computed_predicted − computed_baseline| ≥ min_detectable (None ⇒ uncomputable).
    pub computed_distinct: Option<bool>,
    /// Declared predicted AND baseline within `tolerance` of the computed values (None ⇒ uncomputable).
    pub honest: Option<bool>,
    /// Honesty tolerance — the witness's own claimed experimental resolution.
    pub tolerance: f64,
}

/// Audit every NovelPrediction obligation against the forward model: predictions are computed for
/// the bound background and the Planck-ΛCDM baseline; the declared numbers must agree with the
/// computed truth within the witness's own `min_detectable`.
pub fn audit_novel_predictions(
    bound_background: &CosmologyParams,
    obligations: &[DerivationObligation],
) -> Vec<NovelPredictionAudit> {
    let model = BackgroundForwardModel;
    let baseline = CosmologyParams::planck_lcdm();
    let mut audits = Vec::new();
    for o in obligations {
        if o.kind != DerivationObligationKind::NovelPrediction {
            continue;
        }
        let Some(w) = &o.novel else { continue };
        let canonical = canonicalize_observable_id(&w.observable).map(|c| c.to_id());
        let (computed_predicted, computed_baseline) = match &canonical {
            Some(id) => {
                let ids = vec![id.clone()];
                let predict = |bg: &CosmologyParams| -> Option<f64> {
                    model
                        .predict(bg, &ids)
                        .ok()
                        .and_then(|preds| preds.first().map(|p| p.value))
                        .filter(|v| v.is_finite())
                };
                (predict(bound_background), predict(&baseline))
            }
            None => (None, None),
        };
        let tolerance = w.min_detectable;
        let (computed_distinct, honest) = match (computed_predicted, computed_baseline) {
            (Some(cp), Some(cb)) => (
                Some((cp - cb).abs() >= w.min_detectable),
                Some((w.predicted - cp).abs() <= tolerance && (w.baseline - cb).abs() <= tolerance),
            ),
            _ => (None, None),
        };
        audits.push(NovelPredictionAudit {
            claim_id: o.claim_id.clone(),
            observable_raw: w.observable.clone(),
            observable_canonical: canonical,
            declared_predicted: w.predicted,
            declared_baseline: w.baseline,
            computed_predicted,
            computed_baseline,
            min_detectable: w.min_detectable,
            computed_distinct,
            honest,
            tolerance,
        });
    }
    audits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::{DerivedCertificate, NovelPredictionWitness, Parameter};

    fn theory_with_cert(relation: &str, inputs: &[(&str, f64)], expected: f64) -> Theory {
        let mut t = Theory::baseline_lcdm();
        let cert = DerivedCertificate {
            relation: relation.into(),
            inputs: inputs.iter().map(|(n, v)| (n.to_string(), *v)).collect(),
            expected,
            tolerance: 1e-6,
        };
        t.parameters.push(Parameter {
            symbol: "bound_param".into(),
            value: expected,
            physical_meaning: "test MG parameter".into(),
            provenance: Provenance::derived_certified("test mechanism", cert),
        });
        t
    }

    #[test]
    fn ndgp_beta_2_binds_omega_rc_and_model_recovers_geff_7_6() {
        // β = 2 ⇒ G_eff/G = 7/6, and the inversion must make the growth code see exactly that.
        let t = theory_with_cert("ndgp_geff_over_g", &[("beta", 2.0)], 1.0 + 1.0 / 6.0);
        let out = bind_modified_background(&t);
        assert!(out.vetoes.is_empty(), "{:?}", out.vetoes);
        assert_eq!(out.theory.background.mg_family, MgFamily::Ndgp);
        let omega_rc = out.theory.background.ndgp_omega_rc;
        assert!(omega_rc > 0.0 && omega_rc.is_finite());
        // Roundtrip through the actual growth machinery: μ(a=1) must be ≈ 7/6.
        let bg = &out.theory.background;
        let e1 = bg.e_of_z(0.0);
        let d1 = bg.dln_e_dn(1.0);
        let ndgp = crate::theory::sectors::ndgp::NdgpParams::from_omega_rc(omega_rc);
        let mu = ndgp.response(e1, d1).mu;
        assert!(
            (mu - (1.0 + 1.0 / 6.0)).abs() < 1e-6,
            "model μ(a=1) = {mu}, want 7/6"
        );
        assert!(out.report.bound_non_gr);
    }

    #[test]
    fn ghost_branch_beta_is_unimplemented_kill() {
        let t = theory_with_cert("ndgp_geff_over_g", &[("beta", -2.0)], 1.0 - 1.0 / 6.0);
        let out = bind_modified_background(&t);
        assert!(out
            .vetoes
            .iter()
            .any(|v| matches!(v, VetoReason::UnimplementedModification { .. })));
        assert!(!out.report.bound_non_gr);
    }

    #[test]
    fn planck_mu0_negative_binds_suppressed_growth() {
        let t = theory_with_cert("planck_mu0_geff", &[("mu0", -0.1)], 0.9);
        let out = bind_modified_background(&t);
        assert!(out.vetoes.is_empty(), "{:?}", out.vetoes);
        assert!((out.theory.background.mu0 - (-0.1)).abs() < 1e-12);
        assert!(out.report.bound_non_gr);
    }

    #[test]
    fn binding_is_idempotent_and_lcdm_is_a_noop() {
        let base = Theory::baseline_lcdm();
        let out = bind_modified_background(&base);
        assert!(out.vetoes.is_empty());
        assert_eq!(out.theory.background, base.background);
        assert!(!out.report.bound_non_gr);

        let t = theory_with_cert("planck_mu0_geff", &[("mu0", -0.1)], 0.9);
        let once = bind_modified_background(&t);
        let twice = bind_modified_background(&once.theory);
        assert!(twice.vetoes.is_empty(), "{:?}", twice.vetoes);
        assert_eq!(once.theory.background, twice.theory.background);
    }

    #[test]
    fn conflicting_cert_and_declared_omega_rc_is_a_kill() {
        // The live-proposal-#2 regression: β=2 cert (⇒ Ω_rc ≈ 0.71) + declared Ω_rc = 0.0625.
        let mut t = theory_with_cert("ndgp_geff_over_g", &[("beta", 2.0)], 1.0 + 1.0 / 6.0);
        t.parameters.push(Parameter {
            symbol: "ndgp_omega_rc".into(),
            value: 0.0625,
            physical_meaning: "declared crossover".into(),
            provenance: Provenance::Fundamental,
        });
        let out = bind_modified_background(&t);
        assert!(
            out.vetoes
                .iter()
                .any(|v| matches!(v, VetoReason::ConflictingModification { .. })),
            "{:?}",
            out.vetoes
        );
    }

    #[test]
    fn uncertified_background_mu0_is_unexplained_kill() {
        let mut t = Theory::baseline_lcdm();
        t.background.mu0 = 0.5; // no certificate anywhere
        let out = bind_modified_background(&t);
        assert!(
            out.vetoes
                .iter()
                .any(|v| matches!(v, VetoReason::UnexplainedModification { .. })),
            "{:?}",
            out.vetoes
        );
    }

    #[test]
    fn regime_flag_only_fr_is_unimplemented() {
        // fr_largescale_geff_over_g carries no amplitude: distinct via cert, nothing bindable.
        let t = theory_with_cert("fr_largescale_geff_over_g", &[("regime", 1.0)], 4.0 / 3.0);
        let out = bind_modified_background(&t);
        assert!(
            out.vetoes
                .iter()
                .any(|v| matches!(v, VetoReason::UnimplementedModification { .. })),
            "{:?}",
            out.vetoes
        );
    }

    #[test]
    fn direct_fundamental_omega_rc_binds_without_relation() {
        let mut t = Theory::baseline_lcdm();
        t.parameters.push(Parameter {
            symbol: "ndgp_omega_rc".into(),
            value: 0.25,
            physical_meaning: "fundamental crossover".into(),
            provenance: Provenance::Fundamental,
        });
        let out = bind_modified_background(&t);
        assert!(out.vetoes.is_empty(), "{:?}", out.vetoes);
        assert_eq!(out.theory.background.mg_family, MgFamily::Ndgp);
        assert!((out.theory.background.ndgp_omega_rc - 0.25).abs() < 1e-12);
        assert!(out.report.bound_non_gr);
    }

    #[test]
    fn audit_computes_distinctness_and_honesty() {
        // Bind a suppressed-growth theory and audit a witness against the computed truth.
        let t = theory_with_cert("planck_mu0_geff", &[("mu0", -0.1)], 0.9);
        let out = bind_modified_background(&t);
        let model = BackgroundForwardModel;
        let id = vec!["fsigma8@0.51".to_string()];
        let cp = model
            .predict(&out.theory.background, &id)
            .unwrap()
            .first()
            .unwrap()
            .value;
        let cb = model
            .predict(&CosmologyParams::planck_lcdm(), &id)
            .unwrap()
            .first()
            .unwrap()
            .value;
        assert!(cp < cb, "μ0 < 0 must suppress growth: {cp} vs {cb}");

        let honest_witness = DerivationObligation {
            claim_id: "ob-novel".into(),
            kind: DerivationObligationKind::NovelPrediction,
            detail: "suppressed fσ8".into(),
            certificate: None,
            limit: None,
            citation: None,
            novel: Some(NovelPredictionWitness {
                observable: "fsigma8_z051".into(), // witness grammar → canonicalizes to @0.51
                predicted: cp,
                baseline: cb,
                min_detectable: 0.005,
                falsifier: "DESI/Euclid RSD".into(),
            }),
        };
        let audits = audit_novel_predictions(&out.theory.background, &[honest_witness.clone()]);
        assert_eq!(audits.len(), 1);
        let a = &audits[0];
        assert_eq!(a.observable_canonical.as_deref(), Some("fsigma8@0.51"));
        assert_eq!(a.honest, Some(true));
        assert_eq!(a.computed_distinct, Some((cp - cb).abs() >= 0.005));

        // A dishonest declaration (fabricated big deviation) is flagged, not silently accepted.
        let mut lying = honest_witness;
        lying.novel.as_mut().unwrap().predicted = cb + 0.2;
        let audits = audit_novel_predictions(&out.theory.background, &[lying]);
        assert_eq!(audits[0].honest, Some(false));

        // An unverifiable observable is None-audited (uncomputable, not an error).
        let mut unknown = DerivationObligation {
            claim_id: "ob-x".into(),
            kind: DerivationObligationKind::NovelPrediction,
            detail: "x".into(),
            certificate: None,
            limit: None,
            citation: None,
            novel: Some(NovelPredictionWitness {
                observable: "cl_tt_l220".into(),
                predicted: 1.0,
                baseline: 2.0,
                min_detectable: 0.1,
                falsifier: "CMB".into(),
            }),
        };
        let audits = audit_novel_predictions(&out.theory.background, &[unknown.clone()]);
        assert_eq!(audits[0].observable_canonical, None);
        assert_eq!(audits[0].honest, None);
        unknown.claim_id.clear(); // silence unused-mut lint paranoia in some toolchains
    }
}
