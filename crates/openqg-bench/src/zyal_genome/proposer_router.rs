//! V6: the **router-native proposer** — direct HTTP to the jnoccio-fusion gateway.
//!
//! Replaces the V5 jekko subprocess transport (90–180 s/call, stdout scraping, empty-output
//! blips) with `POST /v1/chat/completions` against the local router: ~131 routed models behind
//! one endpoint, **server-side strict JSON-schema validation with its own repair loop**
//! (`structured_schema_status`), quality-band routing, and `winner_model_id` telemetry per call.
//!
//! Diversity is what the router actually supports: per-slot deterministic **mechanism lanes**
//! (planck_mu0 / dark_scattering / free / null_diagnostic) + **quality-band rotation** per
//! generation, with the winning model ledgered on every attempt. Per-request model pinning does
//! not exist (alias-only gateway), and `/v1/embeddings` is fake on this deployment — both
//! verified, neither used.
//!
//! The transport is injected as a [`RouterCaller`] closure, so the whole machine is unit-tested
//! with no network.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use openqg_core::ObservableRecord;

use super::proposer::{score_proposal, ProposalDoc, Proposer};
use super::proposer_sketch::{
    build_router_prompt, expand_sketch, parse_sketch_response, proposal_sketch_schema, Lane, LANES,
};
use super::theory_population::ProposalAttemptRecord;
use super::token_receipt::{LlmCallReceipt, TokenUsage, TokenizerRegistry};

/// One structured chat call to the router.
#[allow(dead_code)] // sample/attempt indices are receipt fields for test callers
pub(crate) struct RouterRequest {
    pub prompt: String,
    pub quality_band: Option<String>,
    pub temperature: f64,
    pub max_completion_tokens: u64,
    pub schema_name: &'static str,
    pub schema: Value,
    pub sample_index: usize,
    pub attempt_index: usize,
}

/// What came back, projected to what the proposer needs.
#[allow(dead_code)] // structured_status/http_status are diagnostics for analyses
pub(crate) struct RouterResponse {
    /// `choices[0].message.content` — canonical JSON when the router validated it.
    pub content: String,
    /// `jnoccio.winner_model_id`, falling back to the response `model`.
    pub model: String,
    /// Router-side structured-output verdict ("valid" when schema-checked).
    pub structured_status: Option<String>,
    /// Router-internal repair calls spent making the output schema-valid.
    pub upstream_repairs: u64,
    pub elapsed_seconds: f64,
    pub http_status: u16,
    /// Token usage from `usage.prompt_tokens` / `usage.completion_tokens`; 0 when absent.
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

/// Injected transport: production wraps reqwest; tests inject closures.
pub(crate) type RouterCaller = Box<dyn Fn(&RouterRequest) -> Result<RouterResponse> + Send + Sync>;

#[derive(Clone)]
pub(crate) struct RouterConfig {
    pub base_url: String,
    pub token: String,
    pub timeout_seconds: u64,
    /// Best-of-K parallel samples per slot.
    pub samples: usize,
    /// Repair budget per sample (parse repair + at most one oracle repair share it).
    pub repairs: usize,
    pub stagger_seconds: u64,
    /// Quality-band rotation pool; bands shift by one each propose() call.
    pub bands: Vec<String>,
    pub max_completion_tokens: u64,
    pub temperature: f64,
    pub retry_backoff_seconds: u64,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("OPENQG_ROUTER_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:4317".to_string()),
            token: std::env::var("JNOCCIO_TOKEN").unwrap_or_else(|_| "jnoccio-local".to_string()),
            timeout_seconds: 240,
            samples: 4,
            repairs: 2,
            stagger_seconds: 1,
            bands: vec![
                "top20".to_string(),
                "any".to_string(),
                "top50".to_string(),
                "any".to_string(),
            ],
            max_completion_tokens: 16_384,
            temperature: 0.8,
            retry_backoff_seconds: 6,
        }
    }
}

/// Pure retry policy: one retry, only for transient classes. The router already health-routes
/// internally (backups, cooldowns); our retry covers gateway-level 429/5xx and transport errors.
pub(crate) fn should_retry(http_status: Option<u16>, attempt: usize) -> Option<Duration> {
    if attempt >= 2 {
        return None;
    }
    match http_status {
        None => Some(Duration::from_secs(6)), // transport error (connect/timeout)
        Some(429) => Some(Duration::from_secs(6)),
        Some(code) if code >= 500 => Some(Duration::from_secs(6)),
        Some(_) => None, // other 4xx: terminal
    }
}

fn default_caller(cfg: &RouterConfig) -> RouterCaller {
    let base_url = cfg.base_url.clone();
    let token = cfg.token.clone();
    let timeout = cfg.timeout_seconds;
    let backoff = cfg.retry_backoff_seconds;
    Box::new(move |req: &RouterRequest| -> Result<RouterResponse> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(timeout))
            .build()
            .context("build http client")?;
        let mut body = json!({
            "model": "jnoccio/jnoccio-fusion",
            "messages": [{"role": "user", "content": req.prompt}],
            "stream": false,
            "temperature": req.temperature,
            "max_completion_tokens": req.max_completion_tokens,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": req.schema_name,
                    "strict": true,
                    "schema": req.schema,
                }
            }
        });
        if let Some(band) = &req.quality_band {
            body["quality_band"] = json!(band);
        }
        let start = Instant::now();
        let mut attempt = 1usize;
        loop {
            let result = client
                .post(format!("{base_url}/v1/chat/completions"))
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(body.to_string())
                .send();
            let status = result.as_ref().ok().map(|r| r.status().as_u16());
            match result {
                Ok(resp) if resp.status().is_success() => {
                    let code = resp.status().as_u16();
                    let text = resp.text().context("read router response body")?;
                    let v: Value =
                        serde_json::from_str(&text).context("parse router response JSON")?;
                    let content = v["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    // jnoccio returns metadata at v["jnoccio"] (not v["extra"]["jnoccio"]).
                    let meta = &v["jnoccio"];
                    let model = meta["winner_model_id"]
                        .as_str()
                        .or_else(|| v["model"].as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let prompt_tokens =
                        v["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
                    let completion_tokens =
                        v["usage"]["completion_tokens"].as_u64().unwrap_or(0);
                    return Ok(RouterResponse {
                        content,
                        model,
                        structured_status: meta["structured_schema_status"]
                            .as_str()
                            .map(str::to_string),
                        upstream_repairs: meta["structured_repair_attempts"].as_u64().unwrap_or(0),
                        elapsed_seconds: start.elapsed().as_secs_f64(),
                        http_status: code,
                        prompt_tokens,
                        completion_tokens,
                    });
                }
                Ok(resp) => {
                    let code = resp.status().as_u16();
                    if let Some(delay) = should_retry(Some(code), attempt) {
                        std::thread::sleep(delay.max(Duration::from_secs(backoff)));
                        attempt += 1;
                        continue;
                    }
                    let text = resp.text().unwrap_or_default();
                    anyhow::bail!(
                        "router http {code}: {}",
                        text.chars().take(300).collect::<String>()
                    );
                }
                Err(e) => {
                    if let Some(delay) = should_retry(status, attempt) {
                        std::thread::sleep(delay.max(Duration::from_secs(backoff)));
                        attempt += 1;
                        continue;
                    }
                    return Err(e).context("router transport error");
                }
            }
        }
    })
}

/// Token-free preflight: `GET /v1/jnoccio/status`; requires `health.ok` and ≥1 eligible slot.
pub(crate) fn router_preflight(cfg: &RouterConfig) -> Result<()> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("build http client")?;
    let v: String = client
        .get(format!("{}/v1/jnoccio/status", cfg.base_url))
        .header("Authorization", format!("Bearer {}", cfg.token))
        .send()
        .context("router status request — is the jnoccio-fusion gateway up?")?
        .text()
        .context("read router status body")?;
    let v: Value = serde_json::from_str(&v).context("parse router status")?;
    let ok = v["health"]["ok"].as_bool().unwrap_or(false) || v["ok"].as_bool().unwrap_or(false);
    let slots = v["eligible_slot_count"]
        .as_u64()
        .or_else(|| v["health"]["eligible_slot_count"].as_u64())
        .unwrap_or(0);
    anyhow::ensure!(
        ok && slots >= 1,
        "router unhealthy: ok={ok}, eligible_slot_count={slots}"
    );
    Ok(())
}

/// Build a compact negative-memory block from the current run's on-disk attempts file (U3).
///
/// Reads the `proposal-attempts.jsonl` that the engine writes after each generation drain.
/// Returns a non-empty string only when ≥5 disqualified records exist (signal/noise threshold).
/// Pure w.r.t. its input: two calls on the same file produce the same output.
/// Parsed example from a `fabricated_novel_prediction` kill message.
struct FabricatedExample {
    observable: String,
    declared: f64,
    computed: f64,
}

/// Attempt to extract observable/declared/computed from a raw kill reason string.
/// Kill format: "fabricated novel prediction: claim X declared OBS=D but the model computes C ..."
fn parse_fabricated_novel_example(reason: &str) -> Option<FabricatedExample> {
    // Find "declared OBS=D"
    let declared_pos = reason.find("declared ")?;
    let after_declared = &reason[declared_pos + 9..];
    let eq_pos = after_declared.find('=')?;
    let observable = after_declared[..eq_pos].trim().to_string();
    let after_eq = &after_declared[eq_pos + 1..];
    // D ends at first whitespace
    let d_end = after_eq
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after_eq.len());
    let declared: f64 = after_eq[..d_end].parse().ok()?;
    // Find "computes C"
    let computes_pos = reason.find("computes ")?;
    let after_computes = &reason[computes_pos + 9..];
    let c_end = after_computes
        .find(|c: char| c.is_whitespace() || c == '(')
        .unwrap_or(after_computes.len());
    let computed: f64 = after_computes[..c_end].parse().ok()?;
    Some(FabricatedExample { observable, declared, computed })
}

fn build_within_run_kill_block(path: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(path) else {
        return String::new();
    };
    let mut total: usize = 0;
    let mut disq: usize = 0;
    let mut classes: BTreeMap<&'static str, usize> = BTreeMap::new();
    // Phase 40: collect raw fabricated_novel kill reasons for calibration examples.
    let mut fabricated_examples: Vec<FabricatedExample> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        total += 1;
        let is_disq = v.get("disqualified").and_then(serde_json::Value::as_bool) != Some(false)
            || v.get("outcome")
                .and_then(serde_json::Value::as_str)
                .map(|o| o != "winner" && o != "promoted")
                .unwrap_or(false);
        if !is_disq {
            continue;
        }
        disq += 1;
        // Collect kill classes using the same normalizer as proposer_memory.
        for krs_key in ["kill_reasons", "kill_reason"] {
            if let Some(val) = v.get(krs_key) {
                let reasons: Vec<&str> = if let Some(arr) = val.as_array() {
                    arr.iter().filter_map(serde_json::Value::as_str).collect()
                } else if let Some(s) = val.as_str() {
                    vec![s]
                } else {
                    vec![]
                };
                for r in reasons {
                    let class = normalize_kill_class_for_within_run(r);
                    *classes.entry(class).or_insert(0) += 1;
                    // Phase 40: extract calibration examples from fabricated_novel kills.
                    if class == "fabricated_novel_prediction" {
                        if let Some(ex) = parse_fabricated_novel_example(r) {
                            // Keep at most 4 unique observables for prompt brevity.
                            if fabricated_examples.len() < 4
                                && !fabricated_examples.iter().any(|e| e.observable == ex.observable)
                            {
                                fabricated_examples.push(ex);
                            }
                        }
                    }
                }
            }
        }
        if v.get("outcome").and_then(serde_json::Value::as_str) == Some("parse_error") {
            *classes.entry("parse_error").or_insert(0) += 1;
        }
    }
    if disq < 5 {
        return String::new();
    }
    let mut sorted: Vec<(&'static str, usize)> = classes.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let mut out = String::new();
    out.push_str(&format!(
        "\n## WITHIN-RUN KILLS ({total} attempts, {disq} disqualified this run)\n"
    ));
    for (class, count) in sorted.iter().take(6) {
        out.push_str(&format!("  {class}: {count}\n"));
    }
    // Phase 40: inject fabricated_novel calibration examples so the LLM learns the actual
    // engine-computed physical scale, not a hallucinated value.
    if !fabricated_examples.is_empty() {
        out.push_str(
            "FABRICATED NOVEL KILLS — engine-computed values from this run (use these, not guesses):\n",
        );
        for ex in &fabricated_examples {
            out.push_str(&format!(
                "  {obs}: engine={comp:.4} (you declared {decl:.4} — WRONG by {ratio:.1}x)\n",
                obs = ex.observable,
                comp = ex.computed,
                decl = ex.declared,
                ratio = if ex.declared.abs() > 1e-12 {
                    (ex.computed / ex.declared).abs()
                } else {
                    f64::INFINITY
                },
            ));
        }
        out.push_str(
            "  RULE: copy the engine-computed value into NovelPredictionWitness.predicted — \
             never invent a number.\n",
        );
    }
    out.push_str(
        "DON'T repeat mechanism variations that already failed — they will fail again.\n",
    );
    out
}

/// Coarse kill-class normalizer for within-run block (mirrors proposer_memory normalization).
fn normalize_kill_class_for_within_run(reason: &str) -> &'static str {
    if reason.contains("UnknownRelation") {
        "unknown_relation"
    } else if reason.contains("evidence") {
        "unbound_evidence"
    } else if reason.contains("no_hidden_knob") || reason.contains("hidden") {
        "hidden_knob"
    } else if reason.contains("FreeParameter") {
        "free_parameter"
    } else if reason.contains("UnverifiedDerivation") {
        "unverified_derivation"
    } else if reason.contains("FailedDerivationCertificate") {
        "failed_derivation_certificate"
    } else if reason.contains("fabricated novel prediction") || reason.contains("fabricated_novel") {
        "fabricated_novel_prediction"
    } else if reason.contains("data_fit_gate_failed") {
        "data_fit_gate_failed"
    } else if reason.contains("Screening") || reason.contains("screening") {
        "screening_implausible"
    } else if reason.contains("Unimplemented") {
        "unimplemented_modification"
    } else if reason.contains("Unexplained") {
        "unexplained_modification"
    } else if reason.contains("Conflicting") {
        "conflicting_modification"
    } else {
        "other"
    }
}

/// The V6 free-compute proposer: best-of-K parallel samples per slot, each on its own mechanism
/// lane, with router-side schema validation, client parse/oracle repair, and total observability.
pub(crate) struct RouterProposer {
    cfg: RouterConfig,
    observables: Vec<ObservableRecord>,
    /// V7 (P1.4): the proposer pre-scores with the SAME covariance blocks the engine judges by.
    blocks: Vec<openqg_core::scoring::CovarianceBlock>,
    baseline_ll: f64,
    extra_sections: String,
    /// U3 (Phase 33): path to the current run's proposal-attempts.jsonl — read each propose()
    /// call to inject within-run kill feedback into the prompt. None if not set.
    within_run_path: Option<PathBuf>,
    attempts: RefCell<Vec<ProposalAttemptRecord>>,
    /// Phase 38: one LlmCallReceipt per router HTTP call, drained by theory_population into sink.
    receipts: RefCell<Vec<LlmCallReceipt>>,
    /// Deterministic rotation seed: bands shift by one per propose() call.
    call_no: Cell<usize>,
    caller: RouterCaller,
}

impl RouterProposer {
    pub(crate) fn new(
        cfg: RouterConfig,
        observables: Vec<ObservableRecord>,
        blocks: Vec<openqg_core::scoring::CovarianceBlock>,
        baseline_ll: f64,
        extra_sections: String,
    ) -> Self {
        let caller = default_caller(&cfg);
        Self {
            cfg,
            observables,
            blocks,
            baseline_ll,
            extra_sections,
            within_run_path: None,
            attempts: RefCell::new(Vec::new()),
            receipts: RefCell::new(Vec::new()),
            call_no: Cell::new(0),
            caller,
        }
    }

    /// U3: set the path to the current run's proposal-attempts.jsonl for within-run kill feedback.
    pub(crate) fn set_within_run_path(&mut self, path: PathBuf) {
        self.within_run_path = Some(path);
    }

    #[cfg(test)]
    pub(crate) fn with_caller(
        cfg: RouterConfig,
        observables: Vec<ObservableRecord>,
        baseline_ll: f64,
        extra_sections: String,
        caller: RouterCaller,
    ) -> Self {
        Self {
            cfg,
            observables,
            blocks: Vec::new(),
            baseline_ll,
            extra_sections,
            within_run_path: None,
            attempts: RefCell::new(Vec::new()),
            receipts: RefCell::new(Vec::new()),
            call_no: Cell::new(0),
            caller,
        }
    }
}

/// One sample's chain outcome.
struct SampleOutcome {
    doc: Option<ProposalDoc>,
    total: f64,
    disqualified: bool,
    records: Vec<ProposalAttemptRecord>,
    winner_record_idx: Option<usize>,
    /// One receipt per HTTP call in this sample's chain.
    receipts: Vec<LlmCallReceipt>,
}

fn truncate(raw: &str, max: usize) -> String {
    if raw.len() <= max {
        raw.to_string()
    } else {
        let head: String = raw.chars().take(max).collect();
        format!("{head}…[truncated]")
    }
}

fn record(
    source: &str,
    sample: usize,
    attempt: usize,
    repair_kind: &str,
    outcome: &str,
    error: Option<String>,
    raw: &str,
    elapsed: f64,
    model: &str,
    band: &Option<String>,
    lane: Lane,
    upstream_repairs: u64,
) -> ProposalAttemptRecord {
    ProposalAttemptRecord {
        record_kind: "proposal_attempt",
        generation: 0, // stamped by the engine on drain
        source: source.into(),
        sample_index: sample,
        attempt_index: attempt,
        repair_kind: Some(repair_kind.to_string()).filter(|k| k != "none"),
        outcome: outcome.into(),
        error,
        kill_reasons: Vec::new(),
        total: None,
        raw_sha256: openqg_core::sha256_digest(raw.as_bytes()),
        raw_len: raw.len(),
        elapsed_seconds: elapsed,
        winner: false,
        model: model.into(),
        quality_band: band.clone(),
        mechanism_lane: Some(lane.name().into()),
        upstream_repairs,
    }
}

/// The Sync subset of proposer state a sample thread needs.
struct SampleCtx<'a> {
    cfg: &'a RouterConfig,
    observables: &'a [ObservableRecord],
    blocks: &'a [openqg_core::scoring::CovarianceBlock],
    baseline_ll: f64,
    extra_sections: &'a str,
    caller: &'a RouterCaller,
}

impl SampleCtx<'_> {
    fn lane(&self, sample: usize) -> Lane {
        LANES[sample % LANES.len()]
    }

    fn band(&self, sample: usize, call_no: usize) -> Option<String> {
        let band = &self.cfg.bands[(call_no + sample) % self.cfg.bands.len()];
        (band != "any" && !band.is_empty()).then(|| band.clone())
    }

    /// One sample's full chain: call → parse sketch → expand → pre-score → ≤1 oracle repair.
    fn propose_one_sample(&self, sample: usize, call_no: usize) -> SampleOutcome {
        let lane = self.lane(sample);
        let band = self.band(sample, call_no);
        let base_prompt = build_router_prompt(lane, sample, self.extra_sections);
        let schema = proposal_sketch_schema();
        let mut records = Vec::new();
        let mut receipts: Vec<LlmCallReceipt> = Vec::new();
        let registry = TokenizerRegistry::global();
        let mut prompt = base_prompt.clone();
        let mut repair_kind = "none".to_string();
        let mut repairs_left = self.cfg.repairs;
        let mut oracle_repair_done = false;
        let mut attempt = 0usize;
        let mut best: Option<(ProposalDoc, f64, bool, usize)> = None;

        loop {
            attempt += 1;
            let req = RouterRequest {
                prompt: prompt.clone(),
                quality_band: band.clone(),
                temperature: self.cfg.temperature,
                max_completion_tokens: self.cfg.max_completion_tokens,
                schema_name: "openqg_proposal_sketch_v1",
                schema: schema.clone(),
                sample_index: sample,
                attempt_index: attempt,
            };
            let resp = match (self.caller)(&req) {
                Ok(r) => r,
                Err(e) => {
                    records.push(record(
                        "router",
                        sample,
                        attempt,
                        &repair_kind,
                        "llm_error",
                        Some(truncate(&format!("{e:#}"), 400)),
                        "",
                        0.0,
                        "",
                        &band,
                        lane,
                        0,
                    ));
                    break; // terminal for this sample; siblings cover it
                }
            };
            // Phase 38: emit token receipt for every successful HTTP call.
            {
                let usage = TokenUsage::new(resp.prompt_tokens, resp.completion_tokens);
                let flagged = resp.prompt_tokens == 0 && resp.completion_tokens == 0;
                let (cost, _) = registry.estimate_cost(&resp.model, &usage);
                receipts.push(LlmCallReceipt {
                    call_id: format!("call_no={call_no}:sample={sample}:attempt={attempt}"),
                    timestamp_utc: String::new(),
                    provider: "jnoccio".into(),
                    model: resp.model.clone(),
                    usage,
                    cost,
                    attributed_to: "proposer".into(),
                    flagged_estimate: flagged,
                });
            }
            let parsed = parse_sketch_response(&resp.content).and_then(|sk| expand_sketch(&sk));
            match parsed {
                Err(e) => {
                    records.push(record(
                        "router",
                        sample,
                        attempt,
                        &repair_kind,
                        "parse_error",
                        Some(truncate(&format!("{e:#}"), 400)),
                        &resp.content,
                        resp.elapsed_seconds,
                        &resp.model,
                        &band,
                        lane,
                        resp.upstream_repairs,
                    ));
                    if repairs_left == 0 {
                        break;
                    }
                    repairs_left -= 1;
                    repair_kind = "parse".into();
                    prompt = format!(
                        "{base_prompt}\n\n## REPAIR — your previous output failed to expand\n\
                         ERROR: {e:#}\n\nYOUR PREVIOUS OUTPUT:\n{}\n\n\
                         Return ONLY the corrected ProposalSketch JSON object.",
                        truncate(&resp.content, 8_192),
                    );
                }
                Ok(doc) => {
                    let sc = score_proposal(&doc, self.observables, self.blocks, self.baseline_ll);
                    let outcome = if sc.disqualified { "killed" } else { "ok" };
                    let mut rec = record(
                        "router",
                        sample,
                        attempt,
                        &repair_kind,
                        outcome,
                        None,
                        &resp.content,
                        resp.elapsed_seconds,
                        &resp.model,
                        &band,
                        lane,
                        resp.upstream_repairs,
                    );
                    rec.kill_reasons = sc.kill_reasons.clone();
                    rec.total = Some(sc.total);
                    records.push(rec);
                    let idx = records.len() - 1;
                    let better = match &best {
                        None => true,
                        Some((_, t, dq, _)) => {
                            (!sc.disqualified && *dq) || (sc.disqualified == *dq && sc.total > *t)
                        }
                    };
                    if better {
                        best = Some((doc, sc.total, sc.disqualified, idx));
                    }
                    if !sc.disqualified {
                        break; // good proposal — done
                    }
                    if oracle_repair_done || repairs_left == 0 {
                        break; // keep the killed doc (better than nothing)
                    }
                    repairs_left -= 1;
                    oracle_repair_done = true;
                    repair_kind = "oracle".into();
                    // V7 (review-01 risk 5): redact engine-computed values from the repair
                    // prompt — teaching the model to echo the oracle's numerical surface is
                    // not teaching physics. The ledger keeps the full kill reasons.
                    let redacted: Vec<String> = sc
                        .kill_reasons
                        .iter()
                        .map(|r| {
                            if let Some(idx) = r.find("but the model computes") {
                                format!("{}but the model computes [redacted]", &r[..idx])
                            } else {
                                r.clone()
                            }
                        })
                        .collect();
                    prompt = format!(
                        "{base_prompt}\n\n## REPAIR — the oracle KILLED your proposal\n\
                         KILL REASONS:\n{}\n\n\
                         Fix the physics (do not game the rubric) and return ONLY the corrected \
                         ProposalSketch JSON object. For a fabricated-prediction kill: compute \
                         your witness value carefully or widen min_detectable to what the cited \
                         experiment honestly resolves.",
                        redacted.join("\n"),
                    );
                }
            }
        }

        match best {
            Some((doc, total, dq, idx)) => SampleOutcome {
                doc: Some(doc),
                total,
                disqualified: dq,
                records,
                winner_record_idx: Some(idx),
                receipts,
            },
            None => SampleOutcome {
                doc: None,
                total: f64::NEG_INFINITY,
                disqualified: true,
                records,
                winner_record_idx: None,
                receipts,
            },
        }
    }
}

impl Proposer for RouterProposer {
    fn propose(&self) -> Result<ProposalDoc> {
        let call_no = self.call_no.get();
        self.call_no.set(call_no + 1);
        let k = self.cfg.samples.max(1);
        let stagger = self.cfg.stagger_seconds;
        let counter = AtomicUsize::new(0);
        // U3: append within-run kill block if the path is set and has enough data.
        let within = self
            .within_run_path
            .as_deref()
            .map(build_within_run_kill_block)
            .unwrap_or_default();
        let combined_extra: std::borrow::Cow<str> = if within.is_empty() {
            std::borrow::Cow::Borrowed(&self.extra_sections)
        } else {
            std::borrow::Cow::Owned(format!("{}{within}", self.extra_sections))
        };
        let ctx = SampleCtx {
            cfg: &self.cfg,
            observables: &self.observables,
            blocks: &self.blocks,
            baseline_ll: self.baseline_ll,
            extra_sections: combined_extra.as_ref(),
            caller: &self.caller,
        };
        let outcomes: Vec<SampleOutcome> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..k)
                .map(|i| {
                    let counter = &counter;
                    let ctx = &ctx;
                    scope.spawn(move || {
                        let order = counter.fetch_add(1, Ordering::SeqCst);
                        if stagger > 0 && order > 0 {
                            std::thread::sleep(Duration::from_secs(stagger * order as u64));
                        }
                        ctx.propose_one_sample(i, call_no)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("sample thread"))
                .collect()
        });

        // Winner: best non-DQ total beats any DQ; ties keep the earliest sample (deterministic).
        let mut winner: Option<usize> = None;
        for (i, o) in outcomes.iter().enumerate() {
            if o.doc.is_none() {
                continue;
            }
            let better = match winner {
                None => true,
                Some(w) => {
                    let cur = &outcomes[w];
                    (!o.disqualified && cur.disqualified)
                        || (o.disqualified == cur.disqualified && o.total > cur.total)
                }
            };
            if better {
                winner = Some(i);
            }
        }

        let mut attempts = self.attempts.borrow_mut();
        let mut all_receipts = self.receipts.borrow_mut();
        let mut result: Option<ProposalDoc> = None;
        for (i, mut o) in outcomes.into_iter().enumerate() {
            let is_winner = winner == Some(i);
            if is_winner {
                if let Some(idx) = o.winner_record_idx {
                    o.records[idx].winner = true;
                }
                result = o.doc.take();
            }
            attempts.extend(o.records);
            all_receipts.extend(o.receipts);
        }
        drop(all_receipts);
        drop(attempts);
        result.with_context(|| {
            format!("router: no sample produced a parseable proposal (K={k}, call {call_no})")
        })
    }

    fn drain_attempts(&self) -> Vec<ProposalAttemptRecord> {
        std::mem::take(&mut self.attempts.borrow_mut())
    }

    fn drain_token_receipts(&self) -> Vec<LlmCallReceipt> {
        std::mem::take(&mut self.receipts.borrow_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zyal_genome::physics_score::baseline_log_likelihood;
    use crate::zyal_genome::proposer_sketch::fixture_sketch;

    fn obs() -> Vec<ObservableRecord> {
        ["bao_dv_z038", "bao_dv_z051", "fsigma8_z038", "fsigma8_z051"]
            .iter()
            .enumerate()
            .map(|(i, id)| ObservableRecord {
                observable_id: id.to_string(),
                kind: "bao".into(),
                value: 10.0 + i as f64,
                uncertainty: 0.5,
                unit: "dimensionless".into(),
                source: None,
            })
            .collect()
    }

    fn ok_response(content: String) -> RouterResponse {
        RouterResponse {
            content,
            model: "model-not-in-registry".into(),
            structured_status: Some("valid".into()),
            upstream_repairs: 0,
            elapsed_seconds: 0.01,
            http_status: 200,
            prompt_tokens: 0,
            completion_tokens: 0,
        }
    }

    fn proposer_with(caller: RouterCaller, samples: usize, repairs: usize) -> RouterProposer {
        let observables = obs();
        let bll = baseline_log_likelihood(&observables);
        RouterProposer::with_caller(
            RouterConfig {
                samples,
                repairs,
                stagger_seconds: 0,
                ..Default::default()
            },
            observables,
            bll,
            String::new(),
            caller,
        )
    }

    #[test]
    fn closure_caller_round_trips_the_fixture_sketch() {
        let body = serde_json::to_string(&fixture_sketch()).unwrap();
        let p = proposer_with(Box::new(move |_req| Ok(ok_response(body.clone()))), 2, 1);
        let doc = p.propose().expect("proposes");
        assert_eq!(doc.theory.id, fixture_sketch().theory_id);
        let recs = p.drain_attempts();
        assert!(recs.iter().filter(|r| r.winner).count() == 1);
        assert!(recs.iter().all(|r| r.model == "model-not-in-registry"));
        assert!(recs.iter().all(|r| r.mechanism_lane.is_some()));
    }

    #[test]
    fn parse_repair_recovers_once() {
        let body = serde_json::to_string(&fixture_sketch()).unwrap();
        let calls = AtomicUsize::new(0);
        let p = proposer_with(
            Box::new(move |_req| {
                let i = calls.fetch_add(1, Ordering::SeqCst);
                if i == 0 {
                    Ok(ok_response("this is not json at all, sorry".into()))
                } else {
                    Ok(ok_response(body.clone()))
                }
            }),
            1,
            2,
        );
        let doc = p.propose().expect("recovers via parse repair");
        assert!(!doc.theory.id.is_empty());
        let recs = p.drain_attempts();
        let outcomes: Vec<&str> = recs.iter().map(|r| r.outcome.as_str()).collect();
        assert_eq!(outcomes, vec!["parse_error", "ok"]);
        assert_eq!(recs[1].repair_kind.as_deref(), Some("parse"));
    }

    #[test]
    fn oracle_repair_carries_kill_reasons_and_killed_doc_is_returned_when_exhausted() {
        // A sketch that parses but gets killed (uncertified mu0 => UnexplainedModification).
        let mut bad = fixture_sketch();
        bad.theory_id = "killed-proposal".into();
        bad.background.mu0 = -0.3;
        bad.parameters.clear();
        let body = serde_json::to_string(&bad).unwrap();
        let saw_oracle_repair = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let saw = saw_oracle_repair.clone();
        let p = proposer_with(
            Box::new(move |req| {
                if req.prompt.contains("the oracle KILLED") {
                    saw.store(true, Ordering::SeqCst);
                    assert!(req.prompt.contains("UnexplainedModification"));
                }
                Ok(ok_response(body.clone()))
            }),
            1,
            2,
        );
        let doc = p.propose().expect("killed doc is still returned");
        assert_eq!(doc.theory.id, "killed-proposal");
        assert!(
            saw_oracle_repair.load(Ordering::SeqCst),
            "oracle repair fired"
        );
        let recs = p.drain_attempts();
        assert!(recs.iter().all(|r| r.outcome == "killed"));
        assert!(recs.iter().any(|r| !r.kill_reasons.is_empty()));
    }

    #[test]
    fn parallel_k_picks_the_best_and_transport_failure_is_terminal_per_sample() {
        let good = serde_json::to_string(&fixture_sketch()).unwrap();
        let p = proposer_with(
            Box::new(move |req| {
                if req.sample_index == 0 {
                    anyhow::bail!("connection refused")
                }
                Ok(ok_response(good.clone()))
            }),
            3,
            0,
        );
        let doc = p.propose().expect("siblings cover a dead sample");
        assert!(!doc.theory.id.is_empty());
        let recs = p.drain_attempts();
        assert!(recs.iter().any(|r| r.outcome == "llm_error"));
        assert_eq!(recs.iter().filter(|r| r.winner).count(), 1);
    }

    #[test]
    fn rotation_is_deterministic_lanes_stable_bands_shift() {
        let cfg = RouterConfig::default();
        let observables = obs();
        let caller: RouterCaller = Box::new(|_req| anyhow::bail!("net off"));
        let ctx = SampleCtx {
            cfg: &cfg,
            observables: &observables,
            blocks: &[],
            baseline_ll: 0.0,
            extra_sections: "",
            caller: &caller,
        };
        assert_eq!(ctx.lane(0).name(), "planck_mu0");
        assert_eq!(ctx.lane(1).name(), "dark_scattering");
        assert_eq!(ctx.lane(2).name(), "free");
        assert_eq!(ctx.lane(3).name(), "null_diagnostic");
        assert_eq!(ctx.lane(4).name(), "planck_mu0");
        // bands shift with the call counter
        let b00 = ctx.band(0, 0);
        let b01 = ctx.band(0, 1);
        assert_ne!(b00, b01);
        assert_eq!(ctx.band(1, 0), b01); // (call+sample) rotation
    }

    #[test]
    fn should_retry_policy_table() {
        assert!(should_retry(Some(429), 1).is_some());
        assert!(should_retry(Some(503), 1).is_some());
        assert!(should_retry(Some(500), 1).is_some());
        assert!(should_retry(None, 1).is_some());
        assert!(should_retry(Some(404), 1).is_none());
        assert!(should_retry(Some(400), 1).is_none());
        assert!(should_retry(Some(503), 2).is_none()); // one retry only
    }

    #[test]
    fn within_run_kill_block_empty_below_threshold() {
        // Fewer than 5 disqualified records → empty block (noise floor).
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("proposal-attempts.jsonl");
        let records = (0..4)
            .map(|_| {
                serde_json::json!({
                    "disqualified": true,
                    "kill_reasons": ["FreeParameter: extra dial"],
                    "outcome": "disqualified"
                })
                .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, records).unwrap();
        assert!(build_within_run_kill_block(&path).is_empty());
    }

    #[test]
    fn within_run_kill_block_emitted_above_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("proposal-attempts.jsonl");
        let mut lines = Vec::new();
        for _ in 0..8 {
            lines.push(
                serde_json::json!({
                    "disqualified": true,
                    "kill_reasons": ["UnverifiedDerivation: step 2"],
                    "outcome": "disqualified"
                })
                .to_string(),
            );
        }
        // One non-disqualified record — should not inflate disq count.
        lines.push(
            serde_json::json!({
                "disqualified": false,
                "outcome": "winner"
            })
            .to_string(),
        );
        std::fs::write(&path, lines.join("\n")).unwrap();
        let block = build_within_run_kill_block(&path);
        assert!(!block.is_empty(), "block should be emitted for 8 disqualified");
        assert!(
            block.contains("unverified_derivation"),
            "block should name the dominant kill class"
        );
        assert!(
            block.contains("WITHIN-RUN KILLS"),
            "block should have the section header"
        );
    }

    #[test]
    fn within_run_kill_block_missing_file_returns_empty() {
        let path = std::path::Path::new("/nonexistent/path/proposal-attempts.jsonl");
        assert!(build_within_run_kill_block(path).is_empty());
    }
}
