use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum TheoryCommand {
    /// Profile-fit competing model classes to the data and rank them by AIC / ln-evidence — the
    /// fair model-selection adjudicator (re-fits the baseline; covariance-aware; penalizes
    /// parameters). Replaces the fitted-vs-fixed delta-log-likelihood comparison.
    League {
        /// One or more JSONL observable files (merged). Repeat the flag to combine data tiers.
        #[arg(long, default_value = "data/fixtures/cosmology/tier0-combined.jsonl")]
        observables: Vec<PathBuf>,
        /// Optional JSON file of covariance blocks ([{ids:[...], matrix:[[...]]}, ...]) linking
        /// correlated observables. Omitted => every observable independent (diagonal).
        #[arg(long)]
        covariance: Option<PathBuf>,
        /// The reference model every challenger is compared against (Delta-AIC vs this).
        #[arg(long, default_value = "lcdm")]
        reference: String,
        /// League artifact output path (JSON).
        #[arg(long, default_value = "target/openqg/theory/league.json")]
        output: PathBuf,
    },
}
