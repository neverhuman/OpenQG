//! Data tier vocabulary for SYNTHESIS #9: sealed data tiers + value firewall.
//!
//! `DataTierKind` classifies datasets by their sealing status. The manifest fails closed:
//! any dataset ID not explicitly registered is treated as `SealedKill`.
//!
//! The three tiers map to SYNTHESIS #9's firewall protocol:
//! - `OpenFit`: scored in the current fit; proposers may use published summary statistics.
//! - `SealedKill`: values must never reach proposers, ledgers, or certificates before
//!   adjudication; presence in a prompt/ledger is a content-bind violation.
//! - `FutureSealed`: not yet released or release date is pending; a pre-registered forecast
//!   in the prediction registry must exist before this dataset becomes available.

use serde::{Deserialize, Serialize};

/// Classification of a dataset by its sealing status in the engine's data firewall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataTierKind {
    /// Dataset is in the scored fit set. Proposers may reference published summary statistics
    /// but must not fit directly to the held values.
    OpenFit,
    /// Dataset is sealed. Values must not flow through proposer prompts, ledger records,
    /// or derivation certificates before adjudication. Violation → content-bind kill.
    SealedKill,
    /// Dataset has a future or pending release. A pre-registered forecast in the prediction
    /// registry must exist before this dataset's release date.
    FutureSealed,
}

impl DataTierKind {
    /// True when proposers are allowed to reference this dataset's values.
    pub fn proposer_visible(self) -> bool {
        self == DataTierKind::OpenFit
    }

    /// True when this dataset must be firewalled from proposers and scoring.
    pub fn requires_firewall(self) -> bool {
        self != DataTierKind::OpenFit
    }
}

/// A single dataset entry in the engine's data tier manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetRecord {
    /// Machine-readable identifier (e.g. `"desi-dr2-bao"`).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Sealing tier.
    pub tier: DataTierKind,
    /// Short description of the dataset and its role in scoring.
    pub description: String,
}

impl DatasetRecord {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        tier: DataTierKind,
        description: impl Into<String>,
    ) -> Self {
        DatasetRecord {
            id: id.into(),
            name: name.into(),
            tier,
            description: description.into(),
        }
    }
}

/// The engine's data tier manifest — all known datasets and their firewall tier.
///
/// Fails closed: any dataset ID not listed is treated as `SealedKill` by default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DataTierManifest {
    pub datasets: Vec<DatasetRecord>,
}

impl DataTierManifest {
    pub fn new() -> Self {
        DataTierManifest {
            datasets: Vec::new(),
        }
    }

    pub fn add(&mut self, record: DatasetRecord) {
        self.datasets.push(record);
    }

    /// Look up a dataset by ID. Returns `None` when the ID is not registered.
    pub fn get(&self, id: &str) -> Option<&DatasetRecord> {
        self.datasets.iter().find(|d| d.id == id)
    }

    /// Tier for a dataset ID. Returns `SealedKill` for unknown IDs (fails closed).
    pub fn tier_of(&self, id: &str) -> DataTierKind {
        self.get(id)
            .map(|d| d.tier)
            .unwrap_or(DataTierKind::SealedKill)
    }

    /// All datasets that are `OpenFit`.
    pub fn open_fit_datasets(&self) -> Vec<&DatasetRecord> {
        self.datasets
            .iter()
            .filter(|d| d.tier == DataTierKind::OpenFit)
            .collect()
    }

    /// All datasets that require firewalling (SealedKill or FutureSealed).
    pub fn firewalled_datasets(&self) -> Vec<&DatasetRecord> {
        self.datasets
            .iter()
            .filter(|d| d.tier.requires_firewall())
            .collect()
    }
}

/// A known dataset overlap that constitutes double-counting when both are included.
///
/// The canonical cases from SYNTHESIS #9: Pantheon+SH0ES (combined covariance already
/// includes the Cepheid-anchored H0; adding a standalone SH0ES H0 constraint re-uses
/// the same distance ladder measurements); DESI DR1 + DESI DR2 BAO (partial sky overlap
/// with correlated sample; stacking without explicit decorrelation inflates the evidence).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetOverlapRule {
    /// ID of the first dataset in the overlapping pair (matched by prefix or exact ID).
    pub dataset_a: String,
    /// ID of the second dataset in the overlapping pair.
    pub dataset_b: String,
    /// Short description of why simultaneous use is double-counting.
    pub overlap_note: String,
}

/// Outcome of a double-count audit across a list of dataset IDs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoubleCountReport {
    /// Overlap rules that fired (pairs of datasets that must not both appear).
    pub violations: Vec<DatasetOverlapRule>,
    /// True when no overlapping pairs were found (dataset combination is clean).
    pub is_clean: bool,
}

impl DoubleCountReport {
    fn clean() -> Self {
        DoubleCountReport {
            violations: Vec::new(),
            is_clean: true,
        }
    }

    fn with_violations(violations: Vec<DatasetOverlapRule>) -> Self {
        let is_clean = violations.is_empty();
        DoubleCountReport {
            violations,
            is_clean,
        }
    }
}

/// Check a list of dataset IDs against the known overlap rules.
///
/// Returns a [`DoubleCountReport`] listing any pairs that simultaneously appear.
/// Matching is by prefix: `"pantheon-plus-shoes"` matches any ID starting with
/// `"pantheon-plus-shoes"` or `"shoes-h0"`.
pub fn audit_double_count(dataset_ids: &[&str]) -> DoubleCountReport {
    let rules = canonical_overlap_rules();
    let mut violations = Vec::new();

    for rule in &rules {
        let has_a = dataset_ids
            .iter()
            .any(|id| id.starts_with(rule.dataset_a.as_str()));
        let has_b = dataset_ids
            .iter()
            .any(|id| id.starts_with(rule.dataset_b.as_str()));
        if has_a && has_b {
            violations.push(rule.clone());
        }
    }

    DoubleCountReport::with_violations(violations)
}

/// The canonical double-count rules for the V8 growth suppression campaign.
///
/// Each rule names a pair of dataset prefixes whose simultaneous use constitutes
/// re-use of the same underlying measurements without decorrelation.
pub fn canonical_overlap_rules() -> Vec<DatasetOverlapRule> {
    vec![
        DatasetOverlapRule {
            dataset_a: "pantheon-plus-shoes".into(),
            dataset_b: "shoes-h0".into(),
            overlap_note: "Pantheon+SH0ES combined covariance already includes the SH0ES Cepheid \
                           anchor; adding a standalone SH0ES H0 point double-counts the same \
                           distance-ladder measurements without decorrelation"
                .into(),
        },
        DatasetOverlapRule {
            dataset_a: "desi-dr1".into(),
            dataset_b: "desi-dr2".into(),
            overlap_note: "DESI DR1 and DR2 share overlapping sky footprint and galaxy samples; \
                           stacking both BAO signals without an explicit overlap-corrected \
                           covariance inflates the combined evidence"
                .into(),
        },
        DatasetOverlapRule {
            dataset_a: "desi-dr2".into(),
            dataset_b: "desi-dr3".into(),
            overlap_note: "DESI DR2 and DR3 have cumulative sample overlap; \
                           use the latest data release only or supply a decorrelation matrix"
                .into(),
        },
        DatasetOverlapRule {
            dataset_a: "kids-1000".into(),
            dataset_b: "kids-dr5".into(),
            overlap_note: "KiDS-1000 is a subset of the KiDS DR5 (full survey) footprint; \
                           simultaneous use double-counts the shared shape-catalog galaxies"
                .into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_with_all_tiers() -> DataTierManifest {
        let mut m = DataTierManifest::new();
        m.add(DatasetRecord::new(
            "eboss-dr16-rsd",
            "eBOSS DR16 RSD",
            DataTierKind::OpenFit,
            "full-shape RSD power spectrum",
        ));
        m.add(DatasetRecord::new(
            "desi-dr3-bao",
            "DESI DR3 BAO",
            DataTierKind::SealedKill,
            "primary pre-registered forecast target",
        ));
        m.add(DatasetRecord::new(
            "euclid-dr1-wl",
            "Euclid DR1 Weak Lensing",
            DataTierKind::FutureSealed,
            "future sealed — release expected 2027",
        ));
        m
    }

    #[test]
    fn proposer_visible_only_for_open_fit() {
        assert!(DataTierKind::OpenFit.proposer_visible());
        assert!(!DataTierKind::SealedKill.proposer_visible());
        assert!(!DataTierKind::FutureSealed.proposer_visible());
    }

    #[test]
    fn requires_firewall_not_for_open_fit() {
        assert!(!DataTierKind::OpenFit.requires_firewall());
        assert!(DataTierKind::SealedKill.requires_firewall());
        assert!(DataTierKind::FutureSealed.requires_firewall());
    }

    #[test]
    fn unknown_dataset_id_is_sealed_kill() {
        let m = manifest_with_all_tiers();
        assert_eq!(m.tier_of("not-in-manifest"), DataTierKind::SealedKill);
    }

    #[test]
    fn open_fit_datasets_only_returns_open_fit() {
        let m = manifest_with_all_tiers();
        let open = m.open_fit_datasets();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].id, "eboss-dr16-rsd");
    }

    #[test]
    fn firewalled_datasets_excludes_open_fit() {
        let m = manifest_with_all_tiers();
        let fw = m.firewalled_datasets();
        assert_eq!(fw.len(), 2);
        assert!(fw.iter().any(|d| d.id == "desi-dr3-bao"));
        assert!(fw.iter().any(|d| d.id == "euclid-dr1-wl"));
    }

    #[test]
    fn get_returns_correct_record() {
        let m = manifest_with_all_tiers();
        let rec = m.get("euclid-dr1-wl").unwrap();
        assert_eq!(rec.tier, DataTierKind::FutureSealed);
    }

    #[test]
    fn manifest_serde_round_trips() {
        let m = manifest_with_all_tiers();
        let json = serde_json::to_string(&m).unwrap();
        let back: DataTierManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    // ---- DoubleCountAudit ----

    #[test]
    fn no_datasets_is_clean() {
        let report = audit_double_count(&[]);
        assert!(report.is_clean);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn single_dataset_is_clean() {
        let report = audit_double_count(&["eboss-dr16-rsd"]);
        assert!(report.is_clean);
    }

    #[test]
    fn pantheon_plus_shoes_and_shoes_h0_is_violation() {
        let report =
            audit_double_count(&["pantheon-plus-shoes", "shoes-h0-2022", "eboss-dr16-rsd"]);
        assert!(!report.is_clean);
        assert_eq!(report.violations.len(), 1);
        assert!(report.violations[0].overlap_note.contains("Cepheid"));
    }

    #[test]
    fn desi_dr1_and_dr2_is_violation() {
        let report = audit_double_count(&["desi-dr1-bao", "desi-dr2-bao"]);
        assert!(!report.is_clean);
        assert!(report.violations.iter().any(|v| v.dataset_a == "desi-dr1"));
    }

    #[test]
    fn desi_dr2_and_dr3_is_violation() {
        let report = audit_double_count(&["desi-dr2-bao", "desi-dr3-bao"]);
        assert!(!report.is_clean);
        assert!(report.violations.iter().any(|v| v.dataset_a == "desi-dr2"));
    }

    #[test]
    fn kids_dr5_with_kids_1000_is_violation() {
        let report = audit_double_count(&["kids-1000-shear", "kids-dr5-shear"]);
        assert!(!report.is_clean);
    }

    #[test]
    fn disjoint_datasets_are_clean() {
        let report = audit_double_count(&["eboss-dr16-rsd", "desi-dr3-bao", "kids-dr5-shear"]);
        assert!(report.is_clean, "{:?}", report.violations);
    }

    #[test]
    fn canonical_overlap_rules_is_nonempty() {
        assert!(!canonical_overlap_rules().is_empty());
    }

    #[test]
    fn double_count_report_serde_round_trip() {
        let report = audit_double_count(&["pantheon-plus-shoes", "shoes-h0"]);
        let json = serde_json::to_string(&report).unwrap();
        let back: DoubleCountReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.is_clean, report.is_clean);
        assert_eq!(back.violations.len(), report.violations.len());
    }
}
