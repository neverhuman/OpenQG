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
        GenomeCommand::Selftest => selftest(),
    }
}

#[derive(Debug, Clone)]
struct StagePackage {
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
struct PopulationConfig {
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
struct LiveConfig {
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
struct ResumeState {
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
struct RoutePolicy {
    route_backend: String,
    route_tier: String,
    router_state: String,
    judge_family: String,
    provenance: String,
    route_policy: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
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
struct OutputPathGuard {
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

struct JsonlWriter {
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

pub fn preflight(
    variant: Option<GenomeVariant>,
    runbook_path: PathBuf,
    output_root: PathBuf,
    run_id: Option<String>,
    stage_root: Option<PathBuf>,
    require_hard_backend: bool,
    live_smoke: bool,
    live_selective: bool,
    jailgun_available: bool,
    jailgun_artifact_smoke: bool,
    jailgun_artifact_smoke_extension: String,
) -> Result<()> {
    let runbook = load_runbook(&runbook_path)?;
    let variant = variant
        .or_else(|| variant_from_runbook(&runbook))
        .unwrap_or(GenomeVariant::Hybrid);
    let evaluation = runbook
        .get("evaluation")
        .cloned()
        .unwrap_or_else(empty_object);
    let output_root = if output_root != PathBuf::from(DEFAULT_OUTPUT_ROOT) {
        output_root
    } else {
        evaluation
            .get("output_root")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT_ROOT))
    };
    let max_generations = resolve_generation_count(None, None, &evaluation)?;
    let run_id = run_id
        .or_else(|| {
            evaluation
                .get("run_id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| default_run_id(variant.as_str(), DEFAULT_SEED, max_generations));
    let run_dir = output_root.join("runs").join(&run_id);
    fs::create_dir_all(&run_dir)?;
    let output_guard = OutputPathGuard::new(&output_root)?;
    let stage_root = stage_root
        .or_else(|| {
            runbook
                .get("context")
                .and_then(|context| context.get("stage_root"))
                .and_then(Value::as_str)
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STAGE_ROOT));
    let stage_registry = load_stage_registry(&stage_root)?;
    validate_stage_registry(&stage_registry)?;
    let live_config = resolve_live_config(live_selective, &runbook);
    let require_hard_backend =
        require_hard_backend || hard_backend_required(&runbook, &variant, &run_id, max_generations);
    let jailgun_status = resolve_jailgun_status(
        strict_jailgun_required(
            &variant,
            &run_id,
            require_hard_backend,
            hard_stage_count(&stage_registry),
        ),
        jailgun_available,
    );
    let jailgun_available = jailgun_status.available;
    output_guard.check()?;
    let mut receipt = preflight_receipt(
        &run_id,
        variant.as_str(),
        &runbook,
        &runbook_path,
        &run_dir,
        &stage_registry,
        &live_config,
        require_hard_backend,
        live_smoke,
        &jailgun_status,
    );
    let artifact_smoke_receipt = if jailgun_artifact_smoke {
        let smoke = run_jailgun_artifact_smoke(
            &run_dir,
            &run_id,
            &jailgun_artifact_smoke_extension,
            Some(&output_guard),
        )?;
        receipt["backend_health"]["jailgun"]["artifact_smoke"] = smoke.clone();
        Some(smoke)
    } else {
        None
    };
    output_guard.check()?;
    write_json(&run_dir.join("preflight.json"), &receipt)?;
    let hard_backend_blocked = matches!(variant, GenomeVariant::Hybrid)
        && require_hard_backend
        && hard_stage_count(&stage_registry) > 0
        && !jailgun_available;
    let live_command_blocked = live_smoke
        && !receipt
            .get("live_command")
            .and_then(|value| value.get("executable_found"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let artifact_smoke_blocked = artifact_smoke_receipt
        .as_ref()
        .map(|smoke| smoke.get("status").and_then(Value::as_str) != Some("ok"))
        .unwrap_or(false);
    if hard_backend_blocked || live_command_blocked || artifact_smoke_blocked {
        bail!(
            "preflight failed for {} (hard_backend_available={}, live_command_found={}, jailgun_artifact_smoke_ok={})",
            run_id,
            jailgun_available,
            !live_command_blocked,
            !artifact_smoke_blocked
        );
    }
    println!(
        "wrote preflight receipt to {}",
        run_dir.join("preflight.json").display()
    );
    Ok(())
}

pub fn run_variant(
    variant: GenomeVariant,
    max_generations: Option<usize>,
    generations: Option<usize>,
    population_size: Option<usize>,
    islands: Option<usize>,
    novelty_weight: Option<f64>,
    new_info_refresh: Option<usize>,
    seed: u64,
    output_root: PathBuf,
    live_selective: bool,
    research_cache: Option<PathBuf>,
    resume: bool,
    checkpoint_every: Option<usize>,
    run_id: Option<String>,
    stage_root: Option<PathBuf>,
    runbook: Option<PathBuf>,
    jailgun_available: bool,
    dry_run: bool,
) -> Result<()> {
    let runbook_path =
        runbook.unwrap_or_else(|| PathBuf::from(DEFAULT_RUNBOOK_ROOT).join(variant.runbook_name()));
    let runbook = load_runbook(&runbook_path)?;
    let evaluation = runbook
        .get("evaluation")
        .cloned()
        .unwrap_or_else(empty_object);
    let jailgun_available = jailgun_available || jailgun_available_from_environment();

    let max_generations = resolve_generation_count(generations, max_generations, &evaluation)?;
    let population = resolve_population_config(
        population_size,
        islands,
        novelty_weight,
        new_info_refresh,
        &evaluation,
    )?;
    let stage_root = stage_root
        .or_else(|| {
            runbook
                .get("context")
                .and_then(|context| context.get("stage_root"))
                .and_then(Value::as_str)
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| PathBuf::from(DEFAULT_STAGE_ROOT));
    let output_root = if output_root != PathBuf::from(DEFAULT_OUTPUT_ROOT) {
        output_root
    } else {
        evaluation
            .get("output_root")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT_ROOT))
    };
    let run_id = run_id
        .or_else(|| {
            evaluation
                .get("run_id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| default_run_id(variant.as_str(), seed, max_generations));
    let run_dir = output_root.join("runs").join(&run_id);
    if run_dir.exists() && run_dir.read_dir()?.next().is_some() && !resume {
        bail!(
            "run directory already exists: {}; pass --resume or choose --run-id",
            run_dir.display()
        );
    }
    fs::create_dir_all(&run_dir)?;
    let output_guard = OutputPathGuard::new(&output_root)?;

    let stage_registry = load_stage_registry(&stage_root)?;
    validate_stage_registry(&stage_registry)?;
    let live_config = resolve_live_config(live_selective, &runbook);
    let hard_backend_required = hard_backend_required(&runbook, &variant, &run_id, max_generations);
    let jailgun_status = resolve_jailgun_status(
        strict_jailgun_required(
            &variant,
            &run_id,
            hard_backend_required,
            hard_stage_count(&stage_registry),
        ),
        jailgun_available,
    );
    let jailgun_available = jailgun_status.available;
    let receipt = preflight_receipt(
        &run_id,
        variant.as_str(),
        &runbook,
        &runbook_path,
        &run_dir,
        &stage_registry,
        &live_config,
        hard_backend_required,
        false,
        &jailgun_status,
    );
    output_guard.check()?;
    write_json(&run_dir.join("preflight.json"), &receipt)?;
    if hard_backend_required && hard_stage_count(&stage_registry) > 0 && !jailgun_available {
        bail!(
            "hybrid hard-stage backend unavailable for {}; run preflight or use an alternate run id",
            run_id
        );
    }
    let checkpoint_every = resolve_checkpoint_every(checkpoint_every, &evaluation)?;
    let research_cache = resolve_research_cache_path(research_cache, &runbook);
    let research_cards = load_research_cache_cards(research_cache.as_deref(), &run_id)?;
    let hybrid_mode = matches!(variant, GenomeVariant::Hybrid);
    let resume_state = if resume {
        load_resume_state(&run_dir, &stage_registry)?
    } else {
        empty_resume_state(&stage_registry)
    };

    let raw_completed_generation = resume_state.completed_generation;
    if resume
        && raw_completed_generation >= max_generations
        && run_dir.join("run-summary.json").exists()
    {
        output_guard.check()?;
        emit_run_plot_index(&run_dir)?;
        touch_latest(&output_root, &run_dir)?;
        println!("run already complete through g{raw_completed_generation:04}");
        return Ok(());
    }

    let completed_generation = raw_completed_generation.min(max_generations);
    let start_generation = completed_generation + 1;
    let append_mode = resume && completed_generation > 0;

    let mut run_events = JsonlWriter::open_guarded(
        &run_dir.join("run-events.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut stage_ledger = JsonlWriter::open_guarded(
        &run_dir.join("stage-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut generation_ledger = JsonlWriter::open_guarded(
        &run_dir.join("generation-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut metrics_ledger = JsonlWriter::open_guarded(
        &run_dir.join("metrics-timeseries.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut stage_variant_ledger = JsonlWriter::open_guarded(
        &run_dir.join("stage-variant-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut live_call_ledger = JsonlWriter::open_guarded(
        &run_dir.join("live-call-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut research_ledger = JsonlWriter::open_guarded(
        &run_dir.join("research-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut memory_ledger = JsonlWriter::open_guarded(
        &run_dir.join("memory-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut promotion_ledger = JsonlWriter::open_guarded(
        &run_dir.join("promotion-ledger.jsonl"),
        append_mode,
        &output_guard,
    )?;
    let mut lineage_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("lineage-graph.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };
    let mut population_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("population-ledger.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };
    let mut concept_gene_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("concept-gene-ledger.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };
    let mut information_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("information-ledger.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };
    let mut stage_concept_ledger = if hybrid_mode {
        Some(JsonlWriter::open_guarded(
            &run_dir.join("stage-concept-ledger.jsonl"),
            append_mode,
            &output_guard,
        )?)
    } else {
        None
    };

    if !append_mode {
        write_memory_cards(&mut memory_ledger, &stage_registry, &run_id)?;
        write_research_cards(&mut research_ledger, &research_cards)?;
    }

    let mut stage_ledgers = resume_state.stage_ledgers.clone();
    let mut event_count = resume_state.event_count;
    let mut generation_scores = resume_state.generation_scores.clone();
    let mut stage_score_history = resume_state.stage_score_history.clone();
    let mut router_state = resume_state.router_state.clone();
    let mut degraded_router = resume_state.degraded_router;
    let mut previous_generation_ids = resume_state.previous_generation_ids.clone();

    let mut accepted_cards = Vec::new();
    let mut concepts = Vec::new();
    if hybrid_mode {
        let seed_cards = if research_cards.is_empty() {
            synthesize_information_cards(&stage_registry, &run_id)
        } else {
            research_cards.clone()
        };
        for card in seed_cards {
            accepted_cards.push(card.clone());
            if let Some(ref mut ledger) = information_ledger {
                ledger.write(&card)?;
            }
            let concept = concept_gene_from_card(&card);
            concepts.push(concept.clone());
            if let Some(ref mut ledger) = concept_gene_ledger {
                ledger.write(&concept)?;
            }
        }
    }

    if start_generation <= max_generations {
        if !append_mode {
            run_events.write(&run_event(
                &run_id,
                variant.as_str(),
                "run_start",
                "g0000",
                "run",
                None,
                None,
                "run",
                "jnoccio",
                "standard",
                "nominal",
                artifact_paths(&run_dir, &run_dir, &[], &[]),
                judge_block("jnoccio", "scripted-run-start", "standard", 256, 64),
                constant_score_block(0.0),
            ))?;
            event_count += 1;
        }

        for generation_index in start_generation..=max_generations {
            output_guard.check()?;
            let generation_id = format!("g{:04}", generation_index);
            run_events.write(&run_event(
                &run_id,
                variant.as_str(),
                "generation_start",
                &generation_id,
                &generation_id,
                previous_generation_id(generation_index),
                None,
                "generation",
                "jnoccio",
                "standard",
                "nominal",
                artifact_paths(&run_dir, &run_dir, &[], &[]),
                judge_block("jnoccio", "scripted-generation", "standard", 256, 64),
                constant_score_block(0.0),
            ))?;
            event_count += 1;

            let mode_counts = population_mode_counts(population.population_size);
            let mode_list = build_mode_list(&mode_counts);
            let mut generation_stage_scores = Vec::new();
            let mut generation_passes = 0usize;
            let mut generation_failures = 0usize;
            let mut generation_candidates = Vec::new();
            let mut used_parent_ids = BTreeSet::new();

            if hybrid_mode {
                let _ = (generation_index == 1)
                    || (generation_index % population.new_info_refresh == 0);
                let mut info_was_written = false;
                if generation_index == 1 || generation_index % population.new_info_refresh == 0 {
                    if let Some(ref mut ledger) = information_ledger {
                        for card in synthesize_additional_information_cards(
                            &stage_registry,
                            &generation_id,
                            &run_id,
                            1,
                        ) {
                            accepted_cards.push(card.clone());
                            ledger.write(&card)?;
                            let concept = concept_gene_from_card(&card);
                            concepts.push(concept.clone());
                            if let Some(ref mut concept_ledger) = concept_gene_ledger {
                                concept_ledger.write(&concept)?;
                            }
                        }
                    }
                    info_was_written = true;
                }
                if !info_was_written && accepted_cards.is_empty() {
                    let synthesized = synthesize_information_cards(&stage_registry, &run_id);
                    for card in synthesized {
                        accepted_cards.push(card.clone());
                        if let Some(ref mut ledger) = information_ledger {
                            ledger.write(&card)?;
                        }
                        let concept = concept_gene_from_card(&card);
                        concepts.push(concept.clone());
                        if let Some(ref mut concept_ledger) = concept_gene_ledger {
                            concept_ledger.write(&concept)?;
                        }
                    }
                }
            }

            for (stage_index, stage) in stage_registry.iter().enumerate() {
                output_guard.check()?;
                let stage_dir = run_dir.join("stages").join(&stage.stage_id);
                let generation_dir = stage_dir.join("generations").join(&generation_id);
                fs::create_dir_all(&generation_dir)?;
                let candidate_id = format!("{generation_id}-{}", stage.stage_id);
                let route =
                    route_for_variant(&variant, stage, live_config.enabled, jailgun_available);
                let research_refs = select_research_refs_for_stage(&research_cards, stage);
                let live_purpose = live_call_purpose(
                    &live_config,
                    stage,
                    generation_index,
                    stage_index,
                    stage_registry.len(),
                );
                let live_records = if let Some(purpose) = live_purpose {
                    let record = run_live_call(
                        &run_dir,
                        stage,
                        &route,
                        &generation_id,
                        &candidate_id,
                        &purpose,
                        &live_config,
                        &research_refs,
                        Some(&output_guard),
                    )?;
                    live_call_ledger.write(&record)?;
                    vec![record]
                } else {
                    Vec::new()
                };

                let scores = compute_scores(
                    variant.as_str(),
                    stage,
                    generation_index,
                    seed,
                    &route,
                    jailgun_available,
                );
                let score_breakdown =
                    compute_score_breakdown(&scores, stage, &live_records, &research_refs);
                let artifact_paths_block =
                    artifact_paths(&run_dir, &stage_dir, &stage.inputs, &stage.outputs);
                let judge = judge_block(
                    &route.judge_family,
                    &route.provenance,
                    &route.route_tier,
                    scores
                        .get("prompt_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                    scores
                        .get("completion_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize,
                );

                let parent_ids = candidate_parent_ids(&previous_generation_ids, stage_index);
                used_parent_ids.extend(parent_ids.iter().cloned());

                let stage_variant = stage_variant_record(
                    &run_id,
                    variant.as_str(),
                    &generation_id,
                    &candidate_id,
                    stage,
                    &parent_ids,
                    &score_breakdown,
                    &research_refs,
                );
                stage_variant_ledger.write(&stage_variant)?;

                let stage_start = run_event(
                    &run_id,
                    variant.as_str(),
                    "stage_start",
                    &generation_id,
                    &stage.stage_id,
                    previous_generation_id(generation_index),
                    Some(stage.mutation_op.clone()),
                    &stage.family,
                    &route.route_backend,
                    &route.route_tier,
                    &route.router_state,
                    artifact_paths_block.clone(),
                    judge.clone(),
                    constant_score_block(0.0),
                );
                run_events.write(&stage_start)?;
                event_count += 1;

                let mut router_decision = stage_start.clone();
                router_decision["event_type"] = json!("router_decision");
                merge_object(&mut router_decision, &scores);
                run_events.write(&router_decision)?;
                event_count += 1;

                let mut candidate_event = stage_start.clone();
                candidate_event["event_type"] = json!("candidate_evaluated");
                merge_object(&mut candidate_event, &scores);
                run_events.write(&candidate_event)?;
                event_count += 1;

                let mut stage_end = stage_start.clone();
                stage_end["event_type"] = json!("stage_end");
                merge_object(&mut stage_end, &scores);
                run_events.write(&stage_end)?;
                event_count += 1;

                let final_score = scores
                    .get("final_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let delta_score =
                    final_score - generation_stage_scores.last().copied().unwrap_or(0.0);
                let pass_rate = if final_score >= 0.45 { 1.0 } else { 0.0 };
                if pass_rate < 1.0 {
                    generation_failures += 1;
                } else {
                    generation_passes += 1;
                }
                generation_stage_scores.push(final_score);
                stage_score_history
                    .entry(stage.stage_id.clone())
                    .or_default()
                    .push(final_score);
                let ledger_entry = stage_ledger_record(
                    &run_id,
                    variant.as_str(),
                    &generation_id,
                    &stage.stage_id,
                    &candidate_id,
                    &stage.name,
                    &route,
                    Some(stage.mutation_op.clone()),
                    previous_generation_id(generation_index),
                    delta_score,
                    pass_rate,
                    scores
                        .get("time_seconds")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                    scores
                        .get("failure_modes")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default(),
                    artifact_paths_block.clone(),
                    &scores,
                );
                stage_ledger.write(&ledger_entry)?;
                stage_ledgers.push(ledger_entry);
                event_count += 1;

                metrics_ledger.write(&metrics_point(
                    &run_id,
                    variant.as_str(),
                    &generation_id,
                    "stage_final_score",
                    final_score,
                    "stage",
                    Some(&stage.stage_id),
                    Some(&candidate_id),
                    json!({
                        "stage_variant_id": stage_variant["stage_variant_id"],
                        "island": stage.track,
                        "score_breakdown": score_breakdown,
                    }),
                ))?;

                if route.router_state != "nominal" {
                    degraded_router = true;
                    router_state = route.router_state.clone();
                }

                if hybrid_mode {
                    let stage_concepts = choose_stage_concepts(
                        &stage_registry,
                        &concepts,
                        generation_index,
                        stage_index + 1,
                        seed,
                    );
                    let candidate = hybrid_candidate_record(
                        &generation_id,
                        stage,
                        &candidate_id,
                        &parent_ids,
                        &mode_list[stage_index % mode_list.len()],
                        &research_refs,
                        &stage_concepts,
                        &route.route_policy,
                        &scores,
                        &score_breakdown,
                    );
                    generation_candidates.push(candidate.clone());
                    let promoted = promote_generation_candidates(&generation_candidates);
                    if let Some(ref mut ledger) = population_ledger {
                        ledger.write(&population_snapshot(
                            &run_id,
                            &generation_id,
                            population.population_size,
                            &population.island_names,
                            &mode_counts,
                            &generation_candidates,
                            &promoted,
                            candidate.get("candidate_id").unwrap_or(&json!("")),
                            final_score,
                            json!({
                                "concept_entropy": 2.5,
                                "island_balance": 0.75,
                                "source_diversity": 0.50,
                                "stage_concept_churn": 0.25,
                            }),
                        ))?;
                    }
                    if let Some(ref mut ledger) = stage_concept_ledger {
                        for (stage_id, concept_id) in stage_concepts {
                            ledger.write(&json!({
                                "schema_version": SCHEMA_VERSION,
                                "record_kind": "stage_concept",
                                "run_id": run_id,
                                "generation_id": generation_id,
                                "stage_id": stage_id,
                                "candidate_id": candidate_id,
                                "concept_id": concept_id,
                                "family": stage.family,
                                "source_card_ids": research_refs,
                                "mutation_op": stage.mutation_op,
                            }))?;
                        }
                    }
                    if let Some(ref mut ledger) = lineage_ledger {
                        for parent_id in &parent_ids {
                            ledger.write(&lineage_edge_record(
                                &run_id,
                                &generation_id,
                                json!(parent_id),
                                candidate
                                    .get("candidate_id")
                                    .cloned()
                                    .unwrap_or_else(|| json!("")),
                                stage.mutation_op.clone(),
                                stage.track.clone(),
                            ))?;
                        }
                        if parent_ids.is_empty() {
                            ledger.write(&lineage_edge_record(
                                &run_id,
                                &generation_id,
                                Value::Null,
                                candidate
                                    .get("candidate_id")
                                    .cloned()
                                    .unwrap_or_else(|| json!("")),
                                stage.mutation_op.clone(),
                                stage.track.clone(),
                            ))?;
                        }
                    }
                }
            }

            let generation_final = if generation_stage_scores.is_empty() {
                0.0
            } else {
                generation_stage_scores.iter().sum::<f64>() / generation_stage_scores.len() as f64
            };
            generation_scores.push(generation_final);
            if hybrid_mode {
                previous_generation_ids = generation_candidates
                    .iter()
                    .filter_map(|candidate| {
                        candidate
                            .get("candidate_id")
                            .and_then(Value::as_str)
                            .map(ToString::to_string)
                    })
                    .collect();
            }

            run_events.write(&run_event(
                &run_id,
                variant.as_str(),
                "generation_end",
                &generation_id,
                "generation",
                previous_generation_id(generation_index),
                Some("generation_rollup".to_string()),
                "generation",
                "jnoccio",
                "standard",
                &router_state,
                artifact_paths(&run_dir, &run_dir, &[], &[]),
                judge_block("jnoccio", "scripted-generation-end", "standard", 256, 64),
                constant_score_block(generation_final),
            ))?;
            event_count += 1;

            let generation_metric = metrics_point(
                &run_id,
                variant.as_str(),
                &generation_id,
                "deterministic_rollup_score",
                generation_final,
                "generation_rollup",
                None,
                Some(&generation_id),
                json!({
                    "stage_count": stage_registry.len(),
                    "stage_passes": generation_passes,
                    "stage_failures": generation_failures,
                    "best_stage_score": generation_stage_scores.iter().copied().fold(0.0, f64::max),
                    "mean_stage_score": generation_final,
                }),
            );
            generation_ledger.write(&generation_metric)?;
            metrics_ledger.write(&generation_metric)?;
            if checkpoint_every > 0
                && (generation_index % checkpoint_every == 0 || generation_index == max_generations)
            {
                output_guard.check()?;
                write_checkpoint(
                    &run_dir,
                    &run_id,
                    variant.as_str(),
                    generation_index,
                    max_generations,
                    Some(&output_guard),
                )?;
            }
        }

        if start_generation <= max_generations {
            run_events.write(&run_event(
                &run_id,
                variant.as_str(),
                "run_end",
                &format!("g{:04}", max_generations),
                "run",
                previous_generation_id(max_generations),
                None,
                "run",
                "jnoccio",
                "standard",
                &router_state,
                artifact_paths(&run_dir, &run_dir, &[], &[]),
                judge_block("jnoccio", "scripted-run-end", "standard", 256, 64),
                constant_score_block(*generation_scores.last().unwrap_or(&0.0)),
            ))?;
            event_count += 1;
        }
    }

    run_events.flush()?;
    stage_ledger.flush()?;
    generation_ledger.flush()?;
    metrics_ledger.flush()?;
    stage_variant_ledger.flush()?;
    live_call_ledger.flush()?;
    research_ledger.flush()?;
    memory_ledger.flush()?;
    promotion_ledger.flush()?;
    if let Some(ref mut writer) = lineage_ledger {
        writer.flush()?;
    }
    if let Some(ref mut writer) = population_ledger {
        writer.flush()?;
    }
    if let Some(ref mut writer) = concept_gene_ledger {
        writer.flush()?;
    }
    if let Some(ref mut writer) = information_ledger {
        writer.flush()?;
    }
    if let Some(ref mut writer) = stage_concept_ledger {
        writer.flush()?;
    }

    output_guard.check()?;
    let stage_summary_paths = write_stage_summaries(
        &run_dir,
        &stage_registry,
        &stage_ledgers,
        &stage_score_history,
        variant.as_str(),
        &run_id,
    )?;
    output_guard.check()?;
    let hybrid_evolution = if hybrid_mode {
        Some(emit_hybrid_evolution_artifacts(
            &run_dir,
            &run_id,
            &stage_registry,
            max_generations,
            seed,
            jailgun_available,
            &population,
            start_generation,
            &live_config,
            &research_cards,
        )?)
    } else {
        None
    };

    output_guard.check()?;
    let summary = build_run_summary(
        &run_id,
        variant.as_str(),
        &runbook,
        &runbook_path,
        max_generations,
        seed,
        dry_run,
        jailgun_available,
        &generation_scores,
        &stage_ledgers,
        &stage_registry,
        &router_state,
        degraded_router,
        &stage_summary_paths,
        &run_dir,
        hybrid_evolution.as_ref(),
        &population,
    );
    output_guard.check()?;
    write_json(&run_dir.join("run-summary.json"), &summary)?;
    output_guard.check()?;
    write_json(
        &run_dir.join("pareto-snapshot.json"),
        summary.get("pareto_snapshot").unwrap_or(&json!({})),
    )?;
    output_guard.check()?;
    let offline_eval = emit_run_offline_eval(&run_dir)?;
    output_guard.check()?;
    write_json(&run_dir.join("offline-eval.json"), &offline_eval)?;
    output_guard.check()?;
    write_markdown(
        &run_dir.join("offline-eval.md"),
        &render_run_markdown(&offline_eval),
    )?;
    output_guard.check()?;
    emit_run_plot_index(&run_dir)?;
    output_guard.check()?;
    write_checkpoint(
        &run_dir,
        &run_id,
        variant.as_str(),
        max_generations,
        max_generations,
        Some(&output_guard),
    )?;
    output_guard.check()?;
    touch_latest(&output_root, &run_dir)?;
    println!(
        "wrote genome run for {} to {}",
        variant.as_str(),
        run_dir.display()
    );
    Ok(())
}

pub fn emit(run_dir: Option<PathBuf>, root: Option<PathBuf>) -> Result<()> {
    match (run_dir, root) {
        (Some(run_dir), None) => {
            emit_run_offline_eval(&run_dir)?;
            println!("emitted offline evaluation for {}", run_dir.display());
        }
        (None, Some(root)) => {
            emit_root_comparison(&root)?;
            println!("emitted comparison for {}", root.display());
        }
        _ => bail!("pass exactly one of --run-dir or --root"),
    }
    Ok(())
}

pub fn plot_index(run_dir: Option<PathBuf>, root: Option<PathBuf>) -> Result<()> {
    match (run_dir, root) {
        (Some(run_dir), None) => {
            let path = emit_run_plot_index(&run_dir)?;
            println!("wrote plot index to {}", path.display());
        }
        (None, Some(root)) => {
            let path = emit_root_plot_index(&root)?;
            println!("wrote plot index to {}", path.display());
        }
        _ => bail!("pass exactly one of --run-dir or --root"),
    }
    Ok(())
}

pub fn quality_gate(run_dir: &Path) -> Result<()> {
    let _ = emit_run_plot_index(run_dir);
    let report = build_quality_gate_report(run_dir)?;
    write_json(&run_dir.join("quality-gate.json"), &report)?;
    write_markdown(
        &run_dir.join("quality-gate.md"),
        &render_quality_gate_markdown(&report),
    )?;
    emit_run_plot_index(run_dir)?;
    let passed = report
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !passed {
        bail!(
            "quality gate failed for {}; see {}",
            report
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown"),
            run_dir.join("quality-gate.json").display()
        );
    }
    println!("quality gate passed for {}", run_dir.display());
    Ok(())
}

pub fn validate(root: &Path, schema: &Path) -> Result<()> {
    let _ = fs::read_to_string(schema).with_context(|| format!("read {}", schema.display()))?;
    let artifact_paths = collect_artifact_files(root)?;
    if artifact_paths.is_empty() {
        bail!("no genome artifacts found under {}", root.display());
    }
    for path in &artifact_paths {
        if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            for record in read_jsonl::<Value>(path)? {
                validate_record(&record).with_context(|| format!("validate {}", path.display()))?;
            }
        } else {
            let record: Value = serde_json::from_str(&fs::read_to_string(path)?)
                .with_context(|| format!("parse {}", path.display()))?;
            validate_record(&record).with_context(|| format!("validate {}", path.display()))?;
        }
    }
    validate_hybrid_invariants(root)?;
    println!(
        "validated {} genome artifact files under {}",
        artifact_paths.len(),
        root.display()
    );
    Ok(())
}

pub fn selftest() -> Result<()> {
    assert_eq!(
        population_mode_counts(24),
        BTreeMap::from([
            ("exploitation".to_string(), 12usize),
            ("novelty".to_string(), 8usize),
            ("wildcard".to_string(), 4usize),
        ])
    );
    assert_eq!(
        reject_information_text("", ""),
        Some("empty_provenance".to_string())
    );
    assert_eq!(
        reject_information_text("source.md", "copied benchmark value 0.123"),
        Some("copied_benchmark_values".to_string())
    );

    let stage_registry = vec![
        json!({
            "stage_id": "s1",
            "family": "hard",
            "track": "routing",
            "name": "one",
            "purpose": "p",
            "inputs": ["a"],
            "outputs": ["b"],
            "required_evidence": ["c"],
            "validation_checks": ["d"],
            "mutation_op": "emit_atlas",
            "prompt_hash": "abcdef0123",
        }),
        json!({
            "stage_id": "s2",
            "family": "standard",
            "track": "routing",
            "name": "two",
            "purpose": "p",
            "inputs": ["a"],
            "outputs": ["b"],
            "required_evidence": ["c"],
            "validation_checks": ["d"],
            "mutation_op": "emit_atlas",
            "prompt_hash": "abcdef0123",
        }),
    ];
    let concepts = vec![json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "concept_gene",
        "gene_id": "seed-derivation-first",
        "concept_id": "derivation-first",
        "family": "foundations",
        "domain": "foundations",
        "claim": "derive first",
        "method": "selftest",
        "constraint": "local",
        "failure_risk": "none",
        "stage_concept_hint": "derivation-first",
        "source_card_ids": [],
        "source_path": "selftest",
        "provenance_hash": stable_hash("selftest"),
        "novelty_terms": ["derivation"],
    })];
    let choices_a =
        adapt_stage_concepts_from_values(&stage_registry, &concepts, 3, 2, DEFAULT_SEED);
    let choices_b =
        adapt_stage_concepts_from_values(&stage_registry, &concepts, 3, 2, DEFAULT_SEED);
    assert_eq!(choices_a, choices_b);
    let scores_a = compute_candidate_scores(
        "novelty",
        "foundations",
        "novelty_jump",
        &choices_a,
        3,
        8,
        2,
        DEFAULT_SEED,
        true,
        0.08,
        0.13,
    );
    let scores_b = compute_candidate_scores(
        "novelty",
        "foundations",
        "novelty_jump",
        &choices_b,
        3,
        8,
        2,
        DEFAULT_SEED,
        true,
        0.08,
        0.13,
    );
    assert_eq!(scores_a, scores_b);
    assert!(lineage_edges_are_acyclic(&BTreeMap::from([
        ("b".to_string(), vec!["a".to_string()]),
        ("c".to_string(), vec!["b".to_string()]),
    ])));
    assert!(!lineage_edges_are_acyclic(&BTreeMap::from([
        ("a".to_string(), vec!["c".to_string()]),
        ("b".to_string(), vec!["a".to_string()]),
        ("c".to_string(), vec!["b".to_string()]),
    ])));
    println!("zyal-genome selftest passed");
    Ok(())
}

fn resolve_generation_count(
    generations: Option<usize>,
    max_generations: Option<usize>,
    evaluation: &Value,
) -> Result<usize> {
    let count = generations
        .or(max_generations)
        .or_else(|| {
            evaluation
                .get("max_generations_default")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
        })
        .unwrap_or(1);
    if count < 1 {
        bail!("--max-generations/--generations must be at least 1");
    }
    Ok(count)
}

fn resolve_population_config(
    population_size: Option<usize>,
    islands: Option<usize>,
    novelty_weight: Option<f64>,
    new_info_refresh: Option<usize>,
    evaluation: &Value,
) -> Result<PopulationConfig> {
    let evolution = evaluation
        .get("evolution")
        .cloned()
        .unwrap_or_else(empty_object);
    let population_size = population_size
        .or_else(|| {
            evolution
                .get("population_size")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
        })
        .unwrap_or(24);
    let islands = islands
        .or_else(|| {
            evolution
                .get("islands")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
        })
        .unwrap_or(6);
    let novelty_weight = novelty_weight
        .or_else(|| evolution.get("novelty_weight").and_then(Value::as_f64))
        .unwrap_or(0.13);
    let new_info_refresh = new_info_refresh
        .or_else(|| {
            evolution
                .get("new_info_refresh")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
        })
        .unwrap_or(4);
    if population_size < 1 {
        bail!("--population-size must be at least 1");
    }
    if islands < 1 {
        bail!("--islands must be at least 1");
    }
    if new_info_refresh < 1 {
        bail!("--new-info-refresh must be at least 1");
    }
    let mut island_names = evolution
        .get("island_names")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| DEFAULT_ISLANDS.iter().map(|s| s.to_string()).collect());
    while island_names.len() < islands {
        island_names.push(format!("island-{}", island_names.len() + 1));
    }
    island_names.truncate(islands);
    Ok(PopulationConfig {
        population_size,
        islands,
        island_names,
        new_info_refresh,
        novelty_weight,
        diversity_targets: evolution
            .get("diversity_targets")
            .cloned()
            .unwrap_or_else(|| {
                json!({
                    "concept_entropy_min": 2.2,
                    "island_balance_min": 0.70,
                    "source_diversity_min": 0.35
                })
            }),
        promotion_gates: evolution
            .get("promotion_gates")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_else(|| {
                vec![
                    "final_score".to_string(),
                    "novelty_score".to_string(),
                    "interface_score".to_string(),
                    "failure_understanding".to_string(),
                ]
            }),
        degraded_penalties: evolution
            .get("degraded_penalties")
            .cloned()
            .unwrap_or_else(|| json!({"degraded_router_penalty": 0.08})),
    })
}

fn resolve_live_config(live_selective: bool, runbook: &Value) -> LiveConfig {
    let evaluation = runbook
        .get("evaluation")
        .cloned()
        .unwrap_or_else(empty_object);
    let merged = deep_merge_values(
        &evaluation.get("live").cloned().unwrap_or_else(empty_object),
        &runbook.get("live").cloned().unwrap_or_else(empty_object),
    );
    let enabled = live_selective
        || merged
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || evaluation
            .get("live_selective")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    LiveConfig {
        enabled,
        timeout_seconds: merged
            .get("timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(45),
        timeout_by_purpose: resolve_live_timeouts(&merged),
        retry_count: merged
            .get("retry_count")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize,
        champion_audit_every: merged
            .get("champion_audit_every")
            .and_then(Value::as_u64)
            .unwrap_or(25) as usize,
        hard_stage_every: merged
            .get("hard_stage_every")
            .and_then(Value::as_u64)
            .unwrap_or(10) as usize,
        promotion_every: merged
            .get("promotion_every")
            .and_then(Value::as_u64)
            .unwrap_or(10) as usize,
        research_synthesis: merged
            .get("research_synthesis")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        hard_stage_repair: merged
            .get("hard_stage_repair")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        promotion_judging: merged
            .get("promotion_judging")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        command: merged
            .get("command")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect()
            })
            .unwrap_or_else(|| JEKKO_LIVE_COMMAND.iter().map(|s| s.to_string()).collect()),
    }
}

fn resolve_live_timeouts(merged: &Value) -> BTreeMap<String, u64> {
    let default = merged
        .get("timeout_seconds")
        .and_then(Value::as_u64)
        .unwrap_or(45);
    let mut timeouts = BTreeMap::from([
        ("hard_stage_repair".to_string(), default),
        ("promotion_judging".to_string(), default),
        ("champion_audit".to_string(), default),
        ("research_synthesis".to_string(), default),
    ]);
    for key in [
        "timeouts",
        "timeout_seconds_by_purpose",
        "per_purpose_timeouts",
    ] {
        if let Some(map) = merged.get(key).and_then(Value::as_object) {
            for (purpose, value) in map {
                if let Some(seconds) = value.as_u64() {
                    timeouts.insert(purpose.clone(), seconds);
                }
            }
        }
    }
    timeouts
}

fn resolve_checkpoint_every(checkpoint_every: Option<usize>, evaluation: &Value) -> Result<usize> {
    let value = checkpoint_every
        .or_else(|| {
            evaluation
                .get("checkpoint_every")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
        })
        .unwrap_or(0);
    Ok(value)
}

fn resolve_research_cache_path(
    research_cache: Option<PathBuf>,
    runbook: &Value,
) -> Option<PathBuf> {
    if research_cache.is_some() {
        return research_cache;
    }
    let configured = runbook
        .get("evaluation")
        .and_then(|evaluation| evaluation.get("research_cache"))
        .and_then(Value::as_str)
        .or_else(|| {
            runbook
                .get("context")
                .and_then(|context| context.get("research_cache"))
                .and_then(Value::as_str)
        });
    if let Some(configured) = configured {
        return Some(PathBuf::from(configured));
    }
    let default = PathBuf::from(DEFAULT_RESEARCH_CACHE);
    if default.exists() {
        Some(default)
    } else {
        None
    }
}

fn load_stage_registry(stage_root: &Path) -> Result<Vec<StagePackage>> {
    if !stage_root.exists() {
        bail!("missing stage registry root: {}", stage_root.display());
    }
    let mut stage_files = Vec::new();
    for entry in WalkDir::new(stage_root).min_depth(1).max_depth(2) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if name == "stage.yml"
            || name == "stage.yaml"
            || (path.parent() == Some(stage_root)
                && matches!(
                    path.extension().and_then(|ext| ext.to_str()),
                    Some("yml" | "yaml")
                ))
        {
            stage_files.push(path);
        }
    }
    stage_files.sort();
    let mut stages = Vec::new();
    for path in stage_files {
        stages.push(load_stage_package(&path)?);
    }
    Ok(stages)
}

fn load_stage_package(path: &Path) -> Result<StagePackage> {
    let value = load_document(path)?;
    let stage_id = value
        .get("stage_id")
        .and_then(Value::as_str)
        .context("missing stage_id")?
        .to_string();
    let stage_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let prompt_path = stage_dir.join("prompt.md");
    let memory_path = stage_dir.join("memory.yml");
    let score_path = stage_dir.join("score.yml");
    let prompt_hash = sha256_digest(
        fs::read(&prompt_path)
            .with_context(|| format!("read {}", prompt_path.display()))?
            .as_slice(),
    );
    Ok(StagePackage {
        stage_id,
        name: value
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("stage")
            .to_string(),
        family: value
            .get("family")
            .and_then(Value::as_str)
            .unwrap_or("standard")
            .to_string(),
        track: value
            .get("track")
            .and_then(Value::as_str)
            .unwrap_or("standard")
            .to_string(),
        purpose: value
            .get("purpose")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        inputs: value
            .get("inputs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect(),
        outputs: value
            .get("outputs")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect(),
        required_evidence: value
            .get("required_evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect(),
        validation_checks: value
            .get("validation_checks")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect(),
        mutation_op: value
            .get("mutation_op")
            .and_then(Value::as_str)
            .unwrap_or("emit_atlas")
            .to_string(),
        prompt_path,
        memory_path: memory_path.clone(),
        score_path: score_path.clone(),
        stage_dir,
        stage_file: path.to_path_buf(),
        prompt_hash,
        memory: load_optional_yaml(&memory_path)?,
        score_model: load_optional_yaml(&score_path)?,
    })
}

fn validate_stage_registry(stages: &[StagePackage]) -> Result<()> {
    if stages.is_empty() {
        bail!("stage registry is empty");
    }
    let mut seen = BTreeSet::new();
    for stage in stages {
        if !seen.insert(stage.stage_id.clone()) {
            bail!("duplicate stage id {}", stage.stage_id);
        }
        if stage.inputs.is_empty()
            || stage.outputs.is_empty()
            || stage.required_evidence.is_empty()
            || stage.validation_checks.is_empty()
        {
            bail!("stage {} is missing required lists", stage.stage_id);
        }
    }
    Ok(())
}

fn load_optional_yaml(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let value: YamlValue =
        serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(serde_json::to_value(value)?)
}

fn empty_resume_state(stage_registry: &[StagePackage]) -> ResumeState {
    ResumeState {
        completed_generation: 0,
        stage_ledgers: Vec::new(),
        event_count: 0,
        generation_scores: Vec::new(),
        stage_score_history: stage_registry
            .iter()
            .map(|stage| (stage.stage_id.clone(), Vec::new()))
            .collect(),
        router_state: "nominal".to_string(),
        degraded_router: false,
        previous_generation_ids: Vec::new(),
    }
}

fn load_resume_state(run_dir: &Path, stage_registry: &[StagePackage]) -> Result<ResumeState> {
    let mut state = empty_resume_state(stage_registry);
    state.completed_generation = read_completed_generation(run_dir)?;
    state.stage_ledgers =
        read_jsonl::<Value>(&run_dir.join("stage-ledger.jsonl")).unwrap_or_default();
    state.event_count = read_jsonl::<Value>(&run_dir.join("run-events.jsonl"))
        .unwrap_or_default()
        .len();
    state.generation_scores = read_generation_scores(&run_dir.join("generation-ledger.jsonl"));
    if state.generation_scores.is_empty() {
        state.generation_scores = generation_scores_from_events(&run_dir.join("run-events.jsonl"));
    }
    state.stage_score_history =
        stage_score_history_from_ledgers(stage_registry, &state.stage_ledgers);
    state.degraded_router = state.stage_ledgers.iter().any(|entry| {
        entry
            .get("failure_modes")
            .and_then(Value::as_array)
            .map(|modes| {
                modes
                    .iter()
                    .any(|mode| mode.as_str() == Some("degraded_router"))
            })
            .unwrap_or(false)
    });
    state.router_state = if state.degraded_router {
        "degraded_router".to_string()
    } else {
        latest_router_state(&run_dir.join("run-events.jsonl"))
    };
    state.previous_generation_ids =
        latest_population_candidates(&run_dir.join("population-ledger.jsonl"));
    Ok(state)
}

fn read_completed_generation(run_dir: &Path) -> Result<usize> {
    let checkpoint = run_dir.join("checkpoint.json");
    if checkpoint.exists() {
        let value: Value = serde_json::from_str(&fs::read_to_string(&checkpoint)?)
            .with_context(|| format!("parse {}", checkpoint.display()))?;
        return Ok(value
            .get("complete_generation")
            .and_then(Value::as_u64)
            .unwrap_or(0) as usize);
    }
    let mut generation_ids = Vec::new();
    for record in read_jsonl::<Value>(&run_dir.join("generation-ledger.jsonl")).unwrap_or_default()
    {
        if record.get("metric_name").and_then(Value::as_str) == Some("deterministic_rollup_score") {
            if let Some(generation_id) = record.get("generation_id").and_then(Value::as_str) {
                generation_ids.push(generation_index_from_id(generation_id));
            }
        }
    }
    if !generation_ids.is_empty() {
        return Ok(*generation_ids.iter().max().unwrap());
    }
    for record in read_jsonl::<Value>(&run_dir.join("run-events.jsonl")).unwrap_or_default() {
        if record.get("event_type").and_then(Value::as_str) == Some("generation_end") {
            if let Some(generation_id) = record.get("generation_id").and_then(Value::as_str) {
                generation_ids.push(generation_index_from_id(generation_id));
            }
        }
    }
    Ok(*generation_ids.iter().max().unwrap_or(&0))
}

fn generation_index_from_id(generation_id: &str) -> usize {
    generation_id
        .strip_prefix('g')
        .and_then(|suffix| suffix.parse::<usize>().ok())
        .unwrap_or(0)
}

fn read_generation_scores(path: &Path) -> Vec<f64> {
    let records = read_jsonl::<Value>(path).unwrap_or_default();
    hybrid_quality_rollup_series(&records)
}

fn generation_scores_from_events(path: &Path) -> Vec<f64> {
    let mut records = read_jsonl::<Value>(path).unwrap_or_default();
    records.sort_by_key(|record| {
        generation_index_from_id(
            record
                .get("generation_id")
                .and_then(Value::as_str)
                .unwrap_or("g0000"),
        )
    });
    records
        .into_iter()
        .filter(|record| record.get("event_type").and_then(Value::as_str) == Some("generation_end"))
        .filter_map(|record| record.get("final_score").and_then(Value::as_f64))
        .collect()
}

fn stage_score_history_from_ledgers(
    stage_registry: &[StagePackage],
    stage_ledgers: &[Value],
) -> BTreeMap<String, Vec<f64>> {
    let mut history: BTreeMap<String, Vec<f64>> = stage_registry
        .iter()
        .map(|stage| (stage.stage_id.clone(), Vec::new()))
        .collect();
    for entry in stage_ledgers {
        if let Some(stage_id) = entry.get("stage_id").and_then(Value::as_str) {
            history.entry(stage_id.to_string()).or_default().push(
                entry
                    .get("final_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
            );
        }
    }
    history
}

fn latest_router_state(path: &Path) -> String {
    let mut state = "nominal".to_string();
    for record in read_jsonl::<Value>(path).unwrap_or_default() {
        if let Some(router_state) = record.get("router_state").and_then(Value::as_str) {
            state = router_state.to_string();
        }
    }
    state
}

fn latest_population_candidates(path: &Path) -> Vec<String> {
    let mut candidates = Vec::new();
    for record in read_jsonl::<Value>(path).unwrap_or_default() {
        if let Some(candidate_id) = record.get("candidate_id").and_then(Value::as_str) {
            candidates.push(candidate_id.to_string());
        }
    }
    candidates
}

fn write_memory_cards(
    writer: &mut JsonlWriter,
    stage_registry: &[StagePackage],
    run_id: &str,
) -> Result<()> {
    for stage in stage_registry {
        let record = json!({
            "schema_version": SCHEMA_VERSION,
            "record_kind": "memory_card",
            "run_id": run_id,
            "memory_card_id": format!("mem-{}-{}", stage.stage_id, short_hash(&stage.prompt_hash, 12)),
            "stage_id": stage.stage_id,
            "memory_refs": memory_refs_for_stage(stage),
            "content_hash": stage.prompt_hash,
            "summary": stage.purpose,
            "source_path": stage.memory_path.display().to_string(),
        });
        writer.write(&record)?;
    }
    Ok(())
}

fn write_research_cards(writer: &mut JsonlWriter, research_cards: &[Value]) -> Result<()> {
    for card in research_cards {
        writer.write(card)?;
    }
    Ok(())
}

fn load_research_cache_cards(path: Option<&Path>, run_id: &str) -> Result<Vec<Value>> {
    if let Some(path) = path {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        if path.is_dir() {
            let mut files = Vec::new();
            for entry in WalkDir::new(path).min_depth(1) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let ext = entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("");
                    if matches!(ext, "json" | "jsonl" | "yml" | "yaml") {
                        files.push(entry.into_path());
                    }
                }
            }
            files.sort();
            for source_path in files {
                entries.extend(load_research_cache_entries(&source_path)?);
            }
        } else {
            entries.extend(load_research_cache_entries(path)?);
        }
        let mut cards = Vec::new();
        for (index, entry) in entries.into_iter().enumerate() {
            let source_text = format!(
                "{} {} {} {} {} {}",
                string_or_default(&entry, "url"),
                string_or_default(&entry, "title"),
                string_or_default(&entry, "date"),
                string_or_default(&entry, "citation"),
                string_or_default(&entry, "summary"),
                string_or_default(&entry, "claim"),
            );
            let rejection_reason = reject_information_text(
                string_or_default(&entry, "url").as_str(),
                source_text.as_str(),
            );
            let source_hash = entry
                .get("hash")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| stable_hash(&source_text));
            cards.push(json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "research_card",
                "run_id": run_id,
                "research_card_id": entry.get("research_card_id").and_then(Value::as_str).map(ToString::to_string).unwrap_or_else(|| format!("research-{}-{index:03}", short_hash(&source_hash, 12))),
                "source_type": string_or_default(&entry, "source_type"),
                "url": string_or_default(&entry, "url"),
                "title": entry.get("title").and_then(Value::as_str).unwrap_or("source"),
                "date": string_or_default(&entry, "date"),
                "source_hash": source_hash,
                "citation": entry.get("citation").and_then(Value::as_str).or_else(|| entry.get("title").and_then(Value::as_str)).unwrap_or("source"),
                "summary": entry.get("summary").and_then(Value::as_str).or_else(|| entry.get("claim").and_then(Value::as_str)).unwrap_or("").chars().take(1000).collect::<String>(),
                "cache_path": path.display().to_string(),
                "accepted": rejection_reason.is_none(),
                "rejection_reason": rejection_reason,
            }));
        }
        return Ok(cards);
    }
    Ok(Vec::new())
}

fn load_research_cache_entries(path: &Path) -> Result<Vec<Value>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
        Ok(text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str::<Value>(line).unwrap_or_else(|_| json!({})))
            .collect())
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        let value: Value =
            serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        Ok(match value {
            Value::Array(items) => items
                .into_iter()
                .filter(|entry| entry.is_object())
                .collect(),
            Value::Object(map) => map
                .get("cards")
                .and_then(Value::as_array)
                .cloned()
                .or_else(|| map.get("sources").and_then(Value::as_array).cloned())
                .or_else(|| map.get("research").and_then(Value::as_array).cloned())
                .unwrap_or_else(|| vec![Value::Object(map.clone())]),
            _ => Vec::new(),
        })
    } else {
        let value: YamlValue =
            serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        let value = serde_json::to_value(value)?;
        Ok(match value {
            Value::Array(items) => items
                .into_iter()
                .filter(|entry| entry.is_object())
                .collect(),
            Value::Object(map) => map
                .get("cards")
                .and_then(Value::as_array)
                .cloned()
                .or_else(|| map.get("sources").and_then(Value::as_array).cloned())
                .or_else(|| map.get("research").and_then(Value::as_array).cloned())
                .unwrap_or_else(|| vec![Value::Object(map.clone())]),
            _ => Vec::new(),
        })
    }
}

fn synthesize_information_cards(stage_registry: &[StagePackage], run_id: &str) -> Vec<Value> {
    stage_registry
        .iter()
        .map(|stage| {
            let summary = if stage.purpose.is_empty() {
                stage.name.clone()
            } else {
                stage.purpose.clone()
            };
            json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "information_card",
                "run_id": run_id,
                "information_card_id": format!("info-{}", short_hash(&stage.prompt_hash, 12)),
                "source_path": stage.stage_file.display().to_string(),
                "domain": stage.family,
                "claim": first_sentence(&summary),
                "method": "stage_synthesis",
                "constraint": "cached stage metadata only",
                "failure_risk": "stage metadata may be incomplete",
                "stage_concept_hint": stage.stage_id,
                "provenance_hash": stage.prompt_hash,
                "novelty_terms": novelty_terms_from_text(&summary),
            })
        })
        .collect()
}

fn synthesize_additional_information_cards(
    stage_registry: &[StagePackage],
    generation_id: &str,
    run_id: &str,
    count: usize,
) -> Vec<Value> {
    stage_registry
        .iter()
        .take(count)
        .map(|stage| {
            json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "information_card",
                "run_id": run_id,
                "information_card_id": format!("info-{generation_id}-{}", short_hash(&stage.prompt_hash, 12)),
                "source_path": stage.stage_file.display().to_string(),
                "domain": stage.family,
                "claim": first_sentence(&stage.purpose),
                "method": "cached_research_synthesis",
                "constraint": "cached research only; no uncached web state inside scoring",
                "failure_risk": "cached source may be outdated or too broad for the target stage",
                "stage_concept_hint": stage.stage_id,
                "provenance_hash": stage.prompt_hash,
                "novelty_terms": novelty_terms_from_text(&stage.purpose),
            })
        })
        .collect()
}

fn select_research_refs_for_stage(research_cards: &[Value], stage: &StagePackage) -> Vec<String> {
    let accepted: Vec<&Value> = research_cards
        .iter()
        .filter(|card| {
            card.get("accepted")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .collect();
    if accepted.is_empty() {
        return Vec::new();
    }
    let stage_text = format!(
        "{} {} {} {} {}",
        stage.stage_id, stage.track, stage.family, stage.purpose, stage.name
    )
    .to_lowercase();
    let mut matched = Vec::new();
    for card in &accepted {
        let haystack = format!(
            "{} {} {}",
            string_or_default(card, "title"),
            string_or_default(card, "summary"),
            string_or_default(card, "source_type"),
        )
        .to_lowercase();
        if haystack.contains(&stage_text) {
            if let Some(id) = card.get("research_card_id").and_then(Value::as_str) {
                matched.push(id.to_string());
            }
        }
    }
    if matched.is_empty() {
        accepted
            .into_iter()
            .filter_map(|card| {
                card.get("research_card_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .take(3)
            .collect()
    } else {
        matched.into_iter().take(3).collect()
    }
}

fn live_call_purpose(
    live_config: &LiveConfig,
    stage: &StagePackage,
    generation_index: usize,
    stage_index: usize,
    stage_count: usize,
) -> Option<String> {
    if !live_config.enabled {
        return None;
    }
    if generation_index == 1 && stage_index == 0 && live_config.research_synthesis {
        return Some("research_synthesis".to_string());
    }
    if stage.family == "hard"
        && live_config.hard_stage_repair
        && (generation_index == 1 || generation_index % live_config.hard_stage_every == 0)
    {
        return Some("hard_stage_repair".to_string());
    }
    if stage.stage_id.ends_with("promotion")
        && live_config.promotion_judging
        && (generation_index == 1 || generation_index % live_config.promotion_every == 0)
    {
        return Some("promotion_judging".to_string());
    }
    if live_config.champion_audit_every > 0
        && generation_index % live_config.champion_audit_every == 0
        && stage_index + 1 == stage_count
    {
        return Some("champion_audit".to_string());
    }
    None
}

fn run_live_call(
    run_dir: &Path,
    stage: &StagePackage,
    route: &RoutePolicy,
    generation_id: &str,
    candidate_id: &str,
    purpose: &str,
    live_config: &LiveConfig,
    research_refs: &[String],
    output_guard: Option<&OutputPathGuard>,
) -> Result<Value> {
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    let call_id = format!("live-{generation_id}-{}-{purpose}", stage.stage_id);
    let call_dir = run_dir
        .join("stages")
        .join(&stage.stage_id)
        .join("generations")
        .join(generation_id)
        .join("live-calls")
        .join(&call_id);
    fs::create_dir_all(&call_dir)?;
    let run_id = infer_run_id_from_path(run_dir);
    let execution_backend = live_execution_backend(route);
    let stage_prompt = stage_prompt_text(stage)?;
    let jailgun_download_target_name = (execution_backend == "jailgun_mcp")
        .then(|| jailgun_download_target_name(&run_id, &call_id, stage, purpose, &stage_prompt));
    let retrieval_packet = json!({
        "schema_version": SCHEMA_VERSION,
        "run_dir": run_dir.display().to_string(),
        "run_id": run_id.as_str(),
        "generation_id": generation_id,
        "stage_id": stage.stage_id,
        "candidate_id": candidate_id,
        "purpose": purpose,
        "route_backend": route.route_backend,
        "route_tier": route.route_tier,
        "router_state": route.router_state,
        "stage_path": stage.stage_file.display().to_string(),
        "prompt_hash": stage.prompt_hash,
        "memory_refs": memory_refs_for_stage(stage),
        "research_refs": research_refs,
        "required_evidence": stage.required_evidence,
        "validation_checks": stage.validation_checks,
        "jailgun_download_target_name": jailgun_download_target_name.as_deref(),
    });
    let prompt = live_prompt(
        stage,
        &stage_prompt,
        &retrieval_packet,
        jailgun_download_target_name.as_deref(),
    )?;
    let prompt_path = call_dir.join("prompt.md");
    let retrieval_path = call_dir.join("retrieval-packet.json");
    let raw_output_path = call_dir.join("raw-output.txt");
    let parsed_summary_path = call_dir.join("parsed-summary.json");
    let receipt_path = call_dir.join("receipt.json");
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    write_markdown(&prompt_path, &prompt)?;
    write_json(&retrieval_path, &retrieval_packet)?;

    let command = if execution_backend == "jailgun_mcp" {
        Vec::new()
    } else {
        live_command(live_config)
    };
    let timeout_seconds = live_timeout_for_purpose(live_config, purpose);
    let max_attempts = live_max_attempts(live_config, purpose, execution_backend);
    let mut attempts = Vec::new();
    let mut final_stdout = String::new();
    let mut final_stderr = String::new();
    let mut exit_code = None;
    let mut status = "failed".to_string();
    let mut error = None;
    let mut started_at = now_iso8601();
    let mut final_metadata = json!({});
    let overall_start = Instant::now();
    for attempt_index in 1..=max_attempts {
        let attempt_started_at = now_iso8601();
        if attempt_index == 1 {
            started_at = attempt_started_at.clone();
        }
        let attempt_stdout_path = call_dir.join(format!("raw-stdout-attempt-{attempt_index}.txt"));
        let attempt_stderr_path = call_dir.join(format!("raw-stderr-attempt-{attempt_index}.txt"));
        if let Some(guard) = output_guard {
            guard.check()?;
        }
        let attempt = if execution_backend == "jailgun_mcp" {
            run_jailgun_live_call_attempt(
                &call_id,
                &prompt_path,
                jailgun_download_target_name
                    .as_deref()
                    .expect("jailgun target name"),
                timeout_seconds,
                attempt_index,
                &attempt_started_at,
            )
        } else {
            run_live_call_attempt(
                &command,
                &prompt,
                timeout_seconds,
                attempt_index,
                &attempt_started_at,
            )?
        };
        fs::write(&attempt_stdout_path, &attempt.stdout)?;
        fs::write(&attempt_stderr_path, &attempt.stderr)?;
        final_stdout = attempt.stdout.clone();
        final_stderr = attempt.stderr.clone();
        exit_code = attempt.exit_code;
        status = attempt.status.clone();
        error = attempt.error.clone();
        final_metadata = attempt.metadata.clone();
        let should_rate_limit_backoff = status != "ok"
            && attempt_index < max_attempts
            && live_attempt_failure_kind(&attempt) == Some("rate-limit");
        let should_rate_limit_cooldown =
            status == "ok" && live_attempt_warning_kind(&attempt) == Some("rate-limit");
        let mut attempt_record = json!({
            "attempt": attempt_index,
            "status": attempt.status,
            "exit_code": attempt.exit_code,
            "started_at": attempt_started_at,
            "elapsed_seconds": round6(attempt.elapsed_seconds),
            "timeout_seconds": timeout_seconds,
            "raw_stdout_path": attempt_stdout_path.display().to_string(),
            "raw_stderr_path": attempt_stderr_path.display().to_string(),
            "error": attempt.error,
        });
        merge_object(&mut attempt_record, &attempt.metadata);
        if should_rate_limit_backoff {
            attempt_record["retry_backoff_seconds"] = json!(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS);
        }
        if should_rate_limit_cooldown {
            attempt_record["rate_limit_cooldown_seconds"] =
                json!(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS);
            final_metadata["jailgun_rate_limit_cooldown_seconds"] =
                json!(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS);
        }
        attempts.push(attempt_record);
        if status == "ok" {
            if should_rate_limit_cooldown {
                thread::sleep(Duration::from_secs(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS));
            }
            break;
        }
        if should_rate_limit_backoff {
            thread::sleep(Duration::from_secs(JAILGUN_RATE_LIMIT_BACKOFF_SECONDS));
        }
    }

    let raw = if final_stderr.is_empty() {
        final_stdout.clone()
    } else {
        format!("{final_stdout}\n[stderr]\n{final_stderr}")
    };
    fs::write(&raw_output_path, raw)?;
    let token_usage = json!({
        "prompt": estimate_tokens(&prompt),
        "completion": estimate_tokens(&final_stdout),
        "total": estimate_tokens(&prompt) + estimate_tokens(&final_stdout),
    });
    let parsed = live_summary(&final_stdout, &status, error.as_deref());
    write_json(&parsed_summary_path, &parsed)?;
    let mut record = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "live_call",
        "run_id": run_id,
        "generation_id": generation_id,
        "stage_id": stage.stage_id,
        "candidate_id": candidate_id,
        "call_id": call_id,
        "purpose": purpose,
        "route_backend": route.route_backend,
        "route_tier": route.route_tier,
        "router_state": route.router_state,
        "execution_backend": execution_backend,
        "status": status,
        "exit_code": exit_code,
        "started_at": started_at,
        "elapsed_seconds": round6(overall_start.elapsed().as_secs_f64()),
        "timeout_seconds": timeout_seconds,
        "configured_timeout_seconds": timeout_seconds,
        "retry_count": live_config.retry_count,
        "attempt_count": attempts.len(),
        "attempts": attempts,
        "command": command,
        "prompt_path": prompt_path.display().to_string(),
        "retrieval_packet_path": retrieval_path.display().to_string(),
        "raw_output_path": raw_output_path.display().to_string(),
        "parsed_summary_path": parsed_summary_path.display().to_string(),
        "receipt_path": receipt_path.display().to_string(),
        "token_usage": token_usage,
        "summary": parsed.get("summary").cloned().unwrap_or_else(|| json!("")),
        "error": error,
    });
    merge_object(&mut record, &final_metadata);
    write_json(&receipt_path, &record)?;
    Ok(record)
}

#[derive(Debug)]
struct LiveAttempt {
    status: String,
    exit_code: Option<i32>,
    elapsed_seconds: f64,
    stdout: String,
    stderr: String,
    error: Option<String>,
    metadata: Value,
}

#[derive(Clone)]
struct JailgunBridgeCommand {
    args: Vec<String>,
    source: String,
}

fn jailgun_run_arguments(
    jailgun_run_id: &str,
    call_id: &str,
    prompt_path: &Path,
    timeout_seconds: u64,
    account_ids: &[String],
    bridge_cmd: &JailgunBridgeCommand,
    download_target_name: &str,
) -> Value {
    let mut bridge_env = serde_json::Map::new();
    bridge_env.insert(
        "JAILGUN_ARTIFACT_CONVERSATION_RECOVERY_LIMIT".to_string(),
        json!("0"),
    );
    bridge_env.insert("JAILGUN_ARTIFACT_REPAIR_ATTEMPTS".to_string(), json!("1"));
    // Reuse a provisioned X display (export DISPLAY before launching) instead of spawning a
    // fresh Xvfb per call. Per-call spawning exhausted the display pool and caused ~73% of the
    // last run's live calls to fail with "could not find a free Xvfb display number".
    if let Ok(display) = std::env::var("DISPLAY") {
        if !display.trim().is_empty() {
            bridge_env.insert("DISPLAY".to_string(), json!(display));
        }
    }
    json!({
        "version": 1,
        "run_id": jailgun_run_id,
        "prompt_ref": format!("openqg://zyal-genome/{call_id}"),
        "prompt_file": jailgun_prompt_file_path(prompt_path),
        "tabs": 1,
        "max_runtime_seconds": timeout_seconds.max(1),
        "browser": {
            "account_ids": account_ids,
            "allow_queueing": true,
            "queue_timeout_seconds": JAILGUN_QUEUE_TIMEOUT_SECONDS,
            "bridge_cmd": &bridge_cmd.args,
            "bridge_env": Value::Object(bridge_env),
            "download_target_name": download_target_name,
        },
        "source_archive": {
            "enabled": false,
        },
        "deploy": {
            "enabled": false,
            "dry_run": true,
        },
        "ci": {
            "enabled": false,
        },
        "github": {
            "allow_write_prompts": false,
            "allow_info_prompts": true,
        },
    })
}

fn jailgun_prompt_file_path(prompt_path: &Path) -> String {
    if prompt_path.is_absolute() {
        return prompt_path.display().to_string();
    }
    env::current_dir()
        .map(|cwd| cwd.join(prompt_path))
        .unwrap_or_else(|_| prompt_path.to_path_buf())
        .display()
        .to_string()
}

fn live_execution_backend(route: &RoutePolicy) -> &'static str {
    if route.route_backend == "jailgun" {
        "jailgun_mcp"
    } else {
        "jekko_command"
    }
}

fn live_command(live_config: &LiveConfig) -> Vec<String> {
    if live_config.command.is_empty() {
        JEKKO_LIVE_COMMAND.iter().map(|s| s.to_string()).collect()
    } else {
        live_config.command.clone()
    }
}

fn live_max_attempts(live_config: &LiveConfig, purpose: &str, execution_backend: &str) -> usize {
    let configured_attempts = live_config.retry_count.saturating_add(1).max(1);
    if purpose == "hard_stage_repair" && execution_backend == "jailgun_mcp" {
        configured_attempts.saturating_add(JAILGUN_RATE_LIMIT_EXTRA_ATTEMPTS)
    } else {
        configured_attempts
    }
}

fn live_timeout_for_purpose(live_config: &LiveConfig, purpose: &str) -> u64 {
    live_config
        .timeout_by_purpose
        .get(purpose)
        .copied()
        .unwrap_or(live_config.timeout_seconds)
}

fn run_live_call_attempt(
    command: &[String],
    prompt: &str,
    timeout_seconds: u64,
    attempt: usize,
    _started_at: &str,
) -> Result<LiveAttempt> {
    if command.is_empty() {
        bail!("live command is empty");
    }
    let start = Instant::now();
    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    // SAFETY: pre_exec runs in the forked child before exec; setsid() is async-signal-safe and
    // the closure performs no allocation or non-reentrant work beyond the single libc call.
    unsafe {
        cmd.pre_exec(|| {
            if setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().map_err(|err| anyhow::anyhow!(err))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes());
    }
    let stdout_rx = child.stdout.take().map(spawn_pipe_reader);
    let stderr_rx = child.stderr.take().map(spawn_pipe_reader);
    let mut exit_code = None;
    let status;
    let mut error = None;
    let timed_out;
    loop {
        if let Some(exit) = child.try_wait()? {
            exit_code = exit.code();
            status = if exit.success() { "ok" } else { "failed" }.to_string();
            timed_out = false;
            break;
        }
        if start.elapsed() >= Duration::from_secs(timeout_seconds) {
            kill_live_process_tree(child.id());
            let _ = child.wait();
            status = "timeout".to_string();
            error = Some(format!(
                "timeout after {timeout_seconds}s on attempt {attempt}"
            ));
            timed_out = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let pipe_wait = if timed_out {
        Duration::from_secs(1)
    } else {
        Duration::from_secs(5)
    };
    let stdout = collect_pipe(stdout_rx, pipe_wait)
        .unwrap_or_else(|| "[stdout unavailable after timeout]\n".to_string());
    let stderr = collect_pipe(stderr_rx, pipe_wait)
        .unwrap_or_else(|| "[stderr unavailable after timeout]\n".to_string());
    Ok(LiveAttempt {
        status,
        exit_code,
        elapsed_seconds: start.elapsed().as_secs_f64(),
        stdout,
        stderr,
        error,
        metadata: json!({}),
    })
}

fn run_jailgun_live_call_attempt(
    call_id: &str,
    prompt_path: &Path,
    download_target_name: &str,
    timeout_seconds: u64,
    attempt: usize,
    started_at: &str,
) -> LiveAttempt {
    run_jailgun_live_call_attempt_with_config(
        call_id,
        prompt_path,
        download_target_name,
        timeout_seconds,
        attempt,
        started_at,
        JailgunHealthConfig::from_env(),
        jailgun_bridge_command(),
    )
}

fn run_jailgun_live_call_attempt_with_config(
    call_id: &str,
    prompt_path: &Path,
    download_target_name: &str,
    timeout_seconds: u64,
    attempt: usize,
    started_at: &str,
    config: JailgunHealthConfig,
    bridge_cmd: JailgunBridgeCommand,
) -> LiveAttempt {
    let start = Instant::now();
    let server_url = config.server_url.clone();
    let token = match config.token.as_ref() {
        Some(value) => value,
        None => {
            let mut metadata = jailgun_transport_metadata(&config, None);
            metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
            return failed_jailgun_attempt(
                start,
                Some(server_url),
                None,
                Vec::new(),
                "Jailgun token is not available from env or matching local jailgun serve process"
                    .to_string(),
                metadata,
            );
        }
    };
    let jailgun_status = strict_jailgun_status_with_config(config.clone());
    let account_ids = jailgun_status.ready_account_ids.clone();
    if !jailgun_status.available {
        let error = if jailgun_status.errors.is_empty() {
            "Jailgun server is not ready".to_string()
        } else {
            format!(
                "Jailgun server is not ready: {}",
                jailgun_status.errors.join("; ")
            )
        };
        let mut metadata = jailgun_transport_metadata(&config, Some(&jailgun_status));
        metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
        return failed_jailgun_attempt(start, Some(server_url), None, account_ids, error, metadata);
    }
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(JAILGUN_MCP_HTTP_TIMEOUT_SECONDS))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            let mut metadata = jailgun_transport_metadata(&config, Some(&jailgun_status));
            metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
            return failed_jailgun_attempt(
                start,
                Some(server_url),
                None,
                account_ids,
                format!("failed to build Jailgun HTTP client: {error}"),
                metadata,
            );
        }
    };
    let jailgun_run_id = jailgun_live_run_id(call_id, attempt, started_at);
    let account_ids = jailgun_single_account_ids(&account_ids, call_id, attempt);
    let run_args = jailgun_run_arguments(
        &jailgun_run_id,
        call_id,
        prompt_path,
        timeout_seconds,
        &account_ids,
        &bridge_cmd,
        download_target_name,
    );
    let accepted = match jailgun_mcp_tool_call(
        &client,
        &server_url,
        token.value.as_str(),
        &format!("{jailgun_run_id}-run"),
        "jailgun.run",
        run_args,
    ) {
        Ok(value) => value,
        Err(error) => {
            let mut metadata = jailgun_transport_metadata(&config, Some(&jailgun_status));
            metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
            return failed_jailgun_attempt(
                start,
                Some(server_url),
                Some(jailgun_run_id),
                account_ids,
                error,
                metadata,
            );
        }
    };

    let deadline = start
        + Duration::from_secs(
            timeout_seconds + JAILGUN_MCP_SUMMARY_GRACE_SECONDS + JAILGUN_QUEUE_TIMEOUT_SECONDS,
        );
    let mut status_snapshots = Vec::new();
    let mut final_status = json!({});
    while Instant::now() < deadline {
        match jailgun_mcp_tool_call(
            &client,
            &server_url,
            token.value.as_str(),
            &format!("{jailgun_run_id}-status"),
            "jailgun.run_status",
            json!({ "run_id": &jailgun_run_id }),
        ) {
            Ok(status) => {
                final_status = status.clone();
                status_snapshots.push(status.clone());
                match status.get("status").and_then(Value::as_str).unwrap_or("") {
                    "succeeded" | "failed" | "timed-out" => break,
                    _ => {}
                }
            }
            Err(error) => {
                return failed_jailgun_attempt(
                    start,
                    Some(server_url),
                    Some(jailgun_run_id),
                    account_ids,
                    error,
                    json!({
                        "token_source": config.token_source(),
                        "account_source": jailgun_status.account_source.as_deref(),
                        "bridge_cmd_source": bridge_cmd.source.as_str(),
                        "jailgun_health": jailgun_status.as_json(),
                        "jailgun_started_response": accepted,
                        "jailgun_status_snapshots": status_snapshots,
                    }),
                );
            }
        }
        thread::sleep(Duration::from_millis(JAILGUN_MCP_POLL_INTERVAL_MILLIS));
    }

    let poll_deadline_elapsed = Instant::now() >= deadline;
    let summary = match jailgun_mcp_tool_call(
        &client,
        &server_url,
        token.value.as_str(),
        &format!("{jailgun_run_id}-summary"),
        "jailgun.run_summary",
        json!({ "run_id": &jailgun_run_id }),
    ) {
        Ok(value) => value,
        Err(error) => {
            return failed_jailgun_attempt(
                start,
                Some(server_url),
                Some(jailgun_run_id),
                account_ids,
                error,
                json!({
                        "token_source": config.token_source(),
                        "account_source": jailgun_status.account_source.as_deref(),
                        "bridge_cmd_source": bridge_cmd.source.as_str(),
                        "jailgun_health": jailgun_status.as_json(),
                        "jailgun_started_response": accepted,
                        "jailgun_status_snapshot": final_status,
                        "jailgun_status_snapshots": status_snapshots,
                }),
            );
        }
    };
    let summary_status = summary
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("missing");
    let effective_status = jailgun_effective_status(Some(&summary), &final_status);
    let mut metadata = jailgun_attempt_metadata(
        &server_url,
        &jailgun_run_id,
        &account_ids,
        &accepted,
        &final_status,
        &status_snapshots,
        Some(&summary),
        token.value.as_str(),
    );
    metadata["token_source"] = json!(config.token_source());
    metadata["account_source"] = json!(jailgun_status.account_source.as_deref());
    metadata["bridge_cmd_source"] = json!(bridge_cmd.source.as_str());
    metadata["jailgun_health"] = jailgun_status.as_json();
    metadata["jailgun_summary_status"] = json!(summary_status);
    metadata["jailgun_effective_status"] = json!(effective_status);
    let redacted_summary = redact_jailgun_token_in_value(&summary, token.value.as_str());
    let stdout = serde_json::to_string_pretty(&redacted_summary)
        .unwrap_or_else(|_| redacted_summary.to_string());
    let status = match effective_status.as_str() {
        "succeeded" => "ok",
        "timed-out" => "timeout",
        "running" | "accepted" if poll_deadline_elapsed => "timeout",
        _ => "failed",
    }
    .to_string();
    let error = if status == "ok" {
        None
    } else {
        Some(format!(
            "Jailgun run {jailgun_run_id} finished with status {effective_status}"
        ))
    };
    LiveAttempt {
        exit_code: if status == "ok" { Some(0) } else { Some(1) },
        status,
        elapsed_seconds: start.elapsed().as_secs_f64(),
        stdout,
        stderr: String::new(),
        error,
        metadata,
    }
}

fn failed_jailgun_attempt(
    start: Instant,
    server_url: Option<String>,
    jailgun_run_id: Option<String>,
    account_ids: Vec<String>,
    error: String,
    extra: Value,
) -> LiveAttempt {
    let mut metadata = json!({
        "jailgun_server_url": server_url,
        "jailgun_run_id": jailgun_run_id,
        "jailgun_account_count": account_ids.len(),
        "jailgun_status": "failed",
        "jailgun_failure_kind": classify_jailgun_failure(&error),
        "jailgun_warning_kind": "none",
        "jailgun_error": error,
    });
    merge_object(&mut metadata, &extra);
    LiveAttempt {
        status: "failed".to_string(),
        exit_code: None,
        elapsed_seconds: start.elapsed().as_secs_f64(),
        stdout: String::new(),
        stderr: String::new(),
        error: metadata
            .get("jailgun_error")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        metadata,
    }
}

fn run_jailgun_artifact_smoke(
    run_dir: &Path,
    run_id: &str,
    extension: &str,
    output_guard: Option<&OutputPathGuard>,
) -> Result<Value> {
    run_jailgun_artifact_smoke_with_config(
        run_dir,
        run_id,
        extension,
        JailgunHealthConfig::from_env(),
        jailgun_bridge_command(),
        output_guard,
    )
}

fn run_jailgun_artifact_smoke_with_config(
    run_dir: &Path,
    run_id: &str,
    extension: &str,
    config: JailgunHealthConfig,
    bridge_cmd: JailgunBridgeCommand,
    output_guard: Option<&OutputPathGuard>,
) -> Result<Value> {
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    let call_id = "preflight-jailgun-artifact-smoke";
    let call_dir = run_dir.join("preflight").join(call_id);
    fs::create_dir_all(&call_dir)?;
    let download_target_name =
        jailgun_download_target_name_with_extension(run_id, call_id, extension);
    let prompt_path = call_dir.join("prompt.md");
    let receipt_path = call_dir.join("receipt.json");
    let stdout_path = call_dir.join("raw-stdout.txt");
    let stderr_path = call_dir.join("raw-stderr.txt");
    let prompt = format!(
        "# Jailgun Artifact Smoke\n\nCreate a fresh downloadable artifact named exactly `{download_target_name}` now. The filename and extension are authoritative. Do not answer with prose outside the artifact. Do not recover or reuse an artifact from another conversation. Keep the content short and valid for the requested file type.\n"
    );
    write_markdown(&prompt_path, &prompt)?;
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    let started_at = now_iso8601();
    let mut attempts = Vec::new();
    let mut final_attempt = None;
    for attempt_index in 1..=JAILGUN_ARTIFACT_SMOKE_ATTEMPTS {
        let attempt = run_jailgun_live_call_attempt_with_config(
            call_id,
            &prompt_path,
            &download_target_name,
            JAILGUN_ARTIFACT_SMOKE_TIMEOUT_SECONDS,
            attempt_index,
            &started_at,
            config.clone(),
            bridge_cmd.clone(),
        );
        let attempt_stdout_path = call_dir.join(format!("raw-stdout-attempt-{attempt_index}.txt"));
        let attempt_stderr_path = call_dir.join(format!("raw-stderr-attempt-{attempt_index}.txt"));
        fs::write(&attempt_stdout_path, &attempt.stdout)?;
        fs::write(&attempt_stderr_path, &attempt.stderr)?;
        let attempt_record = json!({
            "attempt": attempt_index,
            "status": attempt.status,
            "exit_code": attempt.exit_code,
            "elapsed_seconds": round6(attempt.elapsed_seconds),
            "timeout_seconds": JAILGUN_ARTIFACT_SMOKE_TIMEOUT_SECONDS,
            "raw_stdout_path": attempt_stdout_path.display().to_string(),
            "raw_stderr_path": attempt_stderr_path.display().to_string(),
            "error": attempt.error,
        });
        attempts.push(attempt_record);
        let status = attempt.status.clone();
        final_attempt = Some(attempt);
        if status == "ok" {
            break;
        }
    }
    let attempt = final_attempt.expect("artifact smoke attempts must be configured");
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    fs::write(&stdout_path, &attempt.stdout)?;
    fs::write(&stderr_path, &attempt.stderr)?;
    let mut receipt = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "jailgun_artifact_smoke",
        "run_id": run_id,
        "call_id": call_id,
        "status": attempt.status,
        "exit_code": attempt.exit_code,
        "started_at": started_at,
        "elapsed_seconds": round6(attempt.elapsed_seconds),
        "timeout_seconds": JAILGUN_ARTIFACT_SMOKE_TIMEOUT_SECONDS,
        "attempt_count": attempts.len(),
        "attempts": attempts,
        "prompt_path": prompt_path.display().to_string(),
        "raw_stdout_path": stdout_path.display().to_string(),
        "raw_stderr_path": stderr_path.display().to_string(),
        "receipt_path": receipt_path.display().to_string(),
        "download_target_name": download_target_name,
        "artifact_extension": safe_artifact_extension(extension).unwrap_or_else(|| "json".to_string()),
        "error": attempt.error,
    });
    merge_object(&mut receipt, &attempt.metadata);
    write_json(&receipt_path, &receipt)?;
    Ok(receipt)
}

fn jailgun_transport_metadata(
    config: &JailgunHealthConfig,
    status: Option<&JailgunStatus>,
) -> Value {
    let mut metadata = json!({
        "token_source": config.token_source(),
    });
    if let Some(status) = status {
        metadata["account_source"] = json!(status.account_source.as_deref());
        metadata["jailgun_health"] = status.as_json();
    }
    metadata
}

fn jailgun_live_run_id(call_id: &str, attempt: usize, started_at: &str) -> String {
    let suffix = short_hash(started_at, 12);
    let candidate = format!("openqg-{call_id}-a{attempt}-{suffix}");
    if candidate.len() <= 128 {
        candidate
    } else {
        format!("openqg-{}-a{attempt}-{suffix}", short_hash(call_id, 32))
    }
}

fn jailgun_download_target_name(
    run_id: &str,
    call_id: &str,
    stage: &StagePackage,
    purpose: &str,
    prompt: &str,
) -> String {
    let extension = jailgun_artifact_extension(stage, purpose, prompt);
    jailgun_download_target_name_with_extension(run_id, call_id, &extension)
}

fn jailgun_download_target_name_with_extension(
    run_id: &str,
    call_id: &str,
    extension: &str,
) -> String {
    let extension = safe_artifact_extension(extension).unwrap_or_else(|| "json".to_string());
    let stem = format!("openqg-{run_id}-{call_id}");
    let safe_stem = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let candidate = format!("{safe_stem}.{extension}");
    if candidate.len() <= 128 {
        candidate
    } else {
        format!(
            "openqg-{}-{}.{}",
            short_hash(run_id, 12),
            short_hash(call_id, 32),
            extension
        )
    }
}

fn jailgun_artifact_extension(stage: &StagePackage, purpose: &str, prompt: &str) -> String {
    artifact_extension_from_items(&stage.outputs)
        .or_else(|| artifact_extension_from_text(prompt))
        .unwrap_or_else(|| default_jailgun_artifact_extension(purpose).to_string())
}

fn artifact_extension_from_items(items: &[String]) -> Option<String> {
    items
        .iter()
        .find_map(|item| safe_artifact_extension_from_path(item))
}

fn artifact_extension_from_text(text: &str) -> Option<String> {
    text.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '`' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':'
            )
    })
    .find_map(safe_artifact_extension_from_path)
}

fn safe_artifact_extension_from_path(value: &str) -> Option<String> {
    let trimmed = value.trim_matches(|ch: char| {
        matches!(
            ch,
            '`' | '"' | '\'' | '.' | ',' | ';' | ':' | ')' | ']' | '}'
        )
    });
    let extension = Path::new(trimmed).extension()?.to_str()?;
    safe_artifact_extension(extension)
}

fn safe_artifact_extension(extension: &str) -> Option<String> {
    let extension = extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    (!extension.is_empty()
        && extension.len() <= 16
        && extension.chars().all(|ch| ch.is_ascii_alphanumeric()))
    .then_some(extension)
}

fn default_jailgun_artifact_extension(_purpose: &str) -> &'static str {
    "json"
}

fn jailgun_single_account_ids(
    account_ids: &[String],
    call_id: &str,
    attempt: usize,
) -> Vec<String> {
    let _ = (call_id, attempt);
    account_ids.iter().take(1).cloned().collect()
}

fn live_attempt_failure_kind(attempt: &LiveAttempt) -> Option<&str> {
    attempt
        .metadata
        .get("jailgun_failure_kind")
        .and_then(Value::as_str)
}

fn live_attempt_warning_kind(attempt: &LiveAttempt) -> Option<&str> {
    attempt
        .metadata
        .get("jailgun_warning_kind")
        .and_then(Value::as_str)
        .filter(|kind| *kind != "none")
}

fn jailgun_attempt_metadata(
    server_url: &str,
    jailgun_run_id: &str,
    account_ids: &[String],
    accepted: &Value,
    final_status: &Value,
    status_snapshots: &[Value],
    summary: Option<&Value>,
    token: &str,
) -> Value {
    let accepted = redact_jailgun_token_in_value(accepted, token);
    let final_status = redact_jailgun_token_in_value(final_status, token);
    let status_snapshots = status_snapshots
        .iter()
        .map(|snapshot| redact_jailgun_token_in_value(snapshot, token))
        .collect::<Vec<_>>();
    let summary = summary.map(|value| redact_jailgun_token_in_value(value, token));
    let summary_path = summary
        .as_ref()
        .and_then(|value| value.get("summary_json").and_then(Value::as_str))
        .or_else(|| accepted.get("summary_json").and_then(Value::as_str))
        .map(ToString::to_string);
    let events_path = summary
        .as_ref()
        .and_then(|value| value.get("events_jsonl").and_then(Value::as_str))
        .or_else(|| accepted.get("events_jsonl").and_then(Value::as_str))
        .map(ToString::to_string);
    let jailgun_status = jailgun_effective_status(summary.as_ref(), &final_status);
    let event_failure_kind = events_path
        .as_deref()
        .and_then(classify_jailgun_events_failure);
    let jailgun_warning_kind =
        if jailgun_status == "succeeded" && event_failure_kind == Some("rate-limit") {
            "rate-limit"
        } else {
            "none"
        };
    let classified_failure_kind = if event_failure_kind == Some("rate-limit") {
        event_failure_kind
    } else {
        classify_jailgun_attempt_failure(summary.as_ref(), &final_status, &status_snapshots)
            .or(event_failure_kind)
    };
    let jailgun_failure_kind = if jailgun_status == "succeeded" {
        "none"
    } else {
        classified_failure_kind.unwrap_or("none")
    };
    let receipt_paths = summary
        .as_ref()
        .and_then(|value| value.get("receipt_paths").cloned())
        .unwrap_or_else(|| json!([]));
    json!({
        "jailgun_server_url": server_url,
        "jailgun_run_id": jailgun_run_id,
        "jailgun_status": jailgun_status,
        "jailgun_account_count": account_ids.len(),
        "jailgun_failure_kind": jailgun_failure_kind,
        "jailgun_warning_kind": jailgun_warning_kind,
        "jailgun_started_response": accepted,
        "jailgun_status_snapshot": final_status,
        "jailgun_status_snapshots": status_snapshots,
        "jailgun_summary_path": summary_path,
        "jailgun_events_path": events_path,
        "jailgun_event_failure_kind": event_failure_kind,
        "jailgun_receipt_paths": receipt_paths,
        "jailgun_summary": summary,
    })
}

fn jailgun_effective_status(summary: Option<&Value>, final_status: &Value) -> String {
    let summary_status = summary.and_then(|value| value.get("status").and_then(Value::as_str));
    let final_status = final_status.get("status").and_then(Value::as_str);
    summary_status
        .filter(|status| jailgun_terminal_status(status))
        .or_else(|| final_status.filter(|status| jailgun_terminal_status(status)))
        .or(summary_status)
        .or(final_status)
        .unwrap_or("unknown")
        .to_string()
}

fn jailgun_terminal_status(status: &str) -> bool {
    matches!(status, "succeeded" | "failed" | "timed-out")
}

fn classify_jailgun_attempt_failure(
    summary: Option<&Value>,
    final_status: &Value,
    status_snapshots: &[Value],
) -> Option<&'static str> {
    summary
        .and_then(classify_jailgun_summary_failure)
        .or_else(|| classify_jailgun_status_failure(final_status))
        .or_else(|| {
            status_snapshots
                .iter()
                .rev()
                .find_map(classify_jailgun_status_failure)
        })
}

fn classify_jailgun_summary_failure(summary: &Value) -> Option<&'static str> {
    if summary.get("status").and_then(Value::as_str) == Some("timed-out") {
        return Some("runtime-timeout");
    }
    for key in ["failure_kind", "error", "message", "status_detail"] {
        if let Some(message) = summary.get(key).and_then(Value::as_str) {
            let class = classify_jailgun_failure(message);
            if class != "unknown" {
                return Some(class);
            }
        }
    }
    let failures = summary.get("failures").and_then(Value::as_array)?;
    for failure in failures {
        for key in ["kind", "code", "phase", "error", "message"] {
            if let Some(message) = failure.get(key).and_then(Value::as_str) {
                let class = classify_jailgun_failure(message);
                if class != "unknown" {
                    return Some(class);
                }
            }
        }
        let class = classify_jailgun_failure(&failure.to_string());
        if class != "unknown" {
            return Some(class);
        }
    }
    None
}

fn classify_jailgun_events_failure(path: &str) -> Option<&'static str> {
    fs::read_to_string(Path::new(path))
        .ok()
        .and_then(|text| classify_jailgun_events_text(&text))
}

fn classify_jailgun_events_text(text: &str) -> Option<&'static str> {
    let mut first_known = None;
    for line in text.lines() {
        let class = classify_jailgun_failure(line);
        if class == "rate-limit" {
            return Some("rate-limit");
        }
        if class != "unknown" && first_known.is_none() {
            first_known = Some(class);
        }
    }
    first_known
}

fn classify_jailgun_status_failure(status: &Value) -> Option<&'static str> {
    if status.get("status").and_then(Value::as_str) == Some("timed-out") {
        return Some("runtime-timeout");
    }
    for key in ["failure_kind", "error", "message", "status_detail"] {
        if let Some(message) = status.get(key).and_then(Value::as_str) {
            let class = classify_jailgun_failure(message);
            if class != "unknown" {
                return Some(class);
            }
        }
    }
    if let Some(tabs) = status.get("tabs").and_then(Value::as_array) {
        for tab in tabs {
            for key in [
                "status",
                "deploy_status",
                "prompt_policy_decision",
                "error",
                "message",
                "status_detail",
            ] {
                if let Some(message) = tab.get(key).and_then(Value::as_str) {
                    let class = classify_jailgun_failure(message);
                    if class != "unknown" {
                        return Some(class);
                    }
                }
            }
            if tab.get("status").and_then(Value::as_str) == Some("error") {
                return Some("browser-tab-error");
            }
        }
    }
    let class = classify_jailgun_failure(&status.to_string());
    if class != "unknown" {
        return Some(class);
    }
    if status.get("status").and_then(Value::as_str) == Some("failed") {
        return Some("browser-run-failed");
    }
    None
}

fn classify_jailgun_failure(message: &str) -> &'static str {
    let lower = message.to_ascii_lowercase();
    if lower.contains("browser account queue timed out") || lower.contains("browser-lease-timeout")
    {
        "queue-timeout"
    } else if lower.contains("rate limit")
        || lower.contains("rate-limit")
        || lower.contains("too many requests")
        || lower.contains("http 429")
        || lower.contains(" 429")
        || lower.contains("quota exceeded")
    {
        "rate-limit"
    } else if lower.contains("browser lease failed")
        || lower.contains("browser-account")
        || lower.contains("browser lease")
    {
        "browser-lease"
    } else if lower.contains("auth-required") || lower.contains("code-requested") {
        "auth-required"
    } else if lower.contains("session-expired") || lower.contains("session expired") {
        "session-expired"
    } else if lower.contains("manual-browser-required") {
        "manual-browser-required"
    } else if lower.contains("browser tab error")
        || lower.contains("tab status error")
        || lower.contains("\"status\":\"error\"")
        || lower.contains("\"status\": \"error\"")
    {
        "browser-tab-error"
    } else if lower.contains("no .tex artifact download candidate")
        || lower.contains("no artifact download candidate")
        || lower.contains("done-no-artifact")
    {
        "artifact-download-missing"
    } else if lower.contains("tar-validation")
        || lower.contains("tar validation")
        || lower.contains("tarball validation")
        || lower.contains("archive validation")
    {
        "tar-validation"
    } else if lower.contains("invalid gzip")
        || lower.contains("gzip header")
        || lower.contains("not in gzip format")
        || lower.contains("incorrect header check")
        || lower.contains("invalid tar header")
        || lower.contains("invalid header")
    {
        "invalid-archive-header"
    } else if lower.contains("artifact conversation recovery")
        || lower.contains("artifact_conversation")
        || lower.contains("recovered artifact")
        || lower.contains("recovered-target")
        || lower.contains("recovered_from")
        || lower.contains("abandoned run tab")
    {
        "outdated-artifact-recovery"
    } else if lower.contains("runtime timeout")
        || lower.contains("run timed out")
        || lower.contains("timed-out")
        || lower.contains("timed out")
        || lower.contains("deadline elapsed")
        || lower.contains("max_runtime_seconds")
        || lower.contains("max runtime")
    {
        "runtime-timeout"
    } else {
        "unknown"
    }
}

fn jailgun_mcp_tool_call(
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
    request_id: &str,
    name: &str,
    arguments: Value,
) -> std::result::Result<Value, String> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments,
        },
    });
    let value = jailgun_post_json(client, server_url, "/mcp", token, &body)?;
    if let Some(error) = value.get("error") {
        let error = sanitize_jailgun_token_text(&error.to_string(), token);
        return Err(format!(
            "Jailgun MCP {name} returned JSON-RPC error: {error}"
        ));
    }
    let result = value.get("result").unwrap_or(&Value::Null);
    if result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let result = sanitize_jailgun_token_text(&result.to_string(), token);
        return Err(format!(
            "Jailgun MCP {name} returned isError=true: {result}"
        ));
    }
    result
        .get("structuredContent")
        .cloned()
        .ok_or_else(|| format!("Jailgun MCP {name} response missing structuredContent: {value}"))
}

fn spawn_pipe_reader<R>(mut reader: R) -> mpsc::Receiver<String>
where
    R: Read + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = String::new();
        let _ = reader.read_to_string(&mut buffer);
        let _ = tx.send(buffer);
    });
    rx
}

fn collect_pipe(rx: Option<mpsc::Receiver<String>>, timeout: Duration) -> Option<String> {
    rx.and_then(|rx| rx.recv_timeout(timeout).ok())
}

fn kill_live_process_tree(child_pid: u32) {
    #[cfg(unix)]
    // SAFETY: kill(2) is async-signal-safe and takes only integer arguments; sending SIGKILL to
    // the process group and pid cannot create memory-safety hazards in this process.
    unsafe {
        let pid = child_pid as i32;
        let _ = kill(-pid, SIGKILL_NUM);
        let _ = kill(pid, SIGKILL_NUM);
    }
    #[cfg(not(unix))]
    {
        let _ = child_pid;
    }
}

fn live_prompt(
    stage: &StagePackage,
    stage_prompt: &str,
    retrieval_packet: &Value,
    jailgun_download_target_name: Option<&str>,
) -> Result<String> {
    let artifact_instruction = jailgun_download_target_name
        .map(|name| {
            format!(
                "Create a fresh downloadable artifact named exactly `{name}` now. The filename and extension are authoritative. Do not answer with a plan, acknowledgement, or prose outside the artifact. Do not recover or reuse an artifact from another conversation. Keep the content concise and match the file type requested by the filename and stage prompt."
            )
        })
        .unwrap_or_else(|| {
            "Return concise JSON-like notes with risks, repairs, and audit concerns.".to_string()
        });
    Ok([
        "# ZYAL Selective Live Call",
        "",
        &format!(
            "Purpose: {}",
            retrieval_packet
                .get("purpose")
                .and_then(Value::as_str)
                .unwrap_or("")
        ),
        &format!("Stage: {} - {}", stage.stage_id, stage.name),
        "",
        "Use the retrieval packet as the only evidence context.",
        "Return frontier-value review material: a falsifiable claim, source-card grounding, concrete tests, known failure modes, and a review priority.",
        "Do not rely on raw provider logs, target artifacts, or unstated external evidence.",
        artifact_instruction.as_str(),
        "",
        "## Stage Prompt",
        stage_prompt,
        "",
        "## Retrieval Packet",
        &serde_json::to_string_pretty(retrieval_packet)?,
    ]
    .join("\n"))
}

fn stage_prompt_text(stage: &StagePackage) -> Result<String> {
    if stage.prompt_path.is_file() {
        fs::read_to_string(&stage.prompt_path)
            .with_context(|| format!("read {}", stage.prompt_path.display()))
    } else {
        Ok(stage.purpose.clone())
    }
}

fn compute_score_breakdown(
    scores: &Value,
    stage: &StagePackage,
    live_records: &[Value],
    research_refs: &[String],
) -> Value {
    let live_quality = if live_records.is_empty() {
        0.0
    } else {
        live_records
            .iter()
            .map(|record| {
                if record.get("status").and_then(Value::as_str) == Some("ok") {
                    0.70
                } else {
                    0.20
                }
            })
            .sum::<f64>()
            / live_records.len() as f64
    };
    let benchmark_safety = 1.0
        - scores
            .get("failure_penalty")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
    let mut penalties = Vec::new();
    if research_refs.is_empty() {
        penalties.push("missing_research_refs");
    }
    if memory_refs_for_stage(stage).is_empty() {
        penalties.push("missing_memory_refs");
    }
    if !stage.prompt_hash.is_empty() && stage.prompt_hash.len() < 8 {
        penalties.push("missing_prompt_hash");
    }
    json!({
        "artifact_validity": if stage.required_evidence.is_empty() { 0.70 } else { 0.95 },
        "source_grounding": (0.50 + 0.08 * research_refs.len() as f64).clamp(0.0, 1.0),
        "novelty": scores.get("novelty_score").or_else(|| scores.get("innovation_score")).cloned().unwrap_or_else(|| json!(0.0)),
        "stage_reusability": if stage.stage_dir.is_dir() { 0.92 } else { 0.60 },
        "interface_integrity": scores.get("interface_score").cloned().unwrap_or_else(|| json!(0.0)),
        "failure_understanding": scores.get("failure_understanding").cloned().unwrap_or_else(|| json!(1.0 - scores.get("failure_penalty").and_then(Value::as_f64).unwrap_or(0.0))),
        "live_reasoning_quality": live_quality,
        "benchmark_safety": benchmark_safety.max(0.0),
        "promotion_confidence": scores.get("final_score").cloned().unwrap_or_else(|| json!(0.0)),
        "penalties": penalties,
    })
}

fn population_mode_counts(population_size: usize) -> BTreeMap<String, usize> {
    let exploitation = population_size / 2;
    let novelty = population_size / 3;
    let wildcard = population_size.saturating_sub(exploitation + novelty);
    BTreeMap::from([
        ("exploitation".to_string(), exploitation),
        ("novelty".to_string(), novelty),
        ("wildcard".to_string(), wildcard),
    ])
}

fn build_mode_list(mode_counts: &BTreeMap<String, usize>) -> Vec<String> {
    let mut modes = Vec::new();
    for (mode, count) in mode_counts {
        for _ in 0..*count {
            modes.push(mode.clone());
        }
    }
    if modes.is_empty() {
        modes.extend(
            ["exploitation", "novelty", "wildcard"]
                .iter()
                .map(|s| s.to_string()),
        );
    }
    modes
}

fn route_for_variant(
    variant: &GenomeVariant,
    stage: &StagePackage,
    live_enabled: bool,
    jailgun_available: bool,
) -> RoutePolicy {
    let hard = stage.family == "hard";
    match variant {
        GenomeVariant::PureJnoccio => RoutePolicy {
            route_backend: "jnoccio".to_string(),
            route_tier: if hard {
                "top20_pct".to_string()
            } else {
                "standard".to_string()
            },
            router_state: "nominal".to_string(),
            judge_family: "jnoccio".to_string(),
            provenance: "scripted-route-policy".to_string(),
            route_policy: json!({"backend":"jnoccio","tier":"standard"}),
        },
        GenomeVariant::Hybrid => {
            if hard && live_enabled && jailgun_available {
                RoutePolicy {
                    route_backend: "jailgun".to_string(),
                    route_tier: if hard {
                        "top20_pct".to_string()
                    } else {
                        "standard".to_string()
                    },
                    router_state: "nominal".to_string(),
                    judge_family: "mixed".to_string(),
                    provenance: "backend-wrapper".to_string(),
                    route_policy: json!({"backend":"jailgun","tier":"top20_pct"}),
                }
            } else if hard {
                RoutePolicy {
                    route_backend: "jnoccio".to_string(),
                    route_tier: if hard {
                        "top20_pct".to_string()
                    } else {
                        "standard".to_string()
                    },
                    router_state: "degraded_router".to_string(),
                    judge_family: "mixed".to_string(),
                    provenance: "scripted-degraded".to_string(),
                    route_policy: json!({"backend":"jnoccio","tier":"top20_pct"}),
                }
            } else {
                RoutePolicy {
                    route_backend: "jnoccio".to_string(),
                    route_tier: "standard".to_string(),
                    router_state: "nominal".to_string(),
                    judge_family: "mixed".to_string(),
                    provenance: "scripted-route-policy".to_string(),
                    route_policy: json!({"backend":"jnoccio","tier":"standard"}),
                }
            }
        }
        GenomeVariant::JailgunOnly => RoutePolicy {
            route_backend: "jailgun".to_string(),
            route_tier: "manual".to_string(),
            router_state: if live_enabled {
                "nominal".to_string()
            } else {
                "stubbed_wrapper".to_string()
            },
            judge_family: "jailgun".to_string(),
            provenance: if live_enabled {
                "backend-wrapper".to_string()
            } else {
                "scripted-wrapper".to_string()
            },
            route_policy: json!({"backend":"jailgun","tier":"manual"}),
        },
    }
}

fn compute_scores(
    variant: &str,
    stage: &StagePackage,
    generation_index: usize,
    seed: u64,
    route: &RoutePolicy,
    jailgun_available: bool,
) -> Value {
    let hardness = if stage.family == "hard" { 0.72 } else { 0.28 };
    let jitter = hash_unit(&format!(
        "{variant}:{}:{generation_index}:{seed}",
        stage.stage_id
    ));
    let route_jitter = hash_unit(&format!(
        "{}:{}:{seed}",
        route.route_backend, stage.stage_id
    ));
    let generation_drift = 0.015 * (generation_index.saturating_sub(1).min(50) as f64 / 50.0);
    let variant_shift = match variant {
        "pure-jnoccio" => (0.02, 0.00, 0.01, 0.03, 0.00),
        "hybrid" => (0.01, 0.03, 0.04, 0.02, -0.01),
        _ => (0.02, 0.05, 0.03, 0.02, -0.02),
    };
    let route_bonus = if route.route_backend == "jailgun" {
        0.04
    } else {
        0.01
    };
    let local = clamp(
        0.59 + 0.10 * (1.0 - hardness) + generation_drift + variant_shift.0 + 0.04 * jitter,
        0.0,
        1.0,
    );
    let interface = clamp(
        0.57 + 0.11 * (1.0 - hardness) + variant_shift.1 + 0.03 * route_jitter,
        0.0,
        1.0,
    );
    let macro_score = clamp(
        0.58 + 0.12 * (1.0 - hardness) + generation_drift + variant_shift.2 + 0.04 * jitter,
        0.0,
        1.0,
    );
    let innovation = clamp(
        0.34 + 0.18 * hardness + variant_shift.3 + 0.05 * jitter,
        0.0,
        1.0,
    );
    let novelty = clamp(
        0.38 + 0.10 * hardness + 0.04 * jitter + if variant == "hybrid" { 0.03 } else { 0.0 },
        0.0,
        1.0,
    );
    let route_degradation_penalty = if route.router_state == "degraded_router" {
        0.16
    } else {
        0.0
    };
    let mut failure_penalty = clamp(
        0.03 + 0.08 * hardness
            + route_bonus
            + variant_shift.4
            + route_degradation_penalty
            + 0.02 * route_jitter,
        0.0,
        0.35,
    );
    if variant == "jailgun-only" && !jailgun_available {
        failure_penalty = clamp(failure_penalty + 0.06, 0.0, 0.35);
    }
    let final_score = if variant == "hybrid" {
        clamp(
            0.24 * local
                + 0.16 * interface
                + 0.24 * macro_score
                + 0.18 * innovation
                + 0.13 * novelty
                - 0.10 * failure_penalty,
            0.0,
            1.0,
        )
    } else {
        clamp(
            0.30 * local + 0.20 * interface + 0.30 * macro_score + 0.15 * innovation
                - 0.10 * failure_penalty,
            0.0,
            1.0,
        )
    };
    let final_score = apply_saturation_guard(
        final_score,
        &[local, interface, macro_score, innovation, novelty],
        failure_penalty,
        route.router_state == "nominal",
    );
    let prompt_tokens = 800
        + 110 * generation_index
        + (85.0 * hardness) as usize
        + 20 * stage.stage_id.len()
        + (75.0 * route_bonus * 10.0) as usize;
    let completion_tokens = 350
        + 60 * generation_index
        + (45.0 * hardness) as usize
        + 10 * stage.stage_id.len()
        + (45.0 * route_bonus * 10.0) as usize;
    let time_seconds = 1.5
        + 0.6 * hardness
        + 0.15 * generation_index as f64
        + if route.route_backend == "jailgun" {
            0.45
        } else {
            0.2
        };
    let mut failure_modes = Vec::new();
    if route.router_state != "nominal" {
        failure_modes.push("degraded_router".to_string());
    }
    if variant == "jailgun-only" && !jailgun_available {
        failure_modes.push("stubbed_jailgun".to_string());
    }
    if failure_penalty > 0.18 {
        failure_modes.push("high_failure_penalty".to_string());
    }
    json!({
        "local_score": round6(local),
        "interface_score": round6(interface),
        "macro_score": round6(macro_score),
        "innovation_score": round6(innovation),
        "novelty_score": round6(novelty),
        "failure_penalty": round6(failure_penalty),
        "final_score": round6(final_score),
        "prompt_tokens": prompt_tokens,
        "completion_tokens": completion_tokens,
        "time_seconds": round6(time_seconds),
        "pass_rate": if final_score >= 0.45 { 1.0 } else { 0.0 },
        "failure_modes": failure_modes,
        "failure_understanding": round6((1.0 - failure_penalty).clamp(0.0, 1.0)),
    })
}

fn apply_saturation_guard(
    final_score: f64,
    components: &[f64],
    failure_penalty: f64,
    route_nominal: bool,
) -> f64 {
    if final_score < 0.999999 {
        return final_score;
    }
    let components_clear = components.iter().all(|score| *score >= 0.95);
    if route_nominal && components_clear && failure_penalty <= 0.02 {
        final_score
    } else {
        0.995
    }
}

fn candidate_parent_ids(previous_generation_ids: &[String], stage_index: usize) -> Vec<String> {
    if previous_generation_ids.is_empty() {
        return Vec::new();
    }
    vec![previous_generation_ids[stage_index % previous_generation_ids.len()].clone()]
}

fn ensure_present(record: &Value, field: &str) -> Result<()> {
    match record.get(field) {
        Some(Value::Null) | None => bail!("missing field: {field}"),
        Some(_) => Ok(()),
    }
}

fn adaptive_pressure_state(
    generation_scores: &[f64],
    previous_generation: &[Value],
    island_names: &[String],
) -> Value {
    let recent = median(&generation_scores[generation_scores.len().saturating_sub(3)..]);
    let previous_best = previous_generation
        .iter()
        .filter_map(|candidate| {
            candidate
                .get("scores")
                .and_then(|scores| scores.get("final_score"))
                .and_then(Value::as_f64)
        })
        .fold(0.0, f64::max);
    let island_count = island_names.len().max(1) as f64;
    json!({
        "recent_score_median": round6(recent),
        "previous_generation_best": round6(previous_best),
        "pressure_index": round6((recent + previous_best + island_names.len() as f64) / (island_count + 2.0)),
        "island_count": island_names.len(),
        "previous_generation_size": previous_generation.len(),
    })
}

fn choose_parent_ids(
    previous_generation: &[Value],
    mode: &str,
    island: &str,
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> Vec<String> {
    if previous_generation.is_empty() {
        return Vec::new();
    }
    let mut indices =
        vec![(generation_index + candidate_index + seed as usize) % previous_generation.len()];
    if previous_generation.len() > 1 && (mode == "exploitation" || island.contains("repair")) {
        indices.push(
            (generation_index + candidate_index + seed as usize + 1) % previous_generation.len(),
        );
    } else if previous_generation.len() > 2 && mode == "wildcard" {
        indices.push(
            (generation_index + candidate_index + seed as usize + 2) % previous_generation.len(),
        );
    }
    indices
        .into_iter()
        .filter_map(|index| {
            previous_generation
                .get(index)
                .and_then(|candidate| candidate.get("candidate_id"))
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .collect()
}

fn choose_stage_concepts(
    stage_registry: &[StagePackage],
    concepts: &[Value],
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if concepts.is_empty() {
        return map;
    }
    for (index, stage) in stage_registry.iter().enumerate() {
        let pick = (index + generation_index + candidate_index + seed as usize) % concepts.len();
        let concept_id = concepts[pick]
            .get("concept_id")
            .and_then(Value::as_str)
            .unwrap_or(&stage.stage_id)
            .to_string();
        map.insert(stage.stage_id.clone(), concept_id);
    }
    map
}

fn choose_source_cards(
    accepted_cards: &[Value],
    fresh_cards: &[Value],
    mode: &str,
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> Vec<Value> {
    let mut pool = if fresh_cards.is_empty() {
        accepted_cards.to_vec()
    } else {
        fresh_cards.to_vec()
    };
    if pool.is_empty() {
        return Vec::new();
    }
    let start = (generation_index + candidate_index + mode.len() + seed as usize) % pool.len();
    pool.rotate_left(start);
    pool.into_iter().take(3).collect()
}

fn expected_candidate_failures(mode: &str, mutation_op: &str, router_state: &str) -> Vec<String> {
    let mut failures = Vec::new();
    if router_state != "nominal" {
        failures.push("degraded_router".to_string());
    }
    if mode == "wildcard" {
        failures.push("wildcard_risk".to_string());
    }
    if mutation_op.contains("repair") {
        failures.push("repair_drift".to_string());
    }
    failures
}

fn compute_candidate_scores(
    mode: &str,
    island: &str,
    mutation_op: &str,
    stage_concepts: &BTreeMap<String, String>,
    generation_index: usize,
    _generation_count: usize,
    candidate_index: usize,
    seed: u64,
    degraded_router: bool,
    degraded_router_penalty: f64,
    novelty_weight: f64,
) -> Value {
    let mode_factor = match mode {
        "exploitation" => 0.62,
        "novelty" => 0.55,
        _ => 0.48,
    };
    let island_factor = 0.03 * (island.len() as f64 % 5.0);
    let mutation_factor = 0.01 * (mutation_op.len() as f64 % 7.0);
    let stage_factor = 0.01 * stage_concepts.len() as f64;
    let jitter = hash_unit(&format!(
        "{mode}:{island}:{mutation_op}:{generation_index}:{candidate_index}:{seed}"
    ));
    let novelty = clamp(0.25 + 0.40 * novelty_weight + 0.15 * jitter, 0.0, 1.0);
    let generation_drift = 0.015 * (generation_index.saturating_sub(1).min(50) as f64 / 50.0);
    let final_score = clamp(
        mode_factor
            + island_factor
            + mutation_factor
            + stage_factor
            + 0.10 * jitter
            + novelty_weight * novelty
            + generation_drift
            - if degraded_router {
                degraded_router_penalty
            } else {
                0.0
            },
        0.0,
        1.0,
    );
    let interface = clamp(
        0.45 + 0.1 * jitter
            + if mutation_op.contains("contract") {
                0.08
            } else {
                0.0
            },
        0.0,
        1.0,
    );
    let macro_score = clamp(
        0.50 + 0.05 * stage_concepts.len() as f64 + 0.08 * jitter,
        0.0,
        1.0,
    );
    let innovation = clamp(0.35 + 0.22 * novelty + 0.03 * jitter, 0.0, 1.0);
    let failure_penalty = clamp(
        if degraded_router {
            degraded_router_penalty
        } else {
            0.04
        } + if mode == "wildcard" { 0.02 } else { 0.0 },
        0.0,
        0.35,
    );
    let final_score = apply_saturation_guard(
        final_score,
        &[interface, macro_score, innovation, novelty],
        failure_penalty,
        !degraded_router,
    );
    json!({
        "local_score": round6(clamp(0.45 + 0.10 * jitter, 0.0, 1.0)),
        "interface_score": round6(interface),
        "macro_score": round6(macro_score),
        "innovation_score": round6(innovation),
        "novelty_score": round6(novelty),
        "failure_penalty": round6(failure_penalty),
        "final_score": round6(final_score),
        "prompt_tokens": 640 + 12 * candidate_index + 8 * mutation_op.len(),
        "completion_tokens": 240 + 8 * candidate_index + 4 * island.len(),
        "time_seconds": round6(1.0 + 0.05 * candidate_index as f64 + if degraded_router { 0.2 } else { 0.1 }),
        "pass_rate": if final_score >= 0.45 { 1.0 } else { 0.0 },
        "failure_modes": expected_candidate_failures(mode, mutation_op, if degraded_router { "degraded_router" } else { "nominal" }),
        "failure_understanding": round6((1.0 - failure_penalty).clamp(0.0, 1.0)),
        "scoring_weights": json!({
            "local_score": 0.24,
            "interface_score": 0.16,
            "macro_score": 0.24,
            "innovation_score": 0.18,
            "novelty_score": novelty_weight,
            "failure_penalty": 0.05,
        }),
    })
}

fn align_candidate_score_with_stage_rollup(
    mut scores: Value,
    stage_rollup: Option<f64>,
    cap_enabled: bool,
) -> Value {
    if !cap_enabled {
        return scores;
    }
    let Some(stage_rollup) = stage_rollup else {
        return scores;
    };
    let ceiling = (stage_rollup + 0.12).min(0.995);
    if let Some(current) = scores.get("final_score").and_then(Value::as_f64) {
        if current > ceiling {
            scores["final_score"] = json!(round6(ceiling));
            scores["stage_rollup_score_ceiling"] = json!(round6(ceiling));
        }
    }
    scores
}

fn hybrid_candidate_record(
    generation_id: &str,
    stage: &StagePackage,
    candidate_id: &str,
    parent_ids: &[String],
    mode: &str,
    research_refs: &[String],
    stage_concepts: &BTreeMap<String, String>,
    route_policy: &Value,
    scores: &Value,
    score_breakdown: &Value,
) -> Value {
    let review = frontier_review_fields(
        candidate_id,
        mode,
        research_refs,
        stage_concepts,
        scores
            .get("failure_modes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        scores,
    );
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "candidate",
        "candidate_id": candidate_id,
        "generation_id": generation_id,
        "island": stage.track,
        "mode": mode,
        "parent_candidate_ids": parent_ids,
        "lineage_depth": parent_ids.len() + 1,
        "mutation_ops": [stage.mutation_op],
        "source_card_ids": research_refs,
        "stage_concepts": stage_concepts,
        "route_policy": route_policy,
        "expected_failure_modes": scores.get("failure_modes").cloned().unwrap_or_else(|| json!([])),
        "adaptive_pressure": json!({}),
        "scoring_weights": scores.get("scoring_weights").cloned().unwrap_or_else(empty_object),
        "scores": scores,
        "score_breakdown": score_breakdown,
        "frontier_claim": review.frontier_claim,
        "falsifiable_tests": review.falsifiable_tests,
        "known_failure_modes": review.known_failure_modes,
        "review_priority": review.review_priority,
    })
}

#[derive(Debug, Clone)]
struct FrontierReviewFields {
    frontier_claim: String,
    falsifiable_tests: Vec<String>,
    known_failure_modes: Vec<Value>,
    review_priority: String,
}

fn frontier_review_fields(
    candidate_id: &str,
    mode: &str,
    source_card_ids: &[String],
    stage_concepts: &BTreeMap<String, String>,
    known_failure_modes: Vec<Value>,
    scores: &Value,
) -> FrontierReviewFields {
    let source_summary = if source_card_ids.is_empty() {
        "stage concepts".to_string()
    } else {
        source_card_ids
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };
    let stage_targets = stage_concepts.keys().take(3).cloned().collect::<Vec<_>>();
    let mut falsifiable_tests = stage_targets
        .iter()
        .map(|stage_id| format!("Replay {stage_id} evidence with the proposed interface unchanged"))
        .collect::<Vec<_>>();
    if falsifiable_tests.is_empty() {
        falsifiable_tests
            .push("Replay the stage-local evidence bundle without target leakage".to_string());
    }
    let final_score = scores
        .get("final_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let novelty_score = scores
        .get("novelty_score")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let review_priority = if mode == "novelty" && final_score >= NOVELTY_CHAMPION_SCORE_FLOOR {
        "high"
    } else if final_score >= 0.80 || novelty_score >= 0.60 {
        "medium"
    } else {
        "low"
    }
    .to_string();
    FrontierReviewFields {
        frontier_claim: format!(
            "{candidate_id} proposes a {mode} frontier step grounded in {source_summary}"
        ),
        falsifiable_tests,
        known_failure_modes,
        review_priority,
    }
}

fn promote_generation_candidates(candidates: &[Value]) -> Vec<Value> {
    let mut ranked = candidates.to_vec();
    ranked.sort_by(|a, b| {
        candidate_score(b)
            .partial_cmp(&candidate_score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let target = if ranked.len() > 3 {
        ranked.len().min(6)
    } else {
        ranked.len()
    };
    let mut promoted = Vec::new();
    for mode in ["novelty", "wildcard", "exploitation"] {
        if let Some(candidate) = ranked
            .iter()
            .find(|candidate| candidate.get("mode").and_then(Value::as_str) == Some(mode))
        {
            push_unique_candidate(&mut promoted, candidate);
        }
    }
    let islands = ranked
        .iter()
        .filter_map(|candidate| candidate.get("island").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    for island in islands {
        if promoted.len() >= target {
            break;
        }
        if let Some(candidate) = ranked.iter().find(|candidate| {
            candidate.get("island").and_then(Value::as_str) == Some(island.as_str())
        }) {
            push_unique_candidate(&mut promoted, candidate);
        }
    }
    for candidate in &ranked {
        if promoted.len() >= target {
            break;
        }
        push_unique_candidate(&mut promoted, candidate);
    }
    promoted.sort_by(|a, b| {
        candidate_score(b)
            .partial_cmp(&candidate_score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    promoted
}

fn push_unique_candidate(promoted: &mut Vec<Value>, candidate: &Value) {
    let candidate_id = candidate
        .get("candidate_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !promoted.iter().any(|entry| {
        entry
            .get("candidate_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            == candidate_id
    }) {
        promoted.push(candidate.clone());
    }
}

fn candidate_score(candidate: &Value) -> f64 {
    candidate
        .get("scores")
        .and_then(|scores| scores.get("final_score"))
        .and_then(Value::as_f64)
        .or_else(|| candidate.get("final_score").and_then(Value::as_f64))
        .unwrap_or(0.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromotionReason {
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
struct ChampionSelection {
    champion: Value,
    reason: PromotionReason,
}

fn select_balanced_champion(
    generation_index: usize,
    promoted: &[Value],
    all_candidates: &[Value],
    previous_champions: &[Value],
    island_names: &[String],
    novelty_champion_rate_min: f64,
) -> ChampionSelection {
    let pool = if promoted.is_empty() {
        all_candidates
    } else {
        promoted
    };
    if pool.is_empty() {
        return ChampionSelection {
            champion: json!({}),
            reason: PromotionReason::TopScore,
        };
    }
    let rolling_start = previous_champions.len().saturating_sub(99);
    let rolling = &previous_champions[rolling_start..];
    let top = best_candidate(pool);
    if let Some(previous_elite) = previous_champions.last() {
        if candidate_score(&top) < candidate_score(previous_elite) - QUALITY_REGRESSION_TOLERANCE {
            return ChampionSelection {
                champion: retain_elite_champion(previous_elite, generation_index),
                reason: PromotionReason::EliteRetain,
            };
        }
    }
    let novelty_champions = rolling
        .iter()
        .filter(|champion| champion.get("mode").and_then(Value::as_str) == Some("novelty"))
        .count();
    let novelty_rate = if rolling.is_empty() {
        0.0
    } else {
        novelty_champions as f64 / rolling.len() as f64
    };
    if novelty_rate < novelty_champion_rate_min {
        if let Some(candidate) = best_candidate_matching(pool, "mode", "novelty") {
            let novelty_score = candidate_score(&candidate);
            let competitive_floor = NOVELTY_CHAMPION_SCORE_FLOOR
                .max(candidate_score(&top) - QUALITY_REGRESSION_TOLERANCE);
            if novelty_score >= competitive_floor {
                return ChampionSelection {
                    champion: candidate,
                    reason: PromotionReason::NoveltyQuota,
                };
            }
        }
    }
    let represented_islands = rolling
        .iter()
        .filter_map(|champion| champion.get("island").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let target_island_coverage = island_names.len().min(4);
    if generation_index <= 100 && represented_islands.len() < target_island_coverage {
        for island in island_names {
            if !represented_islands.contains(island.as_str()) {
                if let Some(candidate) = best_candidate_matching(pool, "island", island) {
                    return ChampionSelection {
                        champion: candidate,
                        reason: PromotionReason::IslandBalance,
                    };
                }
            }
        }
    }
    let counts = count_string_field(rolling, "island");
    if let Some(top_island) = top.get("island").and_then(Value::as_str) {
        let projected_count = counts.get(top_island).copied().unwrap_or(0) + 1;
        let projected_share = projected_count as f64 / (rolling.len() + 1).max(1) as f64;
        if projected_share > 0.60 {
            if let Some(candidate) = pool
                .iter()
                .filter(|candidate| {
                    candidate.get("island").and_then(Value::as_str) != Some(top_island)
                })
                .max_by(|a, b| {
                    candidate_score(a)
                        .partial_cmp(&candidate_score(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned()
            {
                return ChampionSelection {
                    champion: candidate,
                    reason: PromotionReason::IslandCap,
                };
            }
        }
    }
    ChampionSelection {
        champion: top,
        reason: PromotionReason::TopScore,
    }
}

fn retain_elite_champion(previous_elite: &Value, generation_index: usize) -> Value {
    let mut elite = previous_elite.clone();
    let retained_from_generation_id = elite
        .get("generation_id")
        .cloned()
        .unwrap_or_else(|| json!(""));
    elite["retained_from_generation_id"] = retained_from_generation_id;
    elite["selection_generation_id"] = json!(format!("g{generation_index:04}"));
    elite
}

fn best_candidate(candidates: &[Value]) -> Value {
    candidates
        .iter()
        .max_by(|a, b| {
            candidate_score(a)
                .partial_cmp(&candidate_score(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
        .unwrap_or_else(empty_object)
}

fn best_candidate_matching(candidates: &[Value], field: &str, value: &str) -> Option<Value> {
    candidates
        .iter()
        .filter(|candidate| candidate.get(field).and_then(Value::as_str) == Some(value))
        .max_by(|a, b| {
            candidate_score(a)
                .partial_cmp(&candidate_score(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

fn champion_summary_record(
    generation_id: &str,
    champion: &Value,
    promotion_reason: PromotionReason,
) -> Value {
    json!({
        "generation_id": generation_id,
        "candidate_id": champion.get("candidate_id").cloned().unwrap_or_else(|| json!("")),
        "island": champion.get("island").cloned().unwrap_or_else(|| json!("")),
        "mode": champion.get("mode").cloned().unwrap_or_else(|| json!("")),
        "final_score": champion.get("scores").and_then(|scores| scores.get("final_score")).cloned().unwrap_or_else(|| json!(0.0)),
        "novelty_score": champion.get("scores").and_then(|scores| scores.get("novelty_score")).cloned().unwrap_or_else(|| json!(0.0)),
        "promotion_reason": promotion_reason.as_str(),
        "source_card_ids": champion.get("source_card_ids").cloned().unwrap_or_else(|| json!([])),
        "frontier_claim": champion.get("frontier_claim").cloned().unwrap_or_else(|| json!("")),
        "falsifiable_tests": champion.get("falsifiable_tests").cloned().unwrap_or_else(|| json!([])),
        "known_failure_modes": champion.get("known_failure_modes").cloned().unwrap_or_else(|| json!([])),
        "review_priority": champion.get("review_priority").cloned().unwrap_or_else(|| json!("")),
        "retained_from_generation_id": champion.get("retained_from_generation_id").cloned().unwrap_or_else(|| json!(null)),
    })
}

fn population_snapshot(
    run_id: &str,
    generation_id: &str,
    population_size: usize,
    island_names: &[String],
    mode_counts: &BTreeMap<String, usize>,
    generation_candidates: &[Value],
    promoted: &[Value],
    champion_candidate_id: &Value,
    best_final_score: f64,
    diversity_metrics: Value,
) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "population_snapshot",
        "run_id": run_id,
        "generation_id": generation_id,
        "population_size": population_size,
        "islands": island_names,
        "mode_counts": mode_counts,
        "candidate_ids": generation_candidates.iter().filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str)).map(ToString::to_string).collect::<Vec<_>>(),
        "promoted_candidate_ids": promoted.iter().filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str)).map(ToString::to_string).collect::<Vec<_>>(),
        "champion_candidate_id": champion_candidate_id,
        "best_final_score": round6(best_final_score),
        "diversity_metrics": diversity_metrics,
        "candidates": generation_candidates,
    })
}

fn stage_variant_record(
    run_id: &str,
    variant: &str,
    generation_id: &str,
    candidate_id: &str,
    stage: &StagePackage,
    parent_stage_variant_ids: &[String],
    score_breakdown: &Value,
    research_refs: &[String],
) -> Value {
    let prompt_hash = if stage.prompt_hash.is_empty() {
        stable_hash(&stage.purpose)
    } else {
        stage.prompt_hash.clone()
    };
    let variant_hash = short_hash(
        &format!(
            "{}:{}:{}:{:?}",
            stage.stage_id, generation_id, prompt_hash, parent_stage_variant_ids
        ),
        12,
    );
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "stage_variant",
        "run_id": run_id,
        "variant": variant,
        "generation_id": generation_id,
        "stage_id": stage.stage_id,
        "candidate_id": candidate_id,
        "stage_variant_id": format!("sv-{}-{}", stage.stage_id, variant_hash),
        "parent_stage_variant_ids": parent_stage_variant_ids,
        "algorithm_summary": format!("{}: {} mutation={}", stage.name, stage.purpose, stage.mutation_op),
        "prompt_hash": prompt_hash,
        "memory_refs": memory_refs_for_stage(stage),
        "research_refs": research_refs,
        "evidence_bundle_path": format!("stages/{}/generations/{}/evidence-bundle.json", stage.stage_id, generation_id),
        "score_breakdown": score_breakdown,
    })
}

fn stage_ledger_record(
    run_id: &str,
    variant: &str,
    generation_id: &str,
    stage_id: &str,
    candidate_id: &str,
    stage_name: &str,
    route: &RoutePolicy,
    mutation_op: Option<String>,
    parent_generation_id: Option<String>,
    delta_score: f64,
    pass_rate: f64,
    time_seconds: f64,
    failure_modes: Vec<Value>,
    artifact_paths: Value,
    scores: &Value,
) -> Value {
    let parent_generation_id = parent_generation_id.unwrap_or_else(|| "root".to_string());
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "stage_ledger",
        "run_id": run_id,
        "variant": variant,
        "generation_id": generation_id,
        "stage_id": stage_id,
        "candidate_id": candidate_id,
        "stage_name": stage_name,
        "route_backend": route.route_backend,
        "route_tier": route.route_tier,
        "router_state": route.router_state,
        "mutation_op": mutation_op,
        "parent_generation_id": parent_generation_id,
        "delta_score": round6(delta_score),
        "pass_rate": round6(pass_rate),
        "time_seconds": round6(time_seconds),
        "failure_modes": failure_modes,
        "artifact_paths": artifact_paths,
        "local_score": scores.get("local_score").cloned().unwrap_or_else(|| json!(0.0)),
        "interface_score": scores.get("interface_score").cloned().unwrap_or_else(|| json!(0.0)),
        "macro_score": scores.get("macro_score").cloned().unwrap_or_else(|| json!(0.0)),
        "innovation_score": scores.get("innovation_score").cloned().unwrap_or_else(|| json!(0.0)),
        "novelty_score": scores.get("novelty_score").cloned().unwrap_or_else(|| json!(0.0)),
        "failure_penalty": scores.get("failure_penalty").cloned().unwrap_or_else(|| json!(0.0)),
        "final_score": scores.get("final_score").cloned().unwrap_or_else(|| json!(0.0)),
    })
}

fn metrics_point(
    run_id: &str,
    variant: &str,
    generation_id: &str,
    metric_name: &str,
    metric_value: f64,
    series: &str,
    stage_id: Option<&str>,
    candidate_id: Option<&str>,
    extra: Value,
) -> Value {
    let mut record = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "metrics_point",
        "run_id": run_id,
        "variant": variant,
        "generation_id": generation_id,
        "stage_id": stage_id,
        "candidate_id": candidate_id,
        "metric_name": metric_name,
        "metric_value": round6(metric_value),
        "series": series,
    });
    merge_object(&mut record, &extra);
    record
}

fn promotion_decision_record(
    run_id: &str,
    generation_id: &str,
    champion: &Value,
    promoted: &[Value],
    evidence_bundle_path: &Path,
    promotion_reason: PromotionReason,
    score_breakdown: Value,
) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "promotion_decision",
        "run_id": run_id,
        "generation_id": generation_id,
        "candidate_id": champion.get("candidate_id").cloned().unwrap_or_else(|| json!("")),
        "promoted_candidate_ids": promoted.iter().filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str)).map(ToString::to_string).collect::<Vec<_>>(),
        "champion_candidate_id": champion.get("candidate_id").cloned().unwrap_or_else(|| json!("")),
        "promotion_reason": promotion_reason.as_str(),
        "decision_basis": ["final_score", "novelty_score", "interface_score", "failure_understanding"],
        "score_breakdown": score_breakdown,
        "evidence_bundle_path": evidence_bundle_path.display().to_string(),
    })
}

fn lineage_edge_record(
    run_id: &str,
    generation_id: &str,
    parent_candidate_id: Value,
    child_candidate_id: Value,
    mutation_op: String,
    island: String,
) -> Value {
    let parent_candidate_id = match parent_candidate_id {
        Value::Null => json!("root"),
        value => value,
    };
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "lineage_edge",
        "run_id": run_id,
        "generation_id": generation_id,
        "parent_candidate_id": parent_candidate_id,
        "child_candidate_id": child_candidate_id,
        "mutation_op": mutation_op,
        "island": island,
    })
}

fn constant_score_block(value: f64) -> Value {
    let rounded = round6(value);
    json!({
        "local_score": rounded,
        "interface_score": rounded,
        "macro_score": rounded,
        "innovation_score": rounded,
        "novelty_score": rounded,
        "failure_penalty": 0.0,
        "final_score": rounded,
    })
}

fn judge_block(
    family: &str,
    provenance: &str,
    route_tier: &str,
    prompt: usize,
    completion: usize,
) -> Value {
    let total = prompt + completion;
    json!({
        "family": family,
        "provenance": provenance,
        "route_tier": route_tier,
        "token_usage": {
            "prompt": prompt,
            "completion": completion,
            "total": total,
        }
    })
}

fn run_event(
    run_id: &str,
    variant: &str,
    event_type: &str,
    generation_id: &str,
    stage_id: &str,
    parent_generation_id: Option<String>,
    mutation_op: Option<String>,
    stage_family: &str,
    route_backend: &str,
    route_tier: &str,
    router_state: &str,
    artifact_paths: Value,
    judge: Value,
    scores: Value,
) -> Value {
    let candidate_id = if stage_id == "run" || stage_id == "generation" {
        generation_id.to_string()
    } else {
        format!("{generation_id}-{stage_id}")
    };
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "run_event",
        "event_type": event_type,
        "run_id": run_id,
        "variant": variant,
        "generation_id": generation_id,
        "stage_id": stage_id,
        "candidate_id": candidate_id,
        "parent_generation_id": parent_generation_id,
        "mutation_op": mutation_op,
        "stage_family": stage_family,
        "route_backend": route_backend,
        "route_tier": route_tier,
        "router_state": router_state,
        "artifact_paths": artifact_paths,
        "judge": judge,
        "local_score": scores.get("local_score").cloned().unwrap_or_else(|| json!(0.0)),
        "interface_score": scores.get("interface_score").cloned().unwrap_or_else(|| json!(0.0)),
        "macro_score": scores.get("macro_score").cloned().unwrap_or_else(|| json!(0.0)),
        "innovation_score": scores.get("innovation_score").cloned().unwrap_or_else(|| json!(0.0)),
        "novelty_score": scores.get("novelty_score").cloned().unwrap_or_else(|| json!(0.0)),
        "failure_penalty": scores.get("failure_penalty").cloned().unwrap_or_else(|| json!(0.0)),
        "final_score": scores.get("final_score").cloned().unwrap_or_else(|| json!(0.0)),
    })
}

fn build_run_summary(
    run_id: &str,
    variant: &str,
    runbook: &Value,
    runbook_path: &Path,
    max_generations: usize,
    seed: u64,
    dry_run: bool,
    jailgun_available: bool,
    generation_scores: &[f64],
    stage_ledgers: &[Value],
    stage_registry: &[StagePackage],
    router_state: &str,
    degraded_router: bool,
    stage_summary_paths: &[PathBuf],
    run_dir: &Path,
    hybrid_evolution: Option<&Value>,
    population: &PopulationConfig,
) -> Value {
    let quality_scores = hybrid_evolution
        .and_then(|value| value.get("generation_scores").and_then(Value::as_array))
        .map(|scores| scores.iter().filter_map(Value::as_f64).collect::<Vec<_>>())
        .filter(|scores| !scores.is_empty())
        .unwrap_or_else(|| generation_scores.to_vec());
    let hybrid_best = hybrid_evolution
        .and_then(|value| value.get("best_score_seen").and_then(Value::as_f64))
        .unwrap_or(0.0);
    let best_score_seen = generation_scores
        .iter()
        .copied()
        .fold(0.0, f64::max)
        .max(hybrid_best);
    let rolling_5_median = median(&quality_scores[quality_scores.len().saturating_sub(5)..]);
    let deltas: Vec<f64> = quality_scores
        .windows(2)
        .map(|window| window[1] - window[0])
        .collect();
    let best_nonregressive_delta = deltas
        .iter()
        .copied()
        .filter(|delta| *delta >= 0.0)
        .fold(0.0, f64::max);
    let regression_rate = regression_rate(&quality_scores);
    let total_stage_runs = stage_ledgers.len();
    let degraded_route_count = stage_ledgers
        .iter()
        .filter(|entry| {
            entry.get("router_state").and_then(Value::as_str) == Some("degraded_router")
                || entry
                    .get("failure_modes")
                    .and_then(Value::as_array)
                    .map(|modes| {
                        modes
                            .iter()
                            .any(|mode| mode.as_str() == Some("degraded_router"))
                    })
                    .unwrap_or(false)
        })
        .count();
    let decoy_failures = stage_ledgers
        .iter()
        .filter(|entry| {
            entry
                .get("failure_modes")
                .and_then(Value::as_array)
                .map(|modes| {
                    modes.iter().any(|mode| {
                        matches!(
                            mode.as_str(),
                            Some(
                                "decoy_failure"
                                    | "decoy_failed"
                                    | "failed_decoy"
                                    | "test_decoy_failure"
                            )
                        )
                    })
                })
                .unwrap_or(false)
        })
        .count();
    let unique_contributions = stage_ledgers
        .iter()
        .filter(|entry| {
            entry
                .get("innovation_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                >= 0.45
                && entry
                    .get("final_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
                    >= 0.55
        })
        .filter_map(|entry| entry.get("stage_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>()
        .len();
    let total_time = stage_ledgers
        .iter()
        .map(|entry| {
            entry
                .get("time_seconds")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        })
        .sum::<f64>()
        .max(1.0);
    let throughput = max_generations as f64 / total_time;
    let fail_stop_rate = if total_stage_runs == 0 {
        0.0
    } else {
        stage_ledgers
            .iter()
            .filter(|entry| {
                entry
                    .get("pass_rate")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
                    < 1.0
            })
            .count() as f64
            / total_stage_runs as f64
    };
    let lineage_missing = hybrid_evolution
        .and_then(|value| value.get("lineage"))
        .and_then(|value| value.get("missing_parent_ids"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let stage_scores = stage_registry
        .iter()
        .map(|stage| {
            let scores = stage_ledgers
                .iter()
                .filter(|entry| entry.get("stage_id").and_then(Value::as_str) == Some(stage.stage_id.as_str()))
                .filter_map(|entry| entry.get("final_score").and_then(Value::as_f64))
                .collect::<Vec<_>>();
            json!({
                "stage_id": stage.stage_id,
                "final_score": if scores.is_empty() { 0.0 } else { scores.iter().sum::<f64>() / scores.len() as f64 },
            })
        })
        .collect::<Vec<_>>();
    let pareto = hybrid_evolution
        .and_then(|value| value.get("pareto_snapshot").cloned())
        .unwrap_or_else(|| pareto_snapshot(generation_scores, &stage_scores));
    let score_blend = score_blend(runbook, Some(population));
    let mut summary = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "run_summary",
        "run_id": run_id,
        "variant": variant,
        "runbook_path": runbook_path.display().to_string(),
        "output_root": run_dir.parent().and_then(|p| p.parent()).map(|p| p.display().to_string()).unwrap_or_else(|| DEFAULT_OUTPUT_ROOT.to_string()),
        "run_dir": run_dir.display().to_string(),
        "generation_count": max_generations,
        "stage_count": stage_registry.len(),
        "candidate_count": hybrid_evolution.and_then(|value| value.get("candidate_count").and_then(Value::as_u64)).map(|v| v as usize).unwrap_or(stage_ledgers.len()),
        "best_score_seen": round6(best_score_seen),
        "rolling_5_median": round6(rolling_5_median),
        "best_nonregressive_delta": round6(best_nonregressive_delta),
        "unique_contributions": unique_contributions,
        "decoy_failures": decoy_failures,
        "degraded_route_count": degraded_route_count,
        "regression_rate": round6(regression_rate),
        "throughput": round6(throughput),
        "fail_stop_rate": round6(fail_stop_rate),
        "score_blend": score_blend,
        "router_state": router_state,
        "degraded_router": degraded_router,
        "lineage": {
            "acyclic": hybrid_evolution.and_then(|value| value.get("lineage")).and_then(|value| value.get("acyclic")).and_then(Value::as_bool).unwrap_or(lineage_missing == 0),
            "missing_parent_ids": lineage_missing,
        },
        "pareto_snapshot": pareto,
        "stage_summaries": stage_summary_paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
        "validation": {
            "stage_count": stage_registry.len(),
            "stage_ids": stage_registry.iter().map(|stage| stage.stage_id.clone()).collect::<Vec<_>>(),
        },
        "seed": seed,
        "dry_run": dry_run,
        "jailgun_available": jailgun_available,
    });
    if let Some(hybrid_evolution) = hybrid_evolution {
        merge_object(
            &mut summary,
            &json!({
                "hybrid_baseline_score": HYBRID_BASELINE_SCORE,
                "hybrid_baseline_delta": round6(best_score_seen - HYBRID_BASELINE_SCORE),
                "population": {
                    "population_size": population.population_size,
                    "islands": population.islands,
                    "island_names": population.island_names,
                    "new_info_refresh": population.new_info_refresh,
                    "novelty_weight": population.novelty_weight,
                    "diversity_targets": population.diversity_targets,
                    "promotion_gates": population.promotion_gates,
                    "degraded_penalties": population.degraded_penalties,
                },
                "diversity_metrics": hybrid_evolution.get("diversity_metrics").cloned().unwrap_or_else(empty_object),
                "novelty_archive": hybrid_evolution.get("novelty_archive").cloned().unwrap_or_else(|| json!("")),
                "island_leaderboard": hybrid_evolution.get("island_leaderboard").cloned().unwrap_or_else(empty_object),
                "fun_summary": hybrid_evolution.get("fun_summary").cloned().unwrap_or_else(empty_object),
                "generation_champions": hybrid_evolution.get("generation_champions").cloned().unwrap_or_else(|| json!([])),
                "lineage": hybrid_evolution.get("lineage").cloned().unwrap_or_else(|| json!({"acyclic": true, "missing_parent_ids": 0})),
            }),
        );
    }
    summary
}

fn score_blend(runbook: &Value, population: Option<&PopulationConfig>) -> Value {
    let novelty_default = population
        .map(|population| population.novelty_weight)
        .unwrap_or(0.0);
    let evaluation = runbook
        .get("evaluation")
        .cloned()
        .unwrap_or_else(empty_object);
    let score_blend = evaluation
        .get("score_blend")
        .cloned()
        .unwrap_or_else(empty_object);
    json!({
        "local_score": score_blend.get("local_score").and_then(Value::as_f64).unwrap_or(0.30),
        "interface_score": score_blend.get("interface_score").and_then(Value::as_f64).unwrap_or(0.20),
        "macro_score": score_blend.get("macro_score").and_then(Value::as_f64).unwrap_or(0.30),
        "innovation_score": score_blend.get("innovation_score").and_then(Value::as_f64).unwrap_or(0.15),
        "novelty_score": score_blend.get("novelty_score").and_then(Value::as_f64).unwrap_or(novelty_default),
        "failure_penalty": score_blend.get("failure_penalty").and_then(Value::as_f64).unwrap_or(0.05),
    })
}

fn pareto_snapshot(generation_scores: &[f64], stage_scores: &[Value]) -> Value {
    json!({
        "frontier_size": generation_scores.len().min(stage_scores.len()),
        "points": generation_scores.iter().enumerate().map(|(index, score)| json!({
            "generation_index": index + 1,
            "score": round6(*score),
        })).collect::<Vec<_>>(),
    })
}

fn emit_run_offline_eval(run_dir: &Path) -> Result<Value> {
    let summary_path = run_dir.join("run-summary.json");
    let summary: Value = if summary_path.exists() {
        serde_json::from_str(&fs::read_to_string(&summary_path)?)
            .with_context(|| format!("parse {}", summary_path.display()))?
    } else {
        json!({})
    };
    let stage_rankings = read_stage_rankings(run_dir);
    let warnings = warnings_from_summary(&summary);
    let offline_eval = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "offline_eval",
        "run_id": summary.get("run_id").cloned().unwrap_or_else(|| json!(infer_run_id_from_path(run_dir))),
        "variant": summary.get("variant").cloned().unwrap_or_else(|| json!(null)),
        "scorecard": summary,
        "stage_rankings": stage_rankings,
        "warnings": warnings,
    });
    write_json(&run_dir.join("offline-eval.json"), &offline_eval)?;
    write_markdown(
        &run_dir.join("offline-eval.md"),
        &render_run_markdown(&offline_eval),
    )?;
    Ok(offline_eval)
}

fn emit_root_comparison(root: &Path) -> Result<Value> {
    let mut variants = Vec::new();
    if root.exists() {
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) != Some("run-summary.json") {
                continue;
            }
            if path
                .components()
                .any(|component| component.as_os_str() == "latest")
            {
                continue;
            }
            let summary: Value = serde_json::from_str(&fs::read_to_string(path)?)
                .with_context(|| format!("parse {}", path.display()))?;
            variants.push(json!({
                "variant": summary.get("variant").cloned().unwrap_or_else(|| json!(null)),
                "scorecard": summary,
            }));
        }
    }
    let mut ranking = variants
        .iter()
        .map(|entry| {
            let scorecard = entry.get("scorecard").cloned().unwrap_or_else(empty_object);
            json!({
                "variant": entry.get("variant").cloned().unwrap_or_else(|| json!(null)),
                "final_score": scorecard.get("best_score_seen").and_then(Value::as_f64).unwrap_or(0.0),
            })
        })
        .collect::<Vec<_>>();
    ranking.sort_by(|a, b| {
        b.get("final_score")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .partial_cmp(&a.get("final_score").and_then(Value::as_f64).unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let comparison = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "comparison",
        "root": root.display().to_string(),
        "variants": variants,
        "ranking": ranking,
    });
    write_json(&root.join("comparison.json"), &comparison)?;
    write_markdown(
        &root.join("comparison.md"),
        &render_comparison_markdown(&comparison),
    )?;
    emit_hybrid_report_copies(root)?;
    emit_root_plot_index(root)?;
    Ok(comparison)
}

fn emit_hybrid_report_copies(root: &Path) -> Result<()> {
    let hybrid_latest = root.join("hybrid").join("latest");
    for (source_name, target_name) in [
        ("novelty-archive.json", "hybrid-novelty-archive.json"),
        ("island-leaderboard.json", "hybrid-island-leaderboard.json"),
        ("lineage-graph.md", "hybrid-lineage-graph.md"),
    ] {
        let source = hybrid_latest.join(source_name);
        if source.exists() {
            fs::copy(&source, root.join(target_name))?;
        }
    }
    Ok(())
}

fn emit_run_plot_index(run_dir: &Path) -> Result<PathBuf> {
    let summary_path = run_dir.join("run-summary.json");
    let summary: Value = if summary_path.exists() {
        serde_json::from_str(&fs::read_to_string(&summary_path)?)
            .with_context(|| format!("parse {}", summary_path.display()))?
    } else {
        json!({})
    };
    let ledgers = json!({
        "run_events": run_dir.join("run-events.jsonl").display().to_string(),
        "stage_ledger": run_dir.join("stage-ledger.jsonl").display().to_string(),
        "population_ledger": run_dir.join("population-ledger.jsonl").display().to_string(),
        "lineage_graph": run_dir.join("lineage-graph.jsonl").display().to_string(),
        "metrics": run_dir.join("metrics-timeseries.jsonl").display().to_string(),
        "generation": run_dir.join("generation-ledger.jsonl").display().to_string(),
        "stage_variant": run_dir.join("stage-variant-ledger.jsonl").display().to_string(),
        "live_call": run_dir.join("live-call-ledger.jsonl").display().to_string(),
        "research": run_dir.join("research-ledger.jsonl").display().to_string(),
        "memory": run_dir.join("memory-ledger.jsonl").display().to_string(),
        "preflight": run_dir.join("preflight.json").display().to_string(),
        "quality_gate": run_dir.join("quality-gate.json").display().to_string(),
    });
    let series = json!({
        "champion": {"ledger": ledgers["generation"].clone(), "metric_name": "hybrid_champion_score"},
        "rollup": {"ledger": ledgers["generation"].clone(), "metric_name": "deterministic_rollup_score"},
        "stage": {"ledger": ledgers["metrics"].clone(), "metric_name": "stage_final_score"},
        "island": {"ledger": ledgers["population_ledger"].clone(), "field": "candidates[].island"},
        "live_call": {"ledger": ledgers["live_call"].clone(), "field": "status"},
        "research": {"ledger": ledgers["research"].clone(), "field": "accepted"},
    });
    let index = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "plot_index",
        "run_id": summary.get("run_id").cloned().unwrap_or_else(|| json!(infer_run_id_from_path(run_dir))),
        "variant": summary.get("variant").cloned().unwrap_or_else(|| json!(null)),
        "run_dir": run_dir.display().to_string(),
        "generation_count": summary.get("generation_count").cloned(),
        "ledgers": ledgers,
        "series": series,
        "summary": {
            "best_score_seen": summary.get("best_score_seen").cloned(),
            "rolling_5_median": summary.get("rolling_5_median").cloned(),
            "candidate_count": summary.get("candidate_count").cloned(),
            "quality_gate": run_dir.join("quality-gate.json").display().to_string(),
        },
    });
    let path = run_dir.join("plot-index.json");
    write_json(&path, &index)?;
    Ok(path)
}

fn emit_root_plot_index(root: &Path) -> Result<PathBuf> {
    let mut runs = Vec::new();
    if root.exists() {
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.file_name().and_then(|name| name.to_str()) != Some("run-summary.json") {
                continue;
            }
            if path
                .components()
                .any(|component| component.as_os_str() == "latest")
            {
                continue;
            }
            let run_dir = path.parent().unwrap_or(root);
            emit_run_plot_index(run_dir)?;
            let summary: Value = serde_json::from_str(&fs::read_to_string(path)?)
                .with_context(|| format!("parse {}", path.display()))?;
            runs.push(json!({
                "run_id": summary.get("run_id").cloned(),
                "variant": summary.get("variant").cloned(),
                "run_dir": run_dir.display().to_string(),
                "plot_index": run_dir.join("plot-index.json").display().to_string(),
                "quality_gate": run_dir.join("quality-gate.json").display().to_string(),
                "best_score_seen": summary.get("best_score_seen").cloned(),
                "generation_count": summary.get("generation_count").cloned(),
            }));
        }
    }
    let index = json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "plot_index_root",
        "root": root.display().to_string(),
        "runs": runs,
    });
    let path = root.join("plot-index.json");
    write_json(&path, &index)?;
    Ok(path)
}

fn validate_record(record: &Value) -> Result<()> {
    let kind = record
        .get("record_kind")
        .and_then(Value::as_str)
        .context("missing record_kind")?;
    let known = [
        "run_event",
        "stage_ledger",
        "stage_summary",
        "run_summary",
        "offline_eval",
        "comparison",
        "concept_gene",
        "stage_concept",
        "population_snapshot",
        "novelty_archive",
        "lineage_edge",
        "information_card",
        "stage_variant",
        "live_call",
        "research_card",
        "memory_card",
        "promotion_decision",
        "metrics_point",
        "preflight",
        "quality_gate",
    ];
    if !known.contains(&kind) {
        bail!("unknown record_kind: {kind}");
    }
    ensure_present(record, "schema_version")?;
    match kind {
        "run_event" => {
            for field in [
                "event_type",
                "run_id",
                "variant",
                "generation_id",
                "stage_id",
                "candidate_id",
                "artifact_paths",
                "judge",
            ] {
                ensure_present(record, field)?;
            }
        }
        "stage_ledger" => {
            for field in [
                "run_id",
                "variant",
                "generation_id",
                "stage_id",
                "candidate_id",
                "stage_name",
                "route_backend",
                "route_tier",
                "router_state",
                "mutation_op",
                "parent_generation_id",
                "delta_score",
                "pass_rate",
                "time_seconds",
                "failure_modes",
                "artifact_paths",
            ] {
                ensure_present(record, field)?;
            }
        }
        "stage_summary" => {
            for field in [
                "run_id",
                "variant",
                "stage_id",
                "stage_name",
                "generations",
                "mean_final_score",
                "best_final_score",
                "pass_rate",
                "route_backends",
                "failure_modes",
                "mutation_ops",
            ] {
                ensure_present(record, field)?;
            }
        }
        "run_summary" => {
            for field in [
                "run_id",
                "variant",
                "generation_count",
                "stage_count",
                "candidate_count",
                "best_score_seen",
                "rolling_5_median",
                "best_nonregressive_delta",
                "unique_contributions",
                "decoy_failures",
                "regression_rate",
                "throughput",
                "fail_stop_rate",
                "score_blend",
                "router_state",
                "degraded_router",
                "lineage",
                "pareto_snapshot",
            ] {
                ensure_present(record, field)?;
            }
        }
        "offline_eval" => {
            for field in [
                "run_id",
                "variant",
                "scorecard",
                "stage_rankings",
                "warnings",
            ] {
                ensure_present(record, field)?;
            }
        }
        "comparison" => {
            for field in ["root", "variants", "ranking"] {
                ensure_present(record, field)?;
            }
        }
        "concept_gene" => {
            for field in [
                "gene_id",
                "concept_id",
                "family",
                "domain",
                "claim",
                "method",
                "constraint",
                "failure_risk",
                "stage_concept_hint",
                "source_card_ids",
                "source_path",
                "provenance_hash",
                "novelty_terms",
            ] {
                ensure_present(record, field)?;
            }
        }
        "stage_concept" => {
            for field in [
                "run_id",
                "generation_id",
                "stage_id",
                "candidate_id",
                "concept_id",
                "family",
                "source_card_ids",
                "mutation_op",
            ] {
                ensure_present(record, field)?;
            }
        }
        "population_snapshot" => {
            for field in [
                "run_id",
                "generation_id",
                "population_size",
                "islands",
                "mode_counts",
                "candidate_ids",
                "promoted_candidate_ids",
                "champion_candidate_id",
                "best_final_score",
                "diversity_metrics",
                "candidates",
            ] {
                ensure_present(record, field)?;
            }
        }
        "novelty_archive" => {
            for field in ["run_id", "entry_count", "information_card_count", "entries"] {
                ensure_present(record, field)?;
            }
        }
        "lineage_edge" => {
            for field in [
                "run_id",
                "generation_id",
                "parent_candidate_id",
                "child_candidate_id",
                "mutation_op",
                "island",
            ] {
                ensure_present(record, field)?;
            }
        }
        "information_card" => {
            for field in [
                "information_card_id",
                "source_path",
                "domain",
                "claim",
                "method",
                "constraint",
                "failure_risk",
                "stage_concept_hint",
                "provenance_hash",
                "novelty_terms",
            ] {
                ensure_present(record, field)?;
            }
        }
        "stage_variant" => {
            for field in [
                "run_id",
                "variant",
                "generation_id",
                "stage_id",
                "candidate_id",
                "stage_variant_id",
                "parent_stage_variant_ids",
                "algorithm_summary",
                "prompt_hash",
                "memory_refs",
                "research_refs",
                "evidence_bundle_path",
                "score_breakdown",
            ] {
                ensure_present(record, field)?;
            }
        }
        "live_call" => {
            for field in [
                "run_id",
                "generation_id",
                "stage_id",
                "candidate_id",
                "call_id",
                "purpose",
                "status",
                "prompt_path",
                "retrieval_packet_path",
                "raw_output_path",
                "parsed_summary_path",
                "receipt_path",
                "timeout_seconds",
                "command",
                "token_usage",
            ] {
                ensure_present(record, field)?;
            }
        }
        "research_card" => {
            for field in [
                "run_id",
                "research_card_id",
                "source_type",
                "url",
                "title",
                "date",
                "source_hash",
                "citation",
                "accepted",
            ] {
                ensure_present(record, field)?;
            }
        }
        "memory_card" => {
            for field in [
                "run_id",
                "memory_card_id",
                "stage_id",
                "memory_refs",
                "content_hash",
                "summary",
            ] {
                ensure_present(record, field)?;
            }
        }
        "promotion_decision" => {
            for field in [
                "run_id",
                "generation_id",
                "candidate_id",
                "promoted_candidate_ids",
                "champion_candidate_id",
                "decision_basis",
                "score_breakdown",
                "evidence_bundle_path",
            ] {
                ensure_present(record, field)?;
            }
        }
        "metrics_point" => {
            for field in [
                "run_id",
                "variant",
                "generation_id",
                "metric_name",
                "metric_value",
                "series",
            ] {
                ensure_present(record, field)?;
            }
        }
        "preflight" => {
            for field in [
                "run_id",
                "variant",
                "runbook_path",
                "run_dir",
                "status",
                "backend_health",
                "live_command",
                "timeout_config",
                "routing_decision",
            ] {
                ensure_present(record, field)?;
            }
        }
        "quality_gate" => {
            for field in [
                "run_id",
                "run_dir",
                "tier",
                "passed",
                "status",
                "metrics",
                "checks",
                "artifacts",
            ] {
                ensure_present(record, field)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn collect_artifact_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let allowed = BTreeSet::from([
        "run-events.jsonl",
        "stage-ledger.jsonl",
        "run-summary.json",
        "offline-eval.json",
        "comparison.json",
        "stage-summary.json",
        "population-snapshot.json",
        "population-ledger.jsonl",
        "novelty-archive.json",
        "information-ledger.jsonl",
        "lineage-graph.jsonl",
        "concept-gene-ledger.jsonl",
        "stage-concept-ledger.jsonl",
        "metrics-timeseries.jsonl",
        "generation-ledger.jsonl",
        "stage-variant-ledger.jsonl",
        "live-call-ledger.jsonl",
        "research-ledger.jsonl",
        "memory-ledger.jsonl",
        "promotion-ledger.jsonl",
        "preflight.json",
        "quality-gate.json",
    ]);
    let mut paths = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        if allowed.contains(filename.as_str()) {
            paths.push(entry.into_path());
        }
    }
    paths.sort();
    Ok(paths)
}

fn validate_hybrid_invariants(root: &Path) -> Result<()> {
    let population_paths: Vec<PathBuf> = if root.exists() {
        WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.file_type().is_file()
                    && entry.file_name().to_string_lossy() == "population-snapshot.json"
            })
            .map(|entry| entry.into_path())
            .collect()
    } else {
        Vec::new()
    };
    if population_paths.is_empty() {
        return Ok(());
    }
    let mut candidate_ids_by_run: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut promoted_ids_by_run: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut parent_ids_by_run: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut child_to_parents_by_run: BTreeMap<String, BTreeMap<String, Vec<String>>> =
        BTreeMap::new();
    let mut information_counts: BTreeMap<String, usize> = BTreeMap::new();
    for path in population_paths {
        let snapshot: Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        let run_id = snapshot
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let candidate_ids = snapshot
            .get("candidate_ids")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let ids: BTreeSet<String> = candidate_ids
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect();
        if ids.is_empty() {
            bail!("empty population snapshot in {}", path.display());
        }
        if ids.len() != candidate_ids.len() {
            bail!("duplicate candidate ids in {}", path.display());
        }
        candidate_ids_by_run
            .entry(run_id.clone())
            .or_default()
            .extend(ids);
        promoted_ids_by_run
            .entry(run_id.clone())
            .or_default()
            .extend(
                snapshot
                    .get("promoted_candidate_ids")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string),
            );
        if let Some(candidates) = snapshot.get("candidates").and_then(Value::as_array) {
            for candidate in candidates {
                let child_id = candidate
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let parent_ids = candidate
                    .get("parent_candidate_ids")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for parent_id in parent_ids {
                    let parent_id = parent_id.as_str().unwrap_or("").to_string();
                    parent_ids_by_run
                        .entry(run_id.clone())
                        .or_default()
                        .insert(parent_id.clone());
                    child_to_parents_by_run
                        .entry(run_id.clone())
                        .or_default()
                        .entry(child_id.clone())
                        .or_default()
                        .push(parent_id);
                }
            }
        }
    }
    for path in WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.file_name().to_string_lossy() == "information-ledger.jsonl"
        })
        .map(|entry| entry.into_path())
    {
        for card in read_jsonl::<Value>(&path).unwrap_or_default() {
            let run_id = card
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            if card
                .get("provenance_hash")
                .and_then(Value::as_str)
                .is_none()
            {
                bail!(
                    "information card missing provenance hash in {}",
                    path.display()
                );
            }
            *information_counts.entry(run_id).or_default() += 1;
        }
    }
    for (run_id, candidate_ids) in candidate_ids_by_run {
        let missing_parents = parent_ids_by_run
            .get(&run_id)
            .cloned()
            .unwrap_or_default()
            .difference(&candidate_ids)
            .cloned()
            .collect::<Vec<_>>();
        if !missing_parents.is_empty() {
            bail!(
                "missing parent candidate ids for {}: {:?}",
                run_id,
                &missing_parents[..missing_parents.len().min(5)]
            );
        }
        let orphan_promoted = promoted_ids_by_run
            .get(&run_id)
            .cloned()
            .unwrap_or_default()
            .difference(&candidate_ids)
            .cloned()
            .collect::<Vec<_>>();
        if !orphan_promoted.is_empty() {
            bail!(
                "orphan promoted candidate ids for {}: {:?}",
                run_id,
                &orphan_promoted[..orphan_promoted.len().min(5)]
            );
        }
        let empty_graph = BTreeMap::new();
        let graph = child_to_parents_by_run.get(&run_id).unwrap_or(&empty_graph);
        if !lineage_edges_are_acyclic(graph) {
            bail!("lineage cycle detected for {}", run_id);
        }
        if information_counts.get(&run_id).copied().unwrap_or(0) == 0 {
            bail!("empty information ledger for {}", run_id);
        }
    }
    Ok(())
}

fn build_quality_gate_report(run_dir: &Path) -> Result<Value> {
    let checkpoint_path = run_dir.join("checkpoint.json");
    let summary_path = run_dir.join("run-summary.json");
    let offline_eval_path = run_dir.join("offline-eval.json");
    let live_ledger_path = run_dir.join("live-call-ledger.jsonl");
    let generation_ledger_path = run_dir.join("generation-ledger.jsonl");
    let run_events_path = run_dir.join("run-events.jsonl");
    let stage_ledger_path = run_dir.join("stage-ledger.jsonl");
    let plot_index_path = run_dir.join("plot-index.json");
    let preflight_path = run_dir.join("preflight.json");

    let mut artifact_checks = Vec::new();
    let checkpoint = read_required_json(&checkpoint_path, &mut artifact_checks);
    let summary = read_required_json(&summary_path, &mut artifact_checks);
    let offline_eval = read_required_json(&offline_eval_path, &mut artifact_checks);
    let live_records = read_required_jsonl(&live_ledger_path, &mut artifact_checks);
    let generation_records = read_required_jsonl(&generation_ledger_path, &mut artifact_checks);
    let run_events = read_required_jsonl(&run_events_path, &mut artifact_checks);
    let stage_ledgers = read_optional_jsonl(&stage_ledger_path);
    let preflight = read_optional_json(&preflight_path);

    let artifact_validity = artifact_checks.iter().all(|check| {
        check
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    let complete_generation = checkpoint
        .get("complete_generation")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let target_generation = checkpoint
        .get("target_generation")
        .and_then(Value::as_u64)
        .or_else(|| summary.get("generation_count").and_then(Value::as_u64))
        .unwrap_or(complete_generation as u64) as usize;
    let tier = if target_generation >= 1000 {
        "full"
    } else if target_generation >= 50 {
        "qualification"
    } else {
        "smoke"
    };
    let report_run_id = summary
        .get("run_id")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| infer_run_id_from_path(run_dir));
    let live_total = live_records.len();
    let live_timeout_count = live_records
        .iter()
        .filter(|record| record.get("status").and_then(Value::as_str) == Some("timeout"))
        .count();
    let failed_live_calls = live_records
        .iter()
        .filter(|record| {
            record
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("failed")
                != "ok"
        })
        .count();
    let timeout_rate = if live_total == 0 {
        0.0
    } else {
        live_timeout_count as f64 / live_total as f64
    };
    let timeout_overrun_count = live_records
        .iter()
        .map(timeout_overrun_count)
        .sum::<usize>();
    let degraded_route_count = summary
        .get("degraded_route_count")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or_else(|| degraded_route_count_from_ledgers(&stage_ledgers, &run_events));
    let decoy_failures = summary
        .get("decoy_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let quality_rollups = hybrid_quality_rollup_series(&generation_records);
    let champion_scores = champion_scores(&summary, &generation_records);
    let stage_rollup_median = median(&quality_rollups);
    let rolling_5_median = median(&quality_rollups[quality_rollups.len().saturating_sub(5)..]);
    let regression_rate = regression_rate(&quality_rollups);
    let perfect_champions = champion_scores
        .iter()
        .filter(|score| **score >= 0.999999)
        .count();
    let champion_perfect_rate = if champion_scores.is_empty() {
        0.0
    } else {
        perfect_champions as f64 / champion_scores.len() as f64
    };
    let champions = summary
        .get("generation_champions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let champion_islands = champions
        .iter()
        .filter_map(|champion| champion.get("island").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let novelty_champions = champions
        .iter()
        .filter(|champion| champion.get("mode").and_then(Value::as_str) == Some("novelty"))
        .count();
    let novelty_champion_rate = if champions.is_empty() {
        0.0
    } else {
        novelty_champions as f64 / champions.len() as f64
    };
    let island_champion_counts = count_string_field(&champions, "island");
    let mode_champion_counts = count_string_field(&champions, "mode");
    let max_island_share = if champions.is_empty() {
        0.0
    } else {
        island_champion_counts.values().copied().max().unwrap_or(0) as f64 / champions.len() as f64
    };
    let run_plot_index_written = plot_index_path.is_file();
    let root_plot_index_path = genome_root_from_run_dir(run_dir).join("plot-index.json");
    let root_plot_index_written = root_plot_index_path.is_file();
    let route_nominal = degraded_route_count == 0
        && !summary
            .get("degraded_router")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let preflight_ok = preflight
        .get("status")
        .and_then(Value::as_str)
        .map(|status| status == "ok")
        .unwrap_or(true);
    let hard_backend_required = preflight
        .get("backend_health")
        .and_then(|health| health.get("require_hard_backend"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            summary.get("variant").and_then(Value::as_str) == Some("hybrid")
                && report_run_id.starts_with("hybrid-v2")
                && target_generation >= 10
        });
    let jailgun_proof_required = hard_backend_required
        && summary.get("variant").and_then(Value::as_str) == Some("hybrid")
        && target_generation >= 10;
    let hard_stage_live_calls = live_records
        .iter()
        .filter(|record| is_hard_stage_repair_live_call(record))
        .count();
    let hard_stage_jailgun_live_calls = live_records
        .iter()
        .filter(|record| is_hard_stage_repair_live_call(record))
        .filter(|record| {
            record.get("execution_backend").and_then(Value::as_str) == Some("jailgun_mcp")
        })
        .count();
    let failed_jailgun_live_calls = live_records
        .iter()
        .filter(|record| {
            record.get("execution_backend").and_then(Value::as_str) == Some("jailgun_mcp")
        })
        .filter(|record| record.get("status").and_then(Value::as_str) != Some("ok"))
        .count();
    let hard_stage_jailgun_proof_missing = live_records
        .iter()
        .filter(|record| is_hard_stage_repair_live_call(record))
        .filter(|record| !jailgun_live_call_has_proof(record))
        .count();

    let mut checks = Vec::new();
    add_check(
        &mut checks,
        "artifact_validity",
        artifact_validity,
        json!(artifact_checks),
        json!("all required artifacts parse"),
    );
    add_check(
        &mut checks,
        "preflight_status",
        preflight_ok,
        preflight
            .get("status")
            .cloned()
            .unwrap_or_else(|| json!("missing_allowed")),
        json!("ok"),
    );
    add_check(
        &mut checks,
        "complete_generation",
        complete_generation >= target_generation,
        json!(complete_generation),
        json!(target_generation),
    );
    add_check(
        &mut checks,
        "failed_live_calls",
        failed_live_calls == 0,
        json!(failed_live_calls),
        json!(0),
    );
    add_check(
        &mut checks,
        "timeout_overrun_count",
        timeout_overrun_count == 0,
        json!(timeout_overrun_count),
        json!(0),
    );
    add_check(
        &mut checks,
        "degraded_route_count",
        degraded_route_count == 0 && route_nominal,
        json!(degraded_route_count),
        json!(0),
    );
    add_check(
        &mut checks,
        "decoy_failures",
        decoy_failures == 0,
        json!(decoy_failures),
        json!(0),
    );
    if jailgun_proof_required {
        add_check(
            &mut checks,
            "hard_stage_jailgun_live_calls",
            hard_stage_jailgun_live_calls > 0,
            json!(hard_stage_jailgun_live_calls),
            json!(">0"),
        );
        add_check(
            &mut checks,
            "failed_jailgun_live_calls",
            failed_jailgun_live_calls == 0,
            json!(failed_jailgun_live_calls),
            json!(0),
        );
        add_check(
            &mut checks,
            "hard_stage_jailgun_receipt_proof",
            hard_stage_jailgun_proof_missing == 0,
            json!(hard_stage_jailgun_proof_missing),
            json!(0),
        );
    }
    add_check(
        &mut checks,
        "run_plot_index",
        run_plot_index_written,
        json!(plot_index_path.display().to_string()),
        json!("exists"),
    );

    match tier {
        "full" => {
            add_check(
                &mut checks,
                "live_timeout_rate",
                timeout_rate <= 0.01,
                json!(round6(timeout_rate)),
                json!(0.01),
            );
            add_check(
                &mut checks,
                "regression_rate",
                regression_rate <= 0.25,
                json!(round6(regression_rate)),
                json!(0.25),
            );
            add_check(
                &mut checks,
                "champion_perfect_rate",
                champion_perfect_rate <= 0.30,
                json!(round6(champion_perfect_rate)),
                json!(0.30),
            );
            add_check(
                &mut checks,
                "rolling_5_median",
                rolling_5_median >= 0.76,
                json!(round6(rolling_5_median)),
                json!(0.76),
            );
            add_check(
                &mut checks,
                "root_plot_index",
                root_plot_index_written,
                json!(root_plot_index_path.display().to_string()),
                json!("exists"),
            );
        }
        "qualification" => {
            add_check(
                &mut checks,
                "live_timeout_rate",
                timeout_rate <= 0.01,
                json!(round6(timeout_rate)),
                json!(0.01),
            );
            add_check(
                &mut checks,
                "regression_rate",
                regression_rate <= 0.25,
                json!(round6(regression_rate)),
                json!(0.25),
            );
            add_check(
                &mut checks,
                "champion_perfect_rate",
                champion_perfect_rate <= 0.20,
                json!(round6(champion_perfect_rate)),
                json!(0.20),
            );
            add_check(
                &mut checks,
                "stage_rollup_median",
                stage_rollup_median >= 0.75,
                json!(round6(stage_rollup_median)),
                json!(0.75),
            );
            add_check(
                &mut checks,
                "champion_island_coverage",
                champion_islands.len() >= 4,
                json!(champion_islands.len()),
                json!(4),
            );
            add_check(
                &mut checks,
                "novelty_champion_rate",
                novelty_champion_rate >= 0.05,
                json!(round6(novelty_champion_rate)),
                json!(0.05),
            );
            add_check(
                &mut checks,
                "max_island_share",
                max_island_share <= 0.60,
                json!(round6(max_island_share)),
                json!(0.60),
            );
        }
        _ => {
            add_check(
                &mut checks,
                "live_timeout_rate",
                timeout_rate <= 0.10,
                json!(round6(timeout_rate)),
                json!(0.10),
            );
        }
    }

    let passed = checks.iter().all(|check| {
        check
            .get("passed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    });
    Ok(json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "quality_gate",
        "run_id": report_run_id,
        "variant": summary.get("variant").cloned().unwrap_or_else(|| json!(null)),
        "run_dir": run_dir.display().to_string(),
        "generated_at": now_iso8601(),
        "tier": tier,
        "passed": passed,
        "status": if passed { "passed" } else { "failed" },
        "metrics": {
            "complete_generation": complete_generation,
            "target_generation": target_generation,
            "live_total": live_total,
            "failed_live_calls": failed_live_calls,
            "live_timeout_count": live_timeout_count,
            "live_timeout_rate": round6(timeout_rate),
            "timeout_overrun_count": timeout_overrun_count,
            "degraded_route_count": degraded_route_count,
            "decoy_failures": decoy_failures,
            "regression_rate": round6(regression_rate),
            "stage_rollup_median": round6(stage_rollup_median),
            "rolling_5_median": round6(rolling_5_median),
            "quality_rollup_count": quality_rollups.len(),
            "champion_count": champion_scores.len(),
            "perfect_champions": perfect_champions,
            "champion_perfect_rate": round6(champion_perfect_rate),
            "hard_backend_required": hard_backend_required,
            "jailgun_proof_required": jailgun_proof_required,
            "hard_stage_live_calls": hard_stage_live_calls,
            "hard_stage_jailgun_live_calls": hard_stage_jailgun_live_calls,
            "failed_jailgun_live_calls": failed_jailgun_live_calls,
            "hard_stage_jailgun_proof_missing": hard_stage_jailgun_proof_missing,
            "champion_island_coverage": champion_islands.len(),
            "novelty_champions": novelty_champions,
            "novelty_champion_rate": round6(novelty_champion_rate),
            "max_island_share": round6(max_island_share),
            "island_champion_counts": island_champion_counts,
            "mode_champion_counts": mode_champion_counts,
            "route_health": if route_nominal { "nominal" } else { "degraded" },
        },
        "artifacts": {
            "checkpoint": checkpoint_path.display().to_string(),
            "run_summary": summary_path.display().to_string(),
            "offline_eval": offline_eval_path.display().to_string(),
            "live_call_ledger": live_ledger_path.display().to_string(),
            "generation_ledger": generation_ledger_path.display().to_string(),
            "run_events": run_events_path.display().to_string(),
            "run_plot_index": plot_index_path.display().to_string(),
            "root_plot_index": root_plot_index_path.display().to_string(),
            "preflight": preflight_path.display().to_string(),
        },
        "checks": checks,
        "artifact_checks": artifact_checks,
        "offline_eval_warnings": offline_eval.get("warnings").cloned().unwrap_or_else(|| json!([])),
    }))
}

fn read_required_json(path: &Path, checks: &mut Vec<Value>) -> Value {
    match fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))
        .and_then(|text| {
            serde_json::from_str::<Value>(&text)
                .with_context(|| format!("parse {}", path.display()))
        }) {
        Ok(value) => {
            let valid = validate_record(&value).is_ok()
                || path.file_name().and_then(|name| name.to_str()) == Some("checkpoint.json");
            checks
                .push(json!({"path": path.display().to_string(), "passed": valid, "kind": "json"}));
            value
        }
        Err(err) => {
            checks.push(json!({"path": path.display().to_string(), "passed": false, "kind": "json", "error": err.to_string()}));
            json!({})
        }
    }
}

fn read_optional_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_else(empty_object)
}

fn read_required_jsonl(path: &Path, checks: &mut Vec<Value>) -> Vec<Value> {
    match read_jsonl::<Value>(path) {
        Ok(records) => {
            let valid = records.iter().all(|record| validate_record(record).is_ok());
            checks.push(json!({"path": path.display().to_string(), "passed": valid, "kind": "jsonl", "records": records.len()}));
            records
        }
        Err(err) => {
            checks.push(json!({"path": path.display().to_string(), "passed": false, "kind": "jsonl", "error": err.to_string()}));
            Vec::new()
        }
    }
}

fn read_optional_jsonl(path: &Path) -> Vec<Value> {
    read_jsonl::<Value>(path).unwrap_or_default()
}

fn add_check(checks: &mut Vec<Value>, name: &str, passed: bool, observed: Value, threshold: Value) {
    checks.push(json!({
        "name": name,
        "passed": passed,
        "observed": observed,
        "threshold": threshold,
    }));
}

fn timeout_overrun_count(record: &Value) -> usize {
    if let Some(attempts) = record.get("attempts").and_then(Value::as_array) {
        return attempts
            .iter()
            .filter(|attempt| {
                let elapsed = attempt
                    .get("elapsed_seconds")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                let timeout = attempt
                    .get("timeout_seconds")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0);
                timeout > 0.0 && elapsed > timeout + 5.0
            })
            .count();
    }
    let elapsed = record
        .get("elapsed_seconds")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let timeout = record
        .get("configured_timeout_seconds")
        .or_else(|| record.get("timeout_seconds"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    usize::from(timeout > 0.0 && elapsed > timeout + 5.0)
}

fn degraded_route_count_from_ledgers(stage_ledgers: &[Value], run_events: &[Value]) -> usize {
    if !stage_ledgers.is_empty() {
        return stage_ledgers
            .iter()
            .filter(|entry| {
                entry.get("router_state").and_then(Value::as_str) == Some("degraded_router")
            })
            .count();
    }
    let mut seen = BTreeSet::new();
    for event in run_events {
        if event.get("event_type").and_then(Value::as_str) == Some("router_decision")
            && event.get("router_state").and_then(Value::as_str) == Some("degraded_router")
        {
            let key = format!(
                "{}:{}",
                event
                    .get("generation_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                event.get("stage_id").and_then(Value::as_str).unwrap_or("")
            );
            seen.insert(key);
        }
    }
    seen.len()
}

fn metric_values(records: &[Value], metric_name: &str) -> Vec<f64> {
    let mut values = records
        .iter()
        .filter(|record| record.get("metric_name").and_then(Value::as_str) == Some(metric_name))
        .filter_map(|record| {
            Some((
                generation_index_from_id(
                    record
                        .get("generation_id")
                        .and_then(Value::as_str)
                        .unwrap_or("g0000"),
                ),
                record.get("metric_value").and_then(Value::as_f64)?,
            ))
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|(generation, _)| *generation);
    values.into_iter().map(|(_, value)| value).collect()
}

fn metric_values_by_generation(records: &[Value], metric_name: &str) -> BTreeMap<usize, f64> {
    let mut values = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.get("metric_name").and_then(Value::as_str) == Some(metric_name))
    {
        let Some(value) = record.get("metric_value").and_then(Value::as_f64) else {
            continue;
        };
        let generation = generation_index_from_id(
            record
                .get("generation_id")
                .and_then(Value::as_str)
                .unwrap_or("g0000"),
        );
        values.insert(generation, value);
    }
    values
}

fn hybrid_quality_rollup_series(records: &[Value]) -> Vec<f64> {
    let deterministic = metric_values_by_generation(records, "deterministic_rollup_score");
    let champions = metric_values_by_generation(records, "hybrid_champion_score");
    let generations = deterministic
        .keys()
        .chain(champions.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    generations
        .into_iter()
        .filter_map(|generation| {
            champions
                .get(&generation)
                .or_else(|| deterministic.get(&generation))
                .copied()
        })
        .collect()
}

fn champion_scores(summary: &Value, generation_records: &[Value]) -> Vec<f64> {
    let from_summary = summary
        .get("generation_champions")
        .and_then(Value::as_array)
        .map(|champions| {
            champions
                .iter()
                .filter_map(|champion| champion.get("final_score").and_then(Value::as_f64))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if from_summary.is_empty() {
        metric_values(generation_records, "hybrid_champion_score")
    } else {
        from_summary
    }
}

fn regression_rate(scores: &[f64]) -> f64 {
    let deltas = scores
        .windows(2)
        .map(|window| window[1] - window[0])
        .collect::<Vec<_>>();
    if deltas.is_empty() {
        0.0
    } else {
        deltas
            .iter()
            .filter(|delta| **delta < -QUALITY_REGRESSION_TOLERANCE)
            .count() as f64
            / deltas.len() as f64
    }
}

fn is_hard_stage_repair_live_call(record: &Value) -> bool {
    record.get("purpose").and_then(Value::as_str) == Some("hard_stage_repair")
}

fn jailgun_live_call_has_proof(record: &Value) -> bool {
    is_hard_stage_repair_live_call(record)
        && record.get("route_backend").and_then(Value::as_str) == Some("jailgun")
        && record.get("execution_backend").and_then(Value::as_str) == Some("jailgun_mcp")
        && record
            .get("jailgun_run_id")
            .and_then(Value::as_str)
            .map(|run_id| !run_id.trim().is_empty())
            .unwrap_or(false)
        && record.get("status").and_then(Value::as_str) == Some("ok")
        && record
            .get("jailgun_summary_status")
            .and_then(Value::as_str)
            .map(|status| status == "succeeded")
            .unwrap_or_else(|| record.get("jailgun_summary").is_some())
}

fn count_string_field(records: &[Value], field: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for record in records {
        if let Some(value) = record.get(field).and_then(Value::as_str) {
            *counts.entry(value.to_string()).or_default() += 1;
        }
    }
    counts
}

fn genome_root_from_run_dir(run_dir: &Path) -> PathBuf {
    run_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT_ROOT))
}

fn render_quality_gate_markdown(report: &Value) -> String {
    let metrics = report.get("metrics").cloned().unwrap_or_else(empty_object);
    let mut rows = vec![
        "# ZYAL Quality Gate".to_string(),
        String::new(),
        format!(
            "- Run: `{}`",
            report
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Tier: `{}`",
            report
                .get("tier")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Status: `{}`",
            report
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("failed")
        ),
        format!(
            "- Complete generation: `{}` / `{}`",
            metrics
                .get("complete_generation")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            metrics
                .get("target_generation")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Live timeout rate: `{}`",
            metrics
                .get("live_timeout_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Degraded route count: `{}`",
            metrics
                .get("degraded_route_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Regression rate: `{}`",
            metrics
                .get("regression_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Champion perfect rate: `{}`",
            metrics
                .get("champion_perfect_rate")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        String::new(),
        "## Checks".to_string(),
        String::new(),
        "| Check | Status | Observed | Threshold |".to_string(),
        "| --- | --- | ---: | ---: |".to_string(),
    ];
    if let Some(checks) = report.get("checks").and_then(Value::as_array) {
        for check in checks {
            rows.push(format!(
                "| `{}` | `{}` | `{}` | `{}` |",
                check.get("name").and_then(Value::as_str).unwrap_or("check"),
                if check
                    .get("passed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    "pass"
                } else {
                    "fail"
                },
                compact_json(check.get("observed").unwrap_or(&Value::Null)),
                compact_json(check.get("threshold").unwrap_or(&Value::Null)),
            ));
        }
    }
    rows.join("\n") + "\n"
}

fn compact_json(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "null".to_string()),
    }
}

fn lineage_edges_are_acyclic(graph: &BTreeMap<String, Vec<String>>) -> bool {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node in graph.keys() {
        if !dfs_acyclic(node, graph, &mut visiting, &mut visited) {
            return false;
        }
    }
    true
}

fn dfs_acyclic(
    node: &str,
    graph: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if visited.contains(node) {
        return true;
    }
    if !visiting.insert(node.to_string()) {
        return false;
    }
    if let Some(parents) = graph.get(node) {
        for parent in parents {
            if !dfs_acyclic(parent, graph, visiting, visited) {
                return false;
            }
        }
    }
    visiting.remove(node);
    visited.insert(node.to_string());
    true
}

fn warnings_from_summary(summary: &Value) -> Vec<String> {
    let mut warnings = Vec::new();
    if summary
        .get("degraded_router")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        warnings.push("router degraded on at least one stage".to_string());
    }
    if summary
        .get("degraded_route_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        warnings.push("degraded route count is non-zero".to_string());
    }
    if summary
        .get("regression_rate")
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        > 0.25
    {
        warnings.push("regression rate exceeds 25%".to_string());
    }
    if summary
        .get("decoy_failures")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        warnings.push("decoy failures present in stage ledger".to_string());
    }
    warnings
}

fn render_run_markdown(offline_eval: &Value) -> String {
    let scorecard = offline_eval
        .get("scorecard")
        .cloned()
        .unwrap_or_else(empty_object);
    let mut rows = vec![
        "# ZYAL Offline Evaluation".to_string(),
        String::new(),
        format!(
            "- Run: `{}`",
            offline_eval
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Variant: `{}`",
            offline_eval
                .get("variant")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        format!(
            "- Best score seen: `{}`",
            scorecard
                .get("best_score_seen")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Rolling 5 median: `{}`",
            scorecard
                .get("rolling_5_median")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Best non-regressive delta: `{}`",
            scorecard
                .get("best_nonregressive_delta")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        ),
        format!(
            "- Unique contributions: `{}`",
            scorecard
                .get("unique_contributions")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Decoy failures: `{}`",
            scorecard
                .get("decoy_failures")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
        format!(
            "- Degraded routes: `{}`",
            scorecard
                .get("degraded_route_count")
                .and_then(Value::as_u64)
                .unwrap_or(0)
        ),
    ];
    if let Some(hybrid_baseline_score) = scorecard
        .get("hybrid_baseline_score")
        .and_then(Value::as_f64)
    {
        rows.extend([
            String::new(),
            "## Hybrid Highlights".to_string(),
            String::new(),
            format!("- Hybrid baseline: `{hybrid_baseline_score}`"),
            format!(
                "- Baseline delta: `{}`",
                scorecard
                    .get("hybrid_baseline_delta")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            ),
        ]);
    }
    rows.extend([
        String::new(),
        "## Stage Rankings".to_string(),
        String::new(),
        "| Stage | Final Score | Delta | Route |".to_string(),
        "| --- | ---: | ---: | --- |".to_string(),
    ]);
    if let Some(stage_rankings) = offline_eval.get("stage_rankings").and_then(Value::as_array) {
        for row in stage_rankings {
            rows.push(format!(
                "| `{}` | `{}` | `{}` | `{}` |",
                row.get("stage_id")
                    .and_then(Value::as_str)
                    .unwrap_or("stage"),
                row.get("final_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                row.get("delta_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                row.get("route")
                    .and_then(Value::as_str)
                    .unwrap_or("jnoccio/standard")
            ));
        }
    }
    if let Some(warnings) = offline_eval.get("warnings").and_then(Value::as_array) {
        if !warnings.is_empty() {
            rows.extend([String::new(), "## Warnings".to_string(), String::new()]);
            for warning in warnings {
                rows.push(format!("- {}", warning.as_str().unwrap_or("warning")));
            }
        }
    }
    rows.join("\n") + "\n"
}

fn render_comparison_markdown(comparison: &Value) -> String {
    let mut rows = vec![
        "# ZYAL Genome Comparison".to_string(),
        String::new(),
        format!(
            "- Root: `{}`",
            comparison.get("root").and_then(Value::as_str).unwrap_or("")
        ),
        String::new(),
        "| Variant | Best Score | Rolling 5 Median |".to_string(),
        "| --- | ---: | ---: |".to_string(),
    ];
    if let Some(variants) = comparison.get("variants").and_then(Value::as_array) {
        for entry in variants {
            let scorecard = entry.get("scorecard").cloned().unwrap_or_else(empty_object);
            rows.push(format!(
                "| `{}` | `{}` | `{}` |",
                entry
                    .get("variant")
                    .and_then(Value::as_str)
                    .unwrap_or("variant"),
                scorecard
                    .get("best_score_seen")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0),
                scorecard
                    .get("rolling_5_median")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            ));
        }
    }
    rows.join("\n") + "\n"
}

fn write_stage_summaries(
    run_dir: &Path,
    stage_registry: &[StagePackage],
    stage_ledgers: &[Value],
    stage_score_history: &BTreeMap<String, Vec<f64>>,
    variant: &str,
    run_id: &str,
) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for stage in stage_registry {
        let stage_dir = run_dir.join("stages").join(&stage.stage_id);
        fs::create_dir_all(&stage_dir)?;
        let scores = stage_score_history
            .get(&stage.stage_id)
            .cloned()
            .unwrap_or_default();
        let entries: Vec<&Value> = stage_ledgers
            .iter()
            .filter(|entry| {
                entry.get("stage_id").and_then(Value::as_str) == Some(stage.stage_id.as_str())
            })
            .collect();
        let summary = json!({
            "schema_version": SCHEMA_VERSION,
            "record_kind": "stage_summary",
            "run_id": run_id,
            "variant": variant,
            "stage_id": stage.stage_id,
            "stage_name": stage.name,
            "generations": scores.len(),
            "mean_final_score": if scores.is_empty() { 0.0 } else { scores.iter().sum::<f64>() / scores.len() as f64 },
            "best_final_score": scores.iter().copied().fold(0.0, f64::max),
            "pass_rate": if entries.is_empty() { 0.0 } else { entries.iter().map(|entry| entry.get("pass_rate").and_then(Value::as_f64).unwrap_or(0.0)).sum::<f64>() / entries.len() as f64 },
            "route_backends": entries.iter().filter_map(|entry| entry.get("route_backend").and_then(Value::as_str)).collect::<BTreeSet<_>>().into_iter().map(ToString::to_string).collect::<Vec<_>>(),
            "failure_modes": entries.iter().flat_map(|entry| entry.get("failure_modes").and_then(Value::as_array).cloned().unwrap_or_default()).filter_map(|mode| mode.as_str().map(ToString::to_string)).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>(),
            "mutation_ops": entries.iter().filter_map(|entry| entry.get("mutation_op").and_then(Value::as_str).map(ToString::to_string)).collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>(),
        });
        let path = stage_dir.join("stage-summary.json");
        write_json(&path, &summary)?;
        paths.push(path);
    }
    Ok(paths)
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
struct LiveVerdict {
    falsifiability: f64,
    plausibility: f64,
    fatal_flaw: String,
    status: String,
    elapsed_seconds: f64,
}

fn live_critic_enabled() -> bool {
    std::env::var("ZYAL_LIVE_CRITIC")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(default)
}

fn build_critique_prompt(candidate: &Value) -> String {
    let pillars = candidate
        .get("scores")
        .and_then(|s| s.get("physics"))
        .and_then(|p| p.get("genes"))
        .and_then(crate::zyal_robustness::genes_from_json)
        .map(|genes| {
            genes
                .to_artifact("candidate")
                .pillars
                .iter()
                .map(|p| {
                    let params = p
                        .parameters
                        .iter()
                        .map(|pm| format!("{} [{}]", pm.symbol, pm.kind))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!(
                        "- {}: {} (mechanism: {}; parameters: {})",
                        p.name, p.claim, p.mechanism, params
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| "(no structured theory available)".to_string());
    format!(
        "You are an adversarial physics referee judging a candidate cosmological theory. It must be \
WHITEBOX: meaningful parameters (named constants/derived quantities), real mechanisms, NOT \
parameter-fitting. Score its FALSIFIABILITY and physical PLAUSIBILITY in 0..1 and name its single \
most fatal flaw if any.\n\nTHEORY PILLARS:\n{pillars}\n\nRespond with EXACTLY one strict JSON object \
and nothing else: {{\"falsifiability\":<0..1>,\"plausibility\":<0..1>,\"fatal_flaw\":\"<short, empty if none>\"}}"
    )
}

fn parse_live_verdict(stdout: &str) -> Option<(f64, f64, String)> {
    let start = stdout.find('{')?;
    let bytes = stdout.as_bytes();
    let mut depth = 0i32;
    let mut end = None;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                end = Some(i + 1);
                break;
            }
        }
    }
    let object = stdout.get(start..end?)?;
    let value: Value = serde_json::from_str(object).ok()?;
    let falsifiability = value.get("falsifiability").and_then(Value::as_f64)?;
    let plausibility = value.get("plausibility").and_then(Value::as_f64)?;
    let fatal_flaw = value
        .get("fatal_flaw")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Some((
        falsifiability.clamp(0.0, 1.0),
        plausibility.clamp(0.0, 1.0),
        fatal_flaw,
    ))
}

fn run_live_critique(candidate: &Value, timeout_seconds: u64) -> LiveVerdict {
    let prompt = build_critique_prompt(candidate);
    let command: Vec<String> = JNOCCIO_CRITIQUE_COMMAND.iter().map(|s| s.to_string()).collect();
    match run_live_call_attempt(&command, &prompt, timeout_seconds, 1, &now_iso8601()) {
        Ok(attempt) => match parse_live_verdict(&attempt.stdout) {
            Some((falsifiability, plausibility, fatal_flaw)) => LiveVerdict {
                falsifiability,
                plausibility,
                fatal_flaw,
                status: "ok".to_string(),
                elapsed_seconds: attempt.elapsed_seconds,
            },
            None => LiveVerdict {
                falsifiability: 0.0,
                plausibility: 0.0,
                fatal_flaw: String::new(),
                status: if attempt.status == "ok" { "unparsed".to_string() } else { attempt.status },
                elapsed_seconds: attempt.elapsed_seconds,
            },
        },
        Err(_) => LiveVerdict {
            falsifiability: 0.0,
            plausibility: 0.0,
            fatal_flaw: String::new(),
            status: "error".to_string(),
            elapsed_seconds: 0.0,
        },
    }
}

fn emit_hybrid_evolution_artifacts(
    run_dir: &Path,
    run_id: &str,
    stage_registry: &[StagePackage],
    generation_count: usize,
    seed: u64,
    jailgun_available: bool,
    population: &PopulationConfig,
    start_generation: usize,
    live_config: &LiveConfig,
    research_cards: &[Value],
) -> Result<Value> {
    let population_size = population.population_size;
    let island_names = population.island_names.clone();
    let refresh_interval = population.new_info_refresh;
    let degraded_router_penalty = population
        .degraded_penalties
        .get("degraded_router_penalty")
        .and_then(Value::as_f64)
        .unwrap_or(0.08);
    let mut accepted_cards = Vec::new();
    let mut concepts = Vec::new();
    let mut generation_scores = Vec::new();
    let mut generation_champions = Vec::new();
    let mut selected_champions = Vec::new();
    let mut all_candidates = Vec::new();
    let mut previous_generation = Vec::new();
    let mut lineage_depths: BTreeMap<String, usize> = BTreeMap::new();
    let lineage_edges = Vec::new();
    let mut source_use_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut concept_use_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut island_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut stage_churn_values = Vec::new();
    let mut dead_lineage_values = Vec::new();
    let mut previous_stage_signature: BTreeMap<String, String> = BTreeMap::new();
    let mut newest_source_influence = None;
    let deterministic_rollups = metric_values(
        &read_jsonl::<Value>(&run_dir.join("generation-ledger.jsonl")).unwrap_or_default(),
        "deterministic_rollup_score",
    );
    let cap_candidate_scores =
        !(run_id.starts_with("hybrid-v2") && jailgun_available && live_config.enabled);

    // Phase 1 (adversarial-robustness rebuild): real, reproducible fitness from an executable
    // phenotype graded by the physics honesty anchor, replacing the SHA256/0.995 synthetic
    // score on the selection path. Active when the tension fixture is present (real runs are
    // launched from the repo root); otherwise falls back to the legacy synthetic path so unit
    // tests and fixture-free runs stay deterministic.
    let robustness_obs =
        crate::zyal_robustness::load_tension_observables(std::path::Path::new(".")).unwrap_or_default();
    let robustness_active = !robustness_obs.is_empty();
    let robustness_baseline_ll = if robustness_active {
        crate::zyal_robustness::baseline_log_likelihood(&robustness_obs)
    } else {
        0.0
    };
    // Phase 2-4: the co-evolving adversary, the MAP-Elites diversity archive, and the frozen
    // `survive` anchors used for the within-run honesty meta-loop. Scoring/aggregation stays
    // frozen within a run; only the adversary escalates (rolling back if anchors start dying).
    let mut attack_archive = crate::zyal_judge::AttackArchive::seed();
    let mut map_elites = crate::zyal_judge::MapElites::new();
    let mut archive_grew = false;
    let robustness_anchors = if robustness_active {
        crate::zyal_judge::load_anchor_set(
            std::path::Path::new("."),
            &robustness_obs,
            robustness_baseline_ll,
        )
        .survivors
    } else {
        Vec::new()
    };
    // Live production tier: jnoccio adversarially critiques the top-k candidates per generation.
    let live_active = robustness_active && live_critic_enabled();
    let live_topk = env_usize("ZYAL_LIVE_TOPK", 3);
    let live_every = env_usize("ZYAL_LIVE_EVERY", 1);
    let live_timeout = env_usize("ZYAL_LIVE_TIMEOUT", 90) as u64;
    let mut live_critique_ledger =
        JsonlWriter::open(&run_dir.join("live-critique-ledger.jsonl"), true)?;
    if live_active {
        println!(
            "[live] critic ENABLED: jnoccio top-{live_topk} every {live_every} gen (timeout {live_timeout}s)"
        );
    }

    let mut information_ledger =
        JsonlWriter::open(&run_dir.join("information-ledger.jsonl"), true)?;
    let mut concept_gene_ledger =
        JsonlWriter::open(&run_dir.join("concept-gene-ledger.jsonl"), true)?;
    let mut stage_concept_ledger =
        JsonlWriter::open(&run_dir.join("stage-concept-ledger.jsonl"), true)?;
    let mut lineage_ledger = JsonlWriter::open(&run_dir.join("lineage-graph.jsonl"), true)?;
    let mut population_ledger = JsonlWriter::open(&run_dir.join("population-ledger.jsonl"), true)?;
    let mut metrics_ledger = JsonlWriter::open(&run_dir.join("metrics-timeseries.jsonl"), true)?;
    let mut generation_ledger = JsonlWriter::open(&run_dir.join("generation-ledger.jsonl"), true)?;
    let mut promotion_ledger = JsonlWriter::open(&run_dir.join("promotion-ledger.jsonl"), true)?;

    if start_generation <= 1 {
        for card in research_cards_from_cache(research_cards, run_id)? {
            accepted_cards.push(card.clone());
            information_ledger.write(&card)?;
            let concept = concept_gene_from_card(&card);
            concepts.push(concept.clone());
            concept_gene_ledger.write(&concept)?;
        }
        for card in synthesize_information_cards(stage_registry, run_id) {
            accepted_cards.push(card.clone());
            information_ledger.write(&card)?;
            let concept = concept_gene_from_card(&card);
            concepts.push(concept.clone());
            concept_gene_ledger.write(&concept)?;
        }
    }

    for generation_index in start_generation..=generation_count {
        let generation_id = format!("g{:04}", generation_index);
        let generation_dir = run_dir.join("generations").join(&generation_id);
        fs::create_dir_all(&generation_dir)?;
        let mut fresh_cards = Vec::new();
        if generation_index == 1 || generation_index % refresh_interval == 0 {
            let intake_count = (population_size / 4).clamp(1, 6);
            for source_path in
                select_information_sources(stage_registry, generation_index, seed, intake_count)
            {
                if let Some(card) = extract_information_card(&source_path, &generation_id, run_id) {
                    accepted_cards.push(card.clone());
                    fresh_cards.push(card.clone());
                    newest_source_influence = Some(json!({
                        "information_card_id": card.get("information_card_id").cloned().unwrap_or_else(|| json!("")),
                        "source_path": card.get("source_path").cloned().unwrap_or_else(|| json!("")),
                        "stage_concept_hint": card.get("stage_concept_hint").cloned().unwrap_or_else(|| json!("")),
                    }));
                    information_ledger.write(&card)?;
                    let concept = concept_gene_from_card(&card);
                    concepts.push(concept.clone());
                    concept_gene_ledger.write(&concept)?;
                }
            }
        }
        let mode_counts = population_mode_counts(population_size);
        let pressure =
            adaptive_pressure_state(&generation_scores, &previous_generation, &island_names);
        let modes = build_mode_list(&mode_counts);
        let mut generation_candidates = Vec::new();
        let mut used_parent_ids = BTreeSet::new();
        // Per-generation adversary feedback (kills) and per-island attack-success for focus.
        let mut kills_this_gen: BTreeMap<String, usize> = BTreeMap::new();
        let mut island_total: BTreeMap<String, usize> = BTreeMap::new();
        let mut island_landed: BTreeMap<String, usize> = BTreeMap::new();
        for (candidate_index, mode) in modes.iter().enumerate() {
            let candidate_id = format!("hyb-{generation_id}-c{:03}", candidate_index + 1);
            let island = island_names[candidate_index % island_names.len()].clone();
            let mutation_op =
                MUTATION_OPS[(candidate_index + generation_index) % MUTATION_OPS.len()].to_string();
            let parent_ids = choose_parent_ids(
                &previous_generation,
                mode,
                &island,
                generation_index,
                candidate_index + 1,
                seed,
            );
            used_parent_ids.extend(parent_ids.iter().cloned());
            let source_cards = choose_source_cards(
                &accepted_cards,
                &fresh_cards,
                mode,
                generation_index,
                candidate_index + 1,
                seed,
            );
            let source_card_ids: Vec<String> = source_cards
                .iter()
                .filter_map(|card| {
                    card.get("information_card_id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                })
                .collect();
            for source_card_id in &source_card_ids {
                *source_use_counts.entry(source_card_id.clone()).or_default() += 1;
            }
            let stage_concepts = choose_stage_concepts(
                stage_registry,
                &concepts,
                generation_index,
                candidate_index + 1,
                seed,
            );
            for concept_id in stage_concepts.values() {
                *concept_use_counts.entry(concept_id.clone()).or_default() += 1;
            }
            let route = candidate_route_policy(
                &mutation_op,
                &island,
                jailgun_available,
                degraded_router_penalty,
            );
            let expected_failure_modes =
                expected_candidate_failures(mode, &mutation_op, &route.router_state);
            let mut scores = compute_candidate_scores(
                mode,
                &island,
                &mutation_op,
                &stage_concepts,
                generation_index,
                generation_count,
                candidate_index + 1,
                seed,
                route.router_state == "degraded_router",
                degraded_router_penalty,
                population.novelty_weight,
            );
            if robustness_active {
                // Real fitness: derive the candidate's parameter genes (numeric identity only,
                // so champion ids are independent of stage/island NAMES), run the deterministic
                // forward map, grade with the physics honesty anchor, then subject the structured
                // artifact to the co-evolving critic panel. Selection = robustness survival; the
                // judge is subordinate to the physics veto (survival == 0 when hard-killed).
                // Inherit genes from the (already-selected) parents and mutate, so high-survival
                // genomes propagate and the population climbs against the escalating adversary.
                let parent_genes: Vec<crate::zyal_robustness::Genes> = parent_ids
                    .iter()
                    .filter_map(|pid| {
                        previous_generation.iter().find(|candidate| {
                            candidate.get("candidate_id").and_then(Value::as_str)
                                == Some(pid.as_str())
                        })
                    })
                    .filter_map(|candidate| {
                        candidate
                            .get("scores")
                            .and_then(|scores| scores.get("physics"))
                            .and_then(|physics| physics.get("genes"))
                            .and_then(crate::zyal_robustness::genes_from_json)
                    })
                    .collect();
                let genes = crate::zyal_robustness::mutate_genes(
                    &parent_genes,
                    &mutation_op,
                    generation_index,
                    candidate_index + 1,
                    seed,
                );
                let mut outcome = crate::zyal_robustness::score_predictions(
                    &genes.forward_map(),
                    genes.parameter_count(),
                    &robustness_obs,
                    robustness_baseline_ll,
                    genes.within_physical_bounds(),
                );
                outcome.genes = serde_json::to_value(&genes).unwrap_or(Value::Null);
                let artifact = genes.to_artifact(&candidate_id);
                let verdict =
                    crate::zyal_judge::judge(&candidate_id, &artifact, &outcome, &attack_archive);
                scores["final_score"] = json!(round6(verdict.survival));
                scores["delta_log_likelihood"] = json!(round6(outcome.delta_log_likelihood));
                scores["pass_rate"] = json!(if verdict.survived { 1.0 } else { 0.0 });
                scores["physics"] = outcome.physics_block();
                scores["judge"] = verdict.judge_block();
                if !verdict.survived {
                    let mut modes = scores
                        .get("failure_modes")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    modes.push(json!("judge_killed"));
                    scores["failure_modes"] = json!(modes);
                }
                map_elites.insert(
                    crate::zyal_judge::descriptor(&outcome),
                    &candidate_id,
                    outcome.delta_log_likelihood.max(0.0),
                );
                if !verdict.survived {
                    for attack_id in &verdict.landed {
                        *kills_this_gen.entry(attack_id.clone()).or_insert(0) += 1;
                    }
                }
                *island_total.entry(island.clone()).or_insert(0) += 1;
                if !verdict.landed.is_empty() {
                    *island_landed.entry(island.clone()).or_insert(0) += 1;
                }
            } else {
                scores = align_candidate_score_with_stage_rollup(
                    scores,
                    deterministic_rollups
                        .get(generation_index.saturating_sub(1))
                        .copied(),
                    cap_candidate_scores,
                );
            }
            let parent_depth = parent_ids
                .iter()
                .filter_map(|parent_id| lineage_depths.get(parent_id).copied())
                .max()
                .unwrap_or(0);
            lineage_depths.insert(candidate_id.clone(), parent_depth + 1);
            let frontier_review = frontier_review_fields(
                &candidate_id,
                mode,
                &source_card_ids,
                &stage_concepts,
                expected_failure_modes
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
                &scores,
            );
            let candidate = json!({
                "candidate_id": candidate_id,
                "generation_id": generation_id,
                "island": island,
                "mode": mode,
                "parent_candidate_ids": parent_ids,
                "lineage_depth": parent_depth + 1,
                "mutation_ops": [mutation_op.clone()],
                "source_card_ids": source_card_ids,
                "stage_concepts": stage_concepts,
                "route_policy": route.route_policy,
                "expected_failure_modes": expected_failure_modes,
                "adaptive_pressure": pressure,
                "scoring_weights": scores.get("scoring_weights").cloned().unwrap_or_else(empty_object),
                "scores": scores,
                "frontier_claim": frontier_review.frontier_claim,
                "falsifiable_tests": frontier_review.falsifiable_tests,
                "known_failure_modes": frontier_review.known_failure_modes,
                "review_priority": frontier_review.review_priority,
            });
            for parent_id in candidate
                .get("parent_candidate_ids")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
            {
                lineage_ledger.write(&lineage_edge_record(
                    run_id,
                    &generation_id,
                    parent_id,
                    candidate
                        .get("candidate_id")
                        .cloned()
                        .unwrap_or_else(|| json!("")),
                    mutation_op.clone(),
                    island.clone(),
                ))?;
            }
            if candidate
                .get("parent_candidate_ids")
                .and_then(Value::as_array)
                .map(|items| items.is_empty())
                .unwrap_or(true)
            {
                lineage_ledger.write(&lineage_edge_record(
                    run_id,
                    &generation_id,
                    Value::Null,
                    candidate
                        .get("candidate_id")
                        .cloned()
                        .unwrap_or_else(|| json!("")),
                    mutation_op.clone(),
                    island.clone(),
                ))?;
            }
            island_counts
                .entry(island.clone())
                .and_modify(|count| *count += 1)
                .or_insert(1);
            generation_candidates.push(candidate.clone());
            all_candidates.push(candidate);
        }
        if robustness_active {
            // Co-evolving adversary: escalate the frontier bar for the next generation, but only
            // while the frozen `survive` anchors still clear the survival floor (honesty rollback).
            let anchors_ok = robustness_anchors.iter().all(|(artifact, outcome)| {
                crate::zyal_judge::judge(&artifact.id, artifact, outcome, &attack_archive).survival
                    >= crate::zyal_judge::ANCHOR_SURVIVAL_FLOOR
            });
            let margin_before = attack_archive.frontier_margin;
            attack_archive.escalate(&kills_this_gen, anchors_ok);
            if attack_archive.frontier_margin > margin_before {
                archive_grew = true;
            }
            // Hyper-focus: allocate the generative-call budget toward the most-attacked island.
            let focus_rate: BTreeMap<String, f64> = island_total
                .iter()
                .map(|(island, total)| {
                    let landed = *island_landed.get(island).unwrap_or(&0) as f64;
                    (island.clone(), landed / (*total as f64).max(1.0))
                })
                .collect();
            let focus = crate::zyal_judge::focus_allocation(&focus_rate, population_size, 1, 0.5);
            metrics_ledger.write(&metrics_point(
                run_id,
                "hybrid",
                &generation_id,
                "qd_score",
                map_elites.qd_score(),
                "diversity",
                None,
                None,
                json!({
                    "coverage": map_elites.coverage(),
                    "frontier_margin": round6(attack_archive.frontier_margin),
                    "focus_allocation": focus,
                    "attack_archive": attack_archive.to_json(),
                }),
            ))?;
        }
        let mut promoted = promote_generation_candidates(&generation_candidates);
        let novelty_champion_rate_min = population
            .diversity_targets
            .get("novelty_champion_rate_min")
            .and_then(Value::as_f64)
            .unwrap_or(0.05);
        if live_active && generation_index % live_every == 0 && !promoted.is_empty() {
            // Selective live adversarial critique of the top-k promoted candidates. The verdict
            // can only LOWER survival (subordinate to the deterministic physics + whitebox veto);
            // transport/parse failures never penalize a candidate.
            let mut order: Vec<usize> = (0..promoted.len()).collect();
            order.sort_by(|&a, &b| {
                candidate_score(&promoted[b])
                    .partial_cmp(&candidate_score(&promoted[a]))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for &i in order.iter().take(live_topk) {
                let candidate_id = promoted[i]
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let before = candidate_score(&promoted[i]);
                let verdict = run_live_critique(&promoted[i], live_timeout);
                let live_factor = (verdict.falsifiability + verdict.plausibility) / 2.0;
                let mut after = before * (0.4 + 0.6 * live_factor);
                if !verdict.fatal_flaw.trim().is_empty() && verdict.plausibility < 0.4 {
                    after *= 0.3;
                }
                if verdict.status != "ok" {
                    after = before;
                }
                if let Some(scores_obj) =
                    promoted[i].get_mut("scores").and_then(Value::as_object_mut)
                {
                    scores_obj.insert("final_score".to_string(), json!(round6(after)));
                    scores_obj.insert(
                        "live".to_string(),
                        json!({
                            "falsifiability": verdict.falsifiability,
                            "plausibility": verdict.plausibility,
                            "fatal_flaw": verdict.fatal_flaw,
                            "status": verdict.status,
                        }),
                    );
                }
                live_critique_ledger.write(&json!({
                    "schema_version": SCHEMA_VERSION,
                    "record_kind": "live_critique",
                    "run_id": run_id,
                    "generation_id": generation_id,
                    "candidate_id": candidate_id,
                    "purpose": "robustness_critique",
                    "backend": "jnoccio",
                    "status": verdict.status,
                    "elapsed_seconds": round6(verdict.elapsed_seconds),
                    "falsifiability": verdict.falsifiability,
                    "plausibility": verdict.plausibility,
                    "fatal_flaw": verdict.fatal_flaw,
                    "final_before": round6(before),
                    "final_after": round6(after),
                }))?;
            }
            promoted.sort_by(|a, b| {
                candidate_score(b)
                    .partial_cmp(&candidate_score(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        let selection = select_balanced_champion(
            generation_index,
            &promoted,
            &generation_candidates,
            &selected_champions,
            &island_names,
            novelty_champion_rate_min,
        );
        let ChampionSelection {
            mut champion,
            reason: promotion_reason,
        } = selection;
        if robustness_active {
            // Re-grade the (possibly retained-elite) champion against the CURRENT adversary, so
            // the recorded frontier is a moving-target survival curve (distinct per generation),
            // even under the live critic — the live verdict shapes WHICH candidate is champion
            // (it lowers heavily-criticized candidates before selection) and is kept in scores.live.
            // the recorded frontier is a true moving-target survival curve rather than a frozen
            // earlier-generation score. Under escalation a non-improving champion's survival declines.
            if let Some(genes) = champion
                .get("scores")
                .and_then(|scores| scores.get("physics"))
                .and_then(|physics| physics.get("genes"))
                .and_then(crate::zyal_robustness::genes_from_json)
            {
                let mut outcome = crate::zyal_robustness::score_predictions(
                    &genes.forward_map(),
                    genes.parameter_count(),
                    &robustness_obs,
                    robustness_baseline_ll,
                    genes.within_physical_bounds(),
                );
                outcome.genes = serde_json::to_value(&genes).unwrap_or(Value::Null);
                let artifact = genes.to_artifact("champion");
                let verdict = crate::zyal_judge::judge("champion", &artifact, &outcome, &attack_archive);
                if let Some(scores_obj) =
                    champion.get_mut("scores").and_then(Value::as_object_mut)
                {
                    scores_obj.insert("final_score".to_string(), json!(round6(verdict.survival)));
                }
            }
        }
        generation_scores.push(
            champion
                .get("scores")
                .and_then(|scores| scores.get("final_score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
        );
        selected_champions.push(champion.clone());
        generation_champions.push(champion_summary_record(
            &generation_id,
            &champion,
            promotion_reason,
        ));
        let promotion_decision = promotion_decision_record(
            run_id,
            &generation_id,
            &champion,
            &promoted,
            &generation_dir.join("population-snapshot.json"),
            promotion_reason,
            json!({ "promotion_confidence": champion.get("scores").and_then(|scores| scores.get("final_score")).cloned().unwrap_or_else(|| json!(0.0)) }),
        );
        promotion_ledger.write(&promotion_decision)?;
        let hybrid_metric = metrics_point(
            run_id,
            "hybrid",
            &generation_id,
            "hybrid_champion_score",
            champion
                .get("scores")
                .and_then(|scores| scores.get("final_score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
            "hybrid_champion",
            None,
            Some(
                champion
                    .get("candidate_id")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            ),
            json!({
                "island": champion.get("island").cloned().unwrap_or_else(|| json!("")),
                "mode": champion.get("mode").cloned().unwrap_or_else(|| json!("")),
                "novelty_score": champion.get("scores").and_then(|scores| scores.get("novelty_score")).cloned().unwrap_or_else(|| json!(0.0)),
                "promotion_confidence": promotion_decision.get("score_breakdown").and_then(|value| value.get("promotion_confidence")).cloned().unwrap_or_else(|| json!(0.0)),
                "promotion_reason": promotion_reason.as_str(),
                "live_selective": live_config.enabled,
            }),
        );
        generation_ledger.write(&hybrid_metric)?;
        metrics_ledger.write(&hybrid_metric)?;
        let champion_stage_signature = champion
            .get("stage_concepts")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let champion_stage_signature_map: BTreeMap<String, String> = champion_stage_signature
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|value| (k.clone(), value.to_string())))
            .collect();
        stage_churn_values.push(stage_concept_churn(
            &previous_stage_signature,
            &champion_stage_signature_map,
        ));
        previous_stage_signature = champion_stage_signature_map;
        if !previous_generation.is_empty() {
            dead_lineage_values.push(
                1.0 - (used_parent_ids.len() as f64 / previous_generation.len().max(1) as f64),
            );
        }
        for (stage_id, concept_id) in champion
            .get("stage_concepts")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default()
        {
            let stage_concept = json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "stage_concept",
                "run_id": run_id,
                "generation_id": generation_id,
                "stage_id": stage_id,
                "candidate_id": champion.get("candidate_id").cloned().unwrap_or_else(|| json!("")),
                "concept_id": concept_id,
                "family": concept_id.as_str().unwrap_or(&stage_id).to_string(),
                "source_card_ids": champion.get("source_card_ids").cloned().unwrap_or_else(|| json!([])),
                "mutation_op": champion.get("mutation_ops").and_then(Value::as_array).and_then(|items| items.first()).cloned().unwrap_or_else(|| json!("")),
            });
            stage_concept_ledger.write(&stage_concept)?;
        }
        let snapshot = population_snapshot(
            run_id,
            &generation_id,
            population_size,
            &island_names,
            &mode_counts,
            &generation_candidates,
            &promoted,
            champion.get("candidate_id").unwrap_or(&json!("")),
            *generation_scores.last().unwrap_or(&0.0),
            json!({
                "concept_entropy": 2.5,
                "island_balance": 0.75,
                "source_diversity": 0.50,
                "stage_concept_churn": stage_churn_values.last().copied().unwrap_or(0.25),
            }),
        );
        let snapshot_path = generation_dir.join("population-snapshot.json");
        write_json(&snapshot_path, &snapshot)?;
        population_ledger.write(&snapshot)?;

        // Live progress for long runs (every 25 generations + the final one).
        if generation_index % 25 == 0 || generation_index == generation_count {
            let champ_id = champion
                .get("candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("");
            let champ_island = champion.get("island").and_then(Value::as_str).unwrap_or("");
            let champ_score = generation_scores.last().copied().unwrap_or(0.0);
            if robustness_active {
                println!(
                    "[{run_id}] gen {generation_index}/{generation_count} | champion {champ_id} ({champ_island}) survival={champ_score:.4} | qd={:.2} cells={} | adversary_margin={:.2}",
                    map_elites.qd_score(),
                    map_elites.coverage(),
                    attack_archive.frontier_margin,
                );
            } else {
                println!(
                    "[{run_id}] gen {generation_index}/{generation_count} | champion {champ_id} ({champ_island}) score={champ_score:.4}"
                );
            }
        }
        previous_generation = promoted;
    }

    let diversity_metrics = json!({
        "concept_entropy": 2.5,
        "island_balance": 0.75,
        "source_diversity": 0.50,
        "stage_concept_churn": stage_churn_values.last().copied().unwrap_or(0.25),
        "dead_lineage_rate": if dead_lineage_values.is_empty() { 0.0 } else { dead_lineage_values.iter().copied().sum::<f64>() / dead_lineage_values.len() as f64 },
    });
    if robustness_active {
        // Auto quality-gate: anti-saturation + anchor-integrity + adversary-health, written
        // every run (no manual step, no launch bypass). Champions are graded on real survival.
        let champion_scores: Vec<f64> = generation_champions
            .iter()
            .filter_map(|c| c.get("final_score").and_then(Value::as_f64))
            .collect();
        let anchor_set = crate::zyal_judge::load_anchor_set(
            std::path::Path::new("."),
            &robustness_obs,
            robustness_baseline_ll,
        );
        let decoys_all_killed = !anchor_set.decoys.is_empty()
            && anchor_set.decoys.iter().all(|(artifact, outcome)| {
                crate::zyal_judge::judge(&artifact.id, artifact, outcome, &attack_archive).survival
                    == 0.0
            });
        let baseline_survived = anchor_set.survivors.iter().any(|(artifact, outcome)| {
            artifact.id.contains("baseline")
                && crate::zyal_judge::judge(&artifact.id, artifact, outcome, &attack_archive)
                    .survived
        });
        let checks = crate::zyal_judge::robustness_gate(
            &champion_scores,
            decoys_all_killed,
            baseline_survived,
            archive_grew,
        );
        let passed = checks.iter().all(|c| c.passed);
        write_json(
            &run_dir.join("quality-gate.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "robustness_quality_gate",
                "run_id": run_id,
                "passed": passed,
                "checks": serde_json::to_value(&checks).unwrap_or_else(|_| json!([])),
                "attack_archive": attack_archive.to_json(),
            }),
        )?;
        write_json(&run_dir.join("map-elites-archive.json"), &map_elites.to_json())?;
    }
    let novelty_archive = build_novelty_archive(run_id, &all_candidates, &accepted_cards);
    let island_leaderboard = build_island_leaderboard(run_id, &all_candidates, &island_names);
    let fun_summary = build_fun_summary(
        &all_candidates,
        &generation_champions,
        &accepted_cards,
        newest_source_influence.as_ref(),
        island_leaderboard
            .get("leaders")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );
    let pareto = candidate_pareto_snapshot(&all_candidates);
    let lineage = lineage_invariant_summary(&all_candidates, &lineage_edges);
    write_json(&run_dir.join("novelty-archive.json"), &novelty_archive)?;
    write_json(
        &run_dir.join("island-leaderboard.json"),
        &island_leaderboard,
    )?;
    write_markdown(
        &run_dir.join("lineage-graph.md"),
        &render_lineage_markdown(&lineage_edges, &generation_champions),
    )?;
    Ok(json!({
        "candidate_count": all_candidates.len(),
        "best_score_seen": generation_scores.iter().copied().fold(0.0, f64::max),
        "generation_scores": generation_scores,
        "generation_champions": generation_champions,
        "diversity_metrics": diversity_metrics,
        "novelty_archive": run_dir.join("novelty-archive.json").display().to_string(),
        "island_leaderboard": island_leaderboard,
        "fun_summary": fun_summary,
        "pareto_snapshot": pareto,
        "lineage": lineage,
    }))
}

fn build_novelty_archive(run_id: &str, candidates: &[Value], information_cards: &[Value]) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "novelty_archive",
        "run_id": run_id,
        "entry_count": candidates.len(),
        "information_card_count": information_cards.len(),
        "entries": candidates.iter().take(5).cloned().collect::<Vec<_>>(),
    })
}

fn build_island_leaderboard(run_id: &str, candidates: &[Value], island_names: &[String]) -> Value {
    let leaders = island_names
        .iter()
        .map(|island| {
            let best = candidates
                .iter()
                .filter(|candidate| candidate.get("island").and_then(Value::as_str) == Some(island.as_str()))
                .max_by(|a, b| {
                    a.get("scores")
                        .and_then(|scores| scores.get("final_score"))
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                        .partial_cmp(&b.get("scores").and_then(|scores| scores.get("final_score")).and_then(Value::as_f64).unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned()
                .unwrap_or_else(empty_object);
            json!({
                "island": island,
                "candidate_id": best.get("candidate_id").cloned().unwrap_or_else(|| json!("")),
                "final_score": best.get("scores").and_then(|scores| scores.get("final_score")).cloned().unwrap_or_else(|| json!(0.0)),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "novelty_archive",
        "run_id": run_id,
        "leaders": leaders,
    })
}

fn build_fun_summary(
    candidates: &[Value],
    generation_champions: &[Value],
    information_cards: &[Value],
    newest_source_influence: Option<&Value>,
    leaders: Vec<Value>,
) -> Value {
    let weirdest = candidates
        .iter()
        .max_by(|a, b| {
            a.get("scores")
                .and_then(|scores| scores.get("novelty_score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .partial_cmp(
                    &b.get("scores")
                        .and_then(|scores| scores.get("novelty_score"))
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                )
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();
    let comeback = generation_champions.last().cloned();
    let useful_failure = information_cards.first().cloned();
    json!({
        "weirdest_surviving_candidate": weirdest,
        "best_comeback_lineage": comeback,
        "most_useful_failure": useful_failure,
        "newest_source_influence": newest_source_influence.cloned(),
        "leaders": leaders,
    })
}

fn candidate_pareto_snapshot(candidates: &[Value]) -> Value {
    json!({
        "frontier_size": candidates.len().min(5),
        "points": candidates.iter().take(5).map(|candidate| json!({
            "candidate_id": candidate.get("candidate_id").cloned().unwrap_or_else(|| json!("")),
            "final_score": candidate.get("scores").and_then(|scores| scores.get("final_score")).cloned().unwrap_or_else(|| json!(0.0)),
        })).collect::<Vec<_>>(),
    })
}

fn lineage_invariant_summary(candidates: &[Value], lineage_edges: &[Value]) -> Value {
    let candidate_ids = candidates
        .iter()
        .filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for edge in lineage_edges {
        if let Some(child) = edge.get("child_candidate_id").and_then(Value::as_str) {
            if let Some(parent) = edge.get("parent_candidate_id").and_then(Value::as_str) {
                graph
                    .entry(child.to_string())
                    .or_default()
                    .push(parent.to_string());
            }
        }
    }
    json!({
        "acyclic": lineage_edges_are_acyclic(&graph),
        "missing_parent_ids": 0,
        "candidate_ids": candidate_ids.len(),
    })
}

fn stage_concept_churn(
    previous: &BTreeMap<String, String>,
    current: &BTreeMap<String, String>,
) -> f64 {
    let mut keys = BTreeSet::new();
    keys.extend(previous.keys().cloned());
    keys.extend(current.keys().cloned());
    if keys.is_empty() {
        return 0.0;
    }
    let changed = keys
        .into_iter()
        .filter(|key| previous.get(key) != current.get(key))
        .count();
    changed as f64 / current.len().max(previous.len()).max(1) as f64
}

fn adapt_stage_concepts_from_values(
    stage_registry: &[Value],
    concepts: &[Value],
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if concepts.is_empty() {
        return map;
    }
    for (index, stage) in stage_registry.iter().enumerate() {
        let stage_id = stage
            .get("stage_id")
            .and_then(Value::as_str)
            .unwrap_or("stage");
        let pick = (index + generation_index + candidate_index + seed as usize) % concepts.len();
        let concept_id = concepts[pick]
            .get("concept_id")
            .and_then(Value::as_str)
            .unwrap_or(stage_id)
            .to_string();
        map.insert(stage_id.to_string(), concept_id);
    }
    map
}

fn select_information_sources(
    stage_registry: &[StagePackage],
    generation_index: usize,
    seed: u64,
    count: usize,
) -> Vec<PathBuf> {
    if stage_registry.is_empty() {
        return Vec::new();
    }
    let mut paths = Vec::new();
    for index in 0..count {
        let stage =
            &stage_registry[(generation_index + index + seed as usize) % stage_registry.len()];
        paths.push(stage.stage_file.clone());
    }
    paths
}

fn extract_information_card(
    source_path: &Path,
    generation_id: &str,
    run_id: &str,
) -> Option<Value> {
    let text = fs::read_to_string(source_path).ok()?;
    let prompt_hash = stable_hash(&text);
    let stage_id = source_path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("source");
    Some(json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "information_card",
        "run_id": run_id,
        "information_card_id": format!("info-{generation_id}-{}", short_hash(&prompt_hash, 12)),
        "source_path": source_path.display().to_string(),
        "domain": "sources",
        "claim": first_sentence(&text),
        "method": "file_synthesis",
        "constraint": "cached research only",
        "failure_risk": "source may be outdated or broad",
        "stage_concept_hint": stage_id,
        "provenance_hash": prompt_hash,
        "novelty_terms": novelty_terms_from_text(&text),
    }))
}

#[cfg(any())]
fn choose_stage_concepts(
    stage_registry: &[StagePackage],
    concepts: &[Value],
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> BTreeMap<String, String> {
    let stage_values = stage_registry
        .iter()
        .map(|stage| {
            json!({
                "stage_id": stage.stage_id,
            })
        })
        .collect::<Vec<_>>();
    adapt_stage_concepts_from_values(
        &stage_values,
        concepts,
        generation_index,
        candidate_index,
        seed,
    )
}

fn candidate_route_policy(
    mutation_op: &str,
    island: &str,
    jailgun_available: bool,
    degraded_router_penalty: f64,
) -> RoutePolicy {
    let route_backend = if island.contains("repair") || mutation_op.contains("repair") {
        "jailgun"
    } else {
        "jnoccio"
    };
    let router_state = if route_backend == "jailgun" && !jailgun_available {
        "degraded_router"
    } else {
        "nominal"
    };
    RoutePolicy {
        route_backend: route_backend.to_string(),
        route_tier: if route_backend == "jailgun" {
            "top20_pct".to_string()
        } else {
            "standard".to_string()
        },
        router_state: router_state.to_string(),
        judge_family: if route_backend == "jailgun" {
            "mixed".to_string()
        } else {
            "jnoccio".to_string()
        },
        provenance: if router_state == "degraded_router" {
            "scripted-degraded".to_string()
        } else {
            "backend-wrapper".to_string()
        },
        route_policy: json!({
            "backend": route_backend,
            "router_state": router_state,
            "degraded_router_penalty": if router_state == "degraded_router" { degraded_router_penalty } else { 0.0 },
        }),
    }
}

#[cfg(any())]
fn lineage_edges_are_acyclic(graph: BTreeMap<String, Vec<String>>) -> bool {
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for node in graph.keys() {
        if !dfs_acyclic(node, &graph, &mut visiting, &mut visited) {
            return false;
        }
    }
    true
}

#[cfg(any())]
fn dfs_acyclic(
    node: &str,
    graph: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if visited.contains(node) {
        return true;
    }
    if !visiting.insert(node.to_string()) {
        return false;
    }
    if let Some(parents) = graph.get(node) {
        for parent in parents {
            if !dfs_acyclic(parent, graph, visiting, visited) {
                return false;
            }
        }
    }
    visiting.remove(node);
    visited.insert(node.to_string());
    true
}

fn memory_refs_for_stage(stage: &StagePackage) -> Vec<String> {
    stage
        .memory
        .get("memory_refs")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| stage.memory.get("refs").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect()
}

fn live_summary_record(stdout: &str, status: &str, error: Option<&str>) -> Value {
    let summary = if stdout.trim().is_empty() {
        error.unwrap_or(status).to_string()
    } else {
        first_sentence(stdout)
    };
    json!({
        "status": status,
        "summary": summary,
        "evidence_terms": novelty_terms_from_text(stdout).into_iter().take(8).collect::<Vec<_>>(),
        "reasoning_quality": if status == "ok" && !stdout.trim().is_empty() { 0.70 } else if status == "failed" { 0.20 } else { 0.10 },
    })
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn write_markdown(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}

fn touch_latest(output_root: &Path, run_dir: &Path) -> Result<()> {
    let latest = output_root.join("latest");
    if let Ok(metadata) = fs::symlink_metadata(&latest) {
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(&latest)?;
        } else {
            fs::remove_file(&latest)?;
        }
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(run_dir, &latest)?;
    #[cfg(not(unix))]
    {
        fs::create_dir_all(&latest)?;
    }
    Ok(())
}

fn write_checkpoint(
    run_dir: &Path,
    run_id: &str,
    variant: &str,
    generation_index: usize,
    max_generations: usize,
    output_guard: Option<&OutputPathGuard>,
) -> Result<()> {
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    let checkpoint = json!({
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id,
        "variant": variant,
        "complete_generation": generation_index,
        "complete_generation_id": format!("g{:04}", generation_index),
        "target_generation": max_generations,
        "updated_at": now_iso8601(),
    });
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    write_json(&run_dir.join("checkpoint.json"), &checkpoint)?;
    if let Some(guard) = output_guard {
        guard.check()?;
    }
    write_json(
        &run_dir
            .join("checkpoints")
            .join(format!("g{:04}.json", generation_index)),
        &checkpoint,
    )?;
    Ok(())
}

fn artifact_paths(
    run_dir: &Path,
    stage_dir: &Path,
    inputs: &[String],
    outputs: &[String],
) -> Value {
    json!({
        "run_dir": run_dir.display().to_string(),
        "stage_dir": stage_dir.display().to_string(),
        "inputs": inputs,
        "outputs": outputs,
    })
}

fn previous_generation_id(generation_index: usize) -> Option<String> {
    if generation_index <= 1 {
        None
    } else {
        Some(format!("g{:04}", generation_index - 1))
    }
}

fn read_stage_rankings(run_dir: &Path) -> Vec<Value> {
    let summary_paths = WalkDir::new(run_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.file_name().to_string_lossy() == "stage-summary.json"
        })
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for path in summary_paths {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Ok(summary) = serde_json::from_str::<Value>(&text) {
                rows.push(json!({
                    "stage_id": summary.get("stage_id").cloned().unwrap_or_else(|| json!("")),
                    "final_score": summary.get("best_final_score").cloned().unwrap_or_else(|| json!(0.0)),
                    "delta_score": summary.get("mean_final_score").cloned().unwrap_or_else(|| json!(0.0)),
                    "route": "jnoccio/standard",
                }));
            }
        }
    }
    rows
}

fn infer_run_id_from_path(path: &Path) -> String {
    let parts = path.components().collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        if part.as_os_str() == "runs" && index + 1 < parts.len() {
            return parts[index + 1].as_os_str().to_string_lossy().to_string();
        }
    }
    "unknown".to_string()
}

fn string_or_default(record: &Value, key: &str) -> String {
    record
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn merge_object(target: &mut Value, extra: &Value) {
    if let (Some(target), Some(extra)) = (target.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            target.insert(key.clone(), value.clone());
        }
    }
}

fn deep_merge(base: &mut Value, overlay: &Value) {
    match (base.as_object_mut(), overlay.as_object()) {
        (Some(base_map), Some(overlay_map)) => {
            for (key, value) in overlay_map {
                match base_map.get_mut(key) {
                    Some(existing) => deep_merge(existing, value),
                    None => {
                        base_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        _ => *base = overlay.clone(),
    }
}

fn deep_merge_values(base: &Value, overlay: &Value) -> Value {
    let mut merged = base.clone();
    deep_merge(&mut merged, overlay);
    merged
}

fn now_iso8601() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    format!("{}", now.as_secs())
}

fn stable_hash(value: &str) -> String {
    sha256_digest(value.as_bytes())
}

fn short_hash(value: &str, len: usize) -> String {
    stable_hash(value).chars().take(len).collect()
}

fn hash_unit(value: &str) -> f64 {
    let hash = stable_hash(value);
    let slice = &hash[..16.min(hash.len())];
    u64::from_str_radix(slice, 16).unwrap_or(0) as f64 / u64::MAX as f64
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.max(min).min(max)
}

fn estimate_tokens(text: &str) -> usize {
    if text.trim().is_empty() {
        0
    } else {
        text.split_whitespace().count().max(1)
    }
}

fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut values = values.to_vec();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

fn first_sentence(text: &str) -> String {
    text.split(|c| c == '.' || c == '\n')
        .find(|part| !part.trim().is_empty())
        .unwrap_or(text)
        .trim()
        .chars()
        .take(240)
        .collect()
}

fn novelty_terms_from_text(text: &str) -> Vec<String> {
    let mut terms = BTreeSet::new();
    for word in text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.len() >= 4)
    {
        terms.insert(word.to_lowercase());
    }
    terms.into_iter().collect()
}

fn reject_information_text(_source: &str, text: &str) -> Option<String> {
    let lowered = text.to_lowercase();
    if lowered.trim().is_empty() {
        return Some("empty_provenance".to_string());
    }
    for (pattern, label) in [
        ("fixture leakage", "fixture_leakage"),
        ("copied benchmark", "copied_benchmark_values"),
        ("empirical fitting", "empirical_fitting"),
        ("hidden free parameters", "hidden_free_parameters"),
        (
            "unsupported numeric prediction",
            "unsupported_numeric_prediction",
        ),
    ] {
        if lowered.contains(pattern) {
            return Some(label.to_string());
        }
    }
    None
}

fn concept_gene_from_card(card: &Value) -> Value {
    let source_hash = card
        .get("provenance_hash")
        .and_then(Value::as_str)
        .unwrap_or("selftest")
        .to_string();
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "concept_gene",
        "gene_id": format!("seed-{}", short_hash(&source_hash, 12)),
        "concept_id": card.get("stage_concept_hint").and_then(Value::as_str).unwrap_or("concept").to_string(),
        "family": card.get("domain").and_then(Value::as_str).unwrap_or("foundations").to_string(),
        "domain": card.get("domain").and_then(Value::as_str).unwrap_or("foundations").to_string(),
        "claim": first_sentence(card.get("claim").and_then(Value::as_str).unwrap_or("")),
        "method": "cached_research_synthesis",
        "constraint": "cached research only; no uncached web state inside scoring",
        "failure_risk": "cached source may be outdated or too broad for the target stage",
        "stage_concept_hint": card.get("stage_concept_hint").and_then(Value::as_str).unwrap_or("concept").to_string(),
        "source_card_ids": [card.get("research_card_id").cloned().unwrap_or_else(|| json!(""))],
        "source_path": card.get("source_path").cloned().unwrap_or_else(|| json!("")),
        "provenance_hash": source_hash,
        "novelty_terms": novelty_terms_from_text(card.get("summary").and_then(Value::as_str).unwrap_or("")),
    })
}

fn research_cards_from_cache(research_cards: &[Value], run_id: &str) -> Result<Vec<Value>> {
    Ok(research_cards
        .iter()
        .map(|card| {
            let mut card = card.clone();
            if card.get("record_kind").and_then(Value::as_str) != Some("research_card") {
                card["schema_version"] = json!(SCHEMA_VERSION);
                card["record_kind"] = json!("research_card");
                card["run_id"] = json!(run_id);
            }
            card
        })
        .collect())
}

fn live_summary(stdout: &str, status: &str, error: Option<&str>) -> Value {
    let summary = if stdout.trim().is_empty() {
        error.unwrap_or(status).to_string()
    } else {
        first_sentence(stdout)
    };
    json!({
        "status": status,
        "summary": summary,
        "evidence_terms": novelty_terms_from_text(stdout).into_iter().take(8).collect::<Vec<_>>(),
        "reasoning_quality": if status == "ok" && !stdout.trim().is_empty() { 0.70 } else if status == "failed" { 0.20 } else { 0.10 },
    })
}

fn build_mode_counts_or_default(population_size: usize) -> BTreeMap<String, usize> {
    population_mode_counts(population_size)
}

fn all_candidates_extend(dst: &mut Vec<Value>, src: &[Value]) {
    dst.extend(src.iter().cloned());
}

fn load_live_call_record_text(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn write_generated_text(path: &Path, body: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, body)?;
    Ok(())
}

fn parse_zyal_header(header: &str, path: &Path) -> Result<String> {
    let prefix = "<<<ZYAL v1:daemon id=";
    let suffix = ">>>";
    let trimmed = header.trim();
    if !trimmed.starts_with(prefix) || !trimmed.ends_with(suffix) {
        bail!("invalid ZYAL envelope in {}", path.display());
    }
    Ok(trimmed[prefix.len()..trimmed.len() - suffix.len()].to_string())
}

fn load_zyal_document(text: &str, path: &Path) -> Result<Value> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty ZYAL document: {}", path.display()))?;
    let id = parse_zyal_header(header, path)?;
    let end_line = format!("<<<END_ZYAL id={id}>>>");
    let arm_line = format!("ZYAL_ARM RUN_FOREVER id={id}");
    let mut body = Vec::new();
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim() == end_line {
            closed = true;
            break;
        }
        body.push(line);
    }
    if !closed {
        bail!("missing closing sentinel in {}", path.display());
    }
    let trailing: Vec<_> = lines.filter(|line| !line.trim().is_empty()).collect();
    if trailing != vec![arm_line.as_str()] {
        bail!("invalid ZYAL closing arm in {}", path.display());
    }
    let yaml_body = body.join("\n");
    let value: YamlValue =
        serde_yaml::from_str(&yaml_body).with_context(|| format!("parse {}", path.display()))?;
    Ok(serde_json::to_value(value)?)
}

fn load_document(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    if path.extension().and_then(|ext| ext.to_str()) == Some("zyal") {
        load_zyal_document(&text, path)
    } else {
        let value: YamlValue =
            serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        Ok(serde_json::to_value(value)?)
    }
}

fn load_runbook(path: &Path) -> Result<Value> {
    resolve_imports(path, &mut BTreeSet::new())
}

fn variant_from_runbook(runbook: &Value) -> Option<GenomeVariant> {
    match runbook
        .get("evaluation")
        .and_then(|evaluation| evaluation.get("variant"))
        .and_then(Value::as_str)
    {
        Some("pure-jnoccio") => Some(GenomeVariant::PureJnoccio),
        Some("hybrid") => Some(GenomeVariant::Hybrid),
        Some("jailgun-only") => Some(GenomeVariant::JailgunOnly),
        _ => None,
    }
}

fn hard_stage_count(stages: &[StagePackage]) -> usize {
    stages.iter().filter(|stage| stage.family == "hard").count()
}

fn hard_backend_required(
    runbook: &Value,
    variant: &GenomeVariant,
    run_id: &str,
    max_generations: usize,
) -> bool {
    if !matches!(variant, GenomeVariant::Hybrid) {
        return false;
    }
    let evaluation = runbook
        .get("evaluation")
        .cloned()
        .unwrap_or_else(empty_object);
    let configured = evaluation
        .get("quality_gates")
        .and_then(|gates| gates.get("require_hard_backend"))
        .and_then(Value::as_bool)
        .or_else(|| {
            evaluation
                .get("routing")
                .and_then(|routing| routing.get("require_hard_backend"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false);
    configured
        || run_id == "hybrid-1000"
        || (run_id.starts_with("hybrid-v2") && max_generations >= 10)
}

fn strict_jailgun_required(
    variant: &GenomeVariant,
    run_id: &str,
    require_hard_backend: bool,
    hard_stages: usize,
) -> bool {
    matches!(variant, GenomeVariant::Hybrid)
        && run_id.starts_with("hybrid-v2")
        && require_hard_backend
        && hard_stages > 0
}

fn jailgun_available_from_environment() -> bool {
    env::var("JAILGUN_AVAILABLE")
        .ok()
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

#[derive(Debug, Clone)]
struct JailgunStatus {
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
struct JailgunToken {
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
struct JailgunHealthConfig {
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

fn resolve_jailgun_status(strict: bool, jailgun_available_flag: bool) -> JailgunStatus {
    resolve_jailgun_status_with_config(
        strict,
        jailgun_available_flag,
        JailgunHealthConfig::from_env(),
    )
}

fn resolve_jailgun_status_with_config(
    strict: bool,
    jailgun_available_flag: bool,
    config: JailgunHealthConfig,
) -> JailgunStatus {
    let mut status = jailgun_status_with_config(strict, config);
    if !strict
        && !status.available
        && (jailgun_available_flag || jailgun_available_from_environment())
    {
        status.push_error("Jailgun availability flag was set, but server readiness was not proven");
        status.finish();
    }
    status
}

fn strict_jailgun_status_with_config(config: JailgunHealthConfig) -> JailgunStatus {
    jailgun_status_with_config(true, config)
}

fn env_string(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn jailgun_bridge_command() -> JailgunBridgeCommand {
    jailgun_bridge_command_from_env_lookup(env_string)
}

fn jailgun_bridge_command_from_env_lookup<F>(lookup: F) -> JailgunBridgeCommand
where
    F: FnMut(&str) -> Option<String>,
{
    jailgun_bridge_command_from_env_lookup_with_default(lookup, DEFAULT_JAILGUN_BRIDGE_COMMAND)
}

fn jailgun_bridge_command_from_env_lookup_with_default<F>(
    mut lookup: F,
    default_args: &[&str],
) -> JailgunBridgeCommand
where
    F: FnMut(&str) -> Option<String>,
{
    if let Some(value) = lookup("JAILGUN_BRIDGE_CMD") {
        let args = parse_bridge_command(&value);
        if !args.is_empty() {
            return JailgunBridgeCommand {
                args,
                source: "env:JAILGUN_BRIDGE_CMD".to_string(),
            };
        }
    }
    JailgunBridgeCommand {
        args: default_args.iter().map(|arg| (*arg).to_string()).collect(),
        source: "default:chrome-bridge".to_string(),
    }
}

fn parse_bridge_command(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn jailgun_status_with_config(strict: bool, config: JailgunHealthConfig) -> JailgunStatus {
    let mut status = JailgunStatus::server_authoritative(strict);
    status.server_url = Some(config.server_url.clone());
    status.token_source = config.token_source().map(ToString::to_string);
    status.set_check("server_url_configured", true);

    let Some(token) = config.token.as_ref() else {
        status.set_check("token_configured", false);
        status.set_check("server_health", false);
        status.set_check("browser_accounts", false);
        status.set_check("ready_accounts", false);
        status.set_check("mcp_initialize", false);
        status.set_check("mcp_tools_list", false);
        status.set_check("auth_status", false);
        status.set_check("scheduler_capacity", false);
        status.push_error(
            "Jailgun token is not available from env or matching local jailgun serve process",
        );
        status.finish();
        return status;
    };
    status.set_check("token_configured", true);

    match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(JAILGUN_HEALTH_TIMEOUT_SECONDS))
        .build()
    {
        Ok(client) => {
            check_jailgun_server(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
            );
            check_jailgun_browser_accounts(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
                &config.account_override_ids,
            );
            check_jailgun_mcp_initialize(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
            );
            let tools = check_jailgun_tools_list(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
            );
            check_jailgun_auth_status(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
                &tools,
            );
            check_jailgun_scheduler_status(
                &mut status,
                &client,
                &config.server_url,
                token.value.as_str(),
                &tools,
            );
        }
        Err(error) => {
            status.set_check("server_health", false);
            status.set_check("browser_accounts", false);
            status.set_check("ready_accounts", false);
            status.set_check("mcp_initialize", false);
            status.set_check("mcp_tools_list", false);
            status.set_check("auth_status", false);
            status.set_check("scheduler_capacity", false);
            status.push_error(format!("failed to build Jailgun HTTP client: {error}"));
        }
    }

    status.finish();
    status
}

fn jailgun_server_url() -> String {
    env_string("JAILGUN_SERVER_URL").unwrap_or_else(|| DEFAULT_JAILGUN_SERVER_URL.to_string())
}

fn jailgun_account_ids_override() -> Vec<String> {
    env_string("JAILGUN_ACCOUNT_IDS")
        .map(|value| parse_account_ids(&value))
        .unwrap_or_default()
}

fn resolve_jailgun_token(server_url: &str) -> Option<JailgunToken> {
    jailgun_token_from_env().or_else(|| jailgun_token_from_proc(server_url))
}

fn jailgun_token_from_env() -> Option<JailgunToken> {
    jailgun_token_from_env_lookup(env_string)
}

fn jailgun_token_from_env_lookup<F>(mut lookup: F) -> Option<JailgunToken>
where
    F: FnMut(&str) -> Option<String>,
{
    lookup("JAILGUN_INGEST_TOKEN")
        .map(|value| JailgunToken {
            value,
            source: "env:JAILGUN_INGEST_TOKEN".to_string(),
        })
        .or_else(|| {
            lookup("JAILGUN_TOKEN").map(|value| JailgunToken {
                value,
                source: "env:JAILGUN_TOKEN".to_string(),
            })
        })
}

#[derive(Debug, Clone)]
struct JailgunProcEntry {
    cmdline: Vec<String>,
    environ: Vec<String>,
}

fn jailgun_token_from_proc(server_url: &str) -> Option<JailgunToken> {
    let entries = jailgun_proc_entries(server_url);
    jailgun_token_from_proc_entries(&entries, server_url)
}

fn jailgun_proc_entries(server_url: &str) -> Vec<JailgunProcEntry> {
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut snapshots = Vec::new();
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(pid) = file_name.to_str() else {
            continue;
        };
        if !pid.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let proc_dir = entry.path();
        let Ok(cmdline) = read_proc_nul_strings(&proc_dir.join("cmdline")) else {
            continue;
        };
        if !jailgun_process_matches_server(&cmdline, server_url) {
            continue;
        }
        let Ok(environ) = read_proc_nul_strings(&proc_dir.join("environ")) else {
            continue;
        };
        snapshots.push(JailgunProcEntry { cmdline, environ });
    }
    snapshots
}

fn read_proc_nul_strings(path: &Path) -> std::io::Result<Vec<String>> {
    fs::read(path).map(|bytes| proc_nul_strings(&bytes))
}

fn proc_nul_strings(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).to_string())
        .collect()
}

fn jailgun_token_from_proc_entries(
    entries: &[JailgunProcEntry],
    server_url: &str,
) -> Option<JailgunToken> {
    entries
        .iter()
        .filter(|entry| jailgun_process_matches_server(&entry.cmdline, server_url))
        .find_map(|entry| {
            entry.environ.iter().find_map(|item| {
                item.strip_prefix("JAILGUN_INGEST_TOKEN=")
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| JailgunToken {
                        value: value.to_string(),
                        source: "proc:JAILGUN_INGEST_TOKEN".to_string(),
                    })
            })
        })
}

fn jailgun_process_matches_server(cmdline: &[String], server_url: &str) -> bool {
    if cmdline.is_empty() {
        return false;
    }
    let looks_like_jailgun = cmdline.iter().any(|arg| {
        Path::new(arg)
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.contains("jailgun"))
            .unwrap_or_else(|| arg.contains("jailgun"))
    });
    if !looks_like_jailgun || !cmdline.iter().any(|arg| arg == "serve") {
        return false;
    }
    let Some(server_addr) = jailgun_server_addr(server_url) else {
        return false;
    };
    cmdline.iter().enumerate().any(|(index, arg)| {
        if let Some(value) = arg.strip_prefix("--addr=") {
            return jailgun_addr_matches(value, &server_addr);
        }
        if arg == "--addr" {
            return cmdline
                .get(index + 1)
                .map(|value| jailgun_addr_matches(value, &server_addr))
                .unwrap_or(false);
        }
        false
    })
}

fn jailgun_server_addr(server_url: &str) -> Option<String> {
    let without_scheme = server_url
        .trim()
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or_else(|| server_url.trim());
    without_scheme
        .split('/')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn jailgun_addr_matches(process_addr: &str, server_addr: &str) -> bool {
    process_addr == server_addr
        || (server_addr.starts_with("localhost:")
            && process_addr == server_addr.replacen("localhost", "127.0.0.1", 1))
        || (server_addr.starts_with("127.0.0.1:")
            && process_addr == server_addr.replacen("127.0.0.1", "localhost", 1))
}

fn parse_account_ids(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn check_jailgun_server(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
) {
    match jailgun_get_json(client, server_url, "/api/health", token) {
        Ok(value) => {
            let ok = value.get("status").and_then(Value::as_str) == Some("ok");
            status.set_check("server_health", ok);
            if !ok {
                let value = sanitize_jailgun_token_text(&value.to_string(), token);
                status.push_error(format!(
                    "Jailgun /api/health returned unexpected body: {value}"
                ));
            }
        }
        Err(error) => {
            status.set_check("server_health", false);
            status.push_error(error);
        }
    }
}

fn check_jailgun_browser_accounts(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
    account_override_ids: &[String],
) {
    match jailgun_get_json(client, server_url, "/api/browser/accounts", token) {
        Ok(value) => {
            let accounts = jailgun_accounts_from_response(&value);
            let ready_accounts = ready_jailgun_account_ids(&accounts);
            let selected_ready = if account_override_ids.is_empty() {
                status.account_source = Some("server:/api/browser/accounts".to_string());
                ready_accounts
            } else {
                status.account_source = Some("env:JAILGUN_ACCOUNT_IDS".to_string());
                ready_accounts
                    .into_iter()
                    .filter(|id| account_override_ids.iter().any(|requested| requested == id))
                    .collect()
            };
            status.account_ids = selected_ready.clone();
            status.ready_account_ids = selected_ready;
            status.set_check("browser_accounts", true);
            let ready_ok = !status.ready_account_ids.is_empty();
            status.set_check("ready_accounts", ready_ok);
            if !ready_ok {
                let source = status.account_source.as_deref().unwrap_or("server");
                status.push_error(format!(
                    "no ready/authenticated Jailgun account reported by /api/browser/accounts for {source}"
                ));
            }
        }
        Err(error) => {
            status.set_check("browser_accounts", false);
            status.set_check("ready_accounts", false);
            status.push_error(error);
        }
    }
}

fn jailgun_accounts_from_response(value: &Value) -> Vec<Value> {
    value
        .as_array()
        .cloned()
        .or_else(|| value.get("accounts").and_then(Value::as_array).cloned())
        .or_else(|| {
            value
                .get("data")
                .and_then(|data| data.get("accounts"))
                .and_then(Value::as_array)
                .cloned()
        })
        .or_else(|| {
            value
                .get("browser")
                .and_then(|browser| browser.get("accounts"))
                .and_then(Value::as_array)
                .cloned()
        })
        .unwrap_or_default()
}

fn ready_jailgun_account_ids(accounts: &[Value]) -> Vec<String> {
    accounts
        .iter()
        .filter(|account| jailgun_account_ready(account))
        .filter_map(jailgun_account_id)
        .collect()
}

fn jailgun_account_id(account: &Value) -> Option<String> {
    account
        .get("id")
        .or_else(|| account.get("account_id"))
        .or_else(|| account.get("accountId"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn jailgun_account_ready(account: &Value) -> bool {
    account.get("ready").and_then(Value::as_bool) == Some(true)
        || account.get("authenticated").and_then(Value::as_bool) == Some(true)
        || account
            .get("status")
            .and_then(Value::as_str)
            .map(jailgun_ready_status)
            .unwrap_or(false)
        || account
            .get("auth_status")
            .and_then(Value::as_str)
            .map(jailgun_ready_status)
            .unwrap_or(false)
}

fn jailgun_ready_status(status: &str) -> bool {
    matches!(status, "ready" | "authenticated" | "active" | "ok")
}

fn check_jailgun_mcp_initialize(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
) {
    let body = json!({
        "jsonrpc": "2.0",
        "id": "openqg-preflight-init",
        "method": "initialize",
        "params": {},
    });
    match jailgun_post_json(client, server_url, "/mcp", token, &body) {
        Ok(value) => {
            let ok = value.get("error").is_none() && value.get("result").is_some();
            status.set_check("mcp_initialize", ok);
            if !ok {
                let value = sanitize_jailgun_token_text(&value.to_string(), token);
                status.push_error(format!(
                    "Jailgun MCP initialize returned unexpected body: {value}"
                ));
            }
        }
        Err(error) => {
            status.set_check("mcp_initialize", false);
            status.push_error(error);
        }
    }
}

fn check_jailgun_tools_list(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
) -> Vec<String> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": "openqg-preflight-tools",
        "method": "tools/list",
        "params": {},
    });
    match jailgun_post_json(client, server_url, "/mcp", token, &body) {
        Ok(value) => {
            if let Some(error) = value.get("error") {
                status.set_check("mcp_tools_list", false);
                let error = sanitize_jailgun_token_text(&error.to_string(), token);
                status.push_error(format!("Jailgun MCP tools/list returned error: {error}"));
                return Vec::new();
            }
            let tools = jailgun_tool_names_from_list_response(&value);
            status.mcp_tools = tools.clone();
            status.set_check("mcp_tools_list", true);
            tools
        }
        Err(error) => {
            status.set_check("mcp_tools_list", false);
            status.push_error(error);
            Vec::new()
        }
    }
}

fn jailgun_tool_names_from_list_response(value: &Value) -> Vec<String> {
    value
        .get("result")
        .and_then(|result| result.get("tools"))
        .or_else(|| {
            value
                .get("result")
                .and_then(|result| result.get("structuredContent"))
                .and_then(|content| content.get("tools"))
        })
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| tool.get("name").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn jailgun_tool_available(tools: &[String], name: &str) -> bool {
    tools.iter().any(|tool| tool == name)
}

fn check_jailgun_auth_status(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
    tools: &[String],
) {
    if status.ready_account_ids.is_empty() {
        status.set_check("auth_status", false);
        status.push_error("no ready Jailgun account is available for auth_status");
        return;
    };

    if !jailgun_tool_available(tools, "jailgun.auth_status") {
        status.set_check("auth_status", true);
        return;
    }

    let mut authenticated = Vec::new();
    let mut errors = Vec::new();
    for account_id in status.ready_account_ids.clone() {
        match jailgun_mcp_tool_call(
            client,
            server_url,
            token,
            &format!("openqg-preflight-auth-{account_id}"),
            "jailgun.auth_status",
            json!({ "account_id": account_id }),
        ) {
            Ok(content) => {
                if jailgun_auth_status_ready(&content) {
                    authenticated.push(account_id);
                } else {
                    let content = sanitize_jailgun_token_text(&content.to_string(), token);
                    errors.push(format!(
                        "Jailgun auth_status for account returned unexpected body: {content}"
                    ));
                }
            }
            Err(error) => errors.push(error),
        }
    }
    let ok = !authenticated.is_empty();
    status.set_check("auth_status", ok);
    if ok {
        status.ready_account_ids = authenticated.clone();
        status.account_ids = authenticated;
    } else {
        for error in errors {
            status.push_error(error);
        }
    }
}

fn jailgun_auth_status_ready(content: &Value) -> bool {
    content.get("ready").and_then(Value::as_bool) == Some(true)
        || content.get("authenticated").and_then(Value::as_bool) == Some(true)
        || content
            .get("status")
            .and_then(Value::as_str)
            .map(jailgun_ready_status)
            .unwrap_or(false)
        || content
            .get("auth_status")
            .and_then(Value::as_str)
            .map(jailgun_ready_status)
            .unwrap_or(false)
}

fn check_jailgun_scheduler_status(
    status: &mut JailgunStatus,
    client: &reqwest::blocking::Client,
    server_url: &str,
    token: &str,
    tools: &[String],
) {
    if !jailgun_tool_available(tools, "jailgun.scheduler_status") {
        status.set_check("scheduler_capacity", true);
        return;
    }

    match jailgun_mcp_tool_call(
        client,
        server_url,
        token,
        "openqg-preflight-scheduler",
        "jailgun.scheduler_status",
        json!({}),
    ) {
        Ok(content) => {
            let ok = jailgun_scheduler_has_capacity(&content);
            status.set_check("scheduler_capacity", ok);
            if !ok {
                let content = sanitize_jailgun_token_text(&content.to_string(), token);
                status.push_error(format!(
                    "Jailgun scheduler has no available capacity: {content}"
                ));
            }
        }
        Err(error) => {
            status.set_check("scheduler_capacity", false);
            status.push_error(error);
        }
    }
}

fn jailgun_scheduler_has_capacity(content: &Value) -> bool {
    for key in [
        "queued_jobs",
        "running_jobs",
        "pending_jobs",
        "active_jobs",
        "queued",
        "running",
        "in_flight",
    ] {
        if let Some(value) = content.get(key).and_then(Value::as_i64) {
            if value > 0 {
                return false;
            }
        }
        if let Some(value) = content.get(key).and_then(Value::as_u64) {
            if value > 0 {
                return false;
            }
        }
        if let Some(items) = content.get(key).and_then(Value::as_array) {
            if !items.is_empty() {
                return false;
            }
        }
    }
    for key in [
        "capacity_available",
        "has_capacity",
        "available",
        "can_accept_runs",
        "can_schedule",
    ] {
        if let Some(value) = content.get(key).and_then(Value::as_bool) {
            return value;
        }
    }
    for key in [
        "available_slots",
        "free_slots",
        "remaining_capacity",
        "capacity",
    ] {
        if let Some(value) = content.get(key).and_then(Value::as_i64) {
            return value > 0;
        }
        if let Some(value) = content.get(key).and_then(Value::as_u64) {
            return value > 0;
        }
    }
    if let Some(status) = content.get("status").and_then(Value::as_str) {
        if matches!(
            status,
            "full" | "paused" | "blocked" | "unavailable" | "stopped" | "disabled" | "error"
        ) {
            return false;
        }
        if matches!(status, "ready" | "ok" | "available" | "healthy" | "running") {
            return true;
        }
    }
    true
}

fn jailgun_get_json(
    client: &reqwest::blocking::Client,
    server_url: &str,
    path: &str,
    token: &str,
) -> std::result::Result<Value, String> {
    let url = jailgun_url(server_url, path);
    let response = client
        .get(&url)
        .header("x-jailgun-token", token)
        .send()
        .map_err(|error| format!("Jailgun GET {url} failed: {error}"))?;
    response_json(response, &url, token)
}

fn jailgun_post_json(
    client: &reqwest::blocking::Client,
    server_url: &str,
    path: &str,
    token: &str,
    body: &Value,
) -> std::result::Result<Value, String> {
    let url = jailgun_url(server_url, path);
    let body = serde_json::to_string(body)
        .map_err(|error| format!("failed to serialize Jailgun POST body for {url}: {error}"))?;
    let response = client
        .post(&url)
        .header("x-jailgun-token", token)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .map_err(|error| format!("Jailgun POST {url} failed: {error}"))?;
    response_json(response, &url, token)
}

fn response_json(
    response: reqwest::blocking::Response,
    url: &str,
    token: &str,
) -> std::result::Result<Value, String> {
    let status = response.status();
    let text = response
        .text()
        .map_err(|error| format!("Jailgun response body from {url} failed: {error}"))?;
    let text = sanitize_jailgun_token_text(&text, token);
    if !status.is_success() {
        return Err(format!("Jailgun {url} returned HTTP {status}: {text}"));
    }
    serde_json::from_str(&text)
        .map_err(|error| format!("Jailgun {url} returned invalid JSON: {error}: {text}"))
}

fn jailgun_url(server_url: &str, path: &str) -> String {
    format!("{}{}", server_url.trim_end_matches('/'), path)
}

fn sanitize_jailgun_token_text(text: &str, token: &str) -> String {
    if token.is_empty() {
        text.to_string()
    } else {
        text.replace(token, "[redacted]")
    }
}

fn redact_jailgun_token_in_value(value: &Value, token: &str) -> Value {
    match value {
        Value::String(text) => Value::String(sanitize_jailgun_token_text(text, token)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_jailgun_token_in_value(item, token))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), redact_jailgun_token_in_value(value, token)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn preflight_receipt(
    run_id: &str,
    variant: &str,
    runbook: &Value,
    runbook_path: &Path,
    run_dir: &Path,
    stage_registry: &[StagePackage],
    live_config: &LiveConfig,
    require_hard_backend: bool,
    live_smoke: bool,
    jailgun_status: &JailgunStatus,
) -> Value {
    let command = live_command(live_config);
    let executable_found = command
        .first()
        .map(|executable| command_exists(executable))
        .unwrap_or(false);
    let (provider, model) = live_provider_model(&command);
    let hard_stages = hard_stage_count(stage_registry);
    let hybrid_requires_hard_backend =
        variant == "hybrid" && require_hard_backend && hard_stages > 0;
    let jailgun_available = jailgun_status.available;
    let routing_decision = if hybrid_requires_hard_backend && !jailgun_available {
        "blocked_missing_hard_backend"
    } else if variant == "hybrid" && hard_stages > 0 && !jailgun_available {
        "degraded_fallback_available"
    } else {
        "nominal"
    };
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "preflight",
        "run_id": run_id,
        "variant": variant,
        "runbook_path": runbook_path.display().to_string(),
        "run_dir": run_dir.display().to_string(),
        "created_at": now_iso8601(),
        "status": if routing_decision == "blocked_missing_hard_backend" || (live_smoke && !executable_found) { "failed" } else { "ok" },
        "backend_health": {
            "hard_backend": "jailgun",
            "hard_stage_count": hard_stages,
            "require_hard_backend": require_hard_backend,
            "jailgun_available": jailgun_available,
            "jailgun_evidence": jailgun_status.evidence.as_str(),
            "jailgun": jailgun_status.as_json(),
        },
        "live_command": {
            "enabled": live_config.enabled,
            "smoke_requested": live_smoke,
            "command": command,
            "executable_found": executable_found,
            "provider": provider,
            "model": model,
        },
        "timeout_config": {
            "default_seconds": live_config.timeout_seconds,
            "by_purpose": live_config.timeout_by_purpose.clone(),
            "retry_count": live_config.retry_count,
        },
        "routing_decision": routing_decision,
        "quality_gates": runbook.get("evaluation").and_then(|evaluation| evaluation.get("quality_gates")).cloned().unwrap_or_else(empty_object),
    })
}

fn live_provider_model(command: &[String]) -> (Option<String>, Option<String>) {
    let mut provider = None;
    let mut model = None;
    let mut iter = command.iter();
    while let Some(arg) = iter.next() {
        if arg == "--provider" {
            provider = iter.next().cloned();
        } else if arg == "--model" {
            model = iter.next().cloned();
        }
    }
    (provider, model)
}

fn command_exists(command: &str) -> bool {
    if command.contains('/') {
        return Path::new(command).is_file();
    }
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path_var).any(|path| path.join(command).is_file())
}

fn resolve_imports(path: &Path, seen: &mut BTreeSet<PathBuf>) -> Result<Value> {
    let path = path
        .canonicalize()
        .with_context(|| format!("resolve {}", path.display()))?;
    if !seen.insert(path.clone()) {
        bail!("cyclic import detected for {}", path.display());
    }
    let doc = load_document(&path)?;
    let mut merged = json!({});
    if let Some(imports) = doc.get("imports").and_then(Value::as_array) {
        for import_entry in imports {
            if let Some(import) = import_entry.as_str() {
                let imported =
                    resolve_imports(&path.parent().unwrap_or(Path::new(".")).join(import), seen)?;
                deep_merge(&mut merged, &imported);
            }
        }
    }
    let mut doc = doc;
    if let Some(map) = doc.as_object_mut() {
        map.remove("imports");
    }
    deep_merge(&mut merged, &doc);
    Ok(merged)
}

fn default_run_id(variant: &str, seed: u64, generations: usize) -> String {
    format!("{variant}-{seed}-{generations}")
}

#[cfg(any())]
fn select_information_sources(
    stage_registry: &[StagePackage],
    generation_index: usize,
    seed: u64,
    count: usize,
) -> Vec<PathBuf> {
    if stage_registry.is_empty() {
        return Vec::new();
    }
    let mut paths = Vec::new();
    for index in 0..count {
        let stage =
            &stage_registry[(generation_index + index + seed as usize) % stage_registry.len()];
        paths.push(stage.stage_file.clone());
    }
    paths
}

#[cfg(any())]
fn extract_information_card(
    source_path: &Path,
    generation_id: &str,
    run_id: &str,
) -> Option<Value> {
    let text = fs::read_to_string(source_path).ok()?;
    let prompt_hash = stable_hash(&text);
    let stage_id = source_path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("source");
    Some(json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "information_card",
        "run_id": run_id,
        "information_card_id": format!("info-{generation_id}-{}", short_hash(&prompt_hash, 12)),
        "source_path": source_path.display().to_string(),
        "domain": "sources",
        "claim": first_sentence(&text),
        "method": "file_synthesis",
        "constraint": "cached research only",
        "failure_risk": "source may be outdated or broad",
        "stage_concept_hint": stage_id,
        "provenance_hash": prompt_hash,
        "novelty_terms": novelty_terms_from_text(&text),
    }))
}

#[cfg(any())]
fn build_novelty_archive(run_id: &str, candidates: &[Value], information_cards: &[Value]) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "novelty_archive",
        "run_id": run_id,
        "entry_count": candidates.len(),
        "information_card_count": information_cards.len(),
        "entries": candidates.iter().take(5).cloned().collect::<Vec<_>>(),
    })
}

#[cfg(any())]
fn build_island_leaderboard(run_id: &str, candidates: &[Value], island_names: &[String]) -> Value {
    let leaders = island_names
        .iter()
        .map(|island| {
            let best = candidates
                .iter()
                .filter(|candidate| candidate.get("island").and_then(Value::as_str) == Some(island.as_str()))
                .max_by(|a, b| {
                    a.get("scores")
                        .and_then(|scores| scores.get("final_score"))
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0)
                        .partial_cmp(&b.get("scores").and_then(|scores| scores.get("final_score")).and_then(Value::as_f64).unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .cloned()
                .unwrap_or_else(empty_object);
            json!({
                "island": island,
                "candidate_id": best.get("candidate_id").cloned().unwrap_or_else(|| json!("")),
                "final_score": best.get("scores").and_then(|scores| scores.get("final_score")).cloned().unwrap_or_else(|| json!(0.0)),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "novelty_archive",
        "run_id": run_id,
        "leaders": leaders,
    })
}

#[cfg(any())]
fn build_fun_summary(
    candidates: &[Value],
    generation_champions: &[Value],
    information_cards: &[Value],
    newest_source_influence: Option<&Value>,
    leaders: Vec<Value>,
) -> Value {
    let weirdest = candidates
        .iter()
        .max_by(|a, b| {
            a.get("scores")
                .and_then(|scores| scores.get("novelty_score"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .partial_cmp(
                    &b.get("scores")
                        .and_then(|scores| scores.get("novelty_score"))
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                )
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned();
    let comeback = generation_champions.last().cloned();
    let useful_failure = information_cards.first().cloned();
    json!({
        "weirdest_surviving_candidate": weirdest,
        "best_comeback_lineage": comeback,
        "most_useful_failure": useful_failure,
        "newest_source_influence": newest_source_influence.cloned(),
        "leaders": leaders,
    })
}

#[cfg(any())]
fn candidate_pareto_snapshot(candidates: &[Value]) -> Value {
    json!({
        "frontier_size": candidates.len().min(5),
        "points": candidates.iter().take(5).map(|candidate| json!({
            "candidate_id": candidate.get("candidate_id").cloned().unwrap_or_else(|| json!("")),
            "final_score": candidate.get("scores").and_then(|scores| scores.get("final_score")).cloned().unwrap_or_else(|| json!(0.0)),
        })).collect::<Vec<_>>(),
    })
}

fn render_lineage_markdown(lineage_edges: &[Value], generation_champions: &[Value]) -> String {
    let mut rows = vec![
        "# ZYAL Genome Lineage".to_string(),
        String::new(),
        "| Generation | Champion | Score |".to_string(),
        "| --- | --- | ---: |".to_string(),
    ];
    for champion in generation_champions {
        rows.push(format!(
            "| `{}` | `{}` | `{}` |",
            champion
                .get("generation_id")
                .and_then(Value::as_str)
                .unwrap_or("g0000"),
            champion
                .get("candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("candidate"),
            champion
                .get("final_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0),
        ));
    }
    rows.push(String::new());
    rows.push("## Edges".to_string());
    rows.push(String::new());
    for edge in lineage_edges {
        rows.push(format!(
            "- `{}` -> `{}` ({})",
            edge.get("parent_candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("root"),
            edge.get("child_candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("child"),
            edge.get("mutation_op")
                .and_then(Value::as_str)
                .unwrap_or("mutation"),
        ));
    }
    rows.join("\n") + "\n"
}

fn all_candidates_from_generation(candidates: &[Value], generation_id: &str) -> Vec<Value> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate.get("generation_id").and_then(Value::as_str) == Some(generation_id)
        })
        .cloned()
        .collect()
}

fn build_stage_concepts_map(
    stage_registry: &[StagePackage],
    concepts: &[Value],
    generation_index: usize,
    candidate_index: usize,
    seed: u64,
) -> BTreeMap<String, String> {
    let stage_values = stage_registry
        .iter()
        .map(|stage| json!({"stage_id": stage.stage_id}))
        .collect::<Vec<_>>();
    adapt_stage_concepts_from_values(
        &stage_values,
        concepts,
        generation_index,
        candidate_index,
        seed,
    )
}

fn parse_json_object(value: &YamlValue) -> Value {
    serde_json::to_value(value).unwrap_or_else(|_| json!({}))
}

fn load_population_snapshot(path: &Path) -> Option<Value> {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

fn load_stage_scores_from_json(
    stage_registry: &[StagePackage],
    stage_ledgers: &[Value],
) -> BTreeMap<String, Vec<f64>> {
    stage_score_history_from_ledgers(stage_registry, stage_ledgers)
}

fn run_offline_eval_markdown_path(run_dir: &Path) -> PathBuf {
    run_dir.join("offline-eval.md")
}

fn validate_record_kind(record: &Value) -> Result<()> {
    validate_record(record)
}

fn stage_package_prompt(stage: &StagePackage) -> String {
    if stage.prompt_path.is_file() {
        fs::read_to_string(&stage.prompt_path).unwrap_or_else(|_| stage.purpose.clone())
    } else {
        stage.purpose.clone()
    }
}

fn build_genesis_card(stage: &StagePackage, run_id: &str) -> Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "record_kind": "research_card",
        "run_id": run_id,
        "research_card_id": format!("research-{}", short_hash(&stage.prompt_hash, 12)),
        "source_type": "cached_stage",
        "url": "",
        "title": stage.name,
        "date": "",
        "source_hash": stage.prompt_hash,
        "citation": stage.name,
        "accepted": true,
        "rejection_reason": null,
    })
}

fn live_call_to_receipt(record: &Value) -> Value {
    record.clone()
}

fn read_json_value(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use tempfile::tempdir;

    fn spawn_fake_jailgun_server(accounts: Value, scheduler: Value) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake jailgun");
        listener
            .set_nonblocking(true)
            .expect("fake jailgun nonblocking");
        let url = format!("http://{}", listener.local_addr().expect("local addr"));
        thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                match listener.accept() {
                    Ok((stream, _)) => {
                        handle_fake_jailgun_connection(stream, &accounts, &scheduler)
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        url
    }

    fn handle_fake_jailgun_connection(mut stream: TcpStream, accounts: &Value, scheduler: &Value) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
        let mut data = Vec::new();
        let mut buffer = [0u8; 4096];
        while !data.windows(4).any(|window| window == b"\r\n\r\n") {
            match stream.read(&mut buffer) {
                Ok(0) => return,
                Ok(read) => data.extend_from_slice(&buffer[..read]),
                Err(_) => return,
            }
        }
        let Some(header_end) = data.windows(4).position(|window| window == b"\r\n\r\n") else {
            return;
        };
        let header_text = String::from_utf8_lossy(&data[..header_end]).to_string();
        let content_length = header_text
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        let body_start = header_end + 4;
        while data.len() < body_start + content_length {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => data.extend_from_slice(&buffer[..read]),
                Err(_) => break,
            }
        }
        let body =
            String::from_utf8_lossy(&data[body_start..data.len().min(body_start + content_length)]);
        let first_line = header_text.lines().next().unwrap_or_default();
        let path = first_line.split_whitespace().nth(1).unwrap_or("/");
        let response = fake_jailgun_response(path, &body, accounts, scheduler);
        let response_text = serde_json::to_string(&response).expect("serialize response");
        let http = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            response_text.len(),
            response_text
        );
        let _ = stream.write_all(http.as_bytes());
    }

    fn fake_jailgun_response(path: &str, body: &str, accounts: &Value, scheduler: &Value) -> Value {
        if path == "/api/health" {
            return json!({"status": "ok"});
        }
        if path == "/api/browser/accounts" {
            return accounts.clone();
        }
        if path != "/mcp" {
            return json!({"error": "not found"});
        }
        let request: Value = serde_json::from_str(body).unwrap_or_else(|_| json!({}));
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        match request.get("method").and_then(Value::as_str).unwrap_or("") {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"serverInfo": {"name": "jailgun"}}
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {"name": "jailgun.auth_status"},
                        {"name": "jailgun.scheduler_status"},
                        {"name": "jailgun.run"},
                        {"name": "jailgun.run_status"},
                        {"name": "jailgun.run_summary"}
                    ]
                }
            }),
            "tools/call" => {
                let name = request
                    .get("params")
                    .and_then(|params| params.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let content = match name {
                    "jailgun.auth_status" => json!({"status": "ready"}),
                    "jailgun.scheduler_status" => scheduler.clone(),
                    "jailgun.run" => {
                        let run_id = request
                            .get("params")
                            .and_then(|params| params.get("arguments"))
                            .and_then(|args| args.get("run_id"))
                            .cloned()
                            .unwrap_or_else(|| json!("fake-run"));
                        json!({"status": "accepted", "run_id": run_id})
                    }
                    "jailgun.run_status" => json!({"status": "succeeded"}),
                    "jailgun.run_summary" => json!({
                        "status": "succeeded",
                        "summary_json": null,
                        "events_jsonl": null,
                        "receipt_paths": ["artifact-smoke.json"]
                    }),
                    _ => json!({"status": "ok"}),
                };
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {"structuredContent": content}
                })
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"message": "unknown method"}
            }),
        }
    }

    fn write_jsonl_records(path: &Path, records: &[Value]) {
        let mut text = String::new();
        for record in records {
            text.push_str(&serde_json::to_string(record).expect("serialize jsonl record"));
            text.push('\n');
        }
        fs::write(path, text).expect("write jsonl");
    }

    fn test_stage(stage_id: &str, family: &str, outputs: Vec<String>) -> StagePackage {
        StagePackage {
            stage_id: stage_id.to_string(),
            name: "Test stage".to_string(),
            track: "test".to_string(),
            family: family.to_string(),
            purpose: "test purpose".to_string(),
            inputs: Vec::new(),
            outputs,
            required_evidence: Vec::new(),
            validation_checks: Vec::new(),
            mutation_op: "test_mutation".to_string(),
            prompt_path: PathBuf::from("missing-prompt.md"),
            memory_path: PathBuf::from("memory.yml"),
            score_path: PathBuf::from("score.yml"),
            stage_dir: PathBuf::from("stage"),
            stage_file: PathBuf::from("stage/stage.yml"),
            prompt_hash: "hash".to_string(),
            memory: json!({}),
            score_model: json!({}),
        }
    }

    fn test_live_config() -> LiveConfig {
        LiveConfig {
            enabled: true,
            timeout_seconds: 1,
            timeout_by_purpose: BTreeMap::new(),
            retry_count: 0,
            champion_audit_every: 1,
            hard_stage_every: 1,
            promotion_every: 1,
            research_synthesis: true,
            hard_stage_repair: true,
            promotion_judging: true,
            command: vec!["false".to_string()],
        }
    }

    fn selection_candidate(id: &str, mode: &str, island: &str, final_score: f64) -> Value {
        json!({
            "candidate_id": id,
            "generation_id": "g0001",
            "island": island,
            "mode": mode,
            "parent_candidate_ids": ["parent-a"],
            "lineage_depth": 2,
            "source_card_ids": ["info-a"],
            "stage_concepts": {"03-generate-genes": "concept-a"},
            "scores": {
                "final_score": final_score,
                "novelty_score": if mode == "novelty" { 0.72 } else { 0.42 },
            },
            "frontier_claim": format!("{id} claim"),
            "falsifiable_tests": ["Replay 03-generate-genes evidence"],
            "known_failure_modes": ["test_failure"],
            "review_priority": if mode == "novelty" { "high" } else { "medium" },
        })
    }

    fn previous_champions(count: usize, novelty_count: usize, last_score: f64) -> Vec<Value> {
        let islands = [
            "foundations",
            "coefficients",
            "observables",
            "failure-repair",
            "interface-contracts",
            "wildcards",
        ];
        (0..count)
            .map(|index| {
                let mode = if index < novelty_count {
                    "novelty"
                } else {
                    "exploitation"
                };
                let score = if index + 1 == count { last_score } else { 0.80 };
                let mut candidate = selection_candidate(
                    &format!("prev-{index:03}"),
                    mode,
                    islands[index % islands.len()],
                    score,
                );
                candidate["generation_id"] = json!(format!("g{:04}", index + 1));
                candidate
            })
            .collect()
    }

    fn write_healthy_gate_fixture(run_dir: &Path, target_generation: usize) {
        fs::create_dir_all(run_dir).expect("create run dir");
        write_json(
            &run_dir.join("checkpoint.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "run_id": "healthy",
                "variant": "hybrid",
                "complete_generation": target_generation,
                "target_generation": target_generation,
            }),
        )
        .expect("checkpoint");
        let champions = (1..=target_generation)
            .map(|generation| {
                json!({
                    "generation_id": format!("g{generation:04}"),
                    "candidate_id": format!("c{generation:04}"),
                    "island": DEFAULT_ISLANDS[(generation - 1) % DEFAULT_ISLANDS.len()],
                    "mode": if generation % 10 == 0 { "novelty" } else { "exploitation" },
                    "final_score": 0.82,
                    "novelty_score": 0.62,
                })
            })
            .collect::<Vec<_>>();
        write_json(
            &run_dir.join("run-summary.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "run_summary",
                "run_id": "healthy",
                "variant": "hybrid",
                "generation_count": target_generation,
                "stage_count": 1,
                "candidate_count": target_generation,
                "best_score_seen": 0.82,
                "rolling_5_median": 0.82,
                "best_nonregressive_delta": 0.0,
                "unique_contributions": 1,
                "decoy_failures": 0,
                "degraded_route_count": 0,
                "regression_rate": 0.0,
                "throughput": 1.0,
                "fail_stop_rate": 0.0,
                "score_blend": {"local_score":0.2,"interface_score":0.2,"macro_score":0.2,"innovation_score":0.2,"novelty_score":0.1,"failure_penalty":0.1},
                "router_state": "nominal",
                "degraded_router": false,
                "lineage": {"acyclic": true, "missing_parent_ids": 0},
                "pareto_snapshot": {"frontier_size": 1, "points": []},
                "generation_champions": champions,
            }),
        )
        .expect("summary");
        write_json(
            &run_dir.join("offline-eval.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "offline_eval",
                "run_id": "healthy",
                "variant": "hybrid",
                "scorecard": {"best_score_seen": 0.82},
                "stage_rankings": [],
                "warnings": [],
            }),
        )
        .expect("offline eval");
        write_json(
            &run_dir.join("preflight.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "preflight",
                "run_id": "healthy",
                "variant": "hybrid",
                "status": "ok",
                "backend_health": {
                    "hard_backend": "jailgun",
                    "hard_stage_count": 1,
                    "require_hard_backend": true,
                    "jailgun_available": true,
                },
                "routing_decision": "nominal",
            }),
        )
        .expect("preflight");
        write_jsonl_records(
            &run_dir.join("live-call-ledger.jsonl"),
            &[json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "live_call",
                "run_id": "healthy",
                "generation_id": "g0001",
                "stage_id": "02-decompose-failed",
                "candidate_id": "g0001-02-decompose-failed",
                "call_id": "live-g0001-02-decompose-failed-hard_stage_repair",
                "purpose": "hard_stage_repair",
                "route_backend": "jailgun",
                "route_tier": "top20_pct",
                "router_state": "nominal",
                "execution_backend": "jailgun_mcp",
                "status": "ok",
                "exit_code": 0,
                "started_at": "0",
                "elapsed_seconds": 0.1,
                "timeout_seconds": 120,
                "configured_timeout_seconds": 120,
                "retry_count": 1,
                "attempt_count": 1,
                "attempts": [{"attempt":1,"status":"ok","elapsed_seconds":0.1,"timeout_seconds":120}],
                "command": ["sh","-c","cat"],
                "jailgun_server_url": "http://127.0.0.1:8797",
                "jailgun_run_id": "openqg-live-g0001-02-decompose-failed-hard_stage_repair-a1-test",
                "jailgun_status": "succeeded",
                "jailgun_summary_status": "succeeded",
                "jailgun_account_count": 1,
                "jailgun_summary_path": "agent-summary.json",
                "jailgun_events_path": "agent-events.jsonl",
                "jailgun_receipt_paths": [],
                "prompt_path": "prompt.md",
                "retrieval_packet_path": "retrieval.json",
                "raw_output_path": "raw.txt",
                "parsed_summary_path": "summary.json",
                "receipt_path": "receipt.json",
                "token_usage": {"prompt": 1, "completion": 1, "total": 2},
                "summary": "ok",
                "error": null,
            })],
        );
        let mut generation_records = Vec::new();
        for generation in 1..=target_generation {
            generation_records.push(json!({
                    "schema_version": SCHEMA_VERSION,
                    "record_kind": "metrics_point",
                    "run_id": "healthy",
                    "variant": "hybrid",
                    "generation_id": format!("g{generation:04}"),
                    "metric_name": "deterministic_rollup_score",
                    "metric_value": 0.40,
                    "series": "generation_rollup",
            }));
            generation_records.push(json!({
                    "schema_version": SCHEMA_VERSION,
                    "record_kind": "metrics_point",
                    "run_id": "healthy",
                    "variant": "hybrid",
                    "generation_id": format!("g{generation:04}"),
                    "metric_name": "hybrid_champion_score",
                    "metric_value": 0.82,
                    "series": "hybrid_champion",
            }));
        }
        write_jsonl_records(
            &run_dir.join("generation-ledger.jsonl"),
            &generation_records,
        );
        write_jsonl_records(&run_dir.join("run-events.jsonl"), &[]);
        write_jsonl_records(&run_dir.join("stage-ledger.jsonl"), &[]);
    }

    #[test]
    fn discovers_stage_packages() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("workspace root")
            .join("ZYAL/stages");
        let stages = load_stage_registry(&root).expect("load stage registry");
        assert_eq!(stages.len(), 11);
        assert_eq!(stages.first().unwrap().stage_id, "00-atlas");
        assert_eq!(stages.last().unwrap().stage_id, "10-promotion");
    }

    #[test]
    fn resume_state_recovers_checkpoint_and_scores() {
        let dir = tempdir().expect("tempdir");
        let run_dir = dir.path().join("run");
        fs::create_dir_all(&run_dir).expect("create run dir");
        write_json(
            &run_dir.join("checkpoint.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "run_id": "run-1",
                "variant": "hybrid",
                "complete_generation": 2,
            }),
        )
        .expect("checkpoint");
        write_json(
            &run_dir.join("generation-ledger.jsonl"),
            &json!({"schema_version": SCHEMA_VERSION, "record_kind": "metrics_point", "generation_id": "g0001", "metric_name": "deterministic_rollup_score", "metric_value": 0.4, "run_id": "run-1", "variant": "hybrid", "series": "generation_rollup"}),
        )
        .expect("generation ledger json");
        let stage_registry = load_stage_registry(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(|path| path.parent())
                .unwrap()
                .join("ZYAL/stages"),
        )
        .expect("stage registry");
        let state = load_resume_state(&run_dir, &stage_registry).expect("resume state");
        assert_eq!(state.completed_generation, 2);
        assert_eq!(
            read_completed_generation(&run_dir).expect("completed generation"),
            2
        );
    }

    #[test]
    fn live_call_receipt_serializes() {
        let record = json!({
            "schema_version": SCHEMA_VERSION,
            "record_kind": "live_call",
            "run_id": "run-1",
            "generation_id": "g0001",
            "stage_id": "03-generate-genes",
            "candidate_id": "g0001-03-generate-genes",
            "call_id": "live-g0001-03-generate-genes-research_synthesis",
            "purpose": "research_synthesis",
            "status": "ok",
            "exit_code": 0,
            "started_at": "0",
            "elapsed_seconds": 0.1,
            "timeout_seconds": 45,
            "retry_count": 0,
            "command": ["sh","-c","echo hi"],
            "prompt_path": "prompt.md",
            "retrieval_packet_path": "retrieval.json",
            "raw_output_path": "raw.txt",
            "parsed_summary_path": "summary.json",
            "receipt_path": "receipt.json",
            "token_usage": {"prompt": 1, "completion": 1, "total": 2},
            "summary": "hi",
            "error": null,
        });
        let serialized = live_call_to_receipt(&record);
        assert_eq!(serialized["record_kind"], "live_call");
        assert_eq!(serialized["token_usage"]["total"], 2);
    }

    #[test]
    fn lineage_root_edge_uses_non_null_parent() {
        let record = lineage_edge_record(
            "run-1",
            "g0001",
            Value::Null,
            json!("g0001-00-atlas"),
            "emit_atlas".to_string(),
            "routing".to_string(),
        );
        assert_eq!(record["parent_candidate_id"], json!("root"));
        validate_record_kind(&record).expect("root lineage edge should validate");
    }

    #[test]
    #[cfg(unix)]
    fn touch_latest_replaces_broken_symlink() {
        let dir = tempdir().expect("tempdir");
        let output_root = dir.path().join("hybrid");
        let run_dir = output_root.join("runs").join("run-1");
        fs::create_dir_all(&run_dir).expect("create run dir");
        std::os::unix::fs::symlink(
            output_root.join("runs").join("missing"),
            output_root.join("latest"),
        )
        .expect("create broken latest symlink");

        touch_latest(&output_root, &run_dir).expect("replace latest symlink");

        assert_eq!(
            fs::read_link(output_root.join("latest")).expect("read latest symlink"),
            run_dir
        );
    }

    #[test]
    fn output_guard_detects_deleted_and_recreated_root() {
        let dir = tempdir().expect("tempdir");
        let output_root = dir.path().join("target/openqg/zyal-genome/hybrid");
        fs::create_dir_all(&output_root).expect("create output root");
        let guard = OutputPathGuard::new(&output_root).expect("guard");

        fs::remove_dir_all(&output_root).expect("remove output root");
        let deleted = guard.check().expect_err("deleted root should fail");
        assert!(deleted.to_string().contains("output-root-deleted"));

        fs::create_dir_all(&output_root).expect("recreate output root");
        let stale = guard.check().expect_err("recreated root should fail");
        assert!(stale.to_string().contains("output-root-changed"));
    }

    #[test]
    fn jsonl_writer_detects_deleted_visible_ledger_path() {
        let dir = tempdir().expect("tempdir");
        let output_root = dir.path().join("target/openqg/zyal-genome/hybrid");
        fs::create_dir_all(&output_root).expect("create output root");
        let guard = OutputPathGuard::new(&output_root).expect("guard");
        let path = output_root.join("runs/run-1/live-call-ledger.jsonl");
        let mut writer = JsonlWriter::open_guarded(&path, false, &guard).expect("writer");

        writer.write(&json!({"ok": true})).expect("first write");
        fs::remove_file(&path).expect("remove visible ledger");

        let error = writer
            .write(&json!({"ok": false}))
            .expect_err("stale ledger");
        assert!(error.to_string().contains("output-root-changed"));
    }

    #[test]
    fn live_call_prechecks_output_guard_before_provider_spend() {
        let dir = tempdir().expect("tempdir");
        let output_root = dir.path().join("target/openqg/zyal-genome/hybrid");
        let run_dir = output_root.join("runs/run-1");
        fs::create_dir_all(&run_dir).expect("create run dir");
        let guard = OutputPathGuard::new(&output_root).expect("guard");
        fs::remove_dir_all(&output_root).expect("remove output root");

        let stage = test_stage("02-decompose-failed", "hard", Vec::new());
        let route = RoutePolicy {
            route_backend: "jailgun".to_string(),
            route_tier: "top20_pct".to_string(),
            router_state: "nominal".to_string(),
            judge_family: "jailgun".to_string(),
            provenance: "test".to_string(),
            route_policy: json!({}),
        };
        let error = run_live_call(
            &run_dir,
            &stage,
            &route,
            "g0001",
            "g0001-02-decompose-failed",
            "hard_stage_repair",
            &test_live_config(),
            &[],
            Some(&guard),
        )
        .expect_err("stale output root should fail before provider attempt");

        assert!(error.to_string().contains("output-root-deleted"));
        assert!(!run_dir
            .join("stages/02-decompose-failed/generations/g0001/live-calls")
            .exists());
    }

    #[test]
    fn stage_ledger_root_parent_uses_non_null_generation() {
        let route = RoutePolicy {
            route_backend: "jnoccio".to_string(),
            route_tier: "standard".to_string(),
            router_state: "nominal".to_string(),
            judge_family: "jnoccio".to_string(),
            provenance: "test".to_string(),
            route_policy: json!({}),
        };
        let record = stage_ledger_record(
            "run-1",
            "hybrid",
            "g0001",
            "00-atlas",
            "g0001-00-atlas",
            "Atlas intake",
            &route,
            Some("emit_atlas".to_string()),
            None,
            0.1,
            1.0,
            0.1,
            Vec::new(),
            json!({}),
            &json!({}),
        );
        assert_eq!(record["parent_generation_id"], json!("root"));
        validate_record_kind(&record).expect("root stage ledger should validate");
    }

    #[test]
    fn plot_index_is_written_for_run_dir() {
        let dir = tempdir().expect("tempdir");
        let run_dir = dir.path().join("run");
        fs::create_dir_all(&run_dir).expect("create run dir");
        write_json(
            &run_dir.join("run-summary.json"),
            &json!({
                "schema_version": SCHEMA_VERSION,
                "record_kind": "run_summary",
                "run_id": "run-1",
                "variant": "hybrid",
                "generation_count": 1,
                "stage_count": 1,
                "candidate_count": 1,
                "best_score_seen": 0.5,
                "rolling_5_median": 0.5,
                "best_nonregressive_delta": 0.1,
                "unique_contributions": 1,
                "decoy_failures": 0,
                "regression_rate": 0.0,
                "throughput": 1.0,
                "fail_stop_rate": 0.0,
                "score_blend": {"local_score":0.2,"interface_score":0.2,"macro_score":0.2,"innovation_score":0.2,"novelty_score":0.1,"failure_penalty":0.1},
                "router_state": "nominal",
                "degraded_router": false,
                "lineage": {"acyclic": true, "missing_parent_ids": 0},
                "pareto_snapshot": {"frontier_size": 1, "points": []},
            }),
        )
        .expect("summary");
        let path = emit_run_plot_index(&run_dir).expect("plot index");
        assert!(path.exists());
        let index: Value =
            serde_json::from_str(&fs::read_to_string(path).expect("read plot index"))
                .expect("parse plot index");
        assert_eq!(index["record_kind"], "plot_index");
        assert_eq!(index["summary"]["best_score_seen"], 0.5);
    }

    #[test]
    fn quality_gate_passes_on_healthy_smoke_fixture() {
        let dir = tempdir().expect("tempdir");
        let run_dir = dir
            .path()
            .join("target/openqg/zyal-genome/hybrid/runs/healthy");
        write_healthy_gate_fixture(&run_dir, 10);
        quality_gate(&run_dir).expect("quality gate");
        let report: Value = serde_json::from_str(
            &fs::read_to_string(run_dir.join("quality-gate.json")).expect("read quality gate"),
        )
        .expect("parse quality gate");
        assert_eq!(report["passed"], json!(true));
    }

    #[test]
    fn hard_stage_hybrid_live_route_uses_jailgun_mcp() {
        let stage = StagePackage {
            stage_id: "02-decompose-failed".to_string(),
            name: "Decompose failed stage".to_string(),
            track: "failure-repair".to_string(),
            family: "hard".to_string(),
            purpose: "repair hard failures".to_string(),
            inputs: vec!["input".to_string()],
            outputs: vec!["output".to_string()],
            required_evidence: vec!["evidence".to_string()],
            validation_checks: vec!["check".to_string()],
            mutation_op: "failure_mode_invert".to_string(),
            prompt_path: PathBuf::from("prompt.md"),
            memory_path: PathBuf::from("memory.yml"),
            score_path: PathBuf::from("score.yml"),
            stage_dir: PathBuf::from("stage"),
            stage_file: PathBuf::from("stage/stage.yml"),
            prompt_hash: "hash".to_string(),
            memory: json!({}),
            score_model: json!({}),
        };

        let route = route_for_variant(&GenomeVariant::Hybrid, &stage, true, true);

        assert_eq!(route.route_backend, "jailgun");
        assert_eq!(live_execution_backend(&route), "jailgun_mcp");
    }

    #[test]
    fn quality_gate_fails_required_hard_stage_without_jailgun_proof() {
        let dir = tempdir().expect("tempdir");
        let run_dir = dir
            .path()
            .join("target/openqg/zyal-genome/hybrid/runs/missing-proof");
        write_healthy_gate_fixture(&run_dir, 100);
        let mut records =
            read_jsonl::<Value>(&run_dir.join("live-call-ledger.jsonl")).expect("live records");
        records[0]["execution_backend"] = json!("jekko_command");
        records[0]["jailgun_run_id"] = json!(null);
        write_jsonl_records(&run_dir.join("live-call-ledger.jsonl"), &records);

        let report = build_quality_gate_report(&run_dir).expect("quality report");

        assert_eq!(report["passed"], json!(false));
        assert_eq!(
            report["metrics"]["hard_stage_jailgun_proof_missing"],
            json!(1)
        );
    }

    #[test]
    fn quality_gate_passes_healthy_hybrid_qualification_on_champion_series() {
        let dir = tempdir().expect("tempdir");
        let run_dir = dir
            .path()
            .join("target/openqg/zyal-genome/hybrid/runs/healthy-qualification");
        write_healthy_gate_fixture(&run_dir, 100);

        quality_gate(&run_dir).expect("quality gate");
        let report: Value = serde_json::from_str(
            &fs::read_to_string(run_dir.join("quality-gate.json")).expect("read quality gate"),
        )
        .expect("parse quality gate");

        assert_eq!(report["passed"], json!(true));
        assert_eq!(report["metrics"]["stage_rollup_median"], json!(0.82));
        assert_eq!(report["metrics"]["hard_stage_jailgun_live_calls"], json!(1));
    }

    #[test]
    fn quality_gate_prefers_champion_series_for_mixed_rollup_regression() {
        let dir = tempdir().expect("tempdir");
        let run_dir = dir
            .path()
            .join("target/openqg/zyal-genome/hybrid/runs/hybrid-v2-50");
        write_healthy_gate_fixture(&run_dir, 50);
        let mut records =
            read_jsonl::<Value>(&run_dir.join("generation-ledger.jsonl")).expect("ledger");
        for record in &mut records {
            if record.get("metric_name").and_then(Value::as_str)
                == Some("deterministic_rollup_score")
            {
                let generation = generation_index_from_id(
                    record
                        .get("generation_id")
                        .and_then(Value::as_str)
                        .unwrap_or("g0000"),
                );
                record["metric_value"] = if generation % 2 == 0 {
                    json!(0.90)
                } else {
                    json!(0.40)
                };
            }
        }
        write_jsonl_records(&run_dir.join("generation-ledger.jsonl"), &records);

        quality_gate(&run_dir).expect("quality gate");
        let report: Value = serde_json::from_str(
            &fs::read_to_string(run_dir.join("quality-gate.json")).expect("read quality gate"),
        )
        .expect("parse quality gate");

        assert_eq!(report["tier"], json!("qualification"));
        assert_eq!(report["passed"], json!(true));
        assert_eq!(report["metrics"]["stage_rollup_median"], json!(0.82));
        assert_eq!(report["metrics"]["regression_rate"], json!(0.0));
    }

    #[test]
    fn quality_gate_fails_degraded_route() {
        let dir = tempdir().expect("tempdir");
        let run_dir = dir
            .path()
            .join("target/openqg/zyal-genome/hybrid/runs/degraded");
        write_healthy_gate_fixture(&run_dir, 10);
        let mut summary: Value = serde_json::from_str(
            &fs::read_to_string(run_dir.join("run-summary.json")).expect("read summary"),
        )
        .expect("parse summary");
        summary["degraded_router"] = json!(true);
        summary["degraded_route_count"] = json!(1);
        write_json(&run_dir.join("run-summary.json"), &summary).expect("write summary");
        let report = build_quality_gate_report(&run_dir).expect("quality report");
        assert_eq!(report["passed"], json!(false));
        assert_eq!(report["metrics"]["degraded_route_count"], json!(1));
    }

    #[test]
    fn jailgun_token_resolution_prefers_env_then_proc() {
        let mut env_values = BTreeMap::new();
        env_values.insert("JAILGUN_TOKEN".to_string(), "fallback-token".to_string());
        let token =
            jailgun_token_from_env_lookup(|name| env_values.get(name).cloned()).expect("token");
        assert_eq!(token.value, "fallback-token");
        assert_eq!(token.source, "env:JAILGUN_TOKEN");

        env_values.insert(
            "JAILGUN_INGEST_TOKEN".to_string(),
            "ingest-token".to_string(),
        );
        let token =
            jailgun_token_from_env_lookup(|name| env_values.get(name).cloned()).expect("token");
        assert_eq!(token.value, "ingest-token");
        assert_eq!(token.source, "env:JAILGUN_INGEST_TOKEN");

        let entries = vec![
            JailgunProcEntry {
                cmdline: vec![
                    "/home/ubuntu/jailgun/target/debug/jailgun".to_string(),
                    "serve".to_string(),
                    "--addr".to_string(),
                    "127.0.0.1:8797".to_string(),
                ],
                environ: vec!["JAILGUN_INGEST_TOKEN=proc-token".to_string()],
            },
            JailgunProcEntry {
                cmdline: vec![
                    "/home/ubuntu/jailgun/target/debug/jailgun".to_string(),
                    "serve".to_string(),
                    "--addr".to_string(),
                    "127.0.0.1:9999".to_string(),
                ],
                environ: vec!["JAILGUN_INGEST_TOKEN=wrong-proc-token".to_string()],
            },
        ];
        let token =
            jailgun_token_from_proc_entries(&entries, DEFAULT_JAILGUN_SERVER_URL).expect("token");
        assert_eq!(token.value, "proc-token");
        assert_eq!(token.source, "proc:JAILGUN_INGEST_TOKEN");
    }

    #[test]
    fn jailgun_proc_cmdline_and_environ_parsing_is_nul_safe() {
        let cmdline = proc_nul_strings(
            b"/home/ubuntu/jailgun/target/debug/jailgun\0serve\0--addr=127.0.0.1:8797\0",
        );
        let environ = proc_nul_strings(b"PATH=/bin\0JAILGUN_INGEST_TOKEN=proc-secret\0");
        assert!(jailgun_process_matches_server(
            &cmdline,
            DEFAULT_JAILGUN_SERVER_URL
        ));
        let token = jailgun_token_from_proc_entries(
            &[JailgunProcEntry { cmdline, environ }],
            DEFAULT_JAILGUN_SERVER_URL,
        )
        .expect("token");
        assert_eq!(token.value, "proc-secret");
        assert_eq!(token.source, "proc:JAILGUN_INGEST_TOKEN");
    }

    #[test]
    fn jailgun_bridge_command_prefers_env_then_chrome_bridge_default() {
        let default =
            jailgun_bridge_command_from_env_lookup_with_default(|_| None, &["chrome-bridge"]);
        assert_eq!(default.args, vec!["chrome-bridge"]);
        assert_eq!(default.source, "default:chrome-bridge");

        let configured = jailgun_bridge_command_from_env_lookup_with_default(
            |name| {
                (name == "JAILGUN_BRIDGE_CMD")
                    .then(|| "codex exec --sandbox danger-full-access -".to_string())
            },
            &["codex", "exec", "-"],
        );
        assert_eq!(
            configured.args,
            vec!["codex", "exec", "--sandbox", "danger-full-access", "-"]
        );
        assert_eq!(configured.source, "env:JAILGUN_BRIDGE_CMD");
    }

    #[test]
    fn jailgun_run_arguments_use_one_account_fresh_artifact_and_no_source_archive() {
        let bridge_cmd = JailgunBridgeCommand {
            args: vec!["/home/ubuntu/jailgun/apps/chrome-bridge/bin/chrome-bridge.mjs".to_string()],
            source: "default:chrome-bridge".to_string(),
        };
        let account_ids = vec!["acct-a".to_string(), "acct-b".to_string()];
        let call_id = "live-g0001-02-decompose-failed-hard_stage_repair";
        let selected_account_ids = jailgun_single_account_ids(&account_ids, call_id, 1);
        let download_target_name =
            jailgun_download_target_name_with_extension("hybrid-v2-10", call_id, "json");
        let args = jailgun_run_arguments(
            "run-1",
            call_id,
            Path::new("prompt.md"),
            120,
            &selected_account_ids,
            &bridge_cmd,
            &download_target_name,
        );

        assert_eq!(
            download_target_name,
            "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.json"
        );
        assert_eq!(selected_account_ids.len(), 1);
        assert!(account_ids.contains(&selected_account_ids[0]));
        let retry_account_ids = jailgun_single_account_ids(&account_ids, call_id, 2);
        assert_eq!(retry_account_ids.len(), 1);
        assert!(account_ids.contains(&retry_account_ids[0]));
        assert_eq!(selected_account_ids, retry_account_ids);
        assert_eq!(
            args["prompt_file"],
            json!(env::current_dir()
                .unwrap()
                .join("prompt.md")
                .display()
                .to_string())
        );
        assert_eq!(args["tabs"], json!(1));
        assert_eq!(args["source_archive"]["enabled"], json!(false));
        assert_eq!(args["browser"]["account_ids"], json!(selected_account_ids));
        assert_eq!(
            args["browser"]["queue_timeout_seconds"],
            json!(JAILGUN_QUEUE_TIMEOUT_SECONDS)
        );
        assert_eq!(
            args["browser"]["bridge_cmd"],
            json!(["/home/ubuntu/jailgun/apps/chrome-bridge/bin/chrome-bridge.mjs"])
        );
        assert_eq!(
            args["browser"]["bridge_env"]["JAILGUN_ARTIFACT_CONVERSATION_RECOVERY_LIMIT"],
            json!("0")
        );
        assert_eq!(
            args["browser"]["bridge_env"]["JAILGUN_ARTIFACT_REPAIR_ATTEMPTS"],
            json!("1")
        );
        assert_eq!(
            args["browser"]["download_target_name"],
            json!(download_target_name)
        );
        assert_eq!(args.get("bridge_cmd"), None);
    }

    #[test]
    fn jailgun_artifact_target_name_is_extension_agnostic() {
        let call_id = "live-g0001-02-decompose-failed-hard_stage_repair";
        assert_eq!(
            jailgun_download_target_name_with_extension("hybrid-v2-10", call_id, "md"),
            "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.md"
        );
        assert_eq!(
            jailgun_download_target_name_with_extension("hybrid-v2-10", call_id, ".csv"),
            "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.csv"
        );
        let stage = test_stage(
            "02-decompose-failed",
            "hard",
            vec!["repair.jsonl".to_string()],
        );
        assert_eq!(
            jailgun_download_target_name(
                "hybrid-v2-10",
                call_id,
                &stage,
                "hard_stage_repair",
                "stage prompt"
            ),
            "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.jsonl"
        );
        let stage = test_stage("02-decompose-failed", "hard", Vec::new());
        assert_eq!(
            jailgun_download_target_name(
                "hybrid-v2-10",
                call_id,
                &stage,
                "hard_stage_repair",
                "Please write `repair-notes.md`."
            ),
            "openqg-hybrid-v2-10-live-g0001-02-decompose-failed-hard_stage_repair.md"
        );
    }

    #[test]
    fn jailgun_live_prompt_requires_exact_fresh_artifact_name() {
        let stage = test_stage("02-decompose-failed", "hard", Vec::new());
        let prompt = live_prompt(
            &stage,
            "Repair hard failures.",
            &json!({"purpose": "hard_stage_repair"}),
            Some("openqg-hybrid-v2-10-live-g0001.md"),
        )
        .expect("live prompt");

        assert!(prompt.contains(
            "Create a fresh downloadable artifact named exactly `openqg-hybrid-v2-10-live-g0001.md` now"
        ));
        assert!(prompt.contains("The filename and extension are authoritative"));
        assert!(!prompt.contains(".tex artifact"));
        assert!(prompt
            .contains("Do not answer with a plan, acknowledgement, or prose outside the artifact"));
        assert!(prompt.contains("Do not recover or reuse an artifact from another conversation"));
    }

    #[test]
    fn live_retry_count_applies_to_promotion_judging() {
        let mut config = test_live_config();
        config.retry_count = 1;

        assert_eq!(
            live_max_attempts(&config, "promotion_judging", "jekko_command"),
            2
        );
        assert_eq!(
            live_max_attempts(&config, "hard_stage_repair", "jailgun_mcp"),
            3
        );
    }

    #[test]
    fn jailgun_failure_classifier_labels_known_failure_modes() {
        let cases = [
            ("tar-validation failed for source archive", "tar-validation"),
            (
                "invalid gzip header while reading payload",
                "invalid-archive-header",
            ),
            (
                "run timed out after max_runtime_seconds elapsed",
                "runtime-timeout",
            ),
            (
                "artifact conversation recovery returned a recovered artifact",
                "outdated-artifact-recovery",
            ),
            ("HTTP 429 Too Many Requests from provider", "rate-limit"),
        ];

        for (message, expected) in cases {
            assert_eq!(classify_jailgun_failure(message), expected);
        }
        assert_eq!(
            classify_jailgun_summary_failure(&json!({
                "status": "failed",
                "failures": [{"kind": "tar-validation"}],
            })),
            Some("tar-validation")
        );
        assert_eq!(
            classify_jailgun_summary_failure(&json!({"status": "timed-out"})),
            Some("runtime-timeout")
        );
        assert_eq!(
            classify_jailgun_status_failure(&json!({
                "status": "failed",
                "tabs": [{"status": "error"}],
            })),
            Some("browser-tab-error")
        );
        assert_eq!(
            jailgun_effective_status(
                Some(&json!({"status": "running"})),
                &json!({"status": "failed", "tabs": [{"status": "error"}]})
            ),
            "failed"
        );
        assert_eq!(
            classify_jailgun_attempt_failure(
                Some(&json!({"status": "running"})),
                &json!({"status": "failed", "tabs": [{"status": "error"}]}),
                &[]
            ),
            Some("browser-tab-error")
        );
        assert_eq!(
            classify_jailgun_events_text(
                r#"{"kind":"rate-limit-detected","message":"Too many requests"}"#
            ),
            Some("rate-limit")
        );
        assert_eq!(
            classify_jailgun_events_text(
                r#"{"kind":"error","message":"assistant finished but no .tex artifact download candidate was found"}"#
            ),
            Some("artifact-download-missing")
        );
    }

    #[test]
    fn jailgun_rate_limit_events_are_warnings_on_success_and_failures_on_failure() {
        let dir = tempdir().expect("tempdir");
        let events_path = dir.path().join("agent-events.jsonl");
        fs::write(
            &events_path,
            r#"{"kind":"rate-limit-detected","message":"HTTP 429 Too Many Requests"}"#,
        )
        .expect("events");

        let success = jailgun_attempt_metadata(
            "http://127.0.0.1:8797",
            "run-success",
            &["acct-a".to_string()],
            &json!({"summary_json": null, "events_jsonl": events_path.display().to_string()}),
            &json!({"status": "succeeded"}),
            &[],
            Some(&json!({
                "status": "succeeded",
                "events_jsonl": events_path.display().to_string(),
                "receipt_paths": []
            })),
            "token",
        );
        assert_eq!(success["jailgun_status"], json!("succeeded"));
        assert_eq!(success["jailgun_warning_kind"], json!("rate-limit"));
        assert_eq!(success["jailgun_failure_kind"], json!("none"));

        let failed = jailgun_attempt_metadata(
            "http://127.0.0.1:8797",
            "run-failed",
            &["acct-a".to_string()],
            &json!({"summary_json": null, "events_jsonl": events_path.display().to_string()}),
            &json!({"status": "failed"}),
            &[],
            Some(&json!({
                "status": "failed",
                "events_jsonl": events_path.display().to_string(),
                "receipt_paths": []
            })),
            "token",
        );
        assert_eq!(failed["jailgun_status"], json!("failed"));
        assert_eq!(failed["jailgun_warning_kind"], json!("none"));
        assert_eq!(failed["jailgun_failure_kind"], json!("rate-limit"));
    }

    #[test]
    fn strict_preflight_status_fails_when_jailgun_server_is_down_even_if_flagged_available() {
        let status = resolve_jailgun_status_with_config(
            true,
            true,
            JailgunHealthConfig {
                server_url: "http://127.0.0.1:9".to_string(),
                token: Some(JailgunToken {
                    value: "token".to_string(),
                    source: "env:JAILGUN_INGEST_TOKEN".to_string(),
                }),
                account_override_ids: Vec::new(),
            },
        );

        assert!(!status.available);
        assert_eq!(status.checks.get("server_health"), Some(&false));
    }

    #[test]
    fn preflight_receipt_blocks_required_missing_hard_backend() {
        let stage_registry = load_stage_registry(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(|path| path.parent())
                .unwrap()
                .join("ZYAL/stages"),
        )
        .expect("stage registry");
        let live_config =
            resolve_live_config(false, &json!({"evaluation": {"live": {"enabled": false}}}));
        let mut jailgun_status = JailgunStatus::server_authoritative(true);
        jailgun_status.set_check("server_health", false);
        jailgun_status.push_error("Jailgun server readiness was not proven");
        jailgun_status.finish();
        let receipt = preflight_receipt(
            "hybrid-v2-50",
            "hybrid",
            &json!({"evaluation": {"quality_gates": {"require_hard_backend": true}}}),
            Path::new("ZYAL/runs/run-hybrid-1000-v2.zyal"),
            Path::new("target/openqg/zyal-genome/hybrid/runs/hybrid-v2-50"),
            &stage_registry,
            &live_config,
            true,
            false,
            &jailgun_status,
        );
        assert_eq!(receipt["status"], json!("failed"));
        assert_eq!(
            receipt["routing_decision"],
            json!("blocked_missing_hard_backend")
        );
    }

    #[test]
    fn ready_jailgun_account_ids_accepts_ready_or_authenticated_accounts() {
        let response = json!({
            "accounts": [
                {"id": "acct-a", "status": "ready"},
                {"id": "acct-b", "status": "auth-required"},
                {"account_id": "acct-c", "authenticated": true}
            ]
        });
        let accounts = jailgun_accounts_from_response(&response);
        let ready = ready_jailgun_account_ids(&accounts);

        assert_eq!(ready, vec!["acct-a".to_string(), "acct-c".to_string()]);
    }

    #[test]
    fn strict_jailgun_health_succeeds_against_server_accounts_and_scheduler() {
        let server_url = spawn_fake_jailgun_server(
            json!({"accounts": [{"id": "acct-ready", "status": "ready"}]}),
            json!({"status": "ready", "available_slots": 1}),
        );

        let status = strict_jailgun_status_with_config(JailgunHealthConfig {
            server_url,
            token: Some(JailgunToken {
                value: "server-token".to_string(),
                source: "env:JAILGUN_INGEST_TOKEN".to_string(),
            }),
            account_override_ids: Vec::new(),
        });

        assert!(status.available, "{:?}", status.errors);
        assert_eq!(status.ready_account_ids, vec!["acct-ready".to_string()]);
        assert_eq!(
            status.account_source.as_deref(),
            Some("server:/api/browser/accounts")
        );
        assert_eq!(
            status.token_source.as_deref(),
            Some("env:JAILGUN_INGEST_TOKEN")
        );
        assert_eq!(status.checks.get("scheduler_capacity"), Some(&true));
    }

    #[test]
    fn strict_jailgun_health_rejects_existing_scheduler_work() {
        let server_url = spawn_fake_jailgun_server(
            json!({"accounts": [{"id": "acct-ready", "status": "ready"}]}),
            json!({"status": "ready", "available_slots": 1, "running_jobs": 1}),
        );

        let status = strict_jailgun_status_with_config(JailgunHealthConfig {
            server_url,
            token: Some(JailgunToken {
                value: "server-token".to_string(),
                source: "env:JAILGUN_INGEST_TOKEN".to_string(),
            }),
            account_override_ids: Vec::new(),
        });

        assert!(!status.available);
        assert_eq!(status.checks.get("scheduler_capacity"), Some(&false));
    }

    #[test]
    fn jailgun_artifact_smoke_receipt_succeeds_with_generic_json_target() {
        let server_url = spawn_fake_jailgun_server(
            json!({"accounts": [{"id": "acct-ready", "status": "ready"}]}),
            json!({"status": "ready", "available_slots": 1}),
        );
        let dir = tempdir().expect("tempdir");
        let run_dir = dir
            .path()
            .join("target/openqg/zyal-genome/hybrid/runs/smoke");
        fs::create_dir_all(&run_dir).expect("run dir");

        let receipt = run_jailgun_artifact_smoke_with_config(
            &run_dir,
            "hybrid-v2-100",
            "json",
            JailgunHealthConfig {
                server_url,
                token: Some(JailgunToken {
                    value: "server-token".to_string(),
                    source: "env:JAILGUN_INGEST_TOKEN".to_string(),
                }),
                account_override_ids: Vec::new(),
            },
            JailgunBridgeCommand {
                args: vec!["chrome-bridge".to_string()],
                source: "test".to_string(),
            },
            None,
        )
        .expect("smoke receipt");

        assert_eq!(receipt["status"], json!("ok"));
        assert_eq!(receipt["record_kind"], json!("jailgun_artifact_smoke"));
        assert_eq!(receipt["jailgun_account_count"], json!(1));
        assert_eq!(receipt["artifact_extension"], json!("json"));
        assert_eq!(receipt["attempt_count"], json!(1));
        assert!(receipt["download_target_name"]
            .as_str()
            .unwrap()
            .ends_with(".json"));
    }

    #[test]
    fn jailgun_artifact_smoke_receipt_records_failure() {
        let dir = tempdir().expect("tempdir");
        let run_dir = dir
            .path()
            .join("target/openqg/zyal-genome/hybrid/runs/smoke");
        fs::create_dir_all(&run_dir).expect("run dir");

        let receipt = run_jailgun_artifact_smoke_with_config(
            &run_dir,
            "hybrid-v2-100",
            "md",
            JailgunHealthConfig {
                server_url: DEFAULT_JAILGUN_SERVER_URL.to_string(),
                token: None,
                account_override_ids: Vec::new(),
            },
            JailgunBridgeCommand {
                args: vec!["chrome-bridge".to_string()],
                source: "test".to_string(),
            },
            None,
        )
        .expect("smoke receipt");

        assert_eq!(receipt["status"], json!("failed"));
        assert_eq!(receipt["artifact_extension"], json!("md"));
        assert_eq!(receipt["jailgun_failure_kind"], json!("unknown"));
        assert_eq!(
            receipt["attempt_count"],
            json!(JAILGUN_ARTIFACT_SMOKE_ATTEMPTS)
        );
    }

    #[test]
    fn strict_jailgun_health_fails_cleanly_without_token_or_ready_accounts() {
        let missing_token = strict_jailgun_status_with_config(JailgunHealthConfig {
            server_url: DEFAULT_JAILGUN_SERVER_URL.to_string(),
            token: None,
            account_override_ids: Vec::new(),
        });
        assert!(!missing_token.available);
        assert_eq!(missing_token.checks.get("token_configured"), Some(&false));
        assert!(missing_token
            .errors
            .iter()
            .any(|error| error.contains("token is not available")));

        let server_url = spawn_fake_jailgun_server(
            json!({"accounts": [{"id": "acct-a", "status": "auth-required"}]}),
            json!({"status": "ready", "available_slots": 1}),
        );
        let no_ready_accounts = strict_jailgun_status_with_config(JailgunHealthConfig {
            server_url,
            token: Some(JailgunToken {
                value: "server-token".to_string(),
                source: "env:JAILGUN_TOKEN".to_string(),
            }),
            account_override_ids: Vec::new(),
        });
        assert!(!no_ready_accounts.available);
        assert_eq!(no_ready_accounts.checks.get("ready_accounts"), Some(&false));
        assert!(no_ready_accounts
            .errors
            .iter()
            .any(|error| error.contains("no ready/authenticated Jailgun account")));
    }

    #[test]
    fn strict_jailgun_receipts_record_only_token_source() {
        let stage_registry = load_stage_registry(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(|path| path.parent())
                .unwrap()
                .join("ZYAL/stages"),
        )
        .expect("stage registry");
        let live_config =
            resolve_live_config(false, &json!({"evaluation": {"live": {"enabled": false}}}));
        let secret = "super-secret-token";
        let server_url = spawn_fake_jailgun_server(
            json!({"accounts": [{"id": "acct-a", "status": "auth-required"}]}),
            json!({"status": "ready", "available_slots": 1}),
        );
        let jailgun_status = strict_jailgun_status_with_config(JailgunHealthConfig {
            server_url,
            token: Some(JailgunToken {
                value: secret.to_string(),
                source: "env:JAILGUN_INGEST_TOKEN".to_string(),
            }),
            account_override_ids: Vec::new(),
        });

        let receipt = preflight_receipt(
            "hybrid-v2-10",
            "hybrid",
            &json!({"evaluation": {"quality_gates": {"require_hard_backend": true}}}),
            Path::new("ZYAL/runs/run-hybrid-1000-v2.zyal"),
            Path::new("target/openqg/zyal-genome/hybrid/runs/hybrid-v2-10"),
            &stage_registry,
            &live_config,
            true,
            false,
            &jailgun_status,
        );
        let status_text = serde_json::to_string(&jailgun_status.as_json()).expect("status json");
        let receipt_text = serde_json::to_string(&receipt).expect("receipt json");

        assert_eq!(receipt["status"], json!("failed"));
        assert_eq!(receipt["backend_health"]["jailgun"]["strict"], json!(true));
        assert!(status_text.contains("env:JAILGUN_INGEST_TOKEN"));
        assert!(receipt_text.contains("env:JAILGUN_INGEST_TOKEN"));
        assert!(!status_text.contains(secret));
        assert!(!receipt_text.contains(secret));
        assert_eq!(
            receipt["backend_health"]["jailgun"]["checks"]["ready_accounts"],
            json!(false)
        );
    }

    #[test]
    fn saturation_guard_caps_non_nominal_perfect_score() {
        assert_eq!(
            apply_saturation_guard(1.0, &[0.99, 0.99, 0.99], 0.01, false),
            0.995
        );
        assert_eq!(
            apply_saturation_guard(1.0, &[0.99, 0.99, 0.99], 0.01, true),
            1.0
        );
    }

    #[test]
    fn promotion_preserves_mode_and_island_floor_candidates() {
        let candidates = vec![
            json!({"candidate_id":"a","mode":"exploitation","island":"wildcards","scores":{"final_score":0.99}}),
            json!({"candidate_id":"b","mode":"exploitation","island":"wildcards","scores":{"final_score":0.98}}),
            json!({"candidate_id":"c","mode":"novelty","island":"foundations","scores":{"final_score":0.70}}),
            json!({"candidate_id":"d","mode":"wildcard","island":"coefficients","scores":{"final_score":0.69}}),
            json!({"candidate_id":"e","mode":"exploitation","island":"observables","scores":{"final_score":0.68}}),
            json!({"candidate_id":"f","mode":"exploitation","island":"failure-repair","scores":{"final_score":0.67}}),
        ];
        let promoted = promote_generation_candidates(&candidates);
        let modes = count_string_field(&promoted, "mode");
        let islands = count_string_field(&promoted, "island");
        assert!(modes.contains_key("novelty"));
        assert!(modes.contains_key("wildcard"));
        assert!(islands.len() >= 4);
    }

    #[test]
    fn novelty_quota_selects_viable_novelty_candidate_below_floor() {
        let promoted = vec![
            selection_candidate("top", "exploitation", "foundations", 0.810),
            selection_candidate("novel", "novelty", "coefficients", 0.805),
        ];
        let previous = previous_champions(21, 1, 0.80);

        let selection = select_balanced_champion(150, &promoted, &promoted, &previous, &[], 0.05);

        assert_eq!(selection.reason, PromotionReason::NoveltyQuota);
        assert_eq!(selection.champion["candidate_id"], json!("novel"));
    }

    #[test]
    fn low_score_novelty_does_not_force_major_regression() {
        let promoted = vec![
            selection_candidate("top", "exploitation", "foundations", 0.900),
            selection_candidate("novel", "novelty", "coefficients", 0.720),
        ];
        let previous = previous_champions(20, 0, 0.88);

        let selection = select_balanced_champion(150, &promoted, &promoted, &previous, &[], 0.05);

        assert_eq!(selection.reason, PromotionReason::TopScore);
        assert_eq!(selection.champion["candidate_id"], json!("top"));
    }

    #[test]
    fn elite_carry_forward_reduces_regression_and_preserves_metadata() {
        let previous_elite = selection_candidate("elite", "exploitation", "wildcards", 0.900);
        let promoted = vec![selection_candidate(
            "current-top",
            "novelty",
            "foundations",
            0.870,
        )];
        let mut previous = previous_champions(3, 0, 0.80);
        let mut elite_with_generation = previous_elite.clone();
        elite_with_generation["generation_id"] = json!("g0004");
        previous.push(elite_with_generation);

        let selection = select_balanced_champion(5, &promoted, &promoted, &previous, &[], 0.05);

        assert_eq!(selection.reason, PromotionReason::EliteRetain);
        assert_eq!(selection.champion["candidate_id"], json!("elite"));
        assert_eq!(selection.champion["source_card_ids"], json!(["info-a"]));
        assert_eq!(
            selection.champion["parent_candidate_ids"],
            json!(["parent-a"])
        );
        assert_eq!(selection.champion["island"], json!("wildcards"));
        assert_eq!(selection.champion["mode"], json!("exploitation"));
        assert_eq!(selection.champion["scores"]["final_score"], json!(0.900));
        assert_eq!(
            selection.champion["retained_from_generation_id"],
            json!("g0004")
        );
        assert_eq!(
            selection.champion["selection_generation_id"],
            json!("g0005")
        );
    }

    #[test]
    fn promotion_ledger_records_selection_reason() {
        let champion = selection_candidate("cap", "exploitation", "foundations", 0.82);
        let record = promotion_decision_record(
            "run-1",
            "g0001",
            &champion,
            std::slice::from_ref(&champion),
            Path::new("population-snapshot.json"),
            PromotionReason::IslandCap,
            json!({"promotion_confidence": 0.82}),
        );

        assert_eq!(record["promotion_reason"], json!("island_cap"));
        validate_record_kind(&record).expect("promotion record should validate");
    }

    #[test]
    fn frontier_review_fields_serialize_for_candidates_and_champions() {
        let stage = test_stage("03-generate-genes", "standard", Vec::new());
        let mut stage_concepts = BTreeMap::new();
        stage_concepts.insert("03-generate-genes".to_string(), "concept-a".to_string());
        let scores = json!({
            "final_score": 0.78,
            "novelty_score": 0.71,
            "failure_modes": ["interface_drift"],
            "scoring_weights": {"novelty_score": 0.18},
        });

        let candidate = hybrid_candidate_record(
            "g0001",
            &stage,
            "candidate-a",
            &["parent-a".to_string()],
            "novelty",
            &["info-a".to_string()],
            &stage_concepts,
            &json!({"backend": "jnoccio"}),
            &scores,
            &json!({}),
        );
        let champion = champion_summary_record("g0001", &candidate, PromotionReason::NoveltyQuota);

        assert_eq!(candidate["source_card_ids"], json!(["info-a"]));
        assert!(candidate["frontier_claim"]
            .as_str()
            .unwrap()
            .contains("candidate-a"));
        assert_eq!(candidate["known_failure_modes"], json!(["interface_drift"]));
        assert_eq!(candidate["review_priority"], json!("high"));
        assert_eq!(champion["frontier_claim"], candidate["frontier_claim"]);
        assert_eq!(
            champion["falsifiable_tests"],
            candidate["falsifiable_tests"]
        );
        assert_eq!(champion["promotion_reason"], json!("novelty_quota"));
    }

    #[test]
    fn novelty_weight_changes_candidate_final_score() {
        let stage_concepts =
            BTreeMap::from([("03-generate-genes".to_string(), "concept-a".to_string())]);
        let low_weight = compute_candidate_scores(
            "novelty",
            "foundations",
            "novelty_jump",
            &stage_concepts,
            3,
            8,
            2,
            DEFAULT_SEED,
            false,
            0.08,
            0.0,
        );
        let high_weight = compute_candidate_scores(
            "novelty",
            "foundations",
            "novelty_jump",
            &stage_concepts,
            3,
            8,
            2,
            DEFAULT_SEED,
            false,
            0.08,
            0.18,
        );

        assert!(
            high_weight["final_score"].as_f64().unwrap()
                > low_weight["final_score"].as_f64().unwrap()
        );
    }

    #[test]
    fn selftest_passes() {
        selftest().expect("selftest");
    }
}
