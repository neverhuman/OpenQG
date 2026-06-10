//! V5: the **jekko proposer** — high-volume theory proposals on the free jnoccio API.
//!
//! The V4.1 live campaign got 2 parseable proposals from ~50 slow browser-ChatGPT calls, with every
//! failure silently destroyed. This module rebuilds the live loop around `rtk jekko run` (free
//! tokens, ~90–180 s/call, top-20% model routing) with the three mechanisms that convert volume
//! into yield:
//!
//! 1. **Repair loops** — a parse failure is re-prompted with the exact error and the previous raw
//!    output; an oracle kill gets one repair pass carrying the `kill_reasons`. The oracle's
//!    machine-readable verdicts are the teacher.
//! 2. **Best-of-K parallel sampling** — K concurrent jekko subprocesses per slot (the transport is
//!    stateless: prompt via stdin, no temp files, per-child `setsid`); the oracle pre-scores every
//!    sample with the *same* `score_proposal` the engine uses and the best survives.
//! 3. **Total observability** — every call, repair, and failure becomes a
//!    [`ProposalAttemptRecord`] drained by the engine into `proposal-attempts.jsonl`.
//!
//! The transport is injected as a [`LiveCaller`] closure, so the whole machine is unit-tested with
//! no network (`command = ["echo", <json>]` or a counting closure).

use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{anyhow, Result};

use openqg_core::ObservableRecord;

use super::jailgun_live::run_live_call_attempt_env;
use super::proposer::{score_proposal, ProposalDoc, Proposer};
use super::proposer_jailgun::parse_proposal_response;
use super::theory_population::ProposalAttemptRecord;
use super::LiveAttempt;

/// Injected transport: `(prompt, attempt_number) -> LiveAttempt`. Production wraps
/// [`run_live_call_attempt_env`]; tests inject closures.
pub(crate) type LiveCaller = Box<dyn Fn(&str, usize) -> Result<LiveAttempt> + Send + Sync>;

#[derive(Clone)]
pub(crate) struct JekkoConfig {
    pub command: Vec<String>,
    pub timeout_seconds: u64,
    /// `Some("top20")` routes to the top-20% jnoccio models via `JEKKO_RUN_QUALITY_BAND`.
    pub quality_band: Option<String>,
    /// Best-of-K parallel samples per slot.
    pub samples: usize,
    /// Repair budget per sample (parse repairs + at most one oracle repair share it).
    pub repairs: usize,
    /// Stagger between thread starts (rate-limit kindness).
    pub stagger_seconds: u64,
}

impl Default for JekkoConfig {
    fn default() -> Self {
        Self {
            command: super::JEKKO_LIVE_COMMAND
                .iter()
                .map(|s| s.to_string())
                .collect(),
            timeout_seconds: 300,
            quality_band: Some("top20".to_string()),
            samples: 4,
            repairs: 2,
            stagger_seconds: 3,
        }
    }
}

/// The free-token proposer. Constructed with the scoring context (observables + baseline) so it can
/// pre-score and oracle-repair internally with the exact function the engine judges by.
pub(crate) struct JekkoProposer {
    cfg: JekkoConfig,
    observables: Vec<ObservableRecord>,
    baseline_ll: f64,
    /// DATA BRIEF + MEMORY sections appended to the base prompt (computed once at construction).
    extra_sections: String,
    attempts: RefCell<Vec<ProposalAttemptRecord>>,
    caller: LiveCaller,
}

impl JekkoProposer {
    pub(crate) fn new(
        cfg: JekkoConfig,
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
            caller,
        }
    }

    /// Test constructor: inject the transport.
    #[cfg(test)]
    pub(crate) fn with_caller(
        cfg: JekkoConfig,
        observables: Vec<ObservableRecord>,
        baseline_ll: f64,
        extra_sections: String,
        caller: LiveCaller,
    ) -> Self {
        Self {
            cfg,
            observables,
            baseline_ll,
            extra_sections,
            attempts: RefCell::new(Vec::new()),
            caller,
        }
    }
}

fn default_caller(cfg: &JekkoConfig) -> LiveCaller {
    let command = cfg.command.clone();
    let timeout = cfg.timeout_seconds;
    let extra_env: Vec<(String, String)> = cfg
        .quality_band
        .as_ref()
        .map(|b| vec![("JEKKO_RUN_QUALITY_BAND".to_string(), b.clone())])
        .unwrap_or_default();
    Box::new(move |prompt, attempt| {
        run_live_call_attempt_env(&command, prompt, timeout, attempt, "v5-jekko", &extra_env)
    })
}

/// Tolerant extraction for `--print-logs` stdout: scan for balanced top-level `{…}` spans
/// (string-aware) and try parsing each from LAST to FIRST — the proposal is emitted at the end.
pub(crate) fn extract_proposal_from_noisy_stdout(stdout: &str) -> Result<ProposalDoc> {
    // Fast path: the strict parser already handles fences + outermost-braces slicing.
    let strict = parse_proposal_response(stdout);
    if strict.is_ok() {
        return strict;
    }
    let bytes = stdout.as_bytes();
    let mut spans: Vec<(usize, usize)> = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            b'}' => {
                if depth > 0 {
                    depth -= 1;
                    if depth == 0 {
                        spans.push((start, i + 1));
                    }
                }
            }
            _ => {}
        }
    }
    let mut last_err = None;
    for (a, b) in spans.iter().rev() {
        match parse_proposal_response(&stdout[*a..*b]) {
            Ok(doc) => return Ok(doc),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| {
        strict
            .err()
            .unwrap_or_else(|| anyhow!("no JSON object found"))
    }))
}

/// One sample's chain outcome.
struct SampleOutcome {
    doc: Option<ProposalDoc>,
    total: f64,
    disqualified: bool,
    records: Vec<ProposalAttemptRecord>,
    winner_record_idx: Option<usize>,
}

fn truncate_raw(raw: &str, head: usize, tail: usize) -> String {
    if raw.len() <= head + tail {
        return raw.to_string();
    }
    let head_end = raw.char_indices().take_while(|(i, _)| *i < head).count();
    let head_str: String = raw.chars().take(head_end).collect();
    let tail_str: String = raw
        .chars()
        .rev()
        .take(tail)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head_str}\n…[truncated]…\n{tail_str}")
}

fn parse_repair_prompt(base: &str, raw: &str, error: &str) -> String {
    format!(
        "{base}\n\n## REPAIR — your previous output failed to parse\n\nYOUR PREVIOUS OUTPUT:\n\
         {}\n\nPARSE ERROR:\n{error}\n\nReturn ONLY the corrected JSON ProposalDoc — one JSON \
         object, no prose, no markdown fences.\n",
        truncate_raw(raw, 12_000, 4_000)
    )
}

fn oracle_repair_prompt(base: &str, doc_json: &str, kill_reasons: &[String]) -> String {
    let reasons = kill_reasons
        .iter()
        .map(|r| format!("- {r}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{base}\n\n## REPAIR — the oracle DISQUALIFIED your previous proposal\n\nYOUR PREVIOUS \
         PROPOSAL:\n{}\n\nORACLE KILL REASONS:\n{reasons}\n\nFix ONLY these violations; keep \
         everything else. Return ONLY the corrected JSON ProposalDoc.\n",
        truncate_raw(doc_json, 12_000, 4_000)
    )
}

#[allow(clippy::too_many_arguments)]
fn propose_one_sample(
    caller: &LiveCaller,
    base_prompt: &str,
    sample_index: usize,
    repairs: usize,
    observables: &[ObservableRecord],
    baseline_ll: f64,
) -> SampleOutcome {
    let mut records: Vec<ProposalAttemptRecord> = Vec::new();
    let mut prompt = base_prompt.to_string();
    let mut repair_kind: Option<String> = None;
    let mut oracle_repaired = false;
    let mut empty_retried = false;
    let empty_backoff_seconds: u64 = 8;
    let mut best: Option<(ProposalDoc, f64, bool, usize)> = None; // doc, total, dq, record idx

    let mut attempt_index = 0usize;
    loop {
        let result = caller(&prompt, attempt_index + 1);
        let mut rec = ProposalAttemptRecord {
            record_kind: "proposal_attempt",
            generation: 0, // stamped by the engine
            source: "jekko".into(),
            sample_index,
            attempt_index,
            repair_kind: repair_kind.clone(),
            outcome: String::new(),
            error: None,
            kill_reasons: Vec::new(),
            total: None,
            raw_sha256: String::new(),
            raw_len: 0,
            elapsed_seconds: 0.0,
            winner: false,
        };
        let attempt = match result {
            Ok(a) => a,
            Err(e) => {
                rec.outcome = "llm_error".into();
                rec.error = Some(format!("{e:#}"));
                records.push(rec);
                break; // transport failure is terminal for this sample — siblings cover it
            }
        };
        rec.elapsed_seconds = attempt.elapsed_seconds();
        let raw = attempt.stdout().to_string();
        rec.raw_sha256 = openqg_core::sha256_digest(raw.as_bytes());
        rec.raw_len = raw.len();
        if attempt.status() != "ok" && attempt.status() != "success" {
            rec.outcome = "llm_error".into();
            rec.error = Some(format!(
                "status {}: {} | stderr: {}",
                attempt.status(),
                attempt.error().unwrap_or("no detail"),
                truncate_raw(attempt.stderr_excerpt(), 240, 0)
            ));
            records.push(rec);
            break;
        }
        // Near-empty stdout is a TRANSPORT failure (contention / silent provider error), not a
        // model-JSON failure: retry once with a short backoff, without burning the parse-repair
        // budget on an empty output.
        if raw.trim().len() < 50 {
            rec.outcome = "empty_output".into();
            rec.error = Some(format!(
                "stdout {} bytes | stderr: {}",
                raw.len(),
                truncate_raw(attempt.stderr_excerpt(), 240, 0)
            ));
            records.push(rec);
            if !empty_retried {
                empty_retried = true;
                std::thread::sleep(Duration::from_secs(empty_backoff_seconds));
                continue; // same prompt, same attempt_index semantics (free transport retry)
            }
            break;
        }
        match extract_proposal_from_noisy_stdout(&raw) {
            Err(e) => {
                rec.outcome = "parse_error".into();
                rec.error = Some(format!("{e:#}"));
                records.push(rec);
                if attempt_index < repairs {
                    prompt = parse_repair_prompt(base_prompt, &raw, &format!("{e:#}"));
                    repair_kind = Some("parse".into());
                    attempt_index += 1;
                    continue;
                }
                break;
            }
            Ok(doc) => {
                let sc = score_proposal(&doc, observables, baseline_ll);
                rec.total = Some(sc.total);
                if sc.disqualified {
                    rec.outcome = "killed".into();
                    rec.kill_reasons = sc.kill_reasons.clone();
                    records.push(rec);
                    let idx = records.len() - 1;
                    let keep = best
                        .as_ref()
                        .map(|(_, t, dq, _)| *dq && sc.total >= *t)
                        .unwrap_or(true);
                    if keep {
                        best = Some((doc.clone(), sc.total, true, idx));
                    }
                    if !oracle_repaired && attempt_index < repairs {
                        oracle_repaired = true;
                        let doc_json = serde_json::to_string(&doc).unwrap_or_default();
                        prompt = oracle_repair_prompt(base_prompt, &doc_json, &sc.kill_reasons);
                        repair_kind = Some("oracle".into());
                        attempt_index += 1;
                        continue;
                    }
                    break;
                } else {
                    rec.outcome = "ok".into();
                    records.push(rec);
                    let idx = records.len() - 1;
                    best = Some((doc, sc.total, false, idx));
                    break;
                }
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

impl Proposer for JekkoProposer {
    fn propose(&self) -> Result<ProposalDoc> {
        let base_prompt = format!(
            "{}\n{}",
            super::proposer_jailgun::build_proposer_prompt(),
            self.extra_sections
        );
        let k = self.cfg.samples.max(1);
        let stagger = Duration::from_secs(self.cfg.stagger_seconds);
        let repairs = self.cfg.repairs;
        let observables = &self.observables;
        let baseline_ll = self.baseline_ll;
        let caller = &self.caller;
        let prompt_ref = &base_prompt;

        let mut outcomes: Vec<SampleOutcome> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..k)
                .map(|i| {
                    s.spawn(move || {
                        if i > 0 {
                            std::thread::sleep(stagger * i as u32);
                        }
                        propose_one_sample(caller, prompt_ref, i, repairs, observables, baseline_ll)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| {
                    h.join().unwrap_or_else(|_| SampleOutcome {
                        doc: None,
                        total: f64::NEG_INFINITY,
                        disqualified: true,
                        records: Vec::new(),
                        winner_record_idx: None,
                    })
                })
                .collect()
        });

        // Pick the winner: best non-disqualified total, else best disqualified (the engine ledgers
        // the kill), else a summarized error.
        let winner_idx = {
            let mut best: Option<(usize, f64, bool)> = None;
            for (i, o) in outcomes.iter().enumerate() {
                if o.doc.is_none() {
                    continue;
                }
                let better = match best {
                    None => true,
                    Some((_, t, dq)) => match (o.disqualified, dq) {
                        (false, true) => true,
                        (true, false) => false,
                        _ => o.total > t,
                    },
                };
                if better {
                    best = Some((i, o.total, o.disqualified));
                }
            }
            best.map(|(i, _, _)| i)
        };

        if let Some(wi) = winner_idx {
            if let Some(ri) = outcomes[wi].winner_record_idx {
                outcomes[wi].records[ri].winner = true;
            }
        }
        let mut all_records = Vec::new();
        for o in &outcomes {
            all_records.extend(o.records.iter().cloned());
        }
        let failure_summary: Vec<String> = outcomes
            .iter()
            .flat_map(|o| o.records.iter())
            .map(|r| format!("s{}a{} {}", r.sample_index, r.attempt_index, r.outcome))
            .collect();
        self.attempts.borrow_mut().extend(all_records);

        match winner_idx {
            Some(wi) => Ok(outcomes
                .swap_remove(wi)
                .doc
                .expect("winner has a doc by construction")),
            None => Err(anyhow!(
                "jekko: no sample produced a parseable proposal ({} attempts: {})",
                failure_summary.len(),
                failure_summary.join(", ")
            )),
        }
    }

    fn drain_attempts(&self) -> Vec<ProposalAttemptRecord> {
        std::mem::take(&mut self.attempts.borrow_mut())
    }
}

/// Preflight: one tiny jekko call to fail fast (clear message) before a multi-hour campaign.
pub(crate) fn jekko_smoke(timeout_seconds: u64) -> Result<()> {
    let cfg = JekkoConfig {
        timeout_seconds,
        samples: 1,
        repairs: 0,
        quality_band: None,
        ..Default::default()
    };
    let caller = default_caller(&cfg);
    let attempt = caller("Reply with exactly: PONG", 1)?;
    if attempt.status() != "ok" && attempt.status() != "success" {
        anyhow::bail!(
            "jekko smoke failed (status {}): {}",
            attempt.status(),
            attempt.error().unwrap_or("no detail")
        );
    }
    if !attempt.stdout().to_uppercase().contains("PONG") {
        anyhow::bail!(
            "jekko smoke got an unexpected reply ({} bytes)",
            attempt.stdout().len()
        );
    }
    Ok(())
}

#[allow(dead_code)]
fn _suppress_unused(_: &AtomicUsize, _: Ordering) {}

#[cfg(test)]
mod tests {
    use super::super::proposer::fixture_proposal;
    use super::*;
    use std::sync::Mutex;

    fn obs() -> Vec<ObservableRecord> {
        ["fsigma8@0.38", "fsigma8@0.51", "s8"]
            .iter()
            .enumerate()
            .map(|(i, id)| ObservableRecord {
                observable_id: (*id).into(),
                kind: "growth".into(),
                value: 0.45 + i as f64 * 0.01,
                uncertainty: 0.04,
                unit: "dimensionless".into(),
                source: None,
            })
            .collect()
    }

    fn fake_attempt(stdout: &str) -> LiveAttempt {
        LiveAttempt {
            status: "ok".into(),
            exit_code: Some(0),
            elapsed_seconds: 0.1,
            stdout: stdout.to_string(),
            stderr: String::new(),
            error: None,
            metadata: serde_json::Value::Null,
        }
    }

    fn cfg(samples: usize, repairs: usize) -> JekkoConfig {
        JekkoConfig {
            samples,
            repairs,
            stagger_seconds: 0,
            ..Default::default()
        }
    }

    #[test]
    fn echo_command_round_trips_a_fixture_proposal() {
        // Real subprocess transport, no network: `echo <json>` IS the model.
        let json = serde_json::to_string(&fixture_proposal()).unwrap();
        let mut config = cfg(1, 0);
        config.command = vec!["echo".to_string(), json];
        config.quality_band = None;
        let observables = obs();
        let baseline = super::super::physics_score::baseline_log_likelihood(&observables);
        let p = JekkoProposer::new(config, observables, baseline, String::new());
        let doc = p.propose().expect("echo transport should parse");
        assert_eq!(doc.theory.id, fixture_proposal().theory.id);
        let attempts = p.drain_attempts();
        assert_eq!(attempts.len(), 1);
        assert_eq!(attempts[0].outcome, "ok");
        assert!(attempts[0].winner);
        assert!(p.drain_attempts().is_empty(), "drain empties the buffer");
    }

    #[test]
    fn repair_loop_recovers_after_one_parse_failure() {
        let json = serde_json::to_string(&fixture_proposal()).unwrap();
        let calls = Mutex::new(0usize);
        let caller: LiveCaller = Box::new(move |_prompt, _attempt| {
            let mut n = calls.lock().unwrap();
            *n += 1;
            Ok(fake_attempt(if *n == 1 {
                "the model wrote a long apologetic paragraph instead of JSON { sorry, here is \
                 my reasoning about modified gravity but no valid object follows the brace"
            } else {
                &json
            }))
        });
        let observables = obs();
        let baseline = super::super::physics_score::baseline_log_likelihood(&observables);
        let p = JekkoProposer::with_caller(cfg(1, 2), observables, baseline, String::new(), caller);
        let doc = p.propose().expect("repair should recover");
        assert_eq!(doc.theory.id, fixture_proposal().theory.id);
        let attempts = p.drain_attempts();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].outcome, "parse_error");
        assert_eq!(attempts[1].outcome, "ok");
        assert_eq!(attempts[1].repair_kind.as_deref(), Some("parse"));
        assert!(attempts[1].winner);
    }

    #[test]
    fn oracle_repair_prompt_carries_kill_reasons() {
        // First call: a laundering doc (cites evidence it doesn't supply) — oracle kills it.
        let mut laundering = fixture_proposal();
        laundering.evidence.clear();
        let bad_json = serde_json::to_string(&laundering).unwrap();
        let good_json = serde_json::to_string(&fixture_proposal()).unwrap();
        let prompts = Mutex::new(Vec::<String>::new());
        let calls = Mutex::new(0usize);
        let caller: LiveCaller = Box::new(move |prompt, _attempt| {
            prompts.lock().unwrap().push(prompt.to_string());
            let mut n = calls.lock().unwrap();
            *n += 1;
            // We can't reach `prompts` after the move, so assert via the repair flow records.
            Ok(fake_attempt(if *n == 1 { &bad_json } else { &good_json }))
        });
        let observables = obs();
        let baseline = super::super::physics_score::baseline_log_likelihood(&observables);
        let p = JekkoProposer::with_caller(cfg(1, 2), observables, baseline, String::new(), caller);
        let doc = p.propose().expect("oracle repair should recover");
        assert_eq!(doc.theory.id, fixture_proposal().theory.id);
        let attempts = p.drain_attempts();
        assert_eq!(attempts.len(), 2);
        assert_eq!(attempts[0].outcome, "killed");
        assert!(
            attempts[0]
                .kill_reasons
                .iter()
                .any(|r| r.contains("evidence")),
            "{:?}",
            attempts[0].kill_reasons
        );
        assert_eq!(attempts[1].repair_kind.as_deref(), Some("oracle"));
        assert_eq!(attempts[1].outcome, "ok");
    }

    #[test]
    fn parallel_k_picks_the_best_and_marks_one_winner() {
        // Sample 0 gets a killed doc, samples 1..3 get the good fixture; winner is non-DQ.
        let mut laundering = fixture_proposal();
        laundering.evidence.clear();
        let bad_json = serde_json::to_string(&laundering).unwrap();
        let good_json = serde_json::to_string(&fixture_proposal()).unwrap();
        let counter = std::sync::atomic::AtomicUsize::new(0);
        let caller: LiveCaller = Box::new(move |_prompt, _attempt| {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            Ok(fake_attempt(if n == 0 { &bad_json } else { &good_json }))
        });
        let observables = obs();
        let baseline = super::super::physics_score::baseline_log_likelihood(&observables);
        let p = JekkoProposer::with_caller(cfg(3, 0), observables, baseline, String::new(), caller);
        let doc = p.propose().expect("parallel propose");
        assert_eq!(doc.theory.id, fixture_proposal().theory.id);
        let attempts = p.drain_attempts();
        assert!(attempts.len() >= 3);
        assert_eq!(attempts.iter().filter(|a| a.winner).count(), 1);
        let w = attempts.iter().find(|a| a.winner).unwrap();
        assert_eq!(w.outcome, "ok");
    }

    #[test]
    fn exhausted_repairs_returns_the_killed_doc_not_err() {
        let mut laundering = fixture_proposal();
        laundering.evidence.clear();
        let bad_json = serde_json::to_string(&laundering).unwrap();
        let caller: LiveCaller = Box::new(move |_prompt, _attempt| Ok(fake_attempt(&bad_json)));
        let observables = obs();
        let baseline = super::super::physics_score::baseline_log_likelihood(&observables);
        let p = JekkoProposer::with_caller(cfg(1, 1), observables, baseline, String::new(), caller);
        let doc = p
            .propose()
            .expect("a parsed-but-killed doc is returned (engine ledgers it)");
        assert_eq!(doc.theory.id, fixture_proposal().theory.id);
        let attempts = p.drain_attempts();
        assert_eq!(attempts.len(), 2, "initial + one oracle repair");
        assert!(attempts.iter().all(|a| a.outcome == "killed"));
    }

    #[test]
    fn noisy_print_logs_stdout_still_parses() {
        let json = serde_json::to_string(&fixture_proposal()).unwrap();
        let noisy = format!(
            "[jekko] starting run {{model: fusion}}\nlog line with braces {{x}}\n{json}\n[jekko] done\n"
        );
        let doc = extract_proposal_from_noisy_stdout(&noisy).expect("noisy parse");
        assert_eq!(doc.theory.id, fixture_proposal().theory.id);
        assert!(extract_proposal_from_noisy_stdout("no json at all").is_err());
    }

    #[test]
    fn transport_failure_yields_err_with_attempt_records() {
        let caller: LiveCaller = Box::new(|_prompt, _attempt| Err(anyhow!("connection refused")));
        let observables = obs();
        let baseline = super::super::physics_score::baseline_log_likelihood(&observables);
        let p = JekkoProposer::with_caller(cfg(2, 2), observables, baseline, String::new(), caller);
        let err = p.propose().expect_err("all transports failed");
        assert!(err.to_string().contains("no sample produced"));
        let attempts = p.drain_attempts();
        assert_eq!(attempts.len(), 2, "one llm_error per sample, no retries");
        assert!(attempts.iter().all(|a| a.outcome == "llm_error"));
    }
}
