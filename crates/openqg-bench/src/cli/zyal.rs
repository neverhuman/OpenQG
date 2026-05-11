use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum ZyalCommand {
    Validate {
        #[arg(long, default_value = "ops/zyal")]
        root: PathBuf,
        #[arg(long, default_value = "target/openqg/zyal/preview.json")]
        output: PathBuf,
    },
}
