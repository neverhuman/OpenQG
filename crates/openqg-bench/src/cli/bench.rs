use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum BenchCommand {
    Run {
        #[arg(long, default_value = "benchmarks/suites/smoke.yml")]
        suite: PathBuf,
        #[arg(long, default_value = "theories/sm-gr-lcdm-mnu/manifest.yml")]
        theory: PathBuf,
        #[arg(long, default_value = "target/openqg/bench-smoke/scorecard.json")]
        output: PathBuf,
        #[arg(long)]
        observables: Option<PathBuf>,
        #[arg(long)]
        predictions: Option<PathBuf>,
    },
}
