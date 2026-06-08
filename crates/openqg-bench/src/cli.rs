pub mod bench;
mod commands;
pub mod data;
mod dispatch;
pub mod release;
pub mod schema;
pub mod score;
pub mod security;
pub mod theory;
pub mod zyal;

use anyhow::Result;
use clap::Parser;

use commands::Commands;

#[derive(Debug, Parser)]
#[command(name = "openqg-bench", version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

pub fn run() -> Result<()> {
    dispatch::run(Cli::parse().command)
}
