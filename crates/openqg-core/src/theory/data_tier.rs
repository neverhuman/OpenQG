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
}
