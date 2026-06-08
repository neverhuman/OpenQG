use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum DataCommand {
    Lock {
        #[arg(long, default_value = "data/registry")]
        root: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Verify {
        #[arg(long, default_value = "data/registry")]
        root: PathBuf,
        #[arg(long, default_value = "target/openqg/data/locks/data-lock.json")]
        lock_output: PathBuf,
    },
    Smoke {
        #[arg(long, default_value = "data/fixtures/smoke/observables.jsonl")]
        observables: PathBuf,
        #[arg(long, default_value = "data/fixtures/smoke/predictions.jsonl")]
        predictions: PathBuf,
    },
    /// Verify every observational fixture against the sha256 recorded in the data manifest
    /// (the reproducibility contract). Fails on any drift or missing fixture.
    Audit {
        #[arg(long, default_value = "data/manifest.json")]
        manifest: PathBuf,
    },
}
