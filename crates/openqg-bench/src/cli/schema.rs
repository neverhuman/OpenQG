use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum SchemaCommand {
    Check {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "contracts/registry.yml")]
        registry: PathBuf,
    },
    Sync {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = "contracts/registry.yml")]
        registry: PathBuf,
    },
}
