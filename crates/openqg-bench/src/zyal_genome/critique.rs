use super::*;

pub(crate) fn live_critic_enabled() -> bool {
    std::env::var("ZYAL_LIVE_CRITIC")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub(crate) fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n >= 1)
        .unwrap_or(default)
}

pub(crate) fn build_critique_prompt(candidate: &Value) -> String {
    let pillars = unwrap_or_value(
        candidate
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
            }),
        "(no structured theory available)".to_string(),
    );
    format!(
        "You are an adversarial physics referee judging a candidate cosmological theory. It must be \
WHITEBOX: meaningful parameters (named constants/derived quantities), real mechanisms, NOT \
parameter-fitting. Score its FALSIFIABILITY and physical PLAUSIBILITY in 0..1 and name its single \
most fatal flaw if any.\n\nTHEORY PILLARS:\n{pillars}\n\nRespond with EXACTLY one strict JSON object \
and nothing else: {{\"falsifiability\":<0..1>,\"plausibility\":<0..1>,\"fatal_flaw\":\"<short, empty if none>\"}}"
    )
}

pub(crate) fn parse_live_verdict(stdout: &str) -> Option<(f64, f64, String)> {
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

pub(crate) fn run_live_critique(candidate: &Value, timeout_seconds: u64) -> LiveVerdict {
    // The critic is a "hard" judgement → request the top-20% jnoccio models (M6).
    run_live_critique_banded(candidate, timeout_seconds, RouteTier::Top20Pct)
}

/// As [`run_live_critique`], but explicitly requests a routing tier — `Top20Pct` forwards
/// `JEKKO_RUN_QUALITY_BAND=top20` to jnoccio-fusion so the call routes to the top-20%-performing
/// models, taming both stochasticity and weak-model noise.
pub(crate) fn run_live_critique_banded(
    candidate: &Value,
    timeout_seconds: u64,
    tier: RouteTier,
) -> LiveVerdict {
    let prompt = build_critique_prompt(candidate);
    let command: Vec<String> = JNOCCIO_CRITIQUE_COMMAND
        .iter()
        .map(|s| s.to_string())
        .collect();
    let extra_env: Vec<(String, String)> = tier
        .quality_band()
        .map(|band| vec![("JEKKO_RUN_QUALITY_BAND".to_string(), band.to_string())])
        .unwrap_or_default();
    match run_live_call_attempt_env(&command, &prompt, timeout_seconds, 1, &now_iso8601(), &extra_env)
    {
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
                status: if attempt.status == "ok" {
                    "unparsed".to_string()
                } else {
                    attempt.status
                },
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

/// The aggregate of several critic verdicts — the multi-sample, self-consistency score that tames
/// jnoccio's run-to-run stochasticity (the reviewer-flagged "scoring inconsistency"). Median of the
/// continuous scores is robust to a single outlier call; the spread surfaces judge disagreement;
/// the fatal-flaw is a majority vote.
#[derive(Debug, Clone)]
pub(crate) struct VotedVerdict {
    /// The aggregated verdict (median scores, majority fatal-flaw, "ok" if any call parsed).
    pub verdict: LiveVerdict,
    /// Total critic calls issued.
    pub votes: usize,
    /// Calls that returned a parseable score.
    pub ok_votes: usize,
    /// max − min of falsifiability across ok votes (large ⇒ low judge confidence).
    pub falsifiability_spread: f64,
    /// max − min of plausibility across ok votes.
    pub plausibility_spread: f64,
    /// Fraction of ok votes that named a fatal flaw (the majority signal).
    pub fatal_flaw_rate: f64,
}

fn median_sorted(mut xs: Vec<f64>) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = xs.len();
    if n % 2 == 1 {
        xs[n / 2]
    } else {
        0.5 * (xs[n / 2 - 1] + xs[n / 2])
    }
}

fn most_common_flaw(ok: &[&LiveVerdict]) -> String {
    use std::collections::BTreeMap;
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for v in ok {
        let f = v.fatal_flaw.trim();
        if !f.is_empty() {
            *counts.entry(f).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(s, _)| s.to_string())
        .unwrap_or_default()
}

/// Aggregate critic verdicts by MEDIAN with a disagreement spread and a majority fatal-flaw vote.
/// Deterministic in the inputs (no RNG/clock), so it is unit-testable without live calls.
pub(crate) fn aggregate_verdicts(verdicts: &[LiveVerdict]) -> VotedVerdict {
    let ok: Vec<&LiveVerdict> = verdicts.iter().filter(|v| v.status == "ok").collect();
    let votes = verdicts.len();
    let ok_votes = ok.len();
    if ok.is_empty() {
        let status = verdicts
            .first()
            .map(|v| v.status.clone())
            .unwrap_or_else(|| "error".to_string());
        return VotedVerdict {
            verdict: LiveVerdict {
                falsifiability: 0.0,
                plausibility: 0.0,
                fatal_flaw: String::new(),
                status,
                elapsed_seconds: 0.0,
            },
            votes,
            ok_votes: 0,
            falsifiability_spread: 0.0,
            plausibility_spread: 0.0,
            fatal_flaw_rate: 0.0,
        };
    }
    let fvals: Vec<f64> = ok.iter().map(|v| v.falsifiability).collect();
    let pvals: Vec<f64> = ok.iter().map(|v| v.plausibility).collect();
    let spread = |xs: &[f64]| {
        xs.iter().cloned().fold(f64::MIN, f64::max) - xs.iter().cloned().fold(f64::MAX, f64::min)
    };
    let flaw_count = ok.iter().filter(|v| !v.fatal_flaw.trim().is_empty()).count();
    let fatal_flaw_rate = flaw_count as f64 / ok_votes as f64;
    // A fatal flaw only carries if a MAJORITY of ok votes name one (self-consistency).
    let fatal_flaw = if fatal_flaw_rate > 0.5 {
        most_common_flaw(&ok)
    } else {
        String::new()
    };
    VotedVerdict {
        verdict: LiveVerdict {
            falsifiability: median_sorted(fvals.clone()),
            plausibility: median_sorted(pvals.clone()),
            fatal_flaw,
            status: "ok".to_string(),
            elapsed_seconds: ok.iter().map(|v| v.elapsed_seconds).sum(),
        },
        votes,
        ok_votes,
        falsifiability_spread: spread(&fvals),
        plausibility_spread: spread(&pvals),
        fatal_flaw_rate,
    }
}

/// Make `votes` critic calls (each requesting the top-20% jnoccio models) and aggregate them — the
/// voting/score-averaging scorer that tames run-to-run LLM stochasticity. A short sleep between
/// calls avoids the rate limiter. `votes` is clamped to ≥ 1.
pub(crate) fn run_live_critique_voted(
    candidate: &Value,
    timeout_seconds: u64,
    votes: usize,
) -> VotedVerdict {
    let n = votes.max(1);
    let mut verdicts = Vec::with_capacity(n);
    for i in 0..n {
        verdicts.push(run_live_critique_banded(
            candidate,
            timeout_seconds,
            RouteTier::Top20Pct,
        ));
        if i + 1 < n {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    aggregate_verdicts(&verdicts)
}

#[cfg(test)]
mod vote_tests {
    use super::*;

    fn v(f: f64, p: f64, flaw: &str, status: &str) -> LiveVerdict {
        LiveVerdict {
            falsifiability: f,
            plausibility: p,
            fatal_flaw: flaw.to_string(),
            status: status.to_string(),
            elapsed_seconds: 1.0,
        }
    }

    #[test]
    fn median_aggregation_is_robust_to_an_outlier_call() {
        // Two consistent calls + one wild outlier: the median tracks the consensus, the spread flags it.
        let verdicts = vec![
            v(0.80, 0.70, "", "ok"),
            v(0.82, 0.72, "", "ok"),
            v(0.10, 0.10, "noise", "ok"),
        ];
        let agg = aggregate_verdicts(&verdicts);
        assert_eq!(agg.ok_votes, 3);
        assert!(
            (agg.verdict.falsifiability - 0.80).abs() < 1e-9,
            "median = {}",
            agg.verdict.falsifiability
        );
        assert!(agg.falsifiability_spread > 0.6, "spread must surface the disagreement");
    }

    #[test]
    fn fatal_flaw_needs_a_majority() {
        let majority = vec![
            v(0.5, 0.5, "gray-box knob", "ok"),
            v(0.5, 0.5, "gray-box knob", "ok"),
            v(0.5, 0.5, "", "ok"),
        ];
        assert_eq!(aggregate_verdicts(&majority).verdict.fatal_flaw, "gray-box knob");

        let minority = vec![
            v(0.5, 0.5, "maybe", "ok"),
            v(0.5, 0.5, "", "ok"),
            v(0.5, 0.5, "", "ok"),
        ];
        assert!(
            aggregate_verdicts(&minority).verdict.fatal_flaw.is_empty(),
            "a 1-of-3 flaw must not win the vote"
        );
    }

    #[test]
    fn all_failed_calls_surface_a_non_ok_status() {
        let agg = aggregate_verdicts(&[v(0.0, 0.0, "", "error"), v(0.0, 0.0, "", "unparsed")]);
        assert_eq!(agg.ok_votes, 0);
        assert_ne!(agg.verdict.status, "ok");
    }
}
