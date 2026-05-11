use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum SecurityCommand {
    Scan {
        #[arg(long, default_value = "target/jankurai/security/evidence.json")]
        output: PathBuf,
    },
}
