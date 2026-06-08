use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum TheoryCommand {
    /// Evolve symbolic physics theories against real data and emit a champion report.
    Evolve {
        #[arg(long, default_value = "data/fixtures/cosmology/bao-desi-dr1.jsonl")]
        observables: PathBuf,
        /// Optional JSONL of theory proposals (one TheoryProposal per line) used to seed the
        /// population alongside the GR/LCDM baseline. Each is run through the derivation-checker.
        #[arg(long)]
        proposals: Option<PathBuf>,
        /// Optional shell command whose stdout is JSONL theory proposals (one per line) — the
        /// generic LLM-proposer hook. For the live jnoccio proposer, pass the jekko invocation
        /// here; for tests/offline, pass any command. Output is run through the derivation-checker.
        #[arg(long)]
        proposer_cmd: Option<String>,
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
