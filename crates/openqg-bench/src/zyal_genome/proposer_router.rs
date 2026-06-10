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
    /// `extra["jnoccio"]["winner_model_id"]`, falling back to the response `model`.
    pub model: String,
    /// Router-side structured-output verdict ("valid" when schema-checked).
    pub structured_status: Option<String>,
    /// Router-internal repair calls spent making the output schema-valid.
    pub upstream_repairs: u64,
    pub elapsed_seconds: f64,
    pub http_status: u16,
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
                    let meta = &v["extra"]["jnoccio"];
                    let model = meta["winner_model_id"]
                        .as_str()
                        .or_else(|| v["model"].as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    return Ok(RouterResponse {
                        content,
                        model,
                        structured_status: meta["structured_schema_status"]
                            .as_str()
                            .map(str::to_string),
                        upstream_repairs: meta["structured_repair_attempts"].as_u64().unwrap_or(0),
                        elapsed_seconds: start.elapsed().as_secs_f64(),
                        http_status: code,
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

/// The V6 free-compute proposer: best-of-K parallel samples per slot, each on its own mechanism
/// lane, with router-side schema validation, client parse/oracle repair, and total observability.
pub(crate) struct RouterProposer {
    cfg: RouterConfig,
    observables: Vec<ObservableRecord>,
    baseline_ll: f64,
    extra_sections: String,
    attempts: RefCell<Vec<ProposalAttemptRecord>>,
    /// Deterministic rotation seed: bands shift by one per propose() call.
    call_no: Cell<usize>,
    caller: RouterCaller,
}

impl RouterProposer {
    pub(crate) fn new(
        cfg: RouterConfig,
        observables: Vec<ObservableRecord>,
        baseline_ll: f64,
        extra_sections: String,
    ) -> Self {
        let caller = default_caller(&cfg);
        Self {
            cfg,
            observables,
            baseline_ll,
            extra_sections,
            attempts: RefCell::new(Vec::new()),
            call_no: Cell::new(0),
            caller,
        }
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
            baseline_ll,
            extra_sections,
            attempts: RefCell::new(Vec::new()),
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
                    let sc = score_proposal(&doc, self.observables, &[], self.baseline_ll);
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
                    prompt = format!(
                        "{base_prompt}\n\n## REPAIR — the oracle KILLED your proposal\n\
                         KILL REASONS:\n{}\n\n\
                         Fix the physics (do not game the rubric) and return ONLY the corrected \
                         ProposalSketch JSON object. NOTE: a `fabricated novel prediction` kill \
                         message CONTAINS the engine-computed value — set your witness's \
                         `predicted` to exactly that computed number (the engine computes the \
                         physics; your declaration is an honesty attestation).",
                        sc.kill_reasons.join("\n"),
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
            },
            None => SampleOutcome {
                doc: None,
                total: f64::NEG_INFINITY,
                disqualified: true,
                records,
                winner_record_idx: None,
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
        let ctx = SampleCtx {
            cfg: &self.cfg,
            observables: &self.observables,
            baseline_ll: self.baseline_ll,
            extra_sections: &self.extra_sections,
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
        }
        drop(attempts);
        result.with_context(|| {
            format!("router: no sample produced a parseable proposal (K={k}, call {call_no})")
        })
    }

    fn drain_attempts(&self) -> Vec<ProposalAttemptRecord> {
        std::mem::take(&mut self.attempts.borrow_mut())
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
            model: "test-model-a".into(),
            structured_status: Some("valid".into()),
            upstream_repairs: 0,
            elapsed_seconds: 0.01,
            http_status: 200,
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
        assert!(recs.iter().all(|r| r.model == "test-model-a"));
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
}
