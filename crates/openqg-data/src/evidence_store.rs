//! Filesystem-backed evidence store (V4).
//!
//! Scoring in OpenQG is otherwise pure and deterministic: every [`EvidenceRef`] is pinned by a
//! SHA-256 of the exact bytes it cites, and the pure adjudicator
//! [`audit_bytes`](openqg_core::theory::audit_bytes) verifies that hash against the actual content.
//! The *one* impure step in the whole trust spine is reading those bytes off disk. This module
//! quarantines that step behind the content-addressed [`EvidenceStore`](openqg_core::theory::EvidenceStore)
//! seam: [`FsEvidenceStore`] is the only place that touches the filesystem, and it immediately hands
//! the bytes to the pure adjudicator. Everything downstream stays testable in-memory and
//! reproducible, because the content hash — not the path or the I/O — is what actually gates a claim.
//!
//! A missing or unopenable file is never a crash: it is a disqualifying *verdict* (`ok = false`),
//! exactly like a hash mismatch.

use openqg_core::theory::{audit_bytes, EvidenceRef, EvidenceStore, MaterializedEvidenceAudit};
use std::path::PathBuf;

/// An [`EvidenceStore`] that resolves [`EvidenceRef`] paths against a `root` directory and reads
/// the bytes off the local filesystem. This is the sole impure (I/O) implementation of the
/// content-addressed evidence seam; it never panics on missing/unopenable files.
pub struct FsEvidenceStore {
    root: PathBuf,
}

impl FsEvidenceStore {
    /// Build a store rooted at `root`. Reference paths are resolved as `root.join(ref.path)`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl EvidenceStore for FsEvidenceStore {
    fn materialize(&self, r: &EvidenceRef, schema_id: &str) -> MaterializedEvidenceAudit {
        let full = self.root.join(&r.path);
        match std::fs::read(&full) {
            // Success: hand the raw bytes to the pure adjudicator, which verifies the content hash.
            Ok(bytes) => audit_bytes(r, &bytes, schema_id),
            // A missing/unopenable file is a disqualifying verdict, not a panic.
            Err(e) => MaterializedEvidenceAudit {
                path: r.path.clone(),
                sha256: String::new(),
                byte_len: 0,
                jsonl_count: 0,
                schema_id: schema_id.to_string(),
                tier: r.tier,
                ok: false,
                detail: format!("evidence path could not be opened: {e}"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openqg_core::theory::EvidenceTier;

    /// A unique scratch directory for this test process. We avoid `tempfile` (not a dependency) and
    /// use `std::env::temp_dir()` keyed on the PID so concurrent test binaries do not collide.
    fn scratch_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("openqg-fsstore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    #[test]
    fn fs_store_materializes_and_verifies() {
        let dir = scratch_dir();
        let bytes = b"{\"a\":1}\n{\"a\":2}\n";

        // 1. Bound, matching reference -> ok.
        let rel = "run-events.jsonl";
        std::fs::write(dir.join(rel), bytes).expect("write evidence file");
        let store = FsEvidenceStore::new(&dir);
        let sha = openqg_core::sha256_digest(bytes);
        let r = EvidenceRef::new(rel, sha.clone(), EvidenceTier::T3);
        assert!(r.is_bound());
        let a = store.materialize(&r, "run-events.v1");
        assert!(a.ok, "{}", a.detail);
        assert_eq!(a.byte_len, bytes.len() as u64);
        assert_eq!(a.sha256, sha);
        assert_eq!(a.jsonl_count, 2);

        // 2. Non-existent path -> ok=false with an "could not be opened" detail (no panic).
        let missing = EvidenceRef::new("absent.jsonl", "0".repeat(64), EvidenceTier::T3);
        let a_missing = store.materialize(&missing, "run-events.v1");
        assert!(!a_missing.ok);
        assert!(
            a_missing.detail.contains("could not be opened"),
            "detail was: {}",
            a_missing.detail
        );

        // 3. Wrong hash on an existing file -> ok=false (hash mismatch).
        let bad = EvidenceRef::new(rel, "f".repeat(64), EvidenceTier::T3);
        let a_bad = store.materialize(&bad, "run-events.v1");
        assert!(!a_bad.ok);

        // Best-effort cleanup; failures here must not fail the test.
        let _ = std::fs::remove_dir_all(&dir);
    }
}
