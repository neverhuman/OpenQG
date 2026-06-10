//! V5: streaming run ledgers — records are written **as they are created**, not at end-of-run.
//!
//! The V4.1 campaign wrote its ledgers only after `evolve_population` returned, so a multi-hour
//! live run was a black box until the end (and a crash lost everything). A [`LedgerSink`] is a
//! pure side-channel: the engine still accumulates its in-memory records (so `EvolutionRun`,
//! replay, and the whitepaper path are unchanged), but every proposal, attempt, and progress
//! record is also streamed to disk the moment it exists, plus an atomic champion checkpoint every
//! N generations. Tests pass [`NullSink`].

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use serde_json::json;

use super::theory_population::{
    GenerationProgress, Individual, LiveProposalRecord, ProposalAttemptRecord,
};

/// Where the engine streams its records. Implementations must be cheap and infallible from the
/// engine's perspective (I/O errors are reported to stderr, never panic the run).
pub(crate) trait LedgerSink {
    fn proposal(&mut self, rec: &LiveProposalRecord);
    fn attempt(&mut self, rec: &ProposalAttemptRecord);
    fn progress(&mut self, g: &GenerationProgress);
    fn champion_checkpoint(&mut self, generation: usize, best: Option<&Individual>);
}

/// The no-op sink — exact pre-V5 behavior (tests, library callers).
pub(crate) struct NullSink;

impl LedgerSink for NullSink {
    fn proposal(&mut self, _rec: &LiveProposalRecord) {}
    fn attempt(&mut self, _rec: &ProposalAttemptRecord) {}
    fn progress(&mut self, _g: &GenerationProgress) {}
    fn champion_checkpoint(&mut self, _generation: usize, _best: Option<&Individual>) {}
}

/// Streams to a run directory: `proposal-ledger.jsonl`, `proposal-attempts.jsonl`,
/// `progress-ledger.jsonl` (append + flush per record), and `champion-checkpoint.json`
/// (temp-file + rename, so a killed run never leaves a torn checkpoint).
pub(crate) struct RunDirSink {
    run_dir: PathBuf,
    proposals: BufWriter<File>,
    attempts: BufWriter<File>,
    progress: BufWriter<File>,
    pub checkpoint_every: usize,
}

impl RunDirSink {
    pub(crate) fn create(
        run_dir: &std::path::Path,
        checkpoint_every: usize,
    ) -> anyhow::Result<Self> {
        fs::create_dir_all(run_dir)?;
        let open = |name: &str| -> anyhow::Result<BufWriter<File>> {
            Ok(BufWriter::new(
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(run_dir.join(name))?,
            ))
        };
        Ok(Self {
            run_dir: run_dir.to_path_buf(),
            proposals: open("proposal-ledger.jsonl")?,
            attempts: open("proposal-attempts.jsonl")?,
            progress: open("progress-ledger.jsonl")?,
            checkpoint_every: checkpoint_every.max(1),
        })
    }

    fn write_line<T: serde::Serialize>(w: &mut BufWriter<File>, rec: &T) {
        match serde_json::to_string(rec) {
            Ok(line) => {
                if writeln!(w, "{line}").and_then(|_| w.flush()).is_err() {
                    eprintln!("ledger sink: write failed (continuing)");
                }
            }
            Err(e) => eprintln!("ledger sink: serialize failed: {e}"),
        }
    }
}

impl LedgerSink for RunDirSink {
    fn proposal(&mut self, rec: &LiveProposalRecord) {
        Self::write_line(&mut self.proposals, rec);
    }

    fn attempt(&mut self, rec: &ProposalAttemptRecord) {
        Self::write_line(&mut self.attempts, rec);
    }

    fn progress(&mut self, g: &GenerationProgress) {
        let value = json!({
            "record_kind": "generation_progress",
            "generation": g.generation,
            "new_fingerprints": g.new_fingerprints,
            "reused_fingerprints": g.reused_fingerprints,
            "distinct_lineages": g.distinct_lineages,
            "promote_lineage_only": g.promote_lineage_only,
        });
        Self::write_line(&mut self.progress, &value);
    }

    fn champion_checkpoint(&mut self, generation: usize, best: Option<&Individual>) {
        if generation % self.checkpoint_every != 0 {
            return;
        }
        let value = json!({
            "record_kind": "champion_checkpoint",
            "generation": generation,
            "best": best.map(|b| json!({
                "id": b.id,
                "generation": b.generation,
                "total": b.scorecard.total,
                "disqualified": b.disqualified,
                "distinct_from_baseline": b.scorecard.distinct_from_baseline,
                "fingerprint": b.fingerprint,
            })),
        });
        let tmp = self.run_dir.join("champion-checkpoint.json.tmp");
        let dest = self.run_dir.join("champion-checkpoint.json");
        let ok = serde_json::to_string_pretty(&value)
            .map_err(|e| e.to_string())
            .and_then(|s| fs::write(&tmp, s).map_err(|e| e.to_string()))
            .and_then(|_| fs::rename(&tmp, &dest).map_err(|e| e.to_string()));
        if let Err(e) = ok {
            eprintln!("ledger sink: checkpoint failed: {e}");
        }
    }
}
