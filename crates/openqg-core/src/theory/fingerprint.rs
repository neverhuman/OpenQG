//! Canonical claim fingerprint (V4 M2).
//!
//! A `claim_fingerprint` is a SHA-256 over the *physically meaningful* content of a [`Theory`],
//! computed in a canonical, order-independent way. It answers exactly one question the V3 audit
//! showed we could not answer: **did this generation produce something genuinely new, or did it
//! re-promote the same lineage?** Two theories that differ only in parameter ordering or `id`
//! share a fingerprint; two that differ in any physical value/structure do not. The evolution
//! engine uses fingerprint deltas to require real per-generation progress and non-root lineage.

use super::{Provenance, Theory};

/// Deterministic float rendering: full-precision scientific so a changed value always changes the
/// string, with a single canonical spelling for the GR/zero point and the non-finite cases.
fn f(v: f64) -> String {
    if v == 0.0 {
        // collapse +0.0/-0.0
        "0".to_string()
    } else if v.is_nan() {
        "nan".to_string()
    } else if v.is_infinite() {
        if v > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        }
    } else {
        format!("{v:.12e}")
    }
}

fn provenance_token(p: &Provenance) -> String {
    match p {
        Provenance::Fundamental => "fundamental".to_string(),
        Provenance::Derived {
            mechanism,
            certificate,
        } => {
            // mechanism text + whether a value-level certificate is bound (form vs value)
            let cert = match certificate {
                Some(c) => format!("cert:{}:{}:{}", c.relation, f(c.expected), f(c.tolerance)),
                None => "cert:none".to_string(),
            };
            format!("derived[{}|{}]", mechanism.trim(), cert)
        }
        Provenance::Free => "free".to_string(),
    }
}

/// Canonical SHA-256 fingerprint of a theory's physics. Order-independent over `parameters` and
/// `terms`; ignores the engineering `id`. Pure and deterministic.
pub fn claim_fingerprint(theory: &Theory) -> String {
    let mut s = String::with_capacity(512);

    // Parameters — sorted by symbol so ordering is irrelevant.
    let mut params: Vec<&super::Parameter> = theory.parameters.iter().collect();
    params.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    s.push_str("params:");
    for p in params {
        s.push_str(&format!(
            "{}={}@{};",
            p.symbol,
            f(p.value),
            provenance_token(&p.provenance)
        ));
    }

    // Action terms — sorted by name (structural identity, ordering irrelevant).
    let mut terms: Vec<&super::Term> = theory.terms.iter().collect();
    terms.sort_by(|a, b| a.name.cmp(&b.name));
    s.push_str("|terms:");
    for t in terms {
        s.push_str(&format!(
            "{}:d{}:l{};",
            t.name, t.mass_dimension, t.free_lorentz_indices
        ));
    }

    // α-basis deviations.
    let a = &theory.alpha;
    s.push_str(&format!(
        "|alpha:m{},b{},k{},t{}",
        f(a.alpha_m),
        f(a.alpha_b),
        f(a.alpha_k),
        f(a.alpha_t)
    ));

    // Scalar-sector stability.
    let st = &theory.stability;
    s.push_str(&format!(
        "|stab:kc{},qs{},cs{},hd{}",
        f(st.kinetic_coefficient),
        f(st.q_s),
        f(st.sound_speed_sq),
        st.has_nondegenerate_higher_derivatives
    ));

    // Screening mechanism + numeric recovery.
    s.push_str("|screen:");
    s.push_str(theory.screening.as_deref().unwrap_or("none"));
    s.push_str(&format!(
        ",rec{}",
        theory
            .screening_recovery
            .map(f)
            .unwrap_or_else(|| "none".into())
    ));

    // Background cosmology — canonical JSON (serde_json Map is sorted by default).
    let bg = serde_json::to_value(&theory.background)
        .map(|v| v.to_string())
        .unwrap_or_default();
    s.push_str("|bg:");
    s.push_str(&bg);

    crate::sha256_digest(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::{AlphaBasis, Parameter, Provenance};

    #[test]
    fn identical_theories_share_a_fingerprint() {
        let a = Theory::baseline_lcdm();
        let b = Theory::baseline_lcdm();
        assert_eq!(claim_fingerprint(&a), claim_fingerprint(&b));
        assert_eq!(claim_fingerprint(&a).len(), 64);
    }

    #[test]
    fn reordering_parameters_does_not_change_the_fingerprint() {
        let mut a = Theory::baseline_lcdm();
        let b_fp = claim_fingerprint(&a);
        a.parameters.reverse();
        assert_eq!(
            claim_fingerprint(&a),
            b_fp,
            "fingerprint must be order-independent"
        );
    }

    #[test]
    fn the_id_does_not_affect_the_fingerprint() {
        let mut a = Theory::baseline_lcdm();
        let fp = claim_fingerprint(&a);
        a.id = "totally-different-label".into();
        assert_eq!(claim_fingerprint(&a), fp);
    }

    #[test]
    fn a_changed_parameter_value_changes_the_fingerprint() {
        let mut a = Theory::baseline_lcdm();
        let fp = claim_fingerprint(&a);
        a.parameters[0].value += 1.0;
        assert_ne!(claim_fingerprint(&a), fp);
    }

    #[test]
    fn a_changed_alpha_or_screening_changes_the_fingerprint() {
        let base = Theory::baseline_lcdm();
        let fp = claim_fingerprint(&base);

        let mut t1 = base.clone();
        t1.alpha = AlphaBasis {
            alpha_m: 0.1,
            ..AlphaBasis::gr()
        };
        assert_ne!(claim_fingerprint(&t1), fp);

        let mut t2 = base.clone();
        t2.screening = Some("vainshtein".into());
        t2.screening_recovery = Some(0.999);
        assert_ne!(claim_fingerprint(&t2), fp);
    }

    #[test]
    fn form_vs_value_provenance_changes_the_fingerprint() {
        // Same value, different provenance (free vs fundamental) must be distinguishable.
        let mut t = Theory::baseline_lcdm();
        let fp = claim_fingerprint(&t);
        t.parameters.push(Parameter {
            symbol: "xi".into(),
            value: 0.1,
            physical_meaning: "test".into(),
            provenance: Provenance::Free,
        });
        let fp_free = claim_fingerprint(&t);
        assert_ne!(fp_free, fp);
        t.parameters.last_mut().unwrap().provenance = Provenance::derived("from symmetry");
        assert_ne!(claim_fingerprint(&t), fp_free);
    }
}
