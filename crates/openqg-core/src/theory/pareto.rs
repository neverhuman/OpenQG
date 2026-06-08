//! Pareto multi-objective dominance over candidate assessments.
//!
//! Selecting on a Pareto front of (data-fit, derivability, cross-domain unification, parsimony)
//! stops a candidate from winning by trading one objective off against another — a better curve
//! fit cannot buy its way past worse derivability or a broken domain, and extra parameters must
//! *earn* their complexity. This is the standard anti-overfit device (PySR / AI-Feynman 2.0,
//! `docs/research/automated-theory-discovery.md` §2).

use super::CandidateAssessment;

/// The objectives extracted from a candidate. Higher is better for every field (parsimony is the
/// negated parameter count), so dominance is a uniform "≥ on all, > on one".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Objectives {
    /// ε: data-fit improvement (−∞ if vetoed/unscored).
    pub epsilon: f64,
    /// β: derivation/consistency.
    pub beta: f64,
    /// Cross-domain unification score.
    pub unification: f64,
    /// Parsimony = −(parameter count): fewer parameters is better.
    pub parsimony: f64,
}

/// Extract the Pareto objectives from an assessment and a parameter count.
pub fn objectives(assessment: &CandidateAssessment, parameter_count: usize) -> Objectives {
    Objectives {
        epsilon: assessment
            .evaluation
            .epsilon_delta_log_likelihood
            .unwrap_or(f64::NEG_INFINITY),
        beta: assessment.evaluation.beta,
        unification: assessment.unification.score,
        parsimony: -(parameter_count as f64),
    }
}

/// True if `a` Pareto-dominates `b`: at least as good in every objective and strictly better in at
/// least one.
pub fn dominates(a: &Objectives, b: &Objectives) -> bool {
    let at_least_as_good = a.epsilon >= b.epsilon
        && a.beta >= b.beta
        && a.unification >= b.unification
        && a.parsimony >= b.parsimony;
    let strictly_better = a.epsilon > b.epsilon
        || a.beta > b.beta
        || a.unification > b.unification
        || a.parsimony > b.parsimony;
    at_least_as_good && strictly_better
}

/// Indices of the non-dominated (Pareto-front) members of a candidate set.
pub fn pareto_front(candidates: &[Objectives]) -> Vec<usize> {
    (0..candidates.len())
        .filter(|&i| {
            !candidates
                .iter()
                .enumerate()
                .any(|(j, o)| j != i && dominates(o, &candidates[i]))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn o(epsilon: f64, beta: f64, unification: f64, params: usize) -> Objectives {
        Objectives {
            epsilon,
            beta,
            unification,
            parsimony: -(params as f64),
        }
    }

    #[test]
    fn strict_domination_is_recognized() {
        let better = o(1.0, 0.9, 0.9, 3);
        let worse = o(0.0, 0.8, 0.8, 4);
        assert!(dominates(&better, &worse));
        assert!(!dominates(&worse, &better));
    }

    #[test]
    fn equal_objectives_do_not_dominate() {
        let a = o(1.0, 0.9, 0.9, 3);
        assert!(!dominates(&a, &a));
    }

    #[test]
    fn pareto_front_keeps_non_dominated_tradeoffs() {
        // A: best fit but more params; B: fewer params but worse fit; C: dominated by A.
        let cands = vec![
            o(2.0, 0.9, 0.9, 5), // A
            o(0.5, 0.9, 0.9, 2), // B (fewer params)
            o(1.0, 0.8, 0.8, 6), // C (dominated by A: worse on all)
        ];
        let front = pareto_front(&cands);
        assert!(front.contains(&0)); // A is non-dominated (best fit)
        assert!(front.contains(&1)); // B is non-dominated (most parsimonious)
        assert!(!front.contains(&2)); // C is dominated by A
    }

    #[test]
    fn a_vetoed_candidate_is_dominated() {
        let real = o(1.0, 0.9, 0.9, 3);
        let vetoed = o(f64::NEG_INFINITY, 0.5, 0.0, 4);
        assert!(dominates(&real, &vetoed));
        let front = pareto_front(&[real, vetoed]);
        assert_eq!(front, vec![0]);
    }
}
