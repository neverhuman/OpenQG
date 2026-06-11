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
#[derive(Debug, Subcommand)]
pub enum GenomeCommand {
    /// V4: run the real theory-population evolution engine (deterministic, physics-scored) and write
    /// a progress ledger + champion + quality gate. The new, trustworthy run path.
    Population {
        #[arg(long)]
        observables: PathBuf,
        /// Covariance fixture JSONs (single- or multi-block) — correlated-survey blocks for the
        /// V6 covariance-aware likelihood. Omit for a pure diagonal likelihood.
        #[arg(long)]
        covariance: Vec<PathBuf>,
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
        /// V6: inject router-native proposals (direct HTTP to the jnoccio-fusion gateway) — the
        /// free-compute workhorse: server-side strict-schema validation, mechanism lanes,
        /// quality-band rotation, repair loops + best-of-K sampling.
        #[arg(long, alias = "with-jekko-proposer")]
        with_router_proposer: bool,
        /// With --with-router-proposer: a live slot on gen 1 and every Nth generation.
        #[arg(long, default_value_t = 40, alias = "jekko-every")]
        router_every: usize,
        /// Best-of-K parallel samples per slot (each on its own mechanism lane).
        #[arg(long, default_value_t = 4, alias = "jekko-samples")]
        router_samples: usize,
        /// Repair budget per sample (parse repair + at most one oracle repair).
        #[arg(long, default_value_t = 2, alias = "jekko-repairs")]
        router_repairs: usize,
        /// Per-call HTTP timeout.
        #[arg(long, default_value_t = 240, alias = "jekko-timeout-seconds")]
        router_timeout_seconds: u64,
        /// Quality-band rotation pool (comma-separated; "any" = all models).
        #[arg(long, default_value = "top20,any,top50,any")]
        router_bands: String,
        /// Router base URL (env OPENQG_ROUTER_URL also works).
        #[arg(long)]
        router_url: Option<String>,
    },
    /// V4 TRUST GATE: compose the decoy/human league + a real population run + determinism checks
    /// into trust-gate.json. Must pass before the 1000–10000-gen campaign. Deterministic, no LLM.
    TrustGate {
        #[arg(long)]
        observables: PathBuf,
        /// Covariance fixture JSONs (single- or multi-block) — correlated-survey blocks for the
        /// V6 covariance-aware likelihood. Omit for a pure diagonal likelihood.
        #[arg(long)]
        covariance: Vec<PathBuf>,
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
        /// Covariance fixture JSONs (single- or multi-block) — correlated-survey blocks for the
        /// V6 covariance-aware likelihood. Omit for a pure diagonal likelihood.
        #[arg(long)]
        covariance: Vec<PathBuf>,
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
        /// V6: inject router-native proposals (direct HTTP to the jnoccio-fusion gateway) — the
        /// free-compute workhorse: server-side strict-schema validation, mechanism lanes,
        /// quality-band rotation, repair loops + best-of-K sampling.
        #[arg(long, alias = "with-jekko-proposer")]
        with_router_proposer: bool,
        /// With --with-router-proposer: a live slot on gen 1 and every Nth generation.
        #[arg(long, default_value_t = 40, alias = "jekko-every")]
        router_every: usize,
        /// Best-of-K parallel samples per slot (each on its own mechanism lane).
        #[arg(long, default_value_t = 4, alias = "jekko-samples")]
        router_samples: usize,
        /// Repair budget per sample (parse repair + at most one oracle repair).
        #[arg(long, default_value_t = 2, alias = "jekko-repairs")]
        router_repairs: usize,
        /// Per-call HTTP timeout.
        #[arg(long, default_value_t = 240, alias = "jekko-timeout-seconds")]
        router_timeout_seconds: u64,
        /// Quality-band rotation pool (comma-separated; "any" = all models).
        #[arg(long, default_value = "top20,any,top50,any")]
        router_bands: String,
        /// Router base URL (env OPENQG_ROUTER_URL also works).
        #[arg(long)]
        router_url: Option<String>,
    },
    /// Score a saved proposal JSON offline (e.g. an external/jailhard-review proposal) through the
    /// full deterministic oracle. No live calls.
    Propose {
        #[arg(long)]
        observables: PathBuf,
        /// Covariance fixture JSONs (single- or multi-block) — correlated-survey blocks for the
        /// V6 covariance-aware likelihood. Omit for a pure diagonal likelihood.
        #[arg(long)]
        covariance: Vec<PathBuf>,
        #[arg(long)]
        proposal_file: PathBuf,
    },
    /// V6: re-score a bare Theory JSON (e.g. a prior campaign's champion) through the CURRENT
    /// unified gate (bind + physics_kills + full scorecard). The acceptance tool: prior champions
    /// must be disqualified or honestly re-cost under V6.
    Rescore {
        #[arg(long)]
        theory: PathBuf,
        #[arg(long)]
        observables: PathBuf,
        /// Covariance fixture JSONs (single- or multi-block) — correlated-survey blocks for the
        /// V6 covariance-aware likelihood. Omit for a pure diagonal likelihood.
        #[arg(long)]
        covariance: Vec<PathBuf>,
    },
    /// V4.1: replay a proposal ledger WITHOUT the LLM — re-score each recorded proposal and confirm
    /// it reproduces the recorded total. Makes a live run's "replayable" claim checkable from artifacts.
    Replay {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        observables: PathBuf,
        /// Covariance fixture JSONs (single- or multi-block) — correlated-survey blocks for the
        /// V6 covariance-aware likelihood. Omit for a pure diagonal likelihood.
        #[arg(long)]
        covariance: Vec<PathBuf>,
    },
}

pub fn run(command: GenomeCommand) -> Result<()> {
    match command {
        GenomeCommand::Population {
            observables,
            covariance,
            output_root,
            max_generations,
            population_size,
            seed,
            run_id,
            with_fixture_proposer,
            with_router_proposer,
            router_every,
            router_samples,
            router_repairs,
            router_timeout_seconds,
            router_bands,
            router_url,
        } => {
            let run_id = run_id.unwrap_or_else(|| format!("population-g{max_generations}-s{seed}"));
            let config = EvolveConfig {
                population_size,
                max_generations,
                seed,
            };
            let fixture = FixtureProposer;
            let router = if with_router_proposer {
                let mut cfg = RouterConfig {
                    timeout_seconds: router_timeout_seconds,
                    samples: router_samples,
                    repairs: router_repairs,
                    bands: router_bands
                        .split(',')
                        .map(|b| b.trim().to_string())
                        .filter(|b| !b.is_empty())
                        .collect(),
                    ..Default::default()
                };
                if let Some(url) = router_url {
                    cfg.base_url = url;
                }
                router_preflight(&cfg).context("router preflight failed")?;
                let obs_loaded = load_observables(&observables)?;
                let blocks = load_covariance_blocks(&covariance)?;
                let baseline = baseline_log_likelihood_cov(&obs_loaded, &blocks);
                let extra = build_proposer_extra_sections(&obs_loaded, &output_root);
                Some(RouterProposer::new(
                    cfg, obs_loaded, blocks, baseline, extra,
                ))
            } else {
                None
            };
            let budgeted;
            let proposer: Option<&dyn Proposer> = if let Some(r) = router.as_ref() {
                budgeted = BudgetedProposer::new(r as &dyn Proposer, router_every);
                Some(&budgeted)
            } else if with_fixture_proposer {
                Some(&fixture)
            } else {
                None
            };
            let dir = run_population(
                &observables,
                &output_root,
                config,
                &run_id,
                &covariance,
                proposer,
            )?;
            println!("wrote v4 population run to {}", dir.display());
            Ok(())
        }
        GenomeCommand::TrustGate {
            observables,
            covariance: _,
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
            covariance,
            output_root,
            max_generations,
            population_size,
            seed,
            run_id,
            with_fixture_proposer,
            with_router_proposer,
            router_every,
            router_samples,
            router_repairs,
            router_timeout_seconds,
            router_bands,
            router_url,
        } => {
            let run_id = run_id.unwrap_or_else(|| format!("whitepaper-g{max_generations}-s{seed}"));
            let config = EvolveConfig {
                population_size,
                max_generations,
                seed,
            };
            let fixture = FixtureProposer;
            let router = if with_router_proposer {
                let mut cfg = RouterConfig {
                    timeout_seconds: router_timeout_seconds,
                    samples: router_samples,
                    repairs: router_repairs,
                    bands: router_bands
                        .split(',')
                        .map(|b| b.trim().to_string())
                        .filter(|b| !b.is_empty())
                        .collect(),
                    ..Default::default()
                };
                if let Some(url) = router_url {
                    cfg.base_url = url;
                }
                router_preflight(&cfg).context("router preflight failed")?;
                let obs_loaded = load_observables(&observables)?;
                let blocks = load_covariance_blocks(&covariance)?;
                let baseline = baseline_log_likelihood_cov(&obs_loaded, &blocks);
                let extra = build_proposer_extra_sections(&obs_loaded, &output_root);
                Some(RouterProposer::new(
                    cfg, obs_loaded, blocks, baseline, extra,
                ))
            } else {
                None
            };
            let budgeted;
            let proposer: Option<&dyn Proposer> = if let Some(r) = router.as_ref() {
                budgeted = BudgetedProposer::new(r as &dyn Proposer, router_every);
                Some(&budgeted)
            } else if with_fixture_proposer {
                Some(&fixture)
            } else {
                None
            };
            let dir = generate_whitepaper(
                &observables,
                &output_root,
                config,
                &run_id,
                &covariance,
                proposer,
            )?;
            println!(
                "wrote white paper to {}",
                dir.join("white-paper.md").display()
            );
            Ok(())
        }
        GenomeCommand::Propose {
            observables,
            covariance,
            proposal_file,
        } => {
            let obs = load_observables(&observables)?;
            anyhow::ensure!(!obs.is_empty(), "no observables loaded");
            let raw = fs::read_to_string(&proposal_file)
                .with_context(|| format!("read proposal file {}", proposal_file.display()))?;
            let doc = parse_proposal_response(&raw)?;
            let blocks = load_covariance_blocks(&covariance)?;
            let sc = score_proposal(
                &doc,
                &obs,
                &blocks,
                baseline_log_likelihood_cov(&obs, &blocks),
            );
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
        GenomeCommand::Rescore {
            theory,
            observables,
            covariance,
        } => {
            let obs = load_observables(&observables)?;
            anyhow::ensure!(!obs.is_empty(), "no observables loaded");
            let raw = fs::read_to_string(&theory)
                .with_context(|| format!("read theory file {}", theory.display()))?;
            let t: openqg_core::Theory = serde_json::from_str(&raw)
                .with_context(|| format!("parse theory JSON {}", theory.display()))?;
            let kills = openqg_core::physics_kills(&t);
            let blocks = load_covariance_blocks(&covariance)?;
            let sc = score_theory(
                &t,
                &obs,
                &blocks,
                baseline_log_likelihood_cov(&obs, &blocks),
            );
            println!(
                "rescore `{}`: total {:.1}/100, disqualified={}",
                t.id, sc.total, sc.disqualified
            );
            for c in &sc.components {
                println!("  {}: {:.1}/{:.0}", c.name, c.points, c.weight);
            }
            if kills.is_empty() {
                println!("  unified gate: PASS (no physics kills)");
            } else {
                for k in &kills {
                    println!("  KILL: {k:?}");
                }
            }
            Ok(())
        }
        GenomeCommand::Replay {
            ledger,
            observables,
            covariance,
        } => {
            let (checked, mismatches) = replay_ledger(&ledger, &observables, &covariance)?;
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
mod proposer_prompt;
pub(crate) use proposer_prompt::*;
mod proposer_router;
mod proposer_sketch;
pub(crate) use proposer_router::*;
mod proposer_memory;
pub(crate) use proposer_memory::*;
mod ledger_sink;

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
