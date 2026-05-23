use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum ResearchCommand {
    Validate {
        #[arg(long, default_value = "research/knowledge/question-bank")]
        root: PathBuf,
        #[arg(
            long,
            default_value = "target/openqg/research/question-bank-validation.json"
        )]
        output: PathBuf,
    },
    DedupeCheck {
        #[arg(long, default_value = "research/knowledge/question-bank")]
        root: PathBuf,
    },
    SmokeFixture {
        #[arg(long, default_value = "research/knowledge/question-bank")]
        root: PathBuf,
    },
}
