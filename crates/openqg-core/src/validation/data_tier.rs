//! V8 Phase 1 (#9): DataTier taxonomy and ValueFirewall.
//!
//! Scored data is partitioned into three tiers to prevent look-elsewhere inflation and ensure
//! that held-out datasets are never seen by the proposer before they are adjudicated:
//!
//! | Tier           | Who can see values     | Notes                                                |
//! |----------------|------------------------|------------------------------------------------------|
//! | `OpenFit`      | Everyone               | Standard fit-set data; ΔlnZ earns 0 novelty credit  |
//! | `SealedKill`   | Engine only (no output)| Held-out killer; a theory must NOT fit this to pass  |
//! | `FutureSealed` | Nobody yet             | Will become `SealedKill` after data are released     |
//!
//! The `ValueFirewall` check enforces that a proposer packet (theory JSON submitted for scoring)
//! does not carry numeric values from `SealedKill` or `FutureSealed` datasets.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The tier of a dataset entry in the scoring pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataTier {
    /// Publicly available fit-set data. Scores here earn 0 novelty credit — they go through the
    /// evidence gate, not the forecast points component.
    OpenFit,
    /// Held-out data that the engine uses as a kill test but whose values are not sent back to the
    /// proposer. A theory that fits `SealedKill` data *well* earns suspicion (it may have leaked).
    SealedKill,
    /// Datasets that do not yet exist publicly. Their expected values are pre-registered and
    /// sealed; they become `SealedKill` entries at adjudication time.
    FutureSealed,
}

impl DataTier {
    /// True when this tier should never have its measured values exposed to the proposer layer.
    pub fn is_sealed(self) -> bool {
        matches!(self, DataTier::SealedKill | DataTier::FutureSealed)
    }

    /// True when scores against this tier earn novelty credit (forecast points).
    /// Only `FutureSealed` entries that were registered before the data existed can earn credit.
    pub fn earns_novelty_credit(self) -> bool {
        self == DataTier::FutureSealed
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DataTier::OpenFit => "open_fit",
            DataTier::SealedKill => "sealed_kill",
            DataTier::FutureSealed => "future_sealed",
        }
    }
}

impl std::fmt::Display for DataTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A record declaring the tier of a named dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataTierManifest {
    /// Stable identifier for this dataset (e.g. "desi-dr3-bao-v1").
    pub dataset_id: String,
    /// The tier classification.
    pub tier: DataTier,
    /// Content hash (SHA-256 hex) of the dataset file at the time of classification.
    /// Empty for `FutureSealed` entries (the data don't exist yet).
    pub content_hash: String,
    /// Human-readable note explaining the tier assignment decision.
    pub note: String,
}

impl DataTierManifest {
    pub fn new(dataset_id: impl Into<String>, tier: DataTier) -> Self {
        DataTierManifest {
            dataset_id: dataset_id.into(),
            tier,
            content_hash: String::new(),
            note: String::new(),
        }
    }
}

/// A violation found by the value firewall check.
#[derive(Debug, Clone, PartialEq)]
pub struct FirewallViolation {
    /// The dataset ID that was violated.
    pub dataset_id: String,
    /// The key path in the proposer packet where the sealed value was found.
    pub key_path: String,
    /// Why this is a violation.
    pub reason: String,
}

/// The result of a value firewall check.
#[derive(Debug, Clone, PartialEq)]
pub struct FirewallReport {
    pub violations: Vec<FirewallViolation>,
    /// True when any violation was found — drives `disqualified = true` in the scorecard.
    pub blocked: bool,
}

impl FirewallReport {
    pub fn clean() -> Self {
        FirewallReport {
            violations: vec![],
            blocked: false,
        }
    }
}

/// Check that a proposer packet (serialized as a JSON value) does not carry values from any
/// sealed dataset.
///
/// The check is conservative: it searches the JSON for any string value that matches a sealed
/// dataset ID. This prevents a trivially circumvented check (the real enforcement is operational,
/// not cryptographic), but it closes the obvious injection path.
///
/// `sealed_ids`: the set of dataset IDs classified as `SealedKill` or `FutureSealed`.
/// `packet_json`: the raw JSON of the proposer theory packet.
pub fn check_value_firewall(
    sealed_ids: &BTreeSet<String>,
    packet_json: &serde_json::Value,
) -> FirewallReport {
    if sealed_ids.is_empty() {
        return FirewallReport::clean();
    }

    let mut violations = Vec::new();
    scan_json(packet_json, sealed_ids, "", &mut violations);

    let blocked = !violations.is_empty();
    FirewallReport {
        violations,
        blocked,
    }
}

fn scan_json(
    value: &serde_json::Value,
    sealed_ids: &BTreeSet<String>,
    path: &str,
    violations: &mut Vec<FirewallViolation>,
) {
    match value {
        serde_json::Value::String(s) if sealed_ids.contains(s) => {
            violations.push(FirewallViolation {
                dataset_id: s.clone(),
                key_path: path.to_string(),
                reason: format!(
                    "proposer packet references sealed dataset '{}' at '{}'",
                    s, path
                ),
            });
        }
        serde_json::Value::String(_) => {}
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let child_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                scan_json(v, sealed_ids, &child_path, violations);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let child_path = format!("{path}[{i}]");
                scan_json(v, sealed_ids, &child_path, violations);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- DataTier ----

    #[test]
    fn open_fit_is_not_sealed() {
        assert!(!DataTier::OpenFit.is_sealed());
    }

    #[test]
    fn sealed_kill_is_sealed() {
        assert!(DataTier::SealedKill.is_sealed());
    }

    #[test]
    fn future_sealed_is_sealed() {
        assert!(DataTier::FutureSealed.is_sealed());
    }

    #[test]
    fn only_future_sealed_earns_novelty_credit() {
        assert!(!DataTier::OpenFit.earns_novelty_credit());
        assert!(!DataTier::SealedKill.earns_novelty_credit());
        assert!(DataTier::FutureSealed.earns_novelty_credit());
    }

    #[test]
    fn ordering_open_lt_sealed_lt_future() {
        assert!(DataTier::OpenFit < DataTier::SealedKill);
        assert!(DataTier::SealedKill < DataTier::FutureSealed);
    }

    #[test]
    fn display_roundtrip() {
        assert_eq!(DataTier::OpenFit.as_str(), "open_fit");
        assert_eq!(DataTier::SealedKill.as_str(), "sealed_kill");
        assert_eq!(DataTier::FutureSealed.as_str(), "future_sealed");
    }

    // ---- DataTierManifest ----

    #[test]
    fn manifest_new_is_empty_hash() {
        let m = DataTierManifest::new("desi-dr3-bao-v1", DataTier::FutureSealed);
        assert_eq!(m.dataset_id, "desi-dr3-bao-v1");
        assert_eq!(m.tier, DataTier::FutureSealed);
        assert!(m.content_hash.is_empty());
    }

    // ---- ValueFirewall ----

    #[test]
    fn clean_packet_produces_no_violations() {
        let sealed = BTreeSet::from(["desi-dr3-bao-v1".to_string()]);
        let packet = json!({ "theory_id": "ndgp", "h0": 67.4 });
        let report = check_value_firewall(&sealed, &packet);
        assert!(!report.blocked);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn packet_referencing_sealed_id_is_blocked() {
        let sealed = BTreeSet::from(["desi-dr3-bao-v1".to_string()]);
        let packet = json!({
            "theory_id": "ndgp",
            "dataset_reference": "desi-dr3-bao-v1"
        });
        let report = check_value_firewall(&sealed, &packet);
        assert!(report.blocked);
        assert_eq!(report.violations.len(), 1);
        assert_eq!(report.violations[0].dataset_id, "desi-dr3-bao-v1");
    }

    #[test]
    fn nested_reference_is_caught() {
        let sealed = BTreeSet::from(["future-euclid-wl".to_string()]);
        let packet = json!({
            "claims": [
                { "dataset": "future-euclid-wl", "value": 0.82 }
            ]
        });
        let report = check_value_firewall(&sealed, &packet);
        assert!(report.blocked);
        assert_eq!(report.violations[0].key_path, "claims[0].dataset");
    }

    #[test]
    fn empty_sealed_set_always_passes() {
        let sealed = BTreeSet::new();
        let packet = json!({ "dataset": "desi-dr3-bao-v1" });
        let report = check_value_firewall(&sealed, &packet);
        assert!(!report.blocked);
    }

    #[test]
    fn multiple_sealed_ids_all_caught() {
        let sealed = BTreeSet::from([
            "desi-dr3-bao-v1".to_string(),
            "future-euclid-wl".to_string(),
        ]);
        let packet = json!({
            "a": "desi-dr3-bao-v1",
            "b": "future-euclid-wl"
        });
        let report = check_value_firewall(&sealed, &packet);
        assert!(report.blocked);
        assert_eq!(report.violations.len(), 2);
    }
}
