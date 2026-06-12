//! Deterministic symbolic mutation + recombination over [`Theory`].
//!
//! Mutation operates on the *structured* theory (α-basis deviations, background parameters,
//! screening, provenanced parameters) rather than on an opaque float vector. The deterministic
//! veto cascade ([`super::vetoes`]) stays the gate: a mutation that wanders into a ghostly /
//! unstable / GW170817-violating region is *killed there*, not prevented here — so the
//! co-evolving adversary can probe the boundaries. A tiny in-repo splitmix64 PRNG keeps runs
//! bit-reproducible from a seed with no external dependency.

use super::{Provenance, Theory};

/// Reproducible splitmix64 PRNG (Steele, Lea & Flood 2014). No external crate so a seed
/// reproduces a run identically across platforms.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform f64 in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform f64 in [-1, 1).
    pub fn signed(&mut self) -> f64 {
        2.0 * self.unit() - 1.0
    }

    /// Bernoulli draw with success probability `p`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }
}

/// Mutate a theory by jittering its continuous deviations within physical scales. α_T is held at
/// the GW170817 point (0) here — a non-zero tensor speed only enters via an explicit structural
/// operator ([`flip_to_quintic_decoy`]) so it is a deliberate (and vetoed) probe, not noise.
/// If a mutation makes the theory modify gravity, it may also acquire a screening mechanism so a
/// genuinely viable modified-gravity candidate can emerge (rather than always tripping the PPN
/// veto).
/// Add a mass-dimension-4 scalar term if the theory lacks one by this name.
fn ensure_term(t: &mut Theory, name: &str) {
    if !t.terms.iter().any(|x| x.name == name) {
        t.terms.push(super::Term {
            name: name.into(),
            mass_dimension: 4,
            free_lorentz_indices: 0,
        });
    }
}

pub fn mutate(theory: &Theory, rng: &mut Rng, rate: f64) -> Theory {
    let mut t = theory.clone();
    if rng.chance(rate) {
        t.alpha.alpha_m += 0.05 * rng.signed();
        ensure_term(&mut t, "horndeski_scalar");
    }
    if rng.chance(rate) {
        t.alpha.alpha_b += 0.05 * rng.signed();
        ensure_term(&mut t, "horndeski_scalar");
    }
    if rng.chance(rate) {
        t.alpha.alpha_k += 0.10 * rng.signed();
        ensure_term(&mut t, "horndeski_scalar");
    }
    if rng.chance(rate) {
        t.background.h += 0.02 * rng.signed();
    }
    if rng.chance(rate) {
        t.background.omega_m += 0.02 * rng.signed();
    }
    if rng.chance(rate) {
        t.background.w0 += 0.10 * rng.signed();
        // V6.1 (P0.6): structure follows the dial — dynamical w needs a generating term.
        ensure_term(&mut t, "quintessence_scalar");
    }
    // V6: a gravity-modifying mutant must carry a QUANTIFIED screening claim to stay PPN-viable —
    // a bare mechanism label was the V5 pass-token cheat (adjudication now demands the number).
    // The recovery here is an explicit, costed modeling choice the oracle re-tests via the
    // Cassini residual; mutants that modify gravity without it die at the unified gate.
    // V8 Phase 31: chameleon is chosen over vainshtein because chameleon is density-threshold
    // based (always structurally plausible per the Phase 22 ScreeningMechanismImplausible veto),
    // while vainshtein requires |alpha_b| >= 0.01 which many one-parameter mutants lack.
    if t.modifies_gravity() && t.screening.is_none() && rng.chance(0.5) {
        t.screening = Some("chameleon".into());
        t.screening_recovery = Some(0.999_999);
    }
    // Bounded, deterministic id (lineage is not encoded in the id to avoid unbounded growth).
    let tag = rng.next_u64() & 0xffff;
    t.id = format!("thy-m{tag:04x}");
    t
}

/// Uniform-crossover recombination: each α component and the screening choice is inherited from
/// one parent or the other; background parameters are blended; parameter lists are unioned by
/// symbol (preferring the higher-provenance entry). Behaviourally a child of two viable parents
/// tends to remain viable, but the veto cascade still adjudicates.
pub fn recombine(a: &Theory, b: &Theory, rng: &mut Rng) -> Theory {
    let pick = |rng: &mut Rng, x: f64, y: f64| if rng.chance(0.5) { x } else { y };
    let mut child = a.clone();
    child.alpha.alpha_m = pick(rng, a.alpha.alpha_m, b.alpha.alpha_m);
    child.alpha.alpha_b = pick(rng, a.alpha.alpha_b, b.alpha.alpha_b);
    child.alpha.alpha_k = pick(rng, a.alpha.alpha_k, b.alpha.alpha_k);
    child.alpha.alpha_t = pick(rng, a.alpha.alpha_t, b.alpha.alpha_t);
    child.background.h = 0.5 * (a.background.h + b.background.h);
    child.background.omega_m = 0.5 * (a.background.omega_m + b.background.omega_m);
    child.background.w0 = 0.5 * (a.background.w0 + b.background.w0);
    child.screening = if rng.chance(0.5) {
        a.screening.clone()
    } else {
        b.screening.clone()
    };
    // Union parameters by symbol, keeping the most-provenanced version.
    for p in &b.parameters {
        match child.parameters.iter_mut().find(|q| q.symbol == p.symbol) {
            Some(existing) => {
                if !existing.provenance.is_whitebox() && p.provenance.is_whitebox() {
                    existing.provenance = p.provenance.clone();
                }
            }
            None => child.parameters.push(p.clone()),
        }
    }
    child.id = format!("thy-x{:04x}", rng.next_u64() & 0xffff);
    child
}

/// Explicit structural decoy operator: switch on a quintic (G5) sector, which generically gives
/// α_T ≠ 0 and non-degenerate higher derivatives — i.e. a theory the cascade MUST kill
/// (GW170817 + Ostrogradsky). Used to seed the adversary with hard negatives.
pub fn flip_to_quintic_decoy(theory: &Theory) -> Theory {
    let mut t = theory.clone();
    t.alpha.alpha_t = 0.4;
    t.stability.has_nondegenerate_higher_derivatives = true;
    t.id = format!("{}-quintic-decoy", theory.id);
    t
}

/// Add a free (un-provenanced) fitting parameter — the gray-box decoy the whitebox veto kills.
pub fn inject_free_parameter(theory: &Theory, symbol: &str, value: f64) -> Theory {
    let mut t = theory.clone();
    t.parameters.push(super::Parameter {
        symbol: symbol.to_string(),
        value,
        physical_meaning: "tuned to fit a tension".to_string(),
        provenance: Provenance::Free,
    });
    t.id = format!("{}-graybox", theory.id);
    t
}

#[cfg(test)]
mod tests {
    use super::super::{run_veto_cascade, Theory};
    use super::*;

    #[test]
    fn rng_is_deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn mutation_is_reproducible_and_changes_the_theory() {
        let base = Theory::baseline_lcdm();
        let m1 = mutate(&base, &mut Rng::new(7), 1.0);
        let m2 = mutate(&base, &mut Rng::new(7), 1.0);
        assert_eq!(m1, m2, "same seed must reproduce the same mutant");
        assert_ne!(m1.background.h, base.background.h);
    }

    #[test]
    fn a_gravity_modifying_mutant_can_stay_viable() {
        // Over many seeds, at least one mutation yields a veto-passing (screened, stable) theory.
        let base = Theory::baseline_lcdm();
        let viable = (0..200u64).any(|s| {
            let m = mutate(&base, &mut Rng::new(s), 1.0);
            run_veto_cascade(&m).is_empty()
        });
        assert!(
            viable,
            "mutation should be able to produce viable candidates"
        );
    }

    #[test]
    fn structural_decoys_are_always_killed() {
        let base = Theory::baseline_lcdm();
        assert!(!run_veto_cascade(&flip_to_quintic_decoy(&base)).is_empty());
        assert!(!run_veto_cascade(&inject_free_parameter(&base, "f_ede", 0.07)).is_empty());
    }

    #[test]
    fn recombination_of_baselines_is_viable() {
        let a = Theory::baseline_lcdm();
        let b = Theory::baseline_lcdm();
        let child = recombine(&a, &b, &mut Rng::new(3));
        assert!(run_veto_cascade(&child).is_empty());
    }
}
