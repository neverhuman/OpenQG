use anyhow::{bail, Context, Result};
use clap::{Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
const SIGKILL_NUM: i32 = 9;

#[cfg(unix)]
extern "C" {
    fn setsid() -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
}

use crate::util::{read_jsonl, sha256_digest};

/// Explicit empty JSON object for optional runbook/config sub-trees that are
/// legitimately absent. A missing optional section documents "use the built-in
/// defaults for this block" — an expected typed state, not an error or a guess.
fn empty_object() -> Value {
    Value::Object(serde_json::Map::new())
}

const SCHEMA_VERSION: &str = "zyal-gene-eval.v1";
const DEFAULT_STAGE_ROOT: &str = "ZYAL/stages";
const DEFAULT_RUNBOOK_ROOT: &str = "ZYAL/runs";
const DEFAULT_OUTPUT_ROOT: &str = "target/openqg/zyal-genome";
const DEFAULT_SCHEMA_PATH: &str = "ZYAL/schemas/zyal-gene-eval.schema.json";
const DEFAULT_RESEARCH_CACHE: &str = "ZYAL/research-cache";
const DEFAULT_JAILGUN_SERVER_URL: &str = "http://127.0.0.1:8797";
const JAILGUN_HEALTH_TIMEOUT_SECONDS: u64 = 3;
const JAILGUN_MCP_HTTP_TIMEOUT_SECONDS: u64 = 15;
const JAILGUN_MCP_POLL_INTERVAL_MILLIS: u64 = 2_000;
const JAILGUN_MCP_SUMMARY_GRACE_SECONDS: u64 = 10;
const JAILGUN_QUEUE_TIMEOUT_SECONDS: u64 = 30 * 60;
const JAILGUN_RATE_LIMIT_BACKOFF_SECONDS: u64 = 180;
const JAILGUN_RATE_LIMIT_EXTRA_ATTEMPTS: usize = 1;
const JAILGUN_ARTIFACT_SMOKE_TIMEOUT_SECONDS: u64 = 120;
const JAILGUN_ARTIFACT_SMOKE_ATTEMPTS: usize = 2;
const QUALITY_REGRESSION_TOLERANCE: f64 = 0.01;
const NOVELTY_CHAMPION_SCORE_FLOOR: f64 = 0.70;
const DEFAULT_SEED: u64 = 20260605;
const HYBRID_BASELINE_SCORE: f64 = 0.616469;
const JEKKO_LIVE_COMMAND: &[&str] = &[
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
const DEFAULT_JAILGUN_BRIDGE_COMMAND: &[&str] =
    &["/home/ubuntu/jailgun/apps/chrome-bridge/bin/chrome-bridge.mjs"];

const DEFAULT_ISLANDS: [&str; 6] = [
    "foundations",
    "coefficients",
    "observables",
    "failure-repair",
    "interface-contracts",
    "wildcards",
];

const MUTATION_OPS: [&str; 8] = [
    "cross_stage_recombine",
    "source_card_graft",
    "failure_mode_invert",
    "domain_bridge",
    "contract_tighten",
    "novelty_jump",
    "router_reassign",
    "score_blend_jitter",
];

#[derive(Debug, Clone, ValueEnum)]
pub enum GenomeVariant {
    PureJnoccio,
    Hybrid,
    JailgunOnly,
}

impl GenomeVariant {
    fn as_str(&self) -> &'static str {
        match self {
            Self::PureJnoccio => "pure-jnoccio",
            Self::Hybrid => "hybrid",
            Self::JailgunOnly => "jailgun-only",
        }
    }

    fn runbook_name(&self) -> &'static str {
        match self {
            Self::PureJnoccio => "run-pure-jnoccio.zyal",
            Self::Hybrid => "run-hybrid.zyal",
            Self::JailgunOnly => "run-jailgun-only.zyal",
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum GenomeCommand {
    Preflight {
        #[arg(long, value_enum)]
        variant: Option<GenomeVariant>,
        #[arg(long)]
        runbook: PathBuf,
        #[arg(long, default_value = DEFAULT_OUTPUT_ROOT)]
        output_root: PathBuf,
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long)]
        stage_root: Option<PathBuf>,
        #[arg(long)]
        require_hard_backend: bool,
        #[arg(long)]
        live_smoke: bool,
        #[arg(long)]
        live_selective: bool,
        #[arg(long)]
        jailgun_available: bool,
        #[arg(long)]
        jailgun_artifact_smoke: bool,
        #[arg(long, default_value = "json")]
        jailgun_artifact_smoke_extension: String,
    },
    Run {
        #[arg(long, value_enum)]
        variant: GenomeVariant,
        #[arg(long)]
        max_generations: Option<usize>,
        #[arg(long)]
        generations: Option<usize>,
        #[arg(long)]
        population_size: Option<usize>,
        #[arg(long)]
        islands: Option<usize>,
        #[arg(long)]
        novelty_weight: Option<f64>,
        #[arg(long)]
        new_info_refresh: Option<usize>,
        #[arg(long, default_value_t = DEFAULT_SEED)]
        seed: u64,
        #[arg(long, default_value = DEFAULT_OUTPUT_ROOT)]
        output_root: PathBuf,
        #[arg(long)]
        live_selective: bool,
        #[arg(long)]
        research_cache: Option<PathBuf>,
        #[arg(long)]
        resume: bool,
        #[arg(long)]
        checkpoint_every: Option<usize>,
        #[arg(long)]
        run_id: Option<String>,
        #[arg(long)]
        stage_root: Option<PathBuf>,
        #[arg(long)]
        runbook: Option<PathBuf>,
        #[arg(long)]
        jailgun_available: bool,
        #[arg(long)]
        dry_run: bool,
    },
    Emit {
        #[arg(long)]
        run_dir: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    Validate {
        #[arg(long, default_value = DEFAULT_OUTPUT_ROOT)]
        root: PathBuf,
        #[arg(long, default_value = DEFAULT_SCHEMA_PATH)]
        schema: PathBuf,
    },
    PlotIndex {
        #[arg(long)]
        run_dir: Option<PathBuf>,
        #[arg(long)]
        root: Option<PathBuf>,
    },
    QualityGate {
        #[arg(long)]
        run_dir: PathBuf,
    },
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
        /// Inject a real LIVE jailgun (ChatGPT) proposal periodically. Calls ChatGPT ONLY through
        /// jailgun (never Claude/direct); requires the jailgun server up.
        #[arg(long)]
        with_live_proposer: bool,
        /// With --with-live-proposer: spend a live jailgun call on gen 1 and every Nth generation
        /// (the rest evolve deterministically). Bounds the live-call budget.
        #[arg(long, default_value_t = 40)]
        live_every: usize,
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
        /// Inject a real LIVE jailgun (ChatGPT) proposal periodically. Calls ChatGPT ONLY through
        /// jailgun (never Claude/direct); requires the jailgun server up.
        #[arg(long)]
        with_live_proposer: bool,
        /// With --with-live-proposer: spend a live jailgun call on gen 1 and every Nth generation
        /// (the rest evolve deterministically). Bounds the live-call budget.
        #[arg(long, default_value_t = 40)]
        live_every: usize,
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
    },
    /// V4 M6b: one LIVE jailgun proposal (ChatGPT via MCP) adjudicated by the deterministic oracle.
    /// Requires the jailgun server up. The LLM proposes; the oracle disposes.
    ProposeLive {
        #[arg(long)]
        observables: PathBuf,
        #[arg(long, default_value_t = 240)]
        timeout_seconds: u64,
        /// Score a proposal JSON from a file instead of calling jailgun (offline; e.g. a saved live
        /// proposal). When set, no live call is made.
        #[arg(long)]
        proposal_file: Option<PathBuf>,
    },
    /// V4.1: replay a proposal ledger WITHOUT the LLM — re-score each recorded proposal and confirm
    /// it reproduces the recorded total. Makes a live run's "replayable" claim checkable from artifacts.
    Replay {
        #[arg(long)]
        ledger: PathBuf,
        #[arg(long)]
        observables: PathBuf,
    },
    Selftest,
}

pub fn run(command: GenomeCommand) -> Result<()> {
    match command {
        GenomeCommand::Preflight {
            variant,
            runbook,
            output_root,
            run_id,
            stage_root,
            require_hard_backend,
            live_smoke,
            live_selective,
            jailgun_available,
            jailgun_artifact_smoke,
            jailgun_artifact_smoke_extension,
        } => preflight(
            variant,
            runbook,
            output_root,
            run_id,
            stage_root,
            require_hard_backend,
            live_smoke,
            live_selective,
            jailgun_available,
            jailgun_artifact_smoke,
            jailgun_artifact_smoke_extension,
        ),
        GenomeCommand::Run {
            variant,
            max_generations,
            generations,
            population_size,
            islands,
            novelty_weight,
            new_info_refresh,
            seed,
            output_root,
            live_selective,
            research_cache,
            resume,
            checkpoint_every,
            run_id,
            stage_root,
            runbook,
            jailgun_available,
            dry_run,
        } => run_variant(
            variant,
            max_generations,
            generations,
            population_size,
            islands,
            novelty_weight,
            new_info_refresh,
            seed,
            output_root,
            live_selective,
            research_cache,
            resume,
            checkpoint_every,
            run_id,
            stage_root,
            runbook,
            jailgun_available,
            dry_run,
        ),
        GenomeCommand::Emit { run_dir, root } => emit(run_dir, root),
        GenomeCommand::Validate { root, schema } => validate(&root, &schema),
        GenomeCommand::PlotIndex { run_dir, root } => plot_index(run_dir, root),
        GenomeCommand::QualityGate { run_dir } => quality_gate(&run_dir),
        GenomeCommand::Population {
            observables,
            output_root,
            max_generations,
            population_size,
            seed,
            run_id,
            with_fixture_proposer,
            with_live_proposer,
            live_every,
            with_jekko_proposer,
            jekko_every,
            jekko_samples,
            jekko_repairs,
            jekko_timeout_seconds,
        } => {
            let run_id = run_id.unwrap_or_else(|| format!("population-g{max_generations}-s{seed}"));
            let config = EvolveConfig {
                population_size,
                max_generations,
                seed,
            };
            let fixture = FixtureProposer;
            let live = JailgunProposer {
                timeout_seconds: 540,
            };
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
            let composite;
            let proposer: Option<&dyn Proposer> = if let Some(j) = jekko.as_ref() {
                composite = CompositeProposer::new(
                    Some((j as &dyn Proposer, jekko_every)),
                    with_live_proposer.then_some((&live as &dyn Proposer, live_every)),
                );
                Some(&composite)
            } else if with_live_proposer {
                budgeted = BudgetedProposer::new(&live, live_every);
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
            with_live_proposer,
            live_every,
            with_jekko_proposer,
            jekko_every,
            jekko_samples,
            jekko_repairs,
            jekko_timeout_seconds,
        } => {
            let run_id = run_id.unwrap_or_else(|| format!("whitepaper-g{max_generations}-s{seed}"));
            let config = EvolveConfig {
                population_size,
                max_generations,
                seed,
            };
            let fixture = FixtureProposer;
            let live = JailgunProposer {
                timeout_seconds: 540,
            };
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
            let composite;
            let proposer: Option<&dyn Proposer> = if let Some(j) = jekko.as_ref() {
                composite = CompositeProposer::new(
                    Some((j as &dyn Proposer, jekko_every)),
                    with_live_proposer.then_some((&live as &dyn Proposer, live_every)),
                );
                Some(&composite)
            } else if with_live_proposer {
                budgeted = BudgetedProposer::new(&live, live_every);
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
        GenomeCommand::ProposeLive {
            observables,
            timeout_seconds,
            proposal_file,
        } => {
            let obs = load_observables(&observables)?;
            anyhow::ensure!(!obs.is_empty(), "no observables loaded");
            let doc = if let Some(path) = proposal_file {
                let raw = fs::read_to_string(&path)
                    .with_context(|| format!("read proposal file {}", path.display()))?;
                parse_proposal_response(&raw)?
            } else {
                JailgunProposer { timeout_seconds }.propose()?
            };
            let sc = score_proposal(&doc, &obs, baseline_log_likelihood(&obs));
            println!(
                "live jailgun proposal: theory `{}` scored {:.1}/100 (disqualified={})",
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
        GenomeCommand::Selftest => selftest(),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StagePackage {
    stage_id: String,
    name: String,
    family: String,
    track: String,
    purpose: String,
    inputs: Vec<String>,
    outputs: Vec<String>,
    required_evidence: Vec<String>,
    validation_checks: Vec<String>,
    mutation_op: String,
    prompt_path: PathBuf,
    memory_path: PathBuf,
    score_path: PathBuf,
    stage_dir: PathBuf,
    stage_file: PathBuf,
    prompt_hash: String,
    memory: Value,
    score_model: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct PopulationConfig {
    population_size: usize,
    islands: usize,
    island_names: Vec<String>,
    new_info_refresh: usize,
    novelty_weight: f64,
    diversity_targets: Value,
    promotion_gates: Vec<String>,
    degraded_penalties: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct LiveConfig {
    enabled: bool,
    timeout_seconds: u64,
    timeout_by_purpose: BTreeMap<String, u64>,
    retry_count: usize,
    champion_audit_every: usize,
    hard_stage_every: usize,
    promotion_every: usize,
    research_synthesis: bool,
    hard_stage_repair: bool,
    promotion_judging: bool,
    command: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ResumeState {
    completed_generation: usize,
    stage_ledgers: Vec<Value>,
    event_count: usize,
    generation_scores: Vec<f64>,
    stage_score_history: BTreeMap<String, Vec<f64>>,
    router_state: String,
    degraded_router: bool,
    previous_generation_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RoutePolicy {
    route_backend: String,
    route_tier: String,
    router_state: String,
    judge_family: String,
    provenance: String,
    route_policy: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileIdentity {
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
}

impl FileIdentity {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            #[cfg(unix)]
            dev: metadata.dev(),
            #[cfg(unix)]
            ino: metadata.ino(),
        }
    }

    fn matches(&self, metadata: &fs::Metadata) -> bool {
        *self == Self::from_metadata(metadata)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct OutputPathGuard {
    root: PathBuf,
    identity: FileIdentity,
    root_handle: Arc<File>,
}

impl OutputPathGuard {
    fn new(root: &Path) -> Result<Self> {
        let metadata = fs::metadata(root)
            .with_context(|| format!("output-root-deleted: {}", root.display()))?;
        if !metadata.is_dir() {
            bail!("output-root-deleted: {} is not a directory", root.display());
        }
        let root_handle =
            File::open(root).with_context(|| format!("output-root-deleted: {}", root.display()))?;
        Ok(Self {
            root: root.to_path_buf(),
            identity: FileIdentity::from_metadata(&metadata),
            root_handle: Arc::new(root_handle),
        })
    }

    fn check(&self) -> Result<()> {
        let handle_metadata = self
            .root_handle
            .metadata()
            .with_context(|| format!("output-root-changed: {}", self.root.display()))?;
        if !self.identity.matches(&handle_metadata) {
            bail!(
                "output-root-changed: {} original handle changed during run",
                self.root.display()
            );
        }
        let metadata = fs::metadata(&self.root)
            .with_context(|| format!("output-root-deleted: {}", self.root.display()))?;
        if !metadata.is_dir() {
            bail!(
                "output-root-deleted: {} is no longer a directory",
                self.root.display()
            );
        }
        if !FileIdentity::from_metadata(&handle_metadata).matches(&metadata) {
            bail!(
                "output-root-changed: {} was replaced during run",
                self.root.display()
            );
        }
        Ok(())
    }
}

pub(crate) struct JsonlWriter {
    path: PathBuf,
    identity: FileIdentity,
    output_guard: Option<OutputPathGuard>,
    writer: std::io::BufWriter<File>,
}

impl JsonlWriter {
    fn open(path: &Path, append: bool) -> Result<Self> {
        Self::open_with_guard(path, append, None)
    }

    fn open_guarded(path: &Path, append: bool, output_guard: &OutputPathGuard) -> Result<Self> {
        Self::open_with_guard(path, append, Some(output_guard.clone()))
    }

    fn open_with_guard(
        path: &Path,
        append: bool,
        output_guard: Option<OutputPathGuard>,
    ) -> Result<Self> {
        if let Some(guard) = output_guard.as_ref() {
            guard.check()?;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = if append {
            OpenOptions::new().create(true).append(true).open(path)?
        } else {
            OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path)?
        };
        let identity = FileIdentity::from_metadata(&file.metadata()?);
        Ok(Self {
            path: path.to_path_buf(),
            identity,
            output_guard,
            writer: std::io::BufWriter::new(file),
        })
    }

    fn check_path(&self) -> Result<()> {
        if let Some(guard) = self.output_guard.as_ref() {
            guard.check()?;
        }
        let metadata = fs::metadata(&self.path).with_context(|| {
            format!(
                "output-root-changed: open ledger path deleted: {}",
                self.path.display()
            )
        })?;
        if !self.identity.matches(&metadata) {
            bail!(
                "output-root-changed: open ledger path replaced: {}",
                self.path.display()
            );
        }
        Ok(())
    }

    fn write(&mut self, value: &Value) -> Result<()> {
        self.check_path()?;
        serde_json::to_writer(&mut self.writer, value)?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.check_path()?;
        self.writer.flush()?;
        self.check_path()?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct LiveAttempt {
    status: String,
    exit_code: Option<i32>,
    elapsed_seconds: f64,
    stdout: String,
    stderr: String,
    error: Option<String>,
    metadata: Value,
}

impl LiveAttempt {
    pub(crate) fn status(&self) -> &str {
        &self.status
    }
    pub(crate) fn stdout(&self) -> &str {
        &self.stdout
    }
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub(crate) fn elapsed_seconds(&self) -> f64 {
        self.elapsed_seconds
    }
    pub(crate) fn stderr_excerpt(&self) -> &str {
        &self.stderr
    }
    pub(crate) fn metadata(&self) -> &Value {
        &self.metadata
    }
}

#[derive(Clone)]
pub(crate) struct JailgunBridgeCommand {
    args: Vec<String>,
    source: String,
}

#[derive(Debug, Clone)]
pub(crate) struct FrontierReviewFields {
    frontier_claim: String,
    falsifiable_tests: Vec<String>,
    known_failure_modes: Vec<Value>,
    review_priority: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromotionReason {
    TopScore,
    EliteRetain,
    NoveltyQuota,
    IslandBalance,
    IslandCap,
}

impl PromotionReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::TopScore => "top_score",
            Self::EliteRetain => "elite_retain",
            Self::NoveltyQuota => "novelty_quota",
            Self::IslandBalance => "island_balance",
            Self::IslandCap => "island_cap",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ChampionSelection {
    champion: Value,
    reason: PromotionReason,
}

// ---------------------------------------------------------------------------------------
// Live critic (live production tier): jnoccio adversarially critiques a whitebox theory and
// returns a strict-JSON verdict that MODULATES survival. It is subordinate to the deterministic
// physics veto + whitebox gate (it can only lower a candidate that already passed; it never
// resurrects a vetoed/gray-box one). Enabled via ZYAL_LIVE_CRITIC=1; selective on the top-k
// candidates per generation to keep the call budget bounded.
const JNOCCIO_CRITIQUE_COMMAND: &[&str] = &[
    "jekko",
    "run",
    "--headless",
    "--ephemeral",
    "--provider",
    "jnoccio",
    "--model",
    "jnoccio/jnoccio-fusion",
    "--cwd",
    "/home/ubuntu/openQG",
];

#[derive(Debug, Clone)]
pub(crate) struct LiveVerdict {
    falsifiability: f64,
    plausibility: f64,
    fatal_flaw: String,
    status: String,
    elapsed_seconds: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct JailgunStatus {
    strict: bool,
    available: bool,
    evidence: String,
    server_url: Option<String>,
    token_source: Option<String>,
    account_source: Option<String>,
    account_ids: Vec<String>,
    ready_account_ids: Vec<String>,
    mcp_tools: Vec<String>,
    checks: BTreeMap<String, bool>,
    errors: Vec<String>,
}

impl JailgunStatus {
    fn server_authoritative(strict: bool) -> Self {
        Self {
            strict,
            available: false,
            evidence: "server_authoritative_health_check".to_string(),
            server_url: None,
            token_source: None,
            account_source: None,
            account_ids: Vec::new(),
            ready_account_ids: Vec::new(),
            mcp_tools: Vec::new(),
            checks: BTreeMap::new(),
            errors: Vec::new(),
        }
    }

    fn set_check(&mut self, name: &str, passed: bool) {
        self.checks.insert(name.to_string(), passed);
    }

    fn push_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    fn finish(&mut self) {
        self.available = self.checks.values().all(|passed| *passed) && self.errors.is_empty();
        if !self.available && self.evidence == "server_authoritative_health_check" {
            self.evidence = "server_authoritative_health_check_failed".to_string();
        }
    }

    fn as_json(&self) -> Value {
        json!({
            "strict": self.strict,
            "available": self.available,
            "evidence": self.evidence,
            "server_url": self.server_url.as_deref(),
            "token_source": self.token_source.as_deref(),
            "account_source": self.account_source.as_deref(),
            "account_ids": &self.account_ids,
            "ready_account_ids": &self.ready_account_ids,
            "mcp_tools": &self.mcp_tools,
            "checks": &self.checks,
            "errors": &self.errors,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct JailgunToken {
    value: String,
    source: String,
}

impl fmt::Debug for JailgunToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JailgunToken")
            .field("value", &"[redacted]")
            .field("source", &self.source)
            .finish()
    }
}

#[derive(Clone)]
pub(crate) struct JailgunHealthConfig {
    server_url: String,
    token: Option<JailgunToken>,
    account_override_ids: Vec<String>,
}

impl JailgunHealthConfig {
    fn from_env() -> Self {
        let server_url = jailgun_server_url();
        Self {
            token: resolve_jailgun_token(&server_url),
            server_url,
            account_override_ids: jailgun_account_ids_override(),
        }
    }

    fn token_source(&self) -> Option<&str> {
        self.token.as_ref().map(|token| token.source.as_str())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct JailgunProcEntry {
    cmdline: Vec<String>,
    environ: Vec<String>,
}

mod candidates;
pub(crate) use candidates::*;
mod config;
pub(crate) use config::*;
mod critique;
pub(crate) use critique::*;
mod eval;
pub(crate) use eval::*;
mod hybrid_artifacts;
pub(crate) use hybrid_artifacts::*;
mod io;
pub(crate) use io::*;
mod jailgun_classify;
pub(crate) use jailgun_classify::*;
mod jailgun_health;
pub(crate) use jailgun_health::*;
mod jailgun_live;
pub(crate) use jailgun_live::*;
mod jailgun_token;
pub(crate) use jailgun_token::*;
mod live_call;
pub(crate) use live_call::*;
mod markdown;
pub(crate) use markdown::*;
mod novelty;
pub(crate) use novelty::*;
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
mod proposer_jailgun;
pub(crate) use proposer_jailgun::*;
mod proposer_memory;
pub(crate) use proposer_memory::*;
mod ledger_sink;
pub(crate) use ledger_sink::*;
mod proposer_jekko;
pub(crate) use proposer_jekko::*;
mod preflight;
pub(crate) use preflight::*;
mod quality_gate;
pub(crate) use quality_gate::*;
mod quality_metrics;
pub(crate) use quality_metrics::*;
mod records;
pub(crate) use records::*;
mod research;
pub(crate) use research::*;
mod route;
pub(crate) use route::RouteTier;
mod resume;
pub(crate) use resume::*;
mod run_summary;
pub(crate) use run_summary::*;
mod run_variant;
pub(crate) use run_variant::*;
mod scoring;
pub(crate) use scoring::*;
mod selection;
pub(crate) use selection::*;
mod selftest;
pub(crate) use selftest::*;
mod stages;
pub(crate) use stages::*;
mod util;
pub(crate) use util::*;
mod validate;
pub(crate) use validate::*;

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

#[cfg(test)]
mod tests;
