use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum ScoreCommand {
    Compare {
        #[arg(long, default_value = "target/openqg/bench-smoke/scorecard.json")]
        scorecard: PathBuf,
        #[arg(long, default_value = "target/jankurai/repo-score.json")]
        json: PathBuf,
        #[arg(long, default_value = "target/jankurai/repo-score.md")]
        md: PathBuf,
    },
}
