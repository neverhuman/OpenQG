use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum ReleaseCommand {
    Pack {
        #[arg(long, default_value = "target/openqg/bench-smoke/scorecard.json")]
        scorecard: PathBuf,
        #[arg(long, default_value = "reports/releases/draft")]
        output: PathBuf,
    },
}
