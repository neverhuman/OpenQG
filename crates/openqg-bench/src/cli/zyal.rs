use clap::Subcommand;
use std::path::PathBuf;

use crate::zyal_genome::GenomeCommand;

#[derive(Debug, Subcommand)]
pub enum ZyalCommand {
    Validate {
        #[arg(long, default_value = "agent/zyal")]
        root: PathBuf,
        #[arg(long, default_value = "target/openqg/zyal/preview.json")]
        output: PathBuf,
    },
    JekkoPreview {
        #[arg(long, default_value = "agent/zyal")]
        root: PathBuf,
        #[arg(long, default_value = "target/openqg/zyal/jekko-preview.jsonl")]
        output: PathBuf,
    },
    Genome {
        #[command(subcommand)]
        command: GenomeCommand,
    },
}
