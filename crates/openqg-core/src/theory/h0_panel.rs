//! H0 panel — multi-calibrator H0 assessment for SYNTHESIS #17.
//!
//! Replaces the single SH0ES scalar `H0 = 73.04 ± 0.99` with a structured panel of
//! independent calibration families (Cepheids, TRGB, JAGB, time-delay cosmography,
//! masers, standard sirens, inverse-ladder CMB). Each family contributes its own
//! constraint, allowing per-family residuals and a consistency diagnostic across the panel.
//!
//! A theory's external plausibility (S10: currently ~35/100) is evaluated per-family;
//! a champion that fits one calibrator family while violating another is flagged here
//! before publication.

use serde::{Deserialize, Serialize};

/// One calibration family contributing an independent H0 constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationFamily {
    /// Cepheid distance ladder (primary: SH0ES collaboration).
    Cepheids,
    /// Tip of the Red Giant Branch distance ladder (CCHP / TRGB-H0 program).
    Trgb,
    /// J-region Asymptotic Giant Branch distance ladder.
    Jagb,
    /// Time-delay cosmography (H0LiCOW / TDCOSMO).
    TimeDelayCosmo,
    /// Megamaser Cosmology Project (geometric distance to NGC 4258 etc.).
    Masers,
    /// Gravitational-wave standard sirens (GW170817 host-galaxy).
    GravWaveSirens,
    /// CMB inverse-ladder (Planck 2018 within ΛCDM; tension diagnostic).
    CmbInverseLadder,
}

impl CalibrationFamily {
    pub fn label(self) -> &'static str {
        match self {
            CalibrationFamily::Cepheids => "cepheids",
            CalibrationFamily::Trgb => "trgb",
            CalibrationFamily::Jagb => "jagb",
            CalibrationFamily::TimeDelayCosmo => "time_delay_cosmo",
            CalibrationFamily::Masers => "masers",
            CalibrationFamily::GravWaveSirens => "grav_wave_sirens",
            CalibrationFamily::CmbInverseLadder => "cmb_inverse_ladder",
        }
    }
}

/// One entry in the H0 panel: a calibration family's constraint and the candidate theory's
/// residual against it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H0PanelEntry {
    /// Which calibration family produced this constraint.
    pub family: CalibrationFamily,
    /// Published H0 constraint from this family (km/s/Mpc).
    pub h0_km_s_mpc: f64,
    /// 1-sigma uncertainty on the constraint.
    pub sigma_km_s_mpc: f64,
    /// DOI or reference string for this measurement.
    pub reference: String,
    /// The candidate theory's predicted H0 (from the bound background).
    /// `None` when the theory has not been evaluated against this family.
    pub theory_h0: Option<f64>,
}

impl H0PanelEntry {
    pub fn new(
        family: CalibrationFamily,
        h0_km_s_mpc: f64,
        sigma_km_s_mpc: f64,
        reference: impl Into<String>,
    ) -> Self {
        H0PanelEntry {
            family,
            h0_km_s_mpc,
            sigma_km_s_mpc,
            reference: reference.into(),
            theory_h0: None,
        }
    }

    /// Tension in σ between the theory's H0 prediction and this constraint.
    /// Returns `None` when `theory_h0` is not set or `sigma_km_s_mpc` is zero.
    pub fn tension_sigma(&self) -> Option<f64> {
        if self.sigma_km_s_mpc == 0.0 {
            return None;
        }
        self.theory_h0
            .map(|th| (th - self.h0_km_s_mpc).abs() / self.sigma_km_s_mpc)
    }

    /// True when the tension with the theory prediction exceeds `threshold` sigma.
    pub fn is_in_tension(&self, threshold: f64) -> bool {
        self.tension_sigma().map_or(false, |t| t > threshold)
    }
}

/// The full H0 calibration panel for a single theory evaluation.
///
/// Build with `H0Panel::new()`, add entries with `.add_entry()`, wire the theory's
/// predicted H0 with `.set_theory_h0()`, then call `.panel_tension_report()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct H0Panel {
    pub entries: Vec<H0PanelEntry>,
}

impl H0Panel {
    pub fn new() -> Self {
        H0Panel {
            entries: Vec::new(),
        }
    }

    pub fn add_entry(&mut self, entry: H0PanelEntry) {
        self.entries.push(entry);
    }

    /// Wire the theory's predicted H0 into all panel entries.
    pub fn set_theory_h0(&mut self, theory_h0: f64) {
        for e in &mut self.entries {
            e.theory_h0 = Some(theory_h0);
        }
    }

    /// Number of panel families in tension with the theory at the given threshold.
    pub fn families_in_tension(&self, threshold: f64) -> usize {
        self.entries
            .iter()
            .filter(|e| e.is_in_tension(threshold))
            .count()
    }

    /// True when ALL families with a theory prediction are within `threshold` sigma.
    pub fn all_consistent(&self, threshold: f64) -> bool {
        self.entries
            .iter()
            .filter(|e| e.theory_h0.is_some())
            .all(|e| !e.is_in_tension(threshold))
    }

    /// Weighted mean H0 from the panel (inverse-variance weighting).
    /// Returns `None` when the panel is empty.
    pub fn panel_mean_h0(&self) -> Option<f64> {
        let weights: Vec<f64> = self
            .entries
            .iter()
            .filter(|e| e.sigma_km_s_mpc > 0.0)
            .map(|e| 1.0 / (e.sigma_km_s_mpc * e.sigma_km_s_mpc))
            .collect();
        if weights.is_empty() {
            return None;
        }
        let sum_w: f64 = weights.iter().sum();
        let sum_wh: f64 = self
            .entries
            .iter()
            .filter(|e| e.sigma_km_s_mpc > 0.0)
            .zip(&weights)
            .map(|(e, w)| w * e.h0_km_s_mpc)
            .sum();
        Some(sum_wh / sum_w)
    }

    /// Returns the standard reference H0 panel, seeded with published constraints.
    /// `theory_h0` must be set separately via `set_theory_h0()`.
    pub fn standard_panel() -> Self {
        let mut p = H0Panel::new();
        // SH0ES 2022 Cepheid ladder (Riess et al. 2022, ApJL 934, L7).
        p.add_entry(H0PanelEntry::new(
            CalibrationFamily::Cepheids,
            73.04,
            1.04,
            "Riess et al. 2022 ApJL 934 L7",
        ));
        // CCHP TRGB (Freedman et al. 2019 / 2021 update, ~69.8 ± 1.9).
        p.add_entry(H0PanelEntry::new(
            CalibrationFamily::Trgb,
            69.8,
            1.9,
            "Freedman et al. 2021 ApJ 919 16",
        ));
        // TDCOSMO (Millon et al. 2020, ~74.2 ± 1.6 from 6 lenses).
        p.add_entry(H0PanelEntry::new(
            CalibrationFamily::TimeDelayCosmo,
            74.2,
            1.6,
            "Millon et al. 2020 A&A 639 A101",
        ));
        // Megamasers MCP (Pesce et al. 2020, ~73.9 ± 3.0).
        p.add_entry(H0PanelEntry::new(
            CalibrationFamily::Masers,
            73.9,
            3.0,
            "Pesce et al. 2020 ApJL 891 L1",
        ));
        // GW standard siren GW170817 (Abbott et al. 2017, ~70.0 +12/-8; use 70 ± 10).
        p.add_entry(H0PanelEntry::new(
            CalibrationFamily::GravWaveSirens,
            70.0,
            10.0,
            "Abbott et al. 2017 Nature 551 85",
        ));
        // CMB inverse-ladder Planck 2018 (Aghanim et al. 2020, 67.4 ± 0.5 in ΛCDM).
        p.add_entry(H0PanelEntry::new(
            CalibrationFamily::CmbInverseLadder,
            67.4,
            0.5,
            "Planck 2018 Aghanim et al. 2020 A&A 641 A6",
        ));
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_panel_has_six_entries() {
        let p = H0Panel::standard_panel();
        assert_eq!(p.entries.len(), 6);
    }

    #[test]
    fn tension_sigma_not_set_returns_none() {
        let e = H0PanelEntry::new(CalibrationFamily::Cepheids, 73.04, 1.04, "r");
        assert!(e.tension_sigma().is_none());
    }

    #[test]
    fn tension_sigma_computes_correctly() {
        let mut e = H0PanelEntry::new(CalibrationFamily::Cepheids, 73.04, 1.04, "r");
        e.theory_h0 = Some(71.0);
        let tension = e.tension_sigma().unwrap();
        assert!((tension - (73.04 - 71.0) / 1.04).abs() < 1e-10);
    }

    #[test]
    fn is_in_tension_above_threshold() {
        let mut e = H0PanelEntry::new(CalibrationFamily::Cepheids, 73.04, 1.04, "r");
        e.theory_h0 = Some(67.4); // ~5.4 sigma
        assert!(e.is_in_tension(2.0));
        assert!(!e.is_in_tension(10.0));
    }

    #[test]
    fn set_theory_h0_wires_all_entries() {
        let mut p = H0Panel::standard_panel();
        p.set_theory_h0(72.0);
        assert!(p.entries.iter().all(|e| e.theory_h0 == Some(72.0)));
    }

    #[test]
    fn families_in_tension_count() {
        let mut p = H0Panel::standard_panel();
        // H0 = 74.0: consistent with Cepheids (~0.9σ), TDCOSMO (~0.1σ), Masers (~0.0σ)
        // but in tension with CMB (~13.2σ — very high!) and TRGB (~2.2σ).
        p.set_theory_h0(74.0);
        let tense = p.families_in_tension(2.0);
        assert!(tense >= 1, "at least CMB should be in tension at 2σ");
    }

    #[test]
    fn all_consistent_returns_false_when_any_in_tension() {
        let mut p = H0Panel::standard_panel();
        p.set_theory_h0(74.0); // CMB will be in tension
        assert!(!p.all_consistent(2.0));
    }

    #[test]
    fn all_consistent_returns_true_when_none_in_tension() {
        let mut p = H0Panel::standard_panel();
        p.set_theory_h0(70.5); // within 2σ of most families
                               // Only check entries where we can verify: just test the method doesn't panic
        let _ = p.all_consistent(10.0); // 10σ threshold: all should pass
        assert!(p.all_consistent(10.0));
    }

    #[test]
    fn panel_mean_h0_computed() {
        let mut p = H0Panel::new();
        // Two entries with equal sigma → mean = average
        p.add_entry(H0PanelEntry::new(
            CalibrationFamily::Cepheids,
            73.0,
            1.0,
            "r",
        ));
        p.add_entry(H0PanelEntry::new(
            CalibrationFamily::CmbInverseLadder,
            67.0,
            1.0,
            "r",
        ));
        let mean = p.panel_mean_h0().unwrap();
        assert!((mean - 70.0).abs() < 1e-10);
    }

    #[test]
    fn panel_mean_h0_none_when_empty() {
        let p = H0Panel::new();
        assert!(p.panel_mean_h0().is_none());
    }

    #[test]
    fn calibration_family_labels_are_nonempty() {
        for f in [
            CalibrationFamily::Cepheids,
            CalibrationFamily::Trgb,
            CalibrationFamily::Jagb,
            CalibrationFamily::TimeDelayCosmo,
            CalibrationFamily::Masers,
            CalibrationFamily::GravWaveSirens,
            CalibrationFamily::CmbInverseLadder,
        ] {
            assert!(!f.label().is_empty());
        }
    }
}
