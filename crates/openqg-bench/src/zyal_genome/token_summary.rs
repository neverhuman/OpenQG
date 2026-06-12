//! SYNTHESIS #1 — `zyal genome tokens`: read `token-ledger.jsonl` from a run directory and emit
//! a token/cost observability summary.
//!
//! The campaign observability guarantee is `unattributed_calls == 0`: every LLM call must appear
//! in the ledger with a provider, model, and attribution role. This command makes that checkable
//! from artifacts alone, without replaying the run.

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};

use super::token_receipt::LlmCallReceipt;

/// Aggregated totals across all receipts in a token ledger.
#[derive(Debug, Default)]
struct TokenTotals {
    call_count: u64,
    input_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
    total_cost_usd: f64,
    flagged_estimate_count: u64,
    by_role: std::collections::BTreeMap<String, RoleTotals>,
}

#[derive(Debug, Default)]
struct RoleTotals {
    calls: u64,
    tokens: u64,
    cost_usd: f64,
}

fn load_ledger(run_dir: &Path) -> Result<Vec<LlmCallReceipt>> {
    let path = run_dir.join("token-ledger.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut receipts = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let r: LlmCallReceipt = serde_json::from_str(line)
            .with_context(|| format!("parse token-ledger.jsonl line {}", i + 1))?;
        receipts.push(r);
    }
    Ok(receipts)
}

fn aggregate(receipts: &[LlmCallReceipt]) -> TokenTotals {
    let mut t = TokenTotals::default();
    for r in receipts {
        t.call_count += 1;
        t.input_tokens += r.usage.input;
        t.output_tokens += r.usage.output;
        t.total_tokens += r.usage.total;
        t.total_cost_usd += r.cost.total_usd;
        if r.flagged_estimate {
            t.flagged_estimate_count += 1;
        }
        let role = t.by_role.entry(r.attributed_to.clone()).or_default();
        role.calls += 1;
        role.tokens += r.usage.total;
        role.cost_usd += r.cost.total_usd;
    }
    t
}

pub(crate) fn run_tokens(run_dir: &Path, as_json: bool) -> Result<()> {
    let receipts = load_ledger(run_dir)?;
    let t = aggregate(&receipts);
    let unattributed = receipts
        .iter()
        .filter(|r| r.attributed_to.is_empty())
        .count() as u64;
    let observability_ok = unattributed == 0;

    if as_json {
        let by_role: serde_json::Map<String, Value> = t
            .by_role
            .iter()
            .map(|(role, rt)| {
                (
                    role.clone(),
                    json!({
                        "calls": rt.calls,
                        "total_tokens": rt.tokens,
                        "total_cost_usd": rt.cost_usd,
                    }),
                )
            })
            .collect();
        let out = json!({
            "record_kind": "token_summary",
            "run_dir": run_dir.display().to_string(),
            "call_count": t.call_count,
            "input_tokens": t.input_tokens,
            "output_tokens": t.output_tokens,
            "total_tokens": t.total_tokens,
            "total_cost_usd": t.total_cost_usd,
            "flagged_estimate_count": t.flagged_estimate_count,
            "unattributed_calls": unattributed,
            "observability_ok": observability_ok,
            "by_role": by_role,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!("Token summary for: {}", run_dir.display());
        println!(
            "  calls:        {}  (flagged-estimate: {})",
            t.call_count, t.flagged_estimate_count
        );
        println!(
            "  tokens:       {} in / {} out / {} total",
            t.input_tokens, t.output_tokens, t.total_tokens
        );
        println!("  cost (USD):   ${:.4}", t.total_cost_usd);
        println!(
            "  observability: {} (unattributed_calls={})",
            if observability_ok { "OK" } else { "FAIL" },
            unattributed
        );
        if !t.by_role.is_empty() {
            println!("  by role:");
            for (role, rt) in &t.by_role {
                println!(
                    "    {role:<14} {:>6} calls  {:>9} tokens  ${:.4}",
                    rt.calls, rt.tokens, rt.cost_usd
                );
            }
        }
        if t.call_count == 0 {
            println!("  (no LLM calls recorded — fixture-proposer or pure-mutation run)");
        }
    }

    if !observability_ok {
        anyhow::bail!("observability_ok=false: {unattributed} unattributed call(s) in ledger");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_receipt(role: &str, flagged: bool) -> LlmCallReceipt {
        use super::super::token_receipt::{CostUsage, TokenUsage};
        LlmCallReceipt {
            call_id: format!("test-{role}"),
            timestamp_utc: "2026-06-12T00:00:00Z".into(),
            provider: "anthropic".into(),
            model: "model-not-in-registry".into(),
            usage: TokenUsage::new(100, 50),
            cost: CostUsage::new(0.0003, 0.00075),
            attributed_to: role.to_string(),
            flagged_estimate: flagged,
        }
    }

    #[test]
    fn empty_ledger_is_ok() {
        let tmp = std::env::temp_dir().join(format!("tksumm-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        // No token-ledger.jsonl — fixture-proposer run.
        run_tokens(&tmp, false).expect("empty ledger must not fail");
        run_tokens(&tmp, true).expect("empty ledger json must not fail");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn attributed_receipts_pass_observability() {
        let tmp = std::env::temp_dir().join(format!("tksumm-attr-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        let r1 = serde_json::to_string(&make_receipt("proposer", false)).unwrap();
        let r2 = serde_json::to_string(&make_receipt("router", true)).unwrap();
        fs::write(tmp.join("token-ledger.jsonl"), format!("{r1}\n{r2}\n")).unwrap();
        run_tokens(&tmp, false).expect("all-attributed ledger must pass");
        run_tokens(&tmp, true).expect("json mode must pass");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unattributed_receipt_fails_observability() {
        let tmp = std::env::temp_dir().join(format!("tksumm-unattr-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        let r = serde_json::to_string(&make_receipt("", false)).unwrap();
        fs::write(tmp.join("token-ledger.jsonl"), format!("{r}\n")).unwrap();
        assert!(
            run_tokens(&tmp, false).is_err(),
            "unattributed call must fail observability gate"
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn aggregation_sums_correctly() {
        let r1 = make_receipt("proposer", false);
        let r2 = make_receipt("router", false);
        let totals = aggregate(&[r1, r2]);
        assert_eq!(totals.call_count, 2);
        assert_eq!(totals.total_tokens, 300); // 150 each
        assert_eq!(totals.by_role.len(), 2);
    }
}
