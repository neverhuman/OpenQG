use clap::Subcommand;

use super::{bench, data, release, schema, score, security, zyal};

#[derive(Debug, Subcommand)]
pub(super) enum Commands {
    Schema {
        #[command(subcommand)]
        command: schema::SchemaCommand,
    },
    Data {
        #[command(subcommand)]
        command: data::DataCommand,
    },
    Bench {
        #[command(subcommand)]
        command: bench::BenchCommand,
    },
    Score {
        #[command(subcommand)]
        command: score::ScoreCommand,
    },
    Release {
        #[command(subcommand)]
        command: release::ReleaseCommand,
    },
    Zyal {
        #[command(subcommand)]
        command: zyal::ZyalCommand,
    },
    Security {
        #[command(subcommand)]
        command: security::SecurityCommand,
    },
}
