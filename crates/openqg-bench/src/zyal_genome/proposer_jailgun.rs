//! V4 M6b: the live jailgun proposer adapter.
//!
//! [`JailgunProposer`] is the live backend for the [`Proposer`] contract proven by `FixtureProposer`:
//! it asks ChatGPT (via the jailgun MCP round-trip) to emit a [`ProposalDoc`] JSON for a candidate
//! unified-physics theory, then parses that output into the typed document. The result flows into
//! the SAME deterministic [`score_proposal`] oracle — the LLM only proposes; it never scores, and a
//! cheating proposal (laundered evidence, hidden knob, free parameter, unverifiable derivation) is
//! disqualified exactly as the fixture tests show.
//!
//! The prompt construction and the response→`ProposalDoc` parsing are pure and unit-tested here; the
//! only non-deterministic step is the jailgun call itself (reusing `run_jailgun_live_call_attempt`),
//! which requires the jailgun server to be up.

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use super::jailgun_live::run_jailgun_live_call_attempt;
use super::proposer::{fixture_proposal, ProposalDoc, Proposer};

const DOWNLOAD_TARGET: &str = "openqg-v4-proposal.json";

/// The live jailgun proposer. Construct with a per-call timeout (seconds); routing/account/token are
/// resolved from the environment by the underlying live-call machinery (the same path the genome
/// already uses), so a missing server / token surfaces as a clear error rather than a panic.
pub(crate) struct JailgunProposer {
    pub timeout_seconds: u64,
}

impl Default for JailgunProposer {
    fn default() -> Self {
        Self {
            timeout_seconds: 240,
        }
    }
}

/// Build the deterministic proposer prompt: it states the goal (a *derivation-rich, critic-proof*
/// candidate), pins the exact `ProposalDoc` schema by example (the fixture proposal serialized), and
/// the hard rules the deterministic oracle will enforce — so the model proposes in a shape that can
/// actually pass the gates rather than be disqualified.
pub(crate) fn build_proposer_prompt() -> String {
    // Compact (not pretty) example keeps the prompt small so the browser round-trip completes.
    let example = serde_json::to_string(&fixture_proposal()).unwrap_or_else(|_| "{}".to_string());
    // The authoritative registry: the LLM otherwise invents relation names the oracle cannot
    // recompute (UnknownRelation kill), so we pin the exact set + their input signatures.
    let relations = openqg_core::theory::registered_relations()
        .iter()
        .map(|r| {
            format!(
                "  - {r}  [{}]",
                openqg_core::theory::relation_signature(r).unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "You are proposing ONE candidate component of a unified theory of physics for the OpenQG \
engine. Your proposal is adjudicated by a DETERMINISTIC oracle — you do not score it, and any \
cheating is disqualified. To earn credit you MUST:\n\
\n\
1. Make every parameter DERIVED, not fitted: each parameter's `provenance` is either \
`\"fundamental\"` or a `derived` object `{{\"derived\":{{\"mechanism\":\"...\",\"certificate\":{{...}}}}}}` \
whose certificate names a closed-form relation from the AVAILABLE RELATIONS list below (any other \
relation name is an UnknownRelation KILL). A free/uncertified knob is KILLED.\n\
2. Attach, for each physics claim, at least one derivation OBLIGATION that VERIFIES (a \
`numeric_witness` carrying the same certificate, or a `limit` witness whose residual is within its \
bound). An unobligated physics claim is KILLED.\n\
3. Provide the actual CONTENT of every cited piece of evidence in the `evidence` map (path → text). \
Citing evidence you do not supply is laundering and is KILLED.\n\
4. If you claim UNIFICATION, every free degree of freedom must be a shared parameter (no hidden \
sector-private knob), or it is KILLED.\n\
5. The theory must pass the physical veto cascade (dimensionally homogeneous terms, no ghost, GR \
recovery / screening for any gravity modification).\n\
6. ALL numeric fields MUST be JSON NUMBERS, never strings or labels: every parameter `value`, every \
term `coefficient`/exponent, and every certificate `inputs` value / `expected` / `tolerance` must \
look like `1.16667`, not `\"ndgp\"` or `\"1.16667\"`. Identifiers (relation names, symbols) are \
strings; physical magnitudes are numbers.\n\
7. Each claim's `sector` MUST be EXACTLY one of: `background`, `growth`, `tensor_sector`, \
`screening_ppn`, `bbn`, `particle`, `quantum`. Do NOT invent sectors like `gravity`, `stability`, or \
`unification` (gravity modifications go in `growth` or `tensor_sector`; a unification statement is \
expressed via the top-level `unification.shared` list, not a claim sector). Each claim `kind` is \
`physics` or `engineering`.\n\
\n\
AVAILABLE RELATIONS — the ONLY relation names the oracle can recompute; use these EXACT names and \
supply the exact named inputs (a certificate's `expected` is recomputed from `inputs` and must match):\n\
{relations}\n\
Note: `screening` is a single STRING label (e.g. \"vainshtein\"), and `screening_recovery` is a single \
NUMBER (the GR-recovery residual, ~0), not objects.\n\
\n\
Return EXACTLY one downloadable JSON artifact named `{DOWNLOAD_TARGET}` and nothing else — no prose, \
no markdown fences. It must match this schema (here is a complete, valid example you should improve \
upon, NOT copy verbatim):\n\
\n\
{example}\n"
    )
}

/// Extract a [`ProposalDoc`] from a raw model/artifact response: tolerate Markdown code fences and
/// leading/trailing prose by slicing the outermost `{ ... }` JSON object, then parse strictly.
pub(crate) fn parse_proposal_response(raw: &str) -> Result<ProposalDoc> {
    let trimmed = raw.trim();
    // Strip a ```json ... ``` fence if present.
    let body = if let Some(rest) = trimmed.strip_prefix("```") {
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        rest.rsplit_once("```").map(|(a, _)| a).unwrap_or(rest)
    } else {
        trimmed
    };
    // Slice the outermost JSON object.
    let start = body
        .find('{')
        .context("no JSON object found in proposal response")?;
    let end = body
        .rfind('}')
        .context("no closing brace in proposal response")?;
    anyhow::ensure!(end > start, "malformed JSON object in proposal response");
    let mut value: Value =
        serde_json::from_str(&body[start..=end]).context("parse proposal JSON")?;
    repair_proposal_value(&mut value);
    serde_json::from_value(value).context("parse proposal document")
}

/// Project the rich shapes the LLM tends to emit onto our strict schema slots — lossless w.r.t. the
/// schema, which only HAS a string/number there: `theory.screening` object → its `mechanism` string;
/// `theory.screening_recovery` object → its `residual` number. The physics claims, parameters,
/// certificates, terms, etc. are left untouched and adjudicated strictly.
fn repair_proposal_value(value: &mut Value) {
    let Some(theory) = value.get_mut("theory").and_then(Value::as_object_mut) else {
        return;
    };
    if theory
        .get("screening")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        let mech = theory["screening"]
            .get("mechanism")
            .and_then(Value::as_str)
            .map(|s| Value::String(s.to_string()))
            .unwrap_or(Value::Null);
        theory.insert("screening".into(), mech);
    }
    if theory
        .get("screening_recovery")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        let num = theory["screening_recovery"]
            .get("residual")
            .filter(|v| v.is_number())
            .cloned()
            .unwrap_or(Value::Null);
        theory.insert("screening_recovery".into(), num);
    }
}

/// Read the model's emitted JSON from a completed live attempt: the jailgun run summary (stdout)
/// lists downloaded artifacts; we read the `{DOWNLOAD_TARGET}` artifact if present, else fall back to
/// the stdout itself (some backends inline the JSON).
fn response_text_from_stdout(stdout: &str) -> Result<String> {
    if let Ok(summary) = serde_json::from_str::<Value>(stdout) {
        if let Some(arts) = summary.get("artifacts").and_then(Value::as_array) {
            for a in arts {
                if let Some(path) = a.get("path").and_then(Value::as_str) {
                    if path.ends_with(DOWNLOAD_TARGET) || path.ends_with(".json") {
                        if let Ok(content) = fs::read_to_string(path) {
                            return Ok(content);
                        }
                    }
                }
            }
        }
    }
    // Fallback: the response itself may already be the JSON.
    Ok(stdout.to_string())
}

impl Proposer for JailgunProposer {
    fn propose(&self) -> Result<ProposalDoc> {
        let prompt = build_proposer_prompt();
        let dir = std::env::temp_dir().join(format!("openqg-v4-proposer-{}", std::process::id()));
        fs::create_dir_all(&dir).context("create proposer temp dir")?;
        let prompt_path: PathBuf = dir.join("proposer-prompt.md");
        fs::write(&prompt_path, &prompt).context("write proposer prompt")?;

        // Unique per invocation: a fixed call_id collides with a prior (e.g. timed-out) run that is
        // still registered in jailgun (`agent-run-conflict: run_id already exists`).
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let call_id = format!("openqg-v4-proposer-{unique}");
        let attempt = run_jailgun_live_call_attempt(
            &call_id,
            &prompt_path,
            DOWNLOAD_TARGET,
            self.timeout_seconds,
            1,
            "v4-proposer",
        );

        if attempt.status() != "ok" && attempt.status() != "success" {
            bail!(
                "jailgun proposer call failed (status {}): {}",
                attempt.status(),
                attempt.error().unwrap_or("no detail")
            );
        }
        let response = response_text_from_stdout(attempt.stdout())?;
        // Persist the raw LLM proposal so a schema-rejection is always inspectable.
        let raw_path = dir.join("last-proposal.json");
        let _ = fs::write(&raw_path, &response);
        parse_proposal_response(&response).with_context(|| {
            format!(
                "parse the jailgun-proposed ProposalDoc (raw saved to {})",
                raw_path.display()
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_proposer_prompt_pins_the_schema_and_rules() {
        let p = build_proposer_prompt();
        assert!(p.contains(DOWNLOAD_TARGET));
        assert!(p.contains("DETERMINISTIC oracle"));
        assert!(p.contains("provenance"));
        assert!(p.contains("obligation") || p.contains("OBLIGATION"));
        // The embedded example is a valid ProposalDoc.
        assert!(
            p.contains("ndgp_geff_over_g"),
            "prompt should embed the schema-by-example"
        );
    }

    #[test]
    fn parse_response_handles_a_plain_json_proposal() {
        let json = serde_json::to_string(&fixture_proposal()).unwrap();
        let doc = parse_proposal_response(&json).expect("parse plain json");
        assert_eq!(doc.theory.id, fixture_proposal().theory.id);
    }

    #[test]
    fn parse_response_handles_markdown_fences_and_prose() {
        let json = serde_json::to_string(&fixture_proposal()).unwrap();
        let wrapped = format!("Here is my proposal:\n```json\n{json}\n```\nThanks!");
        let doc = parse_proposal_response(&wrapped).expect("parse fenced json");
        assert_eq!(doc.theory.id, fixture_proposal().theory.id);
    }

    #[test]
    fn repair_normalizes_rich_screening_shapes() {
        // The LLM tends to emit screening as a rich object; repair projects it onto our schema slots.
        let mut v: Value =
            serde_json::from_str(&serde_json::to_string(&fixture_proposal()).unwrap()).unwrap();
        v["theory"]["screening"] = serde_json::json!({"mechanism": "vainshtein", "active": true});
        v["theory"]["screening_recovery"] =
            serde_json::json!({"residual": 1.0e-12, "bound": 1.0e-6});
        let raw = serde_json::to_string(&v).unwrap();
        let doc = parse_proposal_response(&raw).expect("repair should normalize rich screening");
        assert_eq!(doc.theory.screening.as_deref(), Some("vainshtein"));
        assert_eq!(doc.theory.screening_recovery, Some(1.0e-12));
    }

    #[test]
    fn the_prompt_lists_the_real_registry_relations() {
        let p = build_proposer_prompt();
        assert!(p.contains("AVAILABLE RELATIONS"));
        assert!(
            p.contains("h0_from_h"),
            "prompt must list the real registry relations"
        );
        assert!(p.contains("flat_universe_omega_lambda"));
    }

    #[test]
    fn parse_response_rejects_garbage() {
        assert!(parse_proposal_response("no json here").is_err());
        assert!(parse_proposal_response("{not valid json").is_err());
    }

    #[test]
    fn a_jailgun_proposed_doc_flows_through_the_same_oracle() {
        // Simulate a live response (the fixture JSON) and confirm it scores through the oracle the
        // same way — proving the live path reuses the deterministic adjudication.
        use super::super::physics_score::baseline_log_likelihood;
        use super::super::proposer::score_proposal;
        use openqg_core::ObservableRecord;
        let observables: Vec<ObservableRecord> = ["a", "b", "c", "d"]
            .iter()
            .map(|id| ObservableRecord {
                observable_id: (*id).into(),
                kind: "cosmology".into(),
                value: 1.0,
                uncertainty: 0.1,
                unit: "x".into(),
                source: None,
            })
            .collect();
        let raw = serde_json::to_string(&fixture_proposal()).unwrap();
        let doc = parse_proposal_response(&raw).unwrap();
        let sc = score_proposal(&doc, &observables, baseline_log_likelihood(&observables));
        assert!(!sc.disqualified);
        assert!(sc.total > 40.0);
    }
}
