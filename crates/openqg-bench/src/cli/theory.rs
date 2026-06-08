use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum TheoryCommand {
    /// Evolve symbolic physics theories against real data and emit a champion report.
    Evolve {
        #[arg(long, default_value = "data/fixtures/cosmology/bao-desi-dr1.jsonl")]
        observables: PathBuf,
        #[arg(long, default_value = "target/openqg/theory/champion.json")]
        output: PathBuf,
        #[arg(long, default_value_t = 60)]
        generations: usize,
        #[arg(long, default_value_t = 24)]
        population: usize,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
}
