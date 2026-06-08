//! Linear growth of structure — the Tier-1 sector that actually constrains dark energy's late-time
//! dynamics and modified gravity, which background distances alone cannot (see
//! `docs/zyal-next-level-design.md` §1.6, §2). It is computed by integrating the standard linear
//! growth equation for the matter density contrast δ(a):
//!
//! ```text
//!   δ'' + (2 + dlnH/dN) δ' − (3/2) Ω_m(a) μ(a) δ = 0,    N = ln a,  ' = d/dN
//! ```
//!
//! with Ω_m(a) = Ω_m0 a⁻³ / E(a)² and the effective gravitational coupling μ(a) = G_eff/G. Dark
//! energy enters **exactly** through the background E(a) (which the background module integrates
//! from first principles); modified gravity enters through μ(a), taken in the GW170817-safe
//! (α_T = 0) late-time parametrization μ(a) = 1 + μ0 · ρ_DE(a)/ρ_DE0 normalized so μ0 is the
//! deviation today (Planck-2018 MG, arXiv:1807.06209). GR ⇒ μ ≡ 1, recovering ΛCDM growth.
//!
//! Observables: the growth rate f(a) = dln δ/dN, the redshift-space distortion combination
//! fσ8(z) = σ8 · [δ(a)/δ(1)] · f(a), and the weak-lensing amplitude S8 = σ8 √(Ω_m/0.3).
//!
//! References: Linder 2005 (growth index); Peebles 1980 (growth equation); Planck-2018 MG.

use super::CosmologyParams;

/// Scale factor at which the growth integration starts (deep in matter domination, where δ ∝ a).
const A_INITIAL: f64 = 1.0e-3;
/// Number of RK4 steps from `ln A_INITIAL` to `ln 1` (sub-0.1% on the growth factor).
const GROWTH_STEPS: usize = 512;

/// A solved linear-growth history δ(N), sampled on a uniform `N = ln a` grid from matter
/// domination to today, with the growth rate `f = dln δ/dN` at each sample.
#[derive(Debug, Clone)]
pub struct GrowthHistory {
    /// Uniform `N = ln a` grid (ascending, ending at 0).
    n_grid: Vec<f64>,
    /// δ(N) on the grid (unnormalized).
    delta: Vec<f64>,
    /// f(N) = dln δ/dN on the grid.
    f: Vec<f64>,
    /// δ at a = 1 (the present), for normalization δ(a)/δ(1).
    delta_today: f64,
}

impl CosmologyParams {
    /// Effective gravitational coupling μ(a) = G_eff/G for matter perturbations, in the late-time
    /// parametrization μ(a) = 1 + μ0 · ρ_DE(a)/ρ_DE0 / E(a)² · ... normalized so that μ(a=1) = 1+μ0.
    /// Concretely μ(a) = 1 + μ0 · [ρ_DE(a)/ρ_DE0] / E(a)² · 1/Ω̃ where the normalization makes the
    /// factor unity today; equivalently μ(a) = 1 + μ0 · Ω_DE(a)/Ω_DE0. GR (μ0 = 0) ⇒ 1.
    fn growth_mu(&self, a: f64) -> f64 {
        if self.mu0.abs() < 1e-12 {
            return 1.0;
        }
        let z = 1.0 / a - 1.0;
        let e2 = self.e_of_z(z).powi(2);
        // Ω_DE(a)/Ω_DE0 = [ρ_DE(a)/ρ_DE0] / E(a)²  (and = 1 at a = 1).
        let omega_de_frac = self.de_density_ratio(z) / e2;
        1.0 + self.mu0 * omega_de_frac
    }

    /// dln E/dN by symmetric finite difference (E is smooth in N = ln a).
    fn dln_e_dn(&self, a: f64) -> f64 {
        let h = 1.0e-4_f64;
        let ap = a * h.exp();
        let am = a * (-h).exp();
        let zp = 1.0 / ap - 1.0;
        let zm = 1.0 / am - 1.0;
        (self.e_of_z(zp).ln() - self.e_of_z(zm).ln()) / (2.0 * h)
    }

    /// Solve the linear growth equation from matter domination to today via RK4, using the default
    /// [`GROWTH_STEPS`] step count.
    pub fn growth_history(&self) -> GrowthHistory {
        self.growth_history_with_steps(GROWTH_STEPS)
    }

    /// Solve the linear growth equation from matter domination to today via RK4 with an explicit
    /// step count. Exposed so a convergence test can compare two resolutions; `growth_history`
    /// pins the production value.
    pub fn growth_history_with_steps(&self, steps: usize) -> GrowthHistory {
        let n_i = A_INITIAL.ln();
        let n_f = 0.0_f64;
        let dn = (n_f - n_i) / steps as f64;

        // State y = (δ, v) with v = dδ/dN. Matter-domination initial condition δ ∝ a ⇒ v = δ.
        let mut delta = A_INITIAL;
        let mut v = A_INITIAL;

        let mut n_grid = Vec::with_capacity(steps + 1);
        let mut delta_hist = Vec::with_capacity(steps + 1);
        let mut f_hist = Vec::with_capacity(steps + 1);

        // RHS of the first-order system at N (a = e^N): dδ/dN = v, dv/dN = −(2+dlnH/dN) v +
        // (3/2) Ω_m(a) μ(a) δ.
        let deriv = |n: f64, d: f64, v: f64| -> (f64, f64) {
            let a = n.exp();
            let z = 1.0 / a - 1.0;
            let e2 = self.e_of_z(z).powi(2);
            let omega_m_a = self.omega_m * a.powi(-3) / e2;
            let drag = 2.0 + self.dln_e_dn(a);
            let source = 1.5 * omega_m_a * self.growth_mu(a) * d;
            (v, -drag * v + source)
        };

        for i in 0..=steps {
            let n = n_i + i as f64 * dn;
            n_grid.push(n);
            delta_hist.push(delta);
            f_hist.push(v / delta);
            if i == steps {
                break;
            }
            // RK4 step.
            let (k1d, k1v) = deriv(n, delta, v);
            let (k2d, k2v) = deriv(n + 0.5 * dn, delta + 0.5 * dn * k1d, v + 0.5 * dn * k1v);
            let (k3d, k3v) = deriv(n + 0.5 * dn, delta + 0.5 * dn * k2d, v + 0.5 * dn * k2v);
            let (k4d, k4v) = deriv(n + dn, delta + dn * k3d, v + dn * k3v);
            delta += dn / 6.0 * (k1d + 2.0 * k2d + 2.0 * k3d + k4d);
            v += dn / 6.0 * (k1v + 2.0 * k2v + 2.0 * k3v + k4v);
        }

        let delta_today = *delta_hist.last().unwrap();
        GrowthHistory {
            n_grid,
            delta: delta_hist,
            f: f_hist,
            delta_today,
        }
    }

    /// fσ8(z) = σ8 · [δ(a)/δ(1)] · f(a) — the redshift-space-distortion observable.
    pub fn growth_fsigma8(&self, z: f64) -> f64 {
        let hist = self.growth_history();
        let a = 1.0 / (1.0 + z);
        let (delta, f) = hist.interp(a.ln());
        self.sigma8 * (delta / hist.delta_today) * f
    }

    /// The weak-lensing clustering amplitude S8 = σ8 √(Ω_m / 0.3).
    pub fn s8(&self) -> f64 {
        self.sigma8 * (self.omega_m / 0.3).sqrt()
    }

    /// The present-day linear growth rate f(z=0) = dln δ/dN |_{a=1} (≈ Ω_m^0.55 for ΛCDM).
    pub fn growth_rate_today(&self) -> f64 {
        let hist = self.growth_history();
        *hist.f.last().unwrap()
    }

    // ── M4: scale-dependent derived modified-gravity growth path ──────────────────────────────
    //
    // The legacy `growth_mu(a)` above is a scale-FREE late-time μ(a) (the μ0 parametrization). A
    // modified-gravity theorist needs μ to be *derived* and *scale-dependent* (`docs/theory-league.md`
    // §"Honest limitations"; plan M4). The methods below add that without touching the existing
    // `growth_history`/`growth_fsigma8`/`growth_mu` API or numbers: when `mg_family == None` they fall
    // straight back to the legacy μ(a), so every prior result is byte-identical. They are exercised
    // through the `f_r()` / `ndgp()` ModelClasses (`theory/league.rs`).

    /// Effective gravitational coupling μ(a, k) = G_eff/G evaluated for the *derived* MG family on
    /// the genome (`mg_family`), at scale factor `a` and comoving wavenumber `k` in h/Mpc. f(R)
    /// Hu–Sawicki and nDGP each compute μ from one fundamental parameter (`theory::sectors`); with no
    /// derived family declared it returns the legacy scale-free `growth_mu(a)` (so k is irrelevant and
    /// nothing changes for existing models). Ω_Λ for the sectors is the present dark-energy fraction.
    pub fn growth_mu_kdep(&self, a: f64, k: f64) -> f64 {
        use crate::cosmology::MgFamily;
        use crate::theory::sectors::{fr::FrParams, ndgp::NdgpParams};
        match self.mg_family {
            MgFamily::None => self.growth_mu(a),
            MgFamily::FrHuSawicki => {
                // |f_R0| = 10^{fr_log10_fr0}; a very negative exponent is the GR limit.
                let f_r0_abs = if self.fr_log10_fr0 <= -20.0 {
                    0.0
                } else {
                    10f64.powf(self.fr_log10_fr0)
                };
                FrParams::new(self.fr_n, f_r0_abs)
                    .response(a, k, self.omega_m, self.omega_de())
                    .mu
            }
            MgFamily::Ndgp => {
                // β needs E(a) = H/H₀ and Ḣ/H² = dlnE/dN at this a (computed from the background).
                let z = 1.0 / a - 1.0;
                let e = self.e_of_z(z);
                let dln_e = self.dln_e_dn(a);
                NdgpParams::from_omega_rc(self.ndgp_omega_rc)
                    .response(e, dln_e)
                    .mu
            }
        }
    }

    /// Reference comoving wavenumber (h/Mpc) at which the scale-dependent growth observable is
    /// evaluated. k ≈ 0.1 h/Mpc is the linear RSD pivot probed by DESI/BOSS fσ8 (e.g. Bose et al.
    /// 2018, arXiv:1606.02520) — large enough that the f(R) fifth force is partly active, small
    /// enough to stay linear.
    pub const GROWTH_KREF_H_MPC: f64 = 0.1;

    /// Solve the linear growth equation with a scale-dependent μ(a, k) at fixed comoving k (h/Mpc).
    /// Identical RK4 scheme to [`Self::growth_history`] but with the derived μ(a,k) in the source
    /// term — the only physically-correct way to grow a mode that feels a k-dependent fifth force.
    pub fn growth_history_kdep(&self, k: f64) -> GrowthHistory {
        let n_i = A_INITIAL.ln();
        let n_f = 0.0_f64;
        let steps = GROWTH_STEPS;
        let dn = (n_f - n_i) / steps as f64;

        let mut delta = A_INITIAL;
        let mut v = A_INITIAL;

        let mut n_grid = Vec::with_capacity(steps + 1);
        let mut delta_hist = Vec::with_capacity(steps + 1);
        let mut f_hist = Vec::with_capacity(steps + 1);

        let deriv = |n: f64, d: f64, v: f64| -> (f64, f64) {
            let a = n.exp();
            let z = 1.0 / a - 1.0;
            let e2 = self.e_of_z(z).powi(2);
            let omega_m_a = self.omega_m * a.powi(-3) / e2;
            let drag = 2.0 + self.dln_e_dn(a);
            let source = 1.5 * omega_m_a * self.growth_mu_kdep(a, k) * d;
            (v, -drag * v + source)
        };

        for i in 0..=steps {
            let n = n_i + i as f64 * dn;
            n_grid.push(n);
            delta_hist.push(delta);
            f_hist.push(v / delta);
            if i == steps {
                break;
            }
            let (k1d, k1v) = deriv(n, delta, v);
            let (k2d, k2v) = deriv(n + 0.5 * dn, delta + 0.5 * dn * k1d, v + 0.5 * dn * k1v);
            let (k3d, k3v) = deriv(n + 0.5 * dn, delta + 0.5 * dn * k2d, v + 0.5 * dn * k2v);
            let (k4d, k4v) = deriv(n + dn, delta + dn * k3d, v + dn * k3v);
            delta += dn / 6.0 * (k1d + 2.0 * k2d + 2.0 * k3d + k4d);
            v += dn / 6.0 * (k1v + 2.0 * k2v + 2.0 * k3v + k4v);
        }

        let delta_today = *delta_hist.last().unwrap();
        GrowthHistory {
            n_grid,
            delta: delta_hist,
            f: f_hist,
            delta_today,
        }
    }

    /// fσ8(z) evaluated with the *derived* scale-dependent μ(a, k) at comoving wavenumber k (h/Mpc).
    /// For `mg_family == None` this equals the legacy [`Self::growth_fsigma8`] (μ is scale-free).
    pub fn growth_fsigma8_kdep(&self, z: f64, k: f64) -> f64 {
        let hist = self.growth_history_kdep(k);
        let a = 1.0 / (1.0 + z);
        let (delta, f) = hist.interp(a.ln());
        self.sigma8 * (delta / hist.delta_today) * f
    }

    /// fσ8(z) at the reference wavenumber [`Self::GROWTH_KREF_H_MPC`] (≈ 0.1 h/Mpc) — the
    /// observable the derived-MG families ([`crate::theory::ModelClass::f_r`] / `ndgp`) are scored on.
    pub fn growth_fsigma8_kref(&self, z: f64) -> f64 {
        self.growth_fsigma8_kdep(z, Self::GROWTH_KREF_H_MPC)
    }
}

impl GrowthHistory {
    /// Linear interpolation of (δ, f) at a target `N = ln a` (clamped to the grid range).
    fn interp(&self, n: f64) -> (f64, f64) {
        let first = self.n_grid[0];
        let last = *self.n_grid.last().unwrap();
        if n <= first {
            return (self.delta[0], self.f[0]);
        }
        if n >= last {
            let k = self.delta.len() - 1;
            return (self.delta[k], self.f[k]);
        }
        // Uniform grid: locate the bracketing index directly.
        let dn = self.n_grid[1] - self.n_grid[0];
        let pos = (n - first) / dn;
        let i = pos.floor() as usize;
        let t = pos - i as f64;
        let lerp = |a: f64, b: f64| a + t * (b - a);
        (
            lerp(self.delta[i], self.delta[i + 1]),
            lerp(self.f[i], self.f[i + 1]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::CosmologyParams;

    #[test]
    fn growth_rate_today_matches_the_lcdm_index_approximation() {
        // f(z=0) ≈ Ω_m^γ with γ ≈ 0.55 for ΛCDM (Linder 2005).
        let c = CosmologyParams::planck_lcdm();
        let f0 = c.growth_rate_today();
        let approx = c.omega_m.powf(0.55);
        assert!(
            (f0 - approx).abs() < 0.02,
            "f0 = {f0} vs Ω_m^0.55 = {approx}"
        );
    }

    #[test]
    fn fsigma8_lands_in_the_observed_band() {
        // ΛCDM fσ8 peaks near ~0.47 around z~0.5 and is ~0.43 at z=0 for σ8≈0.81.
        let c = CosmologyParams::planck_lcdm();
        let f0 = c.growth_fsigma8(0.0);
        assert!(f0 > 0.40 && f0 < 0.47, "fσ8(0) = {f0}");
        let f05 = c.growth_fsigma8(0.5);
        assert!(f05 > 0.43 && f05 < 0.50, "fσ8(0.5) = {f05}");
        // fσ8 falls off at high z.
        assert!(c.growth_fsigma8(1.5) < f05, "fσ8 should decline at high z");
    }

    #[test]
    fn delta_grows_monotonically_and_normalizes_today() {
        let c = CosmologyParams::planck_lcdm();
        let h = c.growth_history();
        for w in h.delta.windows(2) {
            assert!(w[1] >= w[0], "δ must grow");
        }
        // δ(a=1)/δ(1) = 1 by construction.
        let (d1, _) = h.interp(0.0);
        assert!((d1 / h.delta_today - 1.0).abs() < 1e-9);
    }

    #[test]
    fn enhanced_gravity_raises_the_growth_rate() {
        // μ0 > 0 (stronger effective gravity) ⇒ more growth ⇒ higher fσ8.
        let mut c = CosmologyParams::planck_lcdm();
        let base = c.growth_fsigma8(0.5);
        c.mu0 = 0.3;
        let boosted = c.growth_fsigma8(0.5);
        assert!(boosted > base, "μ0>0 should raise fσ8: {boosted} vs {base}");
        // μ0 < 0 (weaker gravity, an S8-tension reliever) ⇒ less growth.
        c.mu0 = -0.3;
        assert!(c.growth_fsigma8(0.5) < base);
    }

    #[test]
    fn s8_matches_sigma8_at_fiducial_omega_m() {
        let mut c = CosmologyParams::planck_lcdm();
        c.omega_m = 0.3;
        assert!((c.s8() - c.sigma8).abs() < 1e-12);
    }

    // ----------------------------------------------------------------------------------------
    // M3 solver-truth: analytic growth limits (Einstein–de Sitter, de Sitter) and measured RK4
    // convergence.
    // ----------------------------------------------------------------------------------------

    /// An essentially-Einstein–de Sitter cosmology: Ωm = 1, radiation driven to ~0 (no baryons,
    /// N_eff = 0 leaves only the tiny photon term), so the growing mode is the analytic D ∝ a.
    fn einstein_de_sitter() -> CosmologyParams {
        CosmologyParams {
            h: 1.0,
            omega_m: 1.0,
            omega_b_h2: 0.0,
            n_eff: 0.0,
            sum_mnu: 0.0,
            w0: -1.0,
            wa: 0.0,
            omega_k: 0.0,
            sigma8: 0.8,
            mu0: 0.0,
        }
    }

    #[test]
    fn einstein_de_sitter_growth_is_linear_in_a_with_f_unity() {
        // EdS analytic growing mode: D(a) ∝ a, f = dln D/dln a = 1 exactly (Peebles 1980;
        // Dodelson "Modern Cosmology" §7). Our integrator must reproduce both.
        let c = einstein_de_sitter();
        // f(z=0) → 1.
        let f0 = c.growth_rate_today();
        assert!((f0 - 1.0).abs() < 2e-3, "EdS f(0) = {f0}, expected 1");
        // D(a)/D(1) = a across the history: sample at several a and compare to a.
        let h = c.growth_history();
        for &a in &[0.1_f64, 0.25, 0.5, 0.8] {
            let (delta, f) = h.interp(a.ln());
            let ratio = delta / h.delta_today;
            assert!(
                (ratio - a).abs() / a < 5e-3,
                "EdS D(a)/D(1) = {ratio} at a={a}, expected a (linear growth)"
            );
            // f stays ≈ 1 throughout, not just today.
            assert!((f - 1.0).abs() < 5e-3, "EdS f = {f} at a={a}, expected 1");
        }
    }

    #[test]
    fn de_sitter_expansion_suppresses_growth() {
        // In a Λ-dominated (de Sitter) phase the Hubble drag freezes perturbation growth: the
        // growth rate f is strongly suppressed below the EdS value of 1, and δ approaches a
        // constant. We push Ωm → small so the late universe is nearly de Sitter (Peebles 1980).
        let mut c = CosmologyParams::planck_lcdm();
        c.omega_m = 0.02; // Λ-dominated today ⇒ near-de-Sitter expansion
        c.mu0 = 0.0;
        let f0 = c.growth_rate_today();
        assert!(
            f0 < 0.2,
            "near-de-Sitter growth rate f(0) = {f0} should be strongly suppressed (≪ 1)"
        );
        // Growth is far weaker than in a matter-dominated (EdS) universe.
        assert!(f0 < einstein_de_sitter().growth_rate_today());
    }

    #[test]
    fn rk4_growth_converges_512_vs_1024_steps() {
        // Measured RK4 convergence: refining 512 → 1024 steps must change fσ8 by far less than the
        // ~3–5% observational σ on an RSD measurement. RK4 is O(h⁴) ⇒ ~16× error reduction per
        // halving, so the change here is tiny.
        let c = CosmologyParams::planck_lcdm();
        let z = 0.5_f64;
        let a = 1.0 / (1.0 + z);

        let h512 = c.growth_history_with_steps(512);
        let h1024 = c.growth_history_with_steps(1024);
        let (d512, f512) = h512.interp(a.ln());
        let (d1024, f1024) = h1024.interp(a.ln());
        let fs512 = c.sigma8 * (d512 / h512.delta_today) * f512;
        let fs1024 = c.sigma8 * (d1024 / h1024.delta_today) * f1024;

        let rel = (fs512 - fs1024).abs() / fs1024.abs();
        assert!(
            rel < 1e-4,
            "RK4 fσ8 512 vs 1024 relative change {rel:.3e} should be ≪ 3% obs σ"
        );
    }

    // ── M4: derived scale-dependent MG growth (f(R) Hu–Sawicki, nDGP) ─────────────────────────

    #[test]
    fn scale_dependent_path_is_identical_to_legacy_for_plain_lcdm() {
        // mg_family == None ⇒ the k-dependent path must reproduce the legacy fσ8 bit-for-bit
        // (additive guarantee: nothing changes for existing ΛCDM/μ0 models).
        let c = CosmologyParams::planck_lcdm();
        for &z in &[0.0_f64, 0.5, 1.1] {
            let legacy = c.growth_fsigma8(z);
            let kdep = c.growth_fsigma8_kref(z);
            assert!((legacy - kdep).abs() < 1e-12, "z={z}: {legacy} vs {kdep}");
        }
    }

    #[test]
    fn fr_enhances_growth_and_recovers_gr_as_f_r0_to_zero() {
        use super::super::MgFamily;
        let gr = CosmologyParams::planck_lcdm();
        let base = gr.growth_fsigma8_kref(0.5);

        // Strong f(R): |f_R0| = 1e-4 ⇒ active fifth force ⇒ MORE growth (fσ8 up).
        let mut strong = gr.clone();
        strong.mg_family = MgFamily::FrHuSawicki;
        strong.fr_n = 1.0;
        strong.fr_log10_fr0 = -4.0;
        let boosted = strong.growth_fsigma8_kref(0.5);
        assert!(
            boosted > base,
            "f(R) |f_R0|=1e-4 should raise fσ8: {boosted} vs {base}"
        );

        // Larger |f_R0| ⇒ more enhancement (monotone in the fundamental amplitude).
        let mut stronger = strong.clone();
        stronger.fr_log10_fr0 = -3.5;
        assert!(
            stronger.growth_fsigma8_kref(0.5) > boosted,
            "bigger |f_R0| ⇒ more growth"
        );

        // f_R0 → 0 recovers GR growth exactly (within integrator noise).
        let mut weak = strong.clone();
        weak.fr_log10_fr0 = -30.0;
        let recovered = weak.growth_fsigma8_kref(0.5);
        assert!(
            (recovered - base).abs() < 1e-6,
            "f_R0→0 must recover GR: {recovered} vs {base}"
        );
    }

    #[test]
    fn ndgp_normal_branch_enhances_growth_and_recovers_gr_as_rc_to_infinity() {
        // PHYSICS: the normal branch ENHANCES growth (β>0 ⇒ μ=1+1/(3β)>1; Koyama & Maartens 2006).
        // (The milestone brief said "suppresses"; that is the ghost-unstable self-accelerating
        // branch — see theory/sectors/ndgp.rs and the `deferred` note. We implement the healthy,
        // growth-enhancing normal branch and test the literature-correct sign.)
        use super::super::MgFamily;
        let gr = CosmologyParams::planck_lcdm();
        let base = gr.growth_fsigma8_kref(0.5);

        let mut ndgp = gr.clone();
        ndgp.mg_family = MgFamily::Ndgp;
        ndgp.ndgp_omega_rc = 0.25; // H₀ r_c = 1
        let enhanced = ndgp.growth_fsigma8_kref(0.5);
        assert!(
            enhanced > base,
            "nDGP normal branch should raise fσ8: {enhanced} vs {base}"
        );

        // Smaller r_c (larger Ω_rc) ⇒ stronger fifth force ⇒ even more growth.
        let mut stronger = ndgp.clone();
        stronger.ndgp_omega_rc = 1.0;
        assert!(
            stronger.growth_fsigma8_kref(0.5) > enhanced,
            "smaller r_c ⇒ more growth"
        );

        // r_c → ∞ (Ω_rc → 0) recovers GR growth exactly.
        let mut weak = ndgp.clone();
        weak.ndgp_omega_rc = 0.0;
        let recovered = weak.growth_fsigma8_kref(0.5);
        assert!(
            (recovered - base).abs() < 1e-9,
            "r_c→∞ must recover GR: {recovered} vs {base}"
        );
    }
}
