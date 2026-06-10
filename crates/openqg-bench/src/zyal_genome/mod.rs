//! The V6 ZYAL genome engine CLI: a real theory-population evolution loop ("LLM proposes, the
//! deterministic oracle disposes") with streamed, replayable ledgers.
//!
//! Commands: `population` (run the engine), `trust-gate` (the campaign-blocking calibration gate),
//! `whitepaper` (run + honest report), `propose` (score a saved proposal offline), `replay`
//! (re-score a proposal ledger without any LLM).

use anyhow::{Context, Result};
use clap::Subcommand;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_OUTPUT_ROOT: &str = "target/openqg/zyal-genome";
const DEFAULT_SEED: u64 = 20260605;
/// The jekko subprocess transport command (free jnoccio tokens). Scheduled for removal when the
/// router-native proposer (direct HTTP to the jnoccio-fusion gateway) lands.
pub(crate) const JEKKO_LIVE_COMMAND: &[&str] = &[
    "rtk",
    "jekko",
    "run",
    "--headless",
    "--ephemeral",
    "--print-logs",
    "--provider",
    "jnoccio",
    "--model",
    "jnoccio/jnoccio-fusion",
    "--cwd",
    "/home/ubuntu/openQG",
];

#[derive(Debug, Subcommand)]
pub enum GenomeCommand {
    /// V4: run the real theory-population evolution engine (deterministic, physics-scored) and write
    /// a progress ledger + champion + quality gate. The new, trustworthy run path.
    Population {
        #[arg(long)]
        observables: PathBuf,
        #[arg(long, default_value = DEFAULT_OUTPUT_ROOT)]
        output_root: PathBuf,
        #[arg(long, default_value_t = 8)]
        max_generations: usize,
        #[arg(long, default_value_t = 9)]
        population_size: usize,
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
        #[arg(long)]
        run_id: Option<String>,
        /// Inject a derivation-rich fixture proposal each generation (deterministic demo of the
        /// LLM-proposer path; no live infra).
        #[arg(long)]
        with_fixture_proposer: bool,
        /// V5: inject jekko (free jnoccio API) proposals — the high-volume workhorse with
        /// repair loops + best-of-K sampling. Requires `rtk jekko` available.
        #[arg(long)]
        with_jekko_proposer: bool,
        /// With --with-jekko-proposer: a jekko slot on gen 1 and every Nth generation.
        #[arg(long, default_value_t = 40)]
        jekko_every: usize,
        /// Best-of-K parallel jekko samples per slot.
        #[arg(long, default_value_t = 4)]
        jekko_samples: usize,
        /// Repair budget per sample (parse repairs + at most one oracle repair).
        #[arg(long, default_value_t = 2)]
        jekko_repairs: usize,
        /// Per-call jekko timeout.
        #[arg(long, default_value_t = 300)]
        jekko_timeout_seconds: u64,
        /// jnoccio quality band routing: "top20" or "none" (= all available models; more resilient
        /// when the provider is degraded).
        #[arg(long, default_value = "top20")]
        jekko_quality_band: String,
    },
    /// V4 TRUST GATE: compose the decoy/human league + a real population run + determinism checks
    /// into trust-gate.json. Must pass before the 1000–10000-gen campaign. Deterministic, no LLM.
    TrustGate {
        #[arg(long)]
        observables: PathBuf,
        #[arg(long, default_value = DEFAULT_OUTPUT_ROOT)]
        output_root: PathBuf,
        #[arg(long, default_value_t = 6)]
        max_generations: usize,
        #[arg(long, default_value_t = 9)]
        population_size: usize,
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
        #[arg(long)]
        run_id: Option<String>,
    },
    /// V4: run the engine and write a candidate-theory white paper (champion + per-dimension
    /// scorecard + ranking vs human contenders + trust-gate status). Deterministic, no LLM.
    Whitepaper {
        #[arg(long)]
        observables: PathBuf,
        #[arg(long, default_value = DEFAULT_OUTPUT_ROOT)]
        output_root: PathBuf,
        #[arg(long, default_value_t = 8)]
        max_generations: usize,
        #[arg(long, default_value_t = 9)]
        population_size: usize,
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
        #[arg(long)]
        run_id: Option<String>,
        /// Inject a derivation-rich fixture proposal each generation (deterministic demo of the
        /// LLM-proposer path; no live infra).
        #[arg(long)]
        with_fixture_proposer: bool,
        /// V5: inject jekko (free jnoccio API) proposals — the high-volume workhorse with
        /// repair loops + best-of-K sampling. Requires `rtk jekko` available.
        #[arg(long)]
        with_jekko_proposer: bool,
        /// With --with-jekko-proposer: a jekko slot on gen 1 and every Nth generation.
        #[arg(long, default_value_t = 40)]
        jekko_every: usize,
        /// Best-of-K parallel jekko samples per slot.
        #[arg(long, default_value_t = 4)]
        jekko_samples: usize,
        /// Repair budget per sample (parse repairs + at most one oracle repair).
        #[arg(long, default_value_t = 2)]
        jekko_repairs: usize,
        /// Per-call jekko timeout.
        #[arg(long, default_value_t = 300)]
        jekko_timeout_seconds: u64,
        /// jnoccio quality band routing: "top20" or "none" (= all available models; more resilient
        /// when the provider is degraded).
        #[arg(long, default_value = "top20")]
        jekko_quality_band: String,
    },
    /// Score a saved proposal JSON offline (e.g. an external/jailhard-review proposal) through the
    /// full deterministic oracle. No live calls.
    Propose {
        #[arg(long)]
        observables: PathBuf,
        #[arg(long)]
        proposal_file: PathBuf,
    },
    /// V4.1: replay a proposal ledger WITHOUT the LLM — re-score each recorded proposal and confirm
    /// it reproduces the recorded total. Makes a live run's "replayable" claim checkable from artifacts.
    Replay {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        observables: PathBuf,
    },
}

pub fn run(command: GenomeCommand) -> Result<()> {
    match command {
        GenomeCommand::Population {
            observables,
            output_root,
            max_generations,
            population_size,
            seed,
            run_id,
            with_fixture_proposer,
            with_jekko_proposer,
            jekko_every,
            jekko_samples,
            jekko_repairs,
            jekko_timeout_seconds,
            jekko_quality_band,
        } => {
            let run_id = run_id.unwrap_or_else(|| format!("population-g{max_generations}-s{seed}"));
            let config = EvolveConfig {
                population_size,
                max_generations,
                seed,
            };
            let fixture = FixtureProposer;
            let jekko = if with_jekko_proposer {
                jekko_smoke(120)
                    .context("jekko preflight smoke failed — is `rtk jekko` available?")?;
                let obs_loaded = load_observables(&observables)?;
                let baseline = baseline_log_likelihood(&obs_loaded);
                let extra = build_proposer_extra_sections(&obs_loaded, &output_root);
                Some(JekkoProposer::new(
                    JekkoConfig {
                        timeout_seconds: jekko_timeout_seconds,
                        samples: jekko_samples,
                        repairs: jekko_repairs,
                        quality_band: (jekko_quality_band != "none")
                            .then(|| jekko_quality_band.clone()),
                        ..Default::default()
                    },
                    obs_loaded,
                    baseline,
                    extra,
                ))
            } else {
                None
            };
            let budgeted;
            let proposer: Option<&dyn Proposer> = if let Some(j) = jekko.as_ref() {
                budgeted = BudgetedProposer::new(j as &dyn Proposer, jekko_every);
                Some(&budgeted)
            } else if with_fixture_proposer {
                Some(&fixture)
            } else {
                None
            };
            let dir = run_population(&observables, &output_root, config, &run_id, proposer)?;
            println!("wrote v4 population run to {}", dir.display());
            Ok(())
        }
        GenomeCommand::TrustGate {
            observables,
            output_root,
            max_generations,
            population_size,
            seed,
            run_id,
        } => {
            let run_id = run_id.unwrap_or_else(|| format!("trust-gate-g{max_generations}-s{seed}"));
            let config = EvolveConfig {
                population_size,
                max_generations,
                seed,
            };
            let (dir, passed) = run_trust_gate(&observables, &output_root, config, &run_id)?;
            println!(
                "trust gate {}: wrote {}",
                if passed {
                    "PASSED — campaign unblocked"
                } else {
                    "FAILED — campaign blocked"
                },
                dir.join("trust-gate.json").display()
            );
            if !passed {
                anyhow::bail!("trust gate failed; campaign blocked");
            }
            Ok(())
        }
        GenomeCommand::Whitepaper {
            observables,
            output_root,
            max_generations,
            population_size,
            seed,
            run_id,
            with_fixture_proposer,
            with_jekko_proposer,
            jekko_every,
            jekko_samples,
            jekko_repairs,
            jekko_timeout_seconds,
            jekko_quality_band,
        } => {
            let run_id = run_id.unwrap_or_else(|| format!("whitepaper-g{max_generations}-s{seed}"));
            let config = EvolveConfig {
                population_size,
                max_generations,
                seed,
            };
            let fixture = FixtureProposer;
            let jekko = if with_jekko_proposer {
                jekko_smoke(120)
                    .context("jekko preflight smoke failed — is `rtk jekko` available?")?;
                let obs_loaded = load_observables(&observables)?;
                let baseline = baseline_log_likelihood(&obs_loaded);
                let extra = build_proposer_extra_sections(&obs_loaded, &output_root);
                Some(JekkoProposer::new(
                    JekkoConfig {
                        timeout_seconds: jekko_timeout_seconds,
                        samples: jekko_samples,
                        repairs: jekko_repairs,
                        quality_band: (jekko_quality_band != "none")
                            .then(|| jekko_quality_band.clone()),
                        ..Default::default()
                    },
                    obs_loaded,
                    baseline,
                    extra,
                ))
            } else {
                None
            };
            let budgeted;
            let proposer: Option<&dyn Proposer> = if let Some(j) = jekko.as_ref() {
                budgeted = BudgetedProposer::new(j as &dyn Proposer, jekko_every);
                Some(&budgeted)
            } else if with_fixture_proposer {
                Some(&fixture)
            } else {
                None
            };
            let dir = generate_whitepaper(&observables, &output_root, config, &run_id, proposer)?;
            println!(
                "wrote white paper to {}",
                dir.join("white-paper.md").display()
            );
            Ok(())
        }
        GenomeCommand::Propose {
            observables,
            proposal_file,
        } => {
            let obs = load_observables(&observables)?;
            anyhow::ensure!(!obs.is_empty(), "no observables loaded");
            let raw = fs::read_to_string(&proposal_file)
                .with_context(|| format!("read proposal file {}", proposal_file.display()))?;
            let doc = parse_proposal_response(&raw)?;
            let sc = score_proposal(&doc, &obs, baseline_log_likelihood(&obs));
            println!(
                "proposal: theory `{}` scored {:.1}/100 (disqualified={})",
                doc.theory.id, sc.total, sc.disqualified
            );
            for c in &sc.components {
                println!("  {}: {:.1}/{:.0}", c.name, c.points, c.weight);
            }
            for r in &sc.kill_reasons {
                println!("  KILL: {r}");
            }
            Ok(())
        }
        GenomeCommand::Replay {
            ledger,
            observables,
        } => {
            let (checked, mismatches) = replay_ledger(&ledger, &observables)?;
            println!(
                "replayed {checked} proposal(s) from ledger (no LLM); {mismatches} mismatch(es)"
            );
            if mismatches > 0 {
                anyhow::bail!("ledger replay had {mismatches} mismatch(es) — run not reproducible");
            }
            Ok(())
        }
    }
}

mod physics_score;
pub(crate) use physics_score::*;
mod theory_population;
pub(crate) use theory_population::*;
mod run_population;
pub(crate) use run_population::*;
mod trust_gate;
pub(crate) use trust_gate::*;
mod whitepaper;
pub(crate) use whitepaper::*;
mod proposer;
pub(crate) use proposer::*;
mod live_attempt;
mod proposer_prompt;
pub(crate) use proposer_prompt::*;
mod proposer_memory;
pub(crate) use proposer_memory::*;
mod ledger_sink;
mod proposer_jekko;
pub(crate) use proposer_jekko::*;

/// Assemble the V5 prompt extras: the computed DATA BRIEF (real pulls vs the ΛCDM baseline) plus
/// the cross-run MEMORY section (top scorers, kill histogram, DO/DON'T) from prior run ledgers.
pub(crate) fn build_proposer_extra_sections(
    observables: &[openqg_core::ObservableRecord],
    output_root: &Path,
) -> String {
    let brief = build_data_brief(observables);
    let memory = render_memory_section(&assemble_memory(&output_root.join("runs"), &[]), 1536);
    format!("\n{brief}\n\n{memory}\n")
}
