use anyhow::Result;

use crate::{
    bench as bench_ops, data as data_ops, release as release_ops, schema as schema_ops,
    score as score_ops, security as security_ops, theory_evolve as theory_ops,
    theory_league as theory_league_ops, zyal as zyal_ops, zyal_genome as zyal_genome_ops,
};

use super::{bench, commands::Commands, data, release, schema, score, security, theory, zyal};

pub fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Schema { command } => match command {
            schema::SchemaCommand::Check { root, registry } => schema_ops::check(&root, &registry),
            schema::SchemaCommand::Sync { root, registry } => schema_ops::sync(&root, &registry),
        },
        Commands::Data { command } => match command {
            data::DataCommand::Lock { root, output } => data_ops::lock(&root, &output),
            data::DataCommand::Verify { root, lock_output } => {
                data_ops::verify(&root, &lock_output)
            }
            data::DataCommand::Smoke {
                observables,
                predictions,
            } => data_ops::smoke(&observables, &predictions),
        },
        Commands::Bench { command } => match command {
            bench::BenchCommand::Run {
                suite,
                theory,
                output,
                observables,
                predictions,
            } => bench_ops::run(
                &suite,
                &theory,
                &output,
                observables.as_deref(),
                predictions.as_deref(),
            ),
        },
        Commands::Score { command } => match command {
            score::ScoreCommand::Compare {
                scorecard,
                json,
                md,
            } => score_ops::compare(&scorecard, &json, &md),
        },
        Commands::Release { command } => match command {
            release::ReleaseCommand::Pack { scorecard, output } => {
                release_ops::pack(&scorecard, &output)
            }
        },
        Commands::Zyal { command } => match command {
            zyal::ZyalCommand::Validate { root, output } => zyal_ops::validate(&root, &output),
            zyal::ZyalCommand::JekkoPreview { root, output } => {
                zyal_ops::jekko_preview(&root, &output)
            }
            zyal::ZyalCommand::Genome { command } => zyal_genome_ops::run(command),
        },
        Commands::Theory { command } => match command {
            theory::TheoryCommand::Evolve {
                observables,
                proposals,
                proposer_cmd,
                output_root,
                run_id,
                checkpoint_every,
                generations,
                population,
                seed,
            } => theory_ops::run_evolve(
                &observables,
                proposals.as_deref(),
                proposer_cmd.as_deref(),
                &output_root,
                &run_id,
                checkpoint_every,
                generations,
                population,
                seed,
            ),
            theory::TheoryCommand::League {
                observables,
                covariance,
                reference,
                output,
            } => theory_league_ops::run_league(
                &observables,
                covariance.as_deref(),
                &reference,
                &output,
            ),
        },
        Commands::Security { command } => match command {
            security::SecurityCommand::Scan { output } => security_ops::scan(&output),
        },
    }
}
