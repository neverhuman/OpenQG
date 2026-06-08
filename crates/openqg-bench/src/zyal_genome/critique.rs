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
    let prompt = build_critique_prompt(candidate);
    let command: Vec<String> = JNOCCIO_CRITIQUE_COMMAND
        .iter()
        .map(|s| s.to_string())
        .collect();
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
