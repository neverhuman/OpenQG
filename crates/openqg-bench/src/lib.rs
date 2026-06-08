pub mod bench;
pub mod cli;
pub mod data;
pub mod release;
pub mod schema;
pub mod score;
pub mod security;
pub mod theory_evolve;
pub mod theory_league;
pub mod util;
pub mod zyal;
pub mod zyal_genome;
pub mod zyal_judge;
pub mod zyal_robustness;

pub fn run() -> anyhow::Result<()> {
    cli::run()
}

#[cfg(test)]
mod tests;
