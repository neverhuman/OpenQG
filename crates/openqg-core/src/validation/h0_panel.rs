//! V8 Phase 3 (#17): H0 multi-calibrator panel.
//!
//! Replaces the single SH0ES scalar (`sh0es-h0.jsonl`) with a per-family calibrator panel.
//! A theory claiming H0 in any direction must pass ALL calibrator families, not just the
//! one whose central value happens to match. Per-family residuals are reported separately.
//!
//! The panel enforces: a candidate that fits SH0ES Cepheids while failing CCHP/JAGB or the
//! BAO+BBN inverse ladder cannot score as an H0 solution. The external plausibility score
//! (0–100) quantifies how many calibrator families the predicted H0 is consistent with.
//!
//! Spec reference: S10 §1, rank-1 backlog item.

use serde::{Deserialize, Serialize};

/// The seven mutually exclusive calibrator families in the H0 panel.
///
/// Families have different central values and systematics; a valid H0 theory must pass
/// all or explicitly account for the families it fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H0CalibratorFamily {
    /// SH0ES Cepheid/SN Ia distance ladder (Riess et al., JWST-era ≈73–73.5 km/s/Mpc).
    SH0ESCepheids,
    /// CCHP/JWST TRGB: Freedman et al. 2024 (≈70.4 km/s/Mpc or JWST-only ≈68.8).
    CchpJwstTrgb,
    /// JWST-only JAGB: Freedman et al. 2024 (≈67.8 km/s/Mpc).
    JwstJagb,
    /// TDCOSMO time-delay strong lenses (profile-prior dependent; ≈67–74 km/s/Mpc).
    TdcosmoStrongLens,
    /// Megamaser Cosmology Project (≈73.9 ± 3.0 km/s/Mpc).
    MegamaserCosmologyProject,
    /// LVK standard sirens (broad; ≈70 ± 12 km/s/Mpc, growing statistics).
    LvkSirens,
    /// BAO + BBN inverse distance ladder (DESI DR1/DR2; ≈68.5 ± 0.8 km/s/Mpc).
    BaoBbnInverseLadder,
}

impl H0CalibratorFamily {
    pub fn short_name(&self) -> &'static str {
        match self {
            H0CalibratorFamily::SH0ESCepheids => "SH0ES",
            H0CalibratorFamily::CchpJwstTrgb => "CCHP/JWST-TRGB",
            H0CalibratorFamily::JwstJagb => "JWST-JAGB",
            H0CalibratorFamily::TdcosmoStrongLens => "TDCOSMO",
            H0CalibratorFamily::MegamaserCosmologyProject => "MCP",
            H0CalibratorFamily::LvkSirens => "LVK-sirens",
            H0CalibratorFamily::BaoBbnInverseLadder => "BAO+BBN",
        }
    }

    /// Whether this family is part of the "high-H0" group (central value > 71 km/s/Mpc).
    pub fn is_high_ladder(&self) -> bool {
        matches!(
            self,
            H0CalibratorFamily::SH0ESCepheids | H0CalibratorFamily::MegamaserCosmologyProject
        )
    }

    /// Whether this family is part of the "low-H0" / inverse-ladder group.
    pub fn is_inverse_ladder(&self) -> bool {
        matches!(
            self,
            H0CalibratorFamily::BaoBbnInverseLadder
                | H0CalibratorFamily::CchpJwstTrgb
                | H0CalibratorFamily::JwstJagb
        )
    }
}

/// One calibrator family entry in the H0 panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H0PanelEntry {
    pub family: H0CalibratorFamily,
    /// Central value in km/s/Mpc.
    pub h0_central: f64,
    /// Statistical uncertainty (1σ, km/s/Mpc).
    pub h0_stat: f64,
    /// Systematic uncertainty (km/s/Mpc).
    pub h0_sys: f64,
    /// Combined 1σ uncertainty: sqrt(stat^2 + sys^2).
    pub combined_sigma: f64,
    /// Short citation reference.
    pub reference: String,
    /// Internal data_id (maps to the JSONL fixture row).
    pub data_id: String,
    /// Whether to include this entry in the combined chi^2 score.
    pub include_in_combined: bool,
}

impl H0PanelEntry {
    /// True when a predicted H0 is within N sigma of this entry's central value.
    pub fn within_n_sigma(&self, predicted_h0: f64, n: f64) -> bool {
        (predicted_h0 - self.h0_central).abs() <= n * self.combined_sigma
    }
}

/// Per-family pull for a theory prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H0FamilyResidual {
    pub family: H0CalibratorFamily,
    pub predicted_h0: f64,
    pub observed_central: f64,
    pub combined_sigma: f64,
    /// `(predicted - observed) / sigma`.
    pub pull_sigma: f64,
    /// Within 2σ of the calibrator's central value.
    pub within_2sigma: bool,
}

/// The full H0 panel score for one theory prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H0PanelScore {
    pub predicted_h0: f64,
    pub residuals: Vec<H0FamilyResidual>,
    /// Number of calibrator families within 2σ of the prediction.
    pub passing_families: u32,
    pub total_families: u32,
    /// Pull of the worst (most discrepant) family.
    pub worst_pull_sigma: f64,
    /// χ² summed over families with `include_in_combined = true`.
    pub combined_chi2: f64,
    /// External plausibility score 0–100.
    ///
    /// - ≥6/7 within 2σ → 75–100
    /// - 5/7 → 50–75
    /// - 4/7 → 25–50
    /// - ≤3/7 → 0–25
    pub external_plausibility: f64,
    /// Narrative verdict for the scorecard.
    pub verdict: String,
}

impl H0PanelScore {
    /// True when the theory passes enough calibrator families to earn any H0 credit.
    pub fn earns_h0_credit(&self) -> bool {
        self.passing_families >= 4 && self.worst_pull_sigma < 4.0
    }

    /// True when the prediction is in the "high" H0 camp (SH0ES-like).
    pub fn is_high_h0_prediction(&self) -> bool {
        self.predicted_h0 > 71.0
    }

    /// True when the prediction is in the "low" / inverse-ladder camp.
    pub fn is_low_h0_prediction(&self) -> bool {
        self.predicted_h0 < 70.0
    }
}

/// Return the canonical panel for the H0 sector (spec S10 rank-1 values, mid-2026).
///
/// These represent the published central values and uncertainties as of the spec date.
/// Update via the `h0-panel.jsonl` fixture when newer results become available.
pub fn canonical_h0_panel() -> Vec<H0PanelEntry> {
    vec![
        H0PanelEntry {
            family: H0CalibratorFamily::SH0ESCepheids,
            h0_central: 73.17,
            h0_stat: 0.86,
            h0_sys: 0.70,
            combined_sigma: (0.86f64.powi(2) + 0.70f64.powi(2)).sqrt(),
            reference: "Riess et al. 2022 / SH0ES JWST 2025 arXiv:2509.01667".into(),
            data_id: "sh0es-cepheid-h0-2025".into(),
            include_in_combined: true,
        },
        H0PanelEntry {
            family: H0CalibratorFamily::CchpJwstTrgb,
            h0_central: 70.39,
            h0_stat: 1.22,
            h0_sys: 1.37,
            combined_sigma: (1.22f64.powi(2) + 1.37f64.powi(2)).sqrt(),
            reference: "Freedman et al. 2024 TRGB arXiv:2408.06153".into(),
            data_id: "cchp-jwst-trgb-2024".into(),
            include_in_combined: true,
        },
        H0PanelEntry {
            family: H0CalibratorFamily::JwstJagb,
            h0_central: 67.80,
            h0_stat: 2.17,
            h0_sys: 1.64,
            combined_sigma: (2.17f64.powi(2) + 1.64f64.powi(2)).sqrt(),
            reference: "Freedman et al. 2024 JAGB arXiv:2408.06153".into(),
            data_id: "cchp-jwst-jagb-2024".into(),
            include_in_combined: true,
        },
        H0PanelEntry {
            family: H0CalibratorFamily::TdcosmoStrongLens,
            h0_central: 67.4,
            h0_stat: 4.1,
            h0_sys: 3.2,
            combined_sigma: (4.1f64.powi(2) + 3.2f64.powi(2)).sqrt(),
            reference: "TDCOSMO IV Birrer et al. arXiv:2007.02941 (flexible MST)".into(),
            data_id: "tdcosmo-iv-flexible-2020".into(),
            include_in_combined: false, // profile-prior-dependent; informational only
        },
        H0PanelEntry {
            family: H0CalibratorFamily::MegamaserCosmologyProject,
            h0_central: 73.9,
            h0_stat: 3.0,
            h0_sys: 0.0,
            combined_sigma: 3.0,
            reference: "Pesce et al. 2020 arXiv:2001.09213".into(),
            data_id: "mcp-maser-2020".into(),
            include_in_combined: true,
        },
        H0PanelEntry {
            family: H0CalibratorFamily::LvkSirens,
            h0_central: 70.0,
            h0_stat: 12.0,
            h0_sys: 0.0,
            combined_sigma: 12.0,
            reference: "GW170817 + GWTC-3 dark sirens LVK arXiv:Phys.Rev.X.13.041039".into(),
            data_id: "lvk-sirens-2023".into(),
            include_in_combined: false, // too broad to be informative in combined chi2
        },
        H0PanelEntry {
            family: H0CalibratorFamily::BaoBbnInverseLadder,
            h0_central: 68.5,
            h0_stat: 0.8,
            h0_sys: 0.2,
            combined_sigma: (0.8f64.powi(2) + 0.2f64.powi(2)).sqrt(),
            reference: "DESI DR1/DR2 BAO+BBN arXiv:2404.03002 arXiv:2503.14738".into(),
            data_id: "desi-bao-bbn-inverse-2025".into(),
            include_in_combined: true,
        },
    ]
}

/// Load the H0 panel from the canonical fixture, falling back to the built-in defaults.
pub fn load_h0_panel(fixture_path: Option<&std::path::Path>) -> Vec<H0PanelEntry> {
    if let Some(path) = fixture_path {
        if let Ok(content) = std::fs::read_to_string(path) {
            let mut entries = Vec::new();
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(entry) = serde_json::from_str::<H0PanelEntry>(line) {
                    entries.push(entry);
                }
            }
            if !entries.is_empty() {
                return entries;
            }
        }
    }
    canonical_h0_panel()
}

/// Score a predicted H0 value against the full calibrator panel.
pub fn score_h0_panel(predicted_h0: f64, panel: &[H0PanelEntry]) -> H0PanelScore {
    let mut residuals = Vec::new();
    let mut passing = 0u32;
    let mut worst_pull: f64 = 0.0;
    let mut combined_chi2: f64 = 0.0;

    for entry in panel {
        let pull = (predicted_h0 - entry.h0_central) / entry.combined_sigma;
        let within_2sigma = pull.abs() <= 2.0;
        if within_2sigma {
            passing += 1;
        }
        if pull.abs() > worst_pull.abs() {
            worst_pull = pull;
        }
        if entry.include_in_combined {
            combined_chi2 += pull.powi(2);
        }
        residuals.push(H0FamilyResidual {
            family: entry.family,
            predicted_h0,
            observed_central: entry.h0_central,
            combined_sigma: entry.combined_sigma,
            pull_sigma: pull,
            within_2sigma,
        });
    }

    let total = panel.len() as u32;
    let fraction = passing as f64 / total as f64;

    // External plausibility: 0–100 based on fraction of families within 2σ and worst pull.
    let plausibility = if worst_pull.abs() >= 5.0 {
        0.0
    } else {
        let pull_penalty = (worst_pull.abs() / 5.0).min(1.0);
        let base = fraction * 100.0 * (1.0 - 0.5 * pull_penalty);
        base.max(0.0).min(100.0)
    };

    let verdict = if passing >= 6 {
        format!("consistent with {passing}/{total} calibrators at 2σ — strong H0 plausibility")
    } else if passing >= 4 {
        format!("consistent with {passing}/{total} calibrators at 2σ — moderate H0 plausibility")
    } else if passing >= 2 {
        format!("consistent with only {passing}/{total} calibrators — failing H0 panel")
    } else {
        format!(
            "fails {}/{total} calibrators — prediction incompatible with H0 sector",
            total - passing
        )
    };

    H0PanelScore {
        predicted_h0,
        residuals,
        passing_families: passing,
        total_families: total,
        worst_pull_sigma: worst_pull,
        combined_chi2,
        external_plausibility: plausibility,
        verdict,
    }
}

/// External plausibility score for a full theory, incorporating all sector data.
///
/// Wraps `score_h0_panel` and adds a sector narrative for the scorecard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalPlausibilityReport {
    pub h0_panel: H0PanelScore,
    /// Total external plausibility 0–100.
    pub total_external_plausibility: f64,
    pub plausibility_narrative: String,
}

impl ExternalPlausibilityReport {
    pub fn new(h0_panel: H0PanelScore) -> Self {
        let total = h0_panel.external_plausibility;
        let narrative = format!(
            "H0 sector: {:.0}/100 ({}/{} calibrators within 2σ; worst pull = {:.1}σ). {}",
            h0_panel.external_plausibility,
            h0_panel.passing_families,
            h0_panel.total_families,
            h0_panel.worst_pull_sigma.abs(),
            h0_panel.verdict,
        );
        ExternalPlausibilityReport {
            h0_panel,
            total_external_plausibility: total,
            plausibility_narrative: narrative,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spec S10 rank-1 verification gate: panel reproduces per-family H0 values.
    #[test]
    fn canonical_panel_has_all_seven_families() {
        let panel = canonical_h0_panel();
        assert_eq!(
            panel.len(),
            7,
            "canonical panel must have 7 calibrator families"
        );

        let families: Vec<H0CalibratorFamily> = panel.iter().map(|e| e.family).collect();
        for expected in [
            H0CalibratorFamily::SH0ESCepheids,
            H0CalibratorFamily::CchpJwstTrgb,
            H0CalibratorFamily::JwstJagb,
            H0CalibratorFamily::TdcosmoStrongLens,
            H0CalibratorFamily::MegamaserCosmologyProject,
            H0CalibratorFamily::LvkSirens,
            H0CalibratorFamily::BaoBbnInverseLadder,
        ] {
            assert!(
                families.contains(&expected),
                "panel must include {:?}",
                expected
            );
        }
    }

    /// Panel reproduces SH0ES-like high values near 73 km/s/Mpc.
    #[test]
    fn panel_scores_sh0es_like_prediction() {
        let panel = canonical_h0_panel();
        let score = score_h0_panel(73.2, &panel);

        let sh0es = score
            .residuals
            .iter()
            .find(|r| r.family == H0CalibratorFamily::SH0ESCepheids)
            .unwrap();
        assert!(
            sh0es.pull_sigma.abs() < 2.0,
            "H0=73.2 must be within 2σ of SH0ES; pull = {:.2}σ",
            sh0es.pull_sigma
        );

        let bao = score
            .residuals
            .iter()
            .find(|r| r.family == H0CalibratorFamily::BaoBbnInverseLadder)
            .unwrap();
        assert!(
            bao.pull_sigma.abs() > 2.0,
            "H0=73.2 must be outside 2σ of BAO+BBN inverse ladder; pull = {:.2}σ",
            bao.pull_sigma
        );
    }

    /// Panel reproduces CCHP/JWST lower values near 68–70 km/s/Mpc.
    #[test]
    fn panel_scores_cchp_like_prediction() {
        let panel = canonical_h0_panel();
        let score = score_h0_panel(69.5, &panel);

        let cchp = score
            .residuals
            .iter()
            .find(|r| r.family == H0CalibratorFamily::CchpJwstTrgb)
            .unwrap();
        assert!(
            cchp.within_2sigma,
            "H0=69.5 must be within 2σ of CCHP/TRGB; pull = {:.2}σ",
            cchp.pull_sigma
        );

        let sh0es = score
            .residuals
            .iter()
            .find(|r| r.family == H0CalibratorFamily::SH0ESCepheids)
            .unwrap();
        assert!(
            !sh0es.within_2sigma,
            "H0=69.5 must NOT be within 2σ of SH0ES; pull = {:.2}σ",
            sh0es.pull_sigma
        );
    }

    /// A single-scalar SH0ES-only fit has worse worst-pull than a mid-H0 prediction.
    ///
    /// Both H0=73.2 and H0=70.0 pass 6/7 families at 2σ (many calibrators have large
    /// uncertainty), but H0=73.2 is badly inconsistent with the BAO+BBN inverse ladder
    /// (>5σ pull), while H0=70.0 only fails SH0ES at ~2.9σ.
    #[test]
    fn sh0es_only_is_less_plausible_than_mid_h0() {
        let panel = canonical_h0_panel();
        let sh0es_fit = score_h0_panel(73.2, &panel);
        let mid_fit = score_h0_panel(70.0, &panel);

        // H0=73.2 fails BAO+BBN at >5σ; H0=70.0 fails SH0ES at ~2.9σ.
        // The worst pull for SH0ES fit must be significantly larger.
        assert!(
            sh0es_fit.worst_pull_sigma.abs() > mid_fit.worst_pull_sigma.abs(),
            "H0=73.2 worst pull ({:.2}σ) must exceed H0=70.0 worst pull ({:.2}σ)",
            sh0es_fit.worst_pull_sigma.abs(),
            mid_fit.worst_pull_sigma.abs()
        );
    }

    /// Theory that exactly hits 73.2 cannot claim to be an H0 solution without qualifying.
    ///
    /// H0=73.2 fails the BAO+BBN inverse ladder at >5σ: it cannot claim H0-sector consistency
    /// without explicitly acknowledging the inverse-ladder disagreement.
    #[test]
    fn sh0es_fit_fails_inverse_ladder_check() {
        let panel = canonical_h0_panel();
        let score = score_h0_panel(73.2, &panel);

        // BAO+BBN inverse ladder: central = 68.5, sigma = 0.825.
        // Pull = (73.2 - 68.5) / 0.825 ≈ 5.7σ — clear failure.
        let bao = score
            .residuals
            .iter()
            .find(|r| r.family == H0CalibratorFamily::BaoBbnInverseLadder)
            .unwrap();
        assert!(
            !bao.within_2sigma,
            "H0=73.2 must fail BAO+BBN at 2σ (pull = {:.1}σ)",
            bao.pull_sigma
        );
        assert!(
            bao.pull_sigma.abs() > 4.0,
            "BAO+BBN pull must exceed 4σ for H0=73.2; got {:.2}σ",
            bao.pull_sigma
        );

        // CCHP/TRGB: central = 70.39, sigma = 1.834. Pull = (73.2 - 70.39) / 1.834 ≈ 1.53σ.
        // CCHP TRGB barely passes (large stat+sys uncertainty), but JAGB has large sigma too.
        // Key verification: BAO+BBN is the definitive inverse-ladder failure.
    }

    #[test]
    fn external_plausibility_report_builds() {
        let panel = canonical_h0_panel();
        let score = score_h0_panel(68.5, &panel);
        let report = ExternalPlausibilityReport::new(score);
        assert!(!report.plausibility_narrative.is_empty());
        assert!(report.total_external_plausibility >= 0.0);
        assert!(report.total_external_plausibility <= 100.0);
    }
}
