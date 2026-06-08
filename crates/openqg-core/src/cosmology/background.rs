//! Real, deterministic background-cosmology forward model (pure Rust, no external solver).
//!
//! This is the honest replacement for an identity `forward_map`: a candidate's physical
//! parameters are turned into *derived* observables by actually integrating the Friedmann
//! equation and the sound-horizon integral. Every observable here (luminosity distance,
//! BAO ratios, the drag-epoch sound horizon, the BBN helium fraction) is a non-trivial
//! function of the parameters — you cannot fit them by reading a number off the genome.
//!
//! Scope: the FLRW *background* (expansion history + distances) plus the standard
//! early-universe sound horizon and a BBN abundance fit. Linear perturbations / growth
//! (σ8, fσ8) and the full CMB spectrum are the job of the optional Boltzmann backend; this
//! module deliberately covers the "Tier-0" observables that are exactly computable from the
//! background alone (BAO, SNe distances, BBN), so the default engine stays pure-Rust and
//! reproducible.
//!
//! References (see docs/research/forward-model-and-unification.md):
//! - Hogg 1999, "Distance measures in cosmology" (astro-ph/9905116) for the distance ladder.
//! - Eisenstein & Hu 1998 (astro-ph/9709112) for z_drag and the r_s fitting formula.
//! - CPL parametrization w(a) = w0 + wa (1 - a) (Chevallier-Polarski 2001; Linder 2003).

use serde::{Deserialize, Serialize};

/// Speed of light in km/s.
pub const C_KM_S: f64 = 299_792.458;
/// Photon density today as ωγ = Ωγ h², for T_CMB = 2.7255 K.
const OMEGA_GAMMA_H2: f64 = 2.4728e-5;
/// Neutrino-to-photon energy-density factor per effective species (7/8 (4/11)^(4/3)).
const NU_FACTOR: f64 = 0.227_10;
/// Sum-of-neutrino-masses → Ων h² conversion (eV): Ωνh² = Σmν / 93.14.
const MNU_TO_OMEGA_H2: f64 = 93.14;

/// Background cosmological parameters. These are the *physical* genes a candidate owns; the
/// methods below derive observables from them. Baseline values reproduce Planck-2018 ΛCDM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CosmologyParams {
    /// Dimensionless Hubble parameter h, with H0 = 100 h km/s/Mpc.
    pub h: f64,
    /// Total non-relativistic matter density today Ωm (CDM + baryons + massive ν).
    pub omega_m: f64,
    /// Physical baryon density ωb = Ωb h².
    pub omega_b_h2: f64,
    /// Effective number of relativistic species (radiation era).
    pub n_eff: f64,
    /// Sum of neutrino masses in eV.
    pub sum_mnu: f64,
    /// Dark-energy equation of state today (CPL w0). ΛCDM ⇒ -1.
    pub w0: f64,
    /// Dark-energy equation-of-state evolution (CPL wa). ΛCDM ⇒ 0.
    pub wa: f64,
    /// Curvature density today Ωk (0 ⇒ spatially flat).
    pub omega_k: f64,
}

impl CosmologyParams {
    /// Planck-2018 ΛCDM baseline (`sm-gr-lcdm-mnu`).
    pub fn planck_lcdm() -> Self {
        CosmologyParams {
            h: 0.674,
            omega_m: 0.315,
            omega_b_h2: 0.02237,
            n_eff: 3.046,
            sum_mnu: 0.06,
            w0: -1.0,
            wa: 0.0,
            omega_k: 0.0,
        }
    }

    /// H0 in km/s/Mpc.
    pub fn h0(&self) -> f64 {
        100.0 * self.h
    }

    /// Hubble distance c/H0 in Mpc.
    pub fn hubble_distance(&self) -> f64 {
        C_KM_S / self.h0()
    }

    /// Radiation density today Ωr = Ωγ (1 + 0.2271 N_eff) / h² (photons + relativistic ν).
    pub fn omega_r(&self) -> f64 {
        OMEGA_GAMMA_H2 * (1.0 + NU_FACTOR * self.n_eff) / (self.h * self.h)
    }

    /// Dark-energy density today, fixed by the flatness/closure relation
    /// Ωm + Ωr + Ωk + ΩDE = 1.
    pub fn omega_de(&self) -> f64 {
        1.0 - self.omega_m - self.omega_r() - self.omega_k
    }

    /// Dark-energy density evolution ρDE(z)/ρDE(0) for the CPL parametrization.
    fn de_density_ratio(&self, z: f64) -> f64 {
        let a = 1.0 / (1.0 + z);
        // ρDE(a)/ρDE0 = a^{-3(1+w0+wa)} exp(-3 wa (1-a)).
        a.powf(-3.0 * (1.0 + self.w0 + self.wa)) * (-3.0 * self.wa * (1.0 - a)).exp()
    }

    /// Dimensionless expansion rate E(z) = H(z)/H0.
    pub fn e_of_z(&self, z: f64) -> f64 {
        let zp1 = 1.0 + z;
        let matter = self.omega_m * zp1.powi(3);
        let radiation = self.omega_r() * zp1.powi(4);
        let curvature = self.omega_k * zp1.powi(2);
        let dark = self.omega_de() * self.de_density_ratio(z);
        (matter + radiation + curvature + dark).max(0.0).sqrt()
    }

    /// H(z) in km/s/Mpc.
    pub fn hubble(&self, z: f64) -> f64 {
        self.h0() * self.e_of_z(z)
    }

    /// Line-of-sight comoving distance D_C(z) in Mpc via composite Simpson integration of
    /// the smooth integrand 1/E(z'). 2048 panels gives sub-0.01% accuracy to z ~ 3.
    pub fn comoving_distance(&self, z: f64) -> f64 {
        if z <= 0.0 {
            return 0.0;
        }
        let integral = simpson(0.0, z, 2048, |zp| 1.0 / self.e_of_z(zp));
        self.hubble_distance() * integral
    }

    /// Transverse comoving distance D_M(z) (Mpc), applying curvature when Ωk ≠ 0.
    pub fn transverse_comoving_distance(&self, z: f64) -> f64 {
        let dc = self.comoving_distance(z);
        let dh = self.hubble_distance();
        let ok = self.omega_k;
        if ok.abs() < 1e-8 {
            dc
        } else if ok > 0.0 {
            let sk = ok.sqrt();
            dh / sk * (sk * dc / dh).sinh()
        } else {
            let sk = (-ok).sqrt();
            dh / sk * (sk * dc / dh).sin()
        }
    }

    /// Angular diameter distance D_A(z) in Mpc.
    pub fn angular_diameter_distance(&self, z: f64) -> f64 {
        self.transverse_comoving_distance(z) / (1.0 + z)
    }

    /// Luminosity distance D_L(z) in Mpc.
    pub fn luminosity_distance(&self, z: f64) -> f64 {
        self.transverse_comoving_distance(z) * (1.0 + z)
    }

    /// Distance modulus μ(z) = 5 log10(D_L / 10 pc) in magnitudes (D_L in Mpc).
    pub fn distance_modulus(&self, z: f64) -> f64 {
        5.0 * self.luminosity_distance(z).log10() + 25.0
    }

    /// Baryon-to-photon momentum ratio R(z) = 3ρb / 4ργ at redshift z.
    fn baryon_photon_ratio(&self, z: f64) -> f64 {
        // R = (3 Ωb / 4 Ωγ) a = 0.75 (ωb/ωγ) / (1+z).
        0.75 * (self.omega_b_h2 / OMEGA_GAMMA_H2) / (1.0 + z)
    }

    /// Drag-epoch redshift z_drag (Eisenstein & Hu 1998, eqs. 4).
    pub fn z_drag(&self) -> f64 {
        let om = self.omega_m * self.h * self.h; // ωm
        let ob = self.omega_b_h2;
        let b1 = 0.313 * om.powf(-0.419) * (1.0 + 0.607 * om.powf(0.674));
        let b2 = 0.238 * om.powf(0.223);
        1291.0 * om.powf(0.251) / (1.0 + 0.659 * om.powf(0.828)) * (1.0 + b1 * ob.powf(b2))
    }

    /// Comoving sound horizon r_s(z) (Mpc), by direct integration of the sound speed over the
    /// early universe: r_s = ∫_z^∞ c_s(z')/H(z') dz', with c_s = c / sqrt(3 (1 + R)). Integrated
    /// in x = ln(1+z') where the integrand decays.
    pub fn sound_horizon(&self, z: f64) -> f64 {
        let x_lo = (1.0 + z).ln();
        let x_hi = (1.0 + 1.0e8_f64).ln();
        simpson(x_lo, x_hi, 8192, |x| {
            let zp = x.exp() - 1.0;
            let cs = C_KM_S / (3.0 * (1.0 + self.baryon_photon_ratio(zp))).sqrt();
            cs / self.hubble(zp) * (1.0 + zp)
        })
    }

    /// Comoving sound horizon at the drag epoch r_drag (Mpc), via the Aubourg et al. 2015
    /// (arXiv:1411.1074, eq. 16) fitting formula calibrated to CAMB (accurate to ~0.1% over the
    /// relevant parameter range). NOTE: we deliberately do NOT use `sound_horizon(z_drag())` here
    /// — the EH98 fitting-formula `z_drag` is ~4% low for Planck cosmologies, and feeding it into
    /// the otherwise-accurate integral biases r_drag ~2% high, which would propagate into every
    /// BAO ratio (D_M/r_d, D_H/r_d, D_V/r_d). The CAMB-calibrated fit gives r_drag ≈ 147.1 Mpc for
    /// Planck-2018, matching the fiducial.
    pub fn sound_horizon_drag(&self) -> f64 {
        let omega_b = self.omega_b_h2;
        let omega_nu = self.sum_mnu / MNU_TO_OMEGA_H2; // ω_ν = Σmν / 93.14
        let omega_cb = (self.omega_m * self.h * self.h - omega_nu).max(1e-6); // ω_cb = ω_m − ω_ν
        55.154 * (-72.3 * (omega_nu + 0.0006).powi(2)).exp()
            / (omega_cb.powf(0.25351) * omega_b.powf(0.12807))
    }

    /// Redshift of recombination / last scattering z_* (Hu & Sugiyama 1996 fitting formula).
    pub fn z_star(&self) -> f64 {
        let om = self.omega_m * self.h * self.h; // ωm
        let ob = self.omega_b_h2;
        let g1 = 0.0783 * ob.powf(-0.238) / (1.0 + 39.5 * ob.powf(0.763));
        let g2 = 0.560 / (1.0 + 21.1 * ob.powf(1.81));
        1048.0 * (1.0 + 0.00124 * ob.powf(-0.738)) * (1.0 + g1 * om.powf(g2))
    }

    /// CMB shift parameter R = sqrt(Ω_m) (H0/c) D_M(z_*) (dimensionless) — a compressed CMB
    /// distance observable (Planck-2018: R ≈ 1.7502).
    pub fn cmb_shift_r(&self) -> f64 {
        let z_star = self.z_star();
        self.omega_m.sqrt() * (self.h0() / C_KM_S) * self.transverse_comoving_distance(z_star)
    }

    /// CMB acoustic scale ℓ_A = π D_M(z_*) / r_s(z_*) (Planck-2018: ℓ_A ≈ 301.5).
    pub fn cmb_acoustic_scale(&self) -> f64 {
        let z_star = self.z_star();
        std::f64::consts::PI * self.transverse_comoving_distance(z_star)
            / self.sound_horizon(z_star)
    }

    /// Eisenstein & Hu 1998 closed-form fit for r_s (Mpc), used as an independent cross-check
    /// of the integral above. eq. (26): r_s ≈ 44.5 ln(9.83/ωm) / sqrt(1 + 10 ωb^{3/4}).
    pub fn sound_horizon_drag_eh98_fit(&self) -> f64 {
        let om = self.omega_m * self.h * self.h;
        let ob = self.omega_b_h2;
        44.5 * (9.83 / om).ln() / (1.0 + 10.0 * ob.powf(0.75)).sqrt()
    }

    /// BAO transverse ratio D_M(z) / r_drag (dimensionless).
    pub fn bao_dm_over_rd(&self, z: f64) -> f64 {
        self.transverse_comoving_distance(z) / self.sound_horizon_drag()
    }

    /// BAO radial ratio D_H(z) / r_drag, with D_H(z) = c / H(z).
    pub fn bao_dh_over_rd(&self, z: f64) -> f64 {
        (C_KM_S / self.hubble(z)) / self.sound_horizon_drag()
    }

    /// BAO isotropic ratio D_V(z) / r_drag, D_V = [z D_M² D_H]^{1/3}.
    pub fn bao_dv_over_rd(&self, z: f64) -> f64 {
        let dm = self.transverse_comoving_distance(z);
        let dh = C_KM_S / self.hubble(z);
        let dv = (z * dm * dm * dh).cbrt();
        dv / self.sound_horizon_drag()
    }

    /// Primordial helium mass fraction Y_p from a linearized BBN fit around the fiducial
    /// (ωb = 0.02237, N_eff = 3.046 ⇒ Y_p = 0.2470), consistent with PRIMAT/PArthENoPE to
    /// sub-percent over the relevant range. Slopes: ∂Y_p/∂ωb ≈ 0.5, ∂Y_p/∂N_eff ≈ 0.0134
    /// (the ωb slope is ~0.4–0.7 in PRIMAT/PArthENoPE, not 1.0).
    pub fn bbn_helium_fraction(&self) -> f64 {
        0.2470 + 0.5 * (self.omega_b_h2 - 0.02237) + 0.0134 * (self.n_eff - 3.046)
    }

    /// Density today contributed by massive neutrinos, Ων = Σmν / (93.14 h²). Provided so a
    /// candidate can check that its declared Ωm self-consistently contains the neutrino mass.
    pub fn omega_nu(&self) -> f64 {
        (self.sum_mnu / MNU_TO_OMEGA_H2) / (self.h * self.h)
    }
}

/// Composite Simpson's rule over `panels` (rounded up to even) subintervals of `[a, b]`.
fn simpson<F: Fn(f64) -> f64>(a: f64, b: f64, panels: usize, f: F) -> f64 {
    let n = if panels % 2 == 0 { panels } else { panels + 1 };
    let h = (b - a) / n as f64;
    let mut sum = f(a) + f(b);
    for i in 1..n {
        let x = a + i as f64 * h;
        sum += if i % 2 == 0 { 2.0 } else { 4.0 } * f(x);
    }
    sum * h / 3.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs().max(1e-12)
    }

    #[test]
    fn e_of_z_is_one_at_present_and_increases() {
        let c = CosmologyParams::planck_lcdm();
        assert!(approx(c.e_of_z(0.0), 1.0, 1e-9));
        assert!(c.e_of_z(1.0) > c.e_of_z(0.0));
        assert!(c.e_of_z(2.0) > c.e_of_z(1.0));
    }

    #[test]
    fn closure_relation_holds() {
        let c = CosmologyParams::planck_lcdm();
        let total = c.omega_m + c.omega_r() + c.omega_k + c.omega_de();
        assert!(approx(total, 1.0, 1e-12));
    }

    #[test]
    fn distances_are_ordered_and_self_consistent() {
        let c = CosmologyParams::planck_lcdm();
        assert_eq!(c.comoving_distance(0.0), 0.0);
        // D_L = (1+z)^2 D_A.
        let z = 0.8;
        assert!(approx(
            c.luminosity_distance(z),
            (1.0 + z).powi(2) * c.angular_diameter_distance(z),
            1e-9
        ));
        // Monotone comoving distance.
        assert!(c.comoving_distance(2.0) > c.comoving_distance(1.0));
    }

    #[test]
    fn comoving_distance_matches_known_lcdm() {
        // Flat ΛCDM h=0.7, Ωm=0.3: D_C(z=1) ≈ 3.30–3.40 Gpc (Hogg 1999 regime).
        let c = CosmologyParams {
            h: 0.7,
            omega_m: 0.3,
            omega_b_h2: 0.0224,
            n_eff: 3.046,
            sum_mnu: 0.06,
            w0: -1.0,
            wa: 0.0,
            omega_k: 0.0,
        };
        let dc = c.comoving_distance(1.0);
        assert!(dc > 3250.0 && dc < 3450.0, "D_C(z=1) = {dc} Mpc");
        // Distance modulus to z=1 ~ 44.1 mag.
        let mu = c.distance_modulus(1.0);
        assert!(mu > 43.9 && mu < 44.3, "mu(z=1) = {mu}");
    }

    #[test]
    fn sound_horizon_is_physical_and_fit_cross_checks() {
        let c = CosmologyParams::planck_lcdm();
        let rd = c.sound_horizon_drag();
        // Planck-2018 fiducial r_drag = 147.09 Mpc (CAMB). The Aubourg-2015 fit must match it
        // tightly — a loose band here previously masked a ~2.4% bias.
        assert!(rd > 146.0 && rd < 148.0, "r_drag = {rd} Mpc");
        // The independent EH98 closed-form fit is itself ~2% high, so agree only to ~5%.
        let fit = c.sound_horizon_drag_eh98_fit();
        assert!(approx(rd, fit, 0.05), "r_drag {rd} vs EH98 fit {fit}");
        // z_drag lands in the expected ~1060 band.
        let zd = c.z_drag();
        assert!(zd > 1000.0 && zd < 1100.0, "z_drag = {zd}");
    }

    #[test]
    fn cmb_distance_priors_match_planck() {
        let c = CosmologyParams::planck_lcdm();
        let z_star = c.z_star();
        assert!(z_star > 1080.0 && z_star < 1100.0, "z_star = {z_star}");
        // Planck-2018 compressed CMB priors: R ≈ 1.7502, ℓ_A ≈ 301.5.
        let r = c.cmb_shift_r();
        assert!(r > 1.73 && r < 1.77, "R = {r}");
        let l_a = c.cmb_acoustic_scale();
        assert!(l_a > 299.0 && l_a < 304.0, "l_A = {l_a}");
    }

    #[test]
    fn bao_ratios_land_in_observed_band() {
        let c = CosmologyParams::planck_lcdm();
        // DESI-era D_V/r_d at z≈0.5 is ~13; D_M/r_d at z≈0.5 ~ 13; both O(10).
        let dv = c.bao_dv_over_rd(0.51);
        assert!(dv > 10.0 && dv < 16.0, "D_V/r_d(0.51) = {dv}");
        let dm = c.bao_dm_over_rd(0.51);
        assert!(dm > 10.0 && dm < 18.0, "D_M/r_d(0.51) = {dm}");
    }

    #[test]
    fn bbn_helium_fraction_matches_fiducial() {
        let c = CosmologyParams::planck_lcdm();
        let yp = c.bbn_helium_fraction();
        assert!(yp > 0.244 && yp < 0.250, "Y_p = {yp}");
        // Extra radiation raises Y_p.
        let mut hot = c.clone();
        hot.n_eff = 4.046;
        assert!(hot.bbn_helium_fraction() > yp);
    }

    #[test]
    fn dark_energy_evolution_changes_expansion() {
        // A phantom-ish w0 < -1 raises late-time expansion vs ΛCDM.
        let lcdm = CosmologyParams::planck_lcdm();
        let mut cpl = lcdm.clone();
        cpl.w0 = -1.2;
        assert!(cpl.e_of_z(0.5) != lcdm.e_of_z(0.5));
        // At z=0 both reduce to E=1 regardless of w (DE density ratio = 1 at a=1).
        assert!(approx(cpl.e_of_z(0.0), 1.0, 1e-9));
    }
}
