pub mod bench;
pub mod cli;
pub mod data;
pub mod release;
pub mod schema;
pub mod score;
pub mod security;
pub mod util;
pub mod zyal;

pub fn run() -> anyhow::Result<()> {
    cli::run()
}

#[cfg(test)]
mod tests;
