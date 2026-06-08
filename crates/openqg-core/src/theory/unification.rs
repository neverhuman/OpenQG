//! Cross-domain "unification" likelihoods — independent published constraints that link
//! cosmology to gravity and particle physics.
//!
//! A theory that merely fits cosmological distances is not "unified". A credible candidate must
//! simultaneously survive the *independent* data sets that probe other domains. This channel
//! replaces the old analytic self-consistency heuristics (alpha_T scale, a screening flag, a
//! duplicated Y_p, a toy siren ratio) with **real, separately-published constraint blocks**, each
//! returning a Gaussian log-likelihood of the theory's prediction against a literature measurement
//! (the cited values + theory-side predictions live in [`super::unification_data`]):
//!
//! 1. GW170817 tensor speed `|c_T/c - 1| < ~1e-15`            (Abbott et al. 2017, 1710.05835).
//! 2. LVK standard-siren `H0 = 70 +12/-8 km/s/Mpc`            (Abbott et al. 2017, 1710.05835).
//! 3. Solar-system PPN Cassini `gamma - 1 = (2.1 +/- 2.3)e-5` (Bertotti, Iess & Tortora 2003).
//! 4. Equivalence principle MICROSCOPE `eta = (-1 +/- 2)e-15` (Touboul et al. 2022, 2201.03889).
//! 5. BBN primordial deuterium `D/H = (2.527 +/- 0.030)e-5`   (Cooke, Pettini & Steidel 2018).
//!
//! Every one of these is a **null test**: GR / standard ΛCDM sits at the centre of each
//! measurement (or comfortably inside it), so the literature value is a *consistency surface*, not
//! a discovery. The blocks therefore act as **veto surfaces**: a theory whose prediction violates
//! one is penalised (`score` collapses to 0, the domain is no longer "within tolerance", and
//! [`UnificationReport::is_unified`] is false), but *passing* a null gives **no frontier credit** —
//! the per-block score saturates at 1, it cannot push the report above the GR baseline. Earning
//! genuine "unification" credit would require a predicted *signed, mechanism-backed, non-null*
//! deviation, which is the v3.1 frontier and is deliberately not rewarded here.

use super::unification_data as data;
use super::Theory;

/// One independent cross-domain constraint block. The public shape (`domain`, `score`, `note`,
/// `passed()`) is kept compatible with the previous version; `log_likelihood`, `within_tolerance`,
/// and `is_null_test` are additive.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainCheck {
    pub domain: &'static str,
    /// Soft consistency score in `[0,1]` (0 = hard inconsistency, 1 = comfortably consistent).
    /// Within tolerance it is `exp(log_likelihood)` clamped to `[0,1]`, so a prediction at the
    /// measurement centre scores 1 and one near the tolerance edge scores lower; *outside*
    /// tolerance it collapses to exactly 0 (a hard veto surface). Passing a null *caps* at 1 — it
    /// buys no frontier credit.
    pub score: f64,
    /// Gaussian log-likelihood of the theory's prediction under this published measurement.
    pub log_likelihood: f64,
    /// True iff the prediction lies within the block's tolerance (`|residual| <= n_sigma * sigma`).
    pub within_tolerance: bool,
    /// Every block here is a null test (a consistency surface), so passing it grants no frontier
    /// credit. Recorded so downstream code can never mistake a passed null for evidence of new
    /// physics.
    pub is_null_test: bool,
    pub note: String,
}

impl DomainCheck {
    /// A block "passes" iff the theory is within its tolerance (the veto-surface predicate).
    pub fn passed(&self) -> bool {
        self.within_tolerance
    }
}

/// The cross-domain unification report.
#[derive(Debug, Clone, PartialEq)]
pub struct UnificationReport {
    pub checks: Vec<DomainCheck>,
    /// Overall consistency score in `[0,1]` = the weakest domain (unification is gated by its worst
    /// sector — no trading one domain off against another). A null-test channel: 1 means "fully
    /// consistent with every published constraint", and there is no way to score above 1.
    pub score: f64,
}

impl UnificationReport {
    /// True only if every block is within tolerance (every published constraint is satisfied).
    pub fn is_unified(&self) -> bool {
        self.checks.iter().all(DomainCheck::passed)
    }

    /// The independent `(domain, log_likelihood)` contributions, as the milestone requires. These
    /// are *not* summed into the cosmology fit (they live in a different objective); a downstream
    /// consumer can sum them for a joint cross-domain likelihood if desired.
    pub fn log_likelihood_contributions(&self) -> Vec<(&'static str, f64)> {
        self.checks
            .iter()
            .map(|c| (c.domain, c.log_likelihood))
            .collect()
    }

    /// Total cross-domain log-likelihood (sum of the independent blocks).
    pub fn total_log_likelihood(&self) -> f64 {
        self.checks.iter().map(|c| c.log_likelihood).sum()
    }
}

/// Gaussian log-likelihood `-0.5 (residual/sigma)^2` (matches `scoring::likelihood`).
fn gaussian_log_likelihood(residual: f64, sigma: f64) -> f64 {
    if !sigma.is_finite() || sigma <= 0.0 {
        return f64::NEG_INFINITY;
    }
    -0.5 * (residual * residual) / (sigma * sigma)
}

/// Number of sigmas at which a null block flips from "within tolerance" to "violated".
const TOLERANCE_NSIGMA: f64 = 3.0;

/// Build one null-test block from a residual and sigma. Inside tolerance the score is
/// `exp(logL)` clamped to `[0,1]` (so it caps at 1 — passing a null buys no frontier credit — and
/// decays smoothly toward the tolerance edge). **Outside tolerance the block is a hard veto
/// surface**: the score collapses to exactly `0.0` (not a vanishing `exp(logL)` epsilon), so a
/// violated constraint deterministically gates `is_unified()` and any score gated on this value.
fn null_block(domain: &'static str, residual: f64, sigma: f64, note: String) -> DomainCheck {
    let log_likelihood = gaussian_log_likelihood(residual, sigma);
    let within_tolerance = sigma > 0.0 && residual.abs() <= TOLERANCE_NSIGMA * sigma;
    let score = if within_tolerance {
        log_likelihood.exp().clamp(0.0, 1.0)
    } else {
        0.0
    };
    DomainCheck {
        domain,
        score,
        log_likelihood,
        within_tolerance,
        is_null_test: true,
        note,
    }
}

/// Score a theory across the independent cross-domain likelihood blocks.
pub fn unification_report(theory: &Theory) -> UnificationReport {
    let mut checks = Vec::new();

    // 1. GW170817 tensor speed: c_T/c - 1 must vanish to ~1e-15.
    let ct = data::predicted_ct_excess(theory);
    checks.push(null_block(
        "gw170817_tensor_speed",
        ct, // residual vs the null value 0
        data::CT_EXCESS_SIGMA,
        format!("c_T/c-1 = {ct:.2e} vs GW170817 |c_T/c-1| < few x 1e-15"),
    ));

    // 2. LVK standard-siren H0 (broad, independent). Asymmetric interval → pick the sigma on the
    //    side the prediction falls. GR/ΛCDM H0 ≈ 67–70 sits comfortably inside this wide null.
    let h0 = theory.background.h0();
    let resid = h0 - data::SIREN_H0;
    let sigma = if resid >= 0.0 {
        data::SIREN_H0_SIGMA_HI
    } else {
        data::SIREN_H0_SIGMA_LO
    };
    checks.push(null_block(
        "lvk_standard_siren_h0",
        resid,
        sigma,
        format!("H0 = {h0:.1} vs LVK siren 70 (+12/-8) km/s/Mpc"),
    ));

    // 3. Cassini PPN gamma. GR ⇒ gamma-1 = 0; unscreened modified gravity grossly violates it.
    let gamma_m1 = data::predicted_gamma_minus_one(theory);
    checks.push(null_block(
        "solar_system_ppn_cassini",
        gamma_m1 - data::CASSINI_GAMMA_MINUS_ONE,
        data::CASSINI_GAMMA_SIGMA,
        format!(
            "gamma-1 = {gamma_m1:.2e} vs Cassini (2.1 +/- 2.3)e-5 (screening={})",
            theory.screening.as_deref().unwrap_or("none")
        ),
    ));

    // 4. MICROSCOPE equivalence principle. GR ⇒ eta = 0; an unscreened fifth force violates it.
    let eta = data::predicted_eta(theory);
    checks.push(null_block(
        "equivalence_principle_microscope",
        eta - data::MICROSCOPE_ETA,
        data::MICROSCOPE_ETA_SIGMA,
        format!("eta = {eta:.2e} vs MICROSCOPE (-1 +/- 2)e-15"),
    ));

    // 5. BBN primordial deuterium D/H from the background (omega_b, N_eff). Combined obs + theory
    //    error in quadrature.
    let dh = data::predicted_deuterium(theory);
    let dh_sigma = (data::DH_OBS_SIGMA * data::DH_OBS_SIGMA
        + data::DH_THEORY_SIGMA * data::DH_THEORY_SIGMA)
        .sqrt();
    checks.push(null_block(
        "bbn_primordial_deuterium",
        dh - data::DH_OBS,
        dh_sigma,
        format!(
            "D/H = {:.3e} vs Cooke+2018 (2.527 +/- 0.030)e-5 (omega_b={:.5}, N_eff={:.3})",
            dh, theory.background.omega_b_h2, theory.background.n_eff
        ),
    ));

    // Unification is gated by the weakest sector — no trading one domain off against another.
    let score = checks
        .iter()
        .map(|c| c.score)
        .fold(1.0_f64, |acc, s| acc.min(s));

    UnificationReport { checks, score }
}

#[cfg(test)]
mod tests {
    use super::super::{AlphaBasis, Theory};
    use super::*;

    #[test]
    fn gr_baseline_passes_every_block_without_inflating_the_score() {
        let r = unification_report(&Theory::baseline_lcdm());
        assert!(r.is_unified(), "GR baseline must satisfy every constraint");
        // Passing the nulls gives NO frontier credit: the score caps at 1, never above.
        assert!(
            r.score <= 1.0 + 1e-12,
            "score = {} must not exceed 1",
            r.score
        );
        assert!(r.score > 0.0, "GR must be consistent, score = {}", r.score);
        // Every block is flagged as a null test (no block grants discovery credit).
        assert!(r.checks.iter().all(|c| c.is_null_test));
        // The independent (domain, logL) contributions are reported as a Vec.
        let contribs = r.log_likelihood_contributions();
        assert_eq!(contribs.len(), 5);
        for d in [
            "gw170817_tensor_speed",
            "lvk_standard_siren_h0",
            "solar_system_ppn_cassini",
            "equivalence_principle_microscope",
            "bbn_primordial_deuterium",
        ] {
            assert!(contribs.iter().any(|(dom, _)| *dom == d), "missing {d}");
        }
    }

    #[test]
    fn gr_baseline_total_loglike_is_near_zero() {
        // Each null sits near the centre of its measurement, so the joint cross-domain logL is
        // close to 0 (a perfect-null reference), not strongly positive — passing buys no credit.
        let r = unification_report(&Theory::baseline_lcdm());
        let total = r.total_log_likelihood();
        assert!(
            total <= 1e-9,
            "GR null logL = {total} should not be positive"
        );
        assert!(
            total > -10.0,
            "GR should be a comfortable null, logL = {total}"
        );
    }

    #[test]
    fn unscreened_modified_gravity_is_gated_by_cassini_and_microscope() {
        let mut t = Theory::baseline_lcdm();
        t.alpha = AlphaBasis {
            alpha_m: 0.02,
            alpha_b: 0.05,
            alpha_k: 0.1,
            alpha_t: 0.0,
        };
        // No screening declared ⇒ an O(alpha) PPN/EP deviation, far outside Cassini & MICROSCOPE.
        t.screening = None;
        let r = unification_report(&t);
        assert!(!r.is_unified(), "unscreened modified gravity must be gated");
        let find = |dom| r.checks.iter().find(|c| c.domain == dom).unwrap();
        assert!(
            !find("solar_system_ppn_cassini").passed(),
            "Cassini block must veto the unscreened modification"
        );
        assert!(
            !find("equivalence_principle_microscope").passed(),
            "MICROSCOPE block must veto the unscreened modification"
        );

        // The SAME modification WITH screening recovers GR in the lab and passes both nulls.
        let mut screened = t.clone();
        screened.screening = Some("vainshtein".into());
        let rs = unification_report(&screened);
        let finds = |dom| rs.checks.iter().find(|c| c.domain == dom).unwrap();
        assert!(
            finds("solar_system_ppn_cassini").passed(),
            "screening must restore the Cassini null"
        );
        assert!(
            finds("equivalence_principle_microscope").passed(),
            "screening must restore the MICROSCOPE null"
        );
    }

    #[test]
    fn gw170817_tensor_speed_gates_a_luminal_violation() {
        let mut t = Theory::baseline_lcdm();
        // alpha_T = 0.01 ⇒ c_T/c-1 ≈ 5e-3, ~5e12 sigma beyond the 1e-15 bound.
        t.alpha.alpha_t = 0.01;
        let r = unification_report(&t);
        let gw = r
            .checks
            .iter()
            .find(|c| c.domain == "gw170817_tensor_speed")
            .unwrap();
        assert!(
            !gw.passed(),
            "a tensor-speed excess must be gated by GW170817"
        );
        assert!(!r.is_unified());
    }

    #[test]
    fn bbn_block_penalizes_wrong_omega_b_and_n_eff() {
        let dh_block = |t: &Theory| {
            unification_report(t)
                .checks
                .into_iter()
                .find(|c| c.domain == "bbn_primordial_deuterium")
                .unwrap()
        };
        // Baseline reproduces the observed D/H within tolerance.
        let base = dh_block(&Theory::baseline_lcdm());
        assert!(base.passed());

        // Wrong baryon density → wrong D/H (D/H falls steeply with omega_b): penalised.
        let mut low_ob = Theory::baseline_lcdm();
        low_ob.background.omega_b_h2 = 0.018; // well below Planck 0.02237
        let dh_ob = dh_block(&low_ob);
        assert!(
            !dh_ob.passed(),
            "low omega_b must violate D/H: {}",
            dh_ob.note
        );
        assert!(
            dh_ob.log_likelihood < base.log_likelihood,
            "wrong omega_b must lower the D/H log-likelihood"
        );

        // Extra relativistic species (N_eff) raise D/H: also penalised.
        let mut hot = Theory::baseline_lcdm();
        hot.background.n_eff = 4.5;
        let dh_neff = dh_block(&hot);
        assert!(
            !dh_neff.passed(),
            "high N_eff must violate D/H: {}",
            dh_neff.note
        );
        assert!(
            dh_neff.log_likelihood < base.log_likelihood,
            "extra N_eff must lower the D/H log-likelihood"
        );
    }

    #[test]
    fn score_is_the_weakest_domain() {
        let r = unification_report(&Theory::baseline_lcdm());
        let min = r.checks.iter().map(|c| c.score).fold(1.0_f64, f64::min);
        assert!((r.score - min).abs() < 1e-12);
    }

    #[test]
    fn standard_siren_h0_is_a_broad_independent_null() {
        // ΛCDM H0 = 67.4 sits well inside the wide LVK siren interval ⇒ passes, no inflation.
        let r = unification_report(&Theory::baseline_lcdm());
        let siren = r
            .checks
            .iter()
            .find(|c| c.domain == "lvk_standard_siren_h0")
            .unwrap();
        assert!(
            siren.passed(),
            "Planck H0 must satisfy the broad siren null"
        );
        assert!(siren.score <= 1.0);

        // A wildly wrong H0 (e.g. 120) is gated by the siren null.
        let mut t = Theory::baseline_lcdm();
        t.background.h = 1.20;
        let bad = unification_report(&t);
        assert!(!bad
            .checks
            .iter()
            .find(|c| c.domain == "lvk_standard_siren_h0")
            .unwrap()
            .passed());
    }
}
