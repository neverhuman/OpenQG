//! Content-bound evidence (V4 M0).
//!
//! V3's fatal flaw was *evidence-name laundering*: a claim could cite `run-events.jsonl` (etc.) as
//! "evidence" without anyone ever opening the file, hashing it, or checking the cited fields exist.
//! The live judge was even handed packets with `required_evidence_contents_available_in_packet=false`
//! and (correctly) refused to promote anything. V4 makes evidence **content-bound**: every claim
//! references an [`EvidenceRef`] pinned by a SHA-256 of the exact bytes, and scoring *materializes*
//! every ref ([`MaterializedEvidenceAudit`]) — a ref whose hash does not match the bytes, or that
//! cannot be opened, or that carries no hash at all, is `ok = false` and disqualifies the claim.
//!
//! This module is pure: the filesystem-backed [`EvidenceStore`] lives in `openqg-data`; here we keep
//! the types and the pure [`audit_bytes`] adjudicator so the contract is unit-testable in-memory.

use serde::{Deserialize, Serialize};

/// Evidence tiers (V4). The tier governs *how* a dataset may be used and whether it may ever be
/// shown to the LLM proposer. T5 is a sealed holdout: only the deterministic held-out evaluator may
/// read it, and it must never appear in a proposer-visible packet or the training set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceTier {
    /// Invariants / mathematical identities — always usable.
    T1,
    /// Baseline anchors (e.g. ΛCDM-pinned reference points).
    T2,
    /// Public precision datasets — training.
    T3,
    /// Sector discriminators — validation / macro-test.
    T4,
    /// Sealed holdout — never shown to the proposer; deterministic evaluation only.
    T5,
}

impl EvidenceTier {
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceTier::T1 => "T1",
            EvidenceTier::T2 => "T2",
            EvidenceTier::T3 => "T3",
            EvidenceTier::T4 => "T4",
            EvidenceTier::T5 => "T5",
        }
    }

    pub fn parse(s: &str) -> Option<EvidenceTier> {
        match s.trim().to_ascii_uppercase().as_str() {
            "T1" => Some(EvidenceTier::T1),
            "T2" => Some(EvidenceTier::T2),
            "T3" => Some(EvidenceTier::T3),
            "T4" => Some(EvidenceTier::T4),
            "T5" => Some(EvidenceTier::T5),
            _ => None,
        }
    }

    /// A sealed holdout must never reach the proposer or the training set.
    pub fn is_sealed_holdout(self) -> bool {
        matches!(self, EvidenceTier::T5)
    }

    /// May this tier be inlined into an LLM-proposer-visible packet?
    pub fn proposer_visible(self) -> bool {
        !self.is_sealed_holdout()
    }
}

/// A content-bound reference to a piece of evidence. `sha256` pins the exact bytes; an empty
/// `sha256` is a *path-only* (unbound) reference and is treated as a laundering attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Where the evidence lives (run-relative path or dataset id).
    pub path: String,
    /// SHA-256 of the exact referenced bytes. Empty ⇒ unbound ⇒ disqualifying.
    #[serde(default)]
    pub sha256: String,
    /// Optional RFC-6901 JSON pointer into the referenced document.
    #[serde(default)]
    pub json_pointer: Option<String>,
    /// Optional `(start, end)` byte range within the referenced bytes.
    #[serde(default)]
    pub byte_range: Option<(u64, u64)>,
    /// Optional event id (for `*.jsonl` event ledgers).
    #[serde(default)]
    pub event_id: Option<String>,
    /// Evidence tier governing allowed use.
    #[serde(default = "default_tier")]
    pub tier: EvidenceTier,
}

fn default_tier() -> EvidenceTier {
    EvidenceTier::T3
}

impl EvidenceRef {
    /// A bound reference into a content-addressed path.
    pub fn new(path: impl Into<String>, sha256: impl Into<String>, tier: EvidenceTier) -> Self {
        Self {
            path: path.into(),
            sha256: sha256.into(),
            json_pointer: None,
            byte_range: None,
            event_id: None,
            tier,
        }
    }

    /// True if this reference carries a syntactically valid 64-hex content hash.
    pub fn is_bound(&self) -> bool {
        self.sha256.len() == 64 && self.sha256.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

/// The verdict of opening an [`EvidenceRef`] and hashing the actual bytes. `ok == false` is
/// disqualifying for the owning claim (unbound, hash-mismatch, empty, or unopenable).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializedEvidenceAudit {
    pub path: String,
    /// SHA-256 actually computed over the materialized bytes.
    pub sha256: String,
    pub byte_len: u64,
    /// Count of non-empty lines (useful for `*.jsonl` event ledgers; 0 for non-jsonl is fine).
    pub jsonl_count: u64,
    pub schema_id: String,
    pub tier: EvidenceTier,
    pub ok: bool,
    pub detail: String,
}

/// Adjudicate an [`EvidenceRef`] against the actual `bytes`. Pure and deterministic: a ref is
/// `ok` only when it is content-bound (64-hex sha), the bytes are non-empty, and the pinned hash
/// matches what we recompute. This is the gate that makes name-laundering impossible.
pub fn audit_bytes(r: &EvidenceRef, bytes: &[u8], schema_id: &str) -> MaterializedEvidenceAudit {
    let computed = crate::sha256_digest(bytes);
    let jsonl_count = bytes
        .split(|&b| b == b'\n')
        .filter(|l| {
            !l.iter()
                .all(|&c| c == b' ' || c == b'\t' || c == b'\r' || c == b'\0')
        })
        .filter(|l| !l.is_empty())
        .count() as u64;
    let (ok, detail) = if !r.is_bound() {
        (
            false,
            "unbound evidence reference (missing/invalid sha256) — path-only laundering".into(),
        )
    } else if bytes.is_empty() {
        (false, "evidence materialized to zero bytes".into())
    } else if r.sha256 != computed {
        (
            false,
            format!(
                "evidence hash mismatch: ref={} computed={}",
                r.sha256, computed
            ),
        )
    } else {
        (true, "materialized and content-hash verified".into())
    };
    MaterializedEvidenceAudit {
        path: r.path.clone(),
        sha256: computed,
        byte_len: bytes.len() as u64,
        jsonl_count,
        schema_id: schema_id.to_string(),
        tier: r.tier,
        ok,
        detail,
    }
}

/// Opens [`EvidenceRef`]s and adjudicates them. The filesystem implementation lives in `openqg-data`
/// (`FsEvidenceStore`); core depends only on this trait so scoring stays testable with an in-memory
/// store and so the one impure step (reading bytes) is quarantined behind a content-addressed seam.
pub trait EvidenceStore {
    /// Materialize and audit a single reference. A ref whose `path` cannot be opened MUST return an
    /// `ok = false` audit (never panic), so a missing file is a disqualifying verdict, not a crash.
    fn materialize(&self, r: &EvidenceRef, schema_id: &str) -> MaterializedEvidenceAudit;

    /// Materialize every reference; convenience over [`EvidenceStore::materialize`].
    fn materialize_all(
        &self,
        refs: &[EvidenceRef],
        schema_id: &str,
    ) -> Vec<MaterializedEvidenceAudit> {
        refs.iter()
            .map(|r| self.materialize(r, schema_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// In-memory store for tests: maps path -> bytes. A missing path yields an `ok=false` audit.
    struct MemStore(BTreeMap<String, Vec<u8>>);
    impl EvidenceStore for MemStore {
        fn materialize(&self, r: &EvidenceRef, schema_id: &str) -> MaterializedEvidenceAudit {
            match self.0.get(&r.path) {
                Some(bytes) => audit_bytes(r, bytes, schema_id),
                None => MaterializedEvidenceAudit {
                    path: r.path.clone(),
                    sha256: String::new(),
                    byte_len: 0,
                    jsonl_count: 0,
                    schema_id: schema_id.to_string(),
                    tier: r.tier,
                    ok: false,
                    detail: "evidence path could not be opened".into(),
                },
            }
        }
    }

    fn bound_ref(path: &str, bytes: &[u8], tier: EvidenceTier) -> EvidenceRef {
        EvidenceRef::new(path, crate::sha256_digest(bytes), tier)
    }

    #[test]
    fn bound_matching_evidence_is_ok() {
        let bytes = b"{\"a\":1}\n{\"a\":2}\n";
        let r = bound_ref("run-events.jsonl", bytes, EvidenceTier::T3);
        let a = audit_bytes(&r, bytes, "run-events.v1");
        assert!(a.ok, "{}", a.detail);
        assert_eq!(a.byte_len, bytes.len() as u64);
        assert_eq!(a.jsonl_count, 2);
        assert_eq!(a.sha256.len(), 64);
    }

    #[test]
    fn path_only_reference_is_rejected() {
        // The V3 laundering attempt: a path with no content hash.
        let r = EvidenceRef::new("run-events.jsonl", "", EvidenceTier::T3);
        let a = audit_bytes(&r, b"anything", "x.v1");
        assert!(!a.ok);
        assert!(a.detail.contains("laundering") || a.detail.contains("unbound"));
    }

    #[test]
    fn hash_mismatch_is_rejected() {
        let r = bound_ref("e.json", b"original", EvidenceTier::T2);
        let a = audit_bytes(&r, b"tampered", "x.v1"); // different bytes than the pinned hash
        assert!(!a.ok);
        assert!(a.detail.contains("mismatch"));
    }

    #[test]
    fn empty_bytes_are_rejected() {
        let r = bound_ref("e.json", b"", EvidenceTier::T2);
        let a = audit_bytes(&r, b"", "x.v1");
        assert!(!a.ok);
    }

    #[test]
    fn missing_path_is_a_verdict_not_a_panic() {
        let store = MemStore(BTreeMap::new());
        let r = EvidenceRef::new("absent.jsonl", "0".repeat(64), EvidenceTier::T3);
        let a = store.materialize(&r, "x.v1");
        assert!(!a.ok);
        assert!(a.detail.contains("could not be opened"));
    }

    #[test]
    fn tier_t5_is_sealed_and_not_proposer_visible() {
        assert!(EvidenceTier::T5.is_sealed_holdout());
        assert!(!EvidenceTier::T5.proposer_visible());
        assert!(EvidenceTier::T3.proposer_visible());
        assert_eq!(EvidenceTier::parse("t4"), Some(EvidenceTier::T4));
    }
}
