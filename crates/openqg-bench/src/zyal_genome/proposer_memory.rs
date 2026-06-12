//! Proposer memory + computed data brief — deterministic context for the LLM proposer.
//!
//! Two pure pieces (no LLM, no network; filesystem reads only):
//!
//! 1. [`build_data_brief`] predicts the Planck-ΛCDM baseline on the *actual* observables with the
//!    same deterministic [`BackgroundForwardModel`] the oracle scores against, and reports the real
//!    pulls — so the proposer sees exactly which tensions exist and their signs, instead of
//!    folklore about "the H0 tension".
//! 2. [`assemble_memory`] + [`render_memory_section`] mine prior run ledgers (plus the current
//!    run's in-flight records) for top scorers and normalized kill classes, and synthesize DO /
//!    DON'T directives — the cross-run memory that stops the proposer re-trying known-dead moves.
//!
//! Both are pure given their inputs, so a rendered brief replays byte-equal without the LLM.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use openqg_core::cosmology::{BackgroundForwardModel, CosmologyParams, ForwardModel};
use openqg_core::ObservableRecord;
use serde_json::Value;

/// Controls how much of the per-observable pull data the LLM proposer receives.
///
/// S09 identifies the data brief as a contamination channel: a proposer that sees exact pulls
/// per observable can tune parameters to hit the training data rather than making a genuine
/// theoretical prediction. `BlindedToTensions` strips the individual pulls to eliminate this
/// route, while still providing the proposer with enough structural information to work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProposerBriefPolicy {
    /// Report all pulls as computed — the default for exploratory runs.
    FullPulls,
    /// Report only the count and direction of significant tensions (|pull| > 1.5σ), not the
    /// individual values. Prevents the proposer from chasing specific per-observable residuals.
    SummaryOnly,
    /// Strip all per-observable pulls and replace with a policy notice. Used when the proposer
    /// must remain blind to the tension pattern to prevent post-hoc parameter fitting.
    BlindedToTensions,
}

/// The verbatim directive that closes every data brief.
const GOAL_LINE: &str = "GOAL: reduce the largest |pull| WITHOUT inflating the others.";

/// The verbatim growth directive, emitted only when the data actually pull growth downward.
const GROWTH_NOTE_LINE: &str = "NOTE: the growth pulls are negative — growth must be SUPPRESSED \
(G_eff/G < 1, e.g. planck_mu0_geff with mu0 < 0), not enhanced.";

/// True for the growth-of-structure observable ids whose pull sign decides the growth NOTE.
fn is_growth_id(observable_id: &str) -> bool {
    let id = observable_id.to_ascii_lowercase();
    id.contains("fsigma8") || id.contains("sigma8") || id == "s8"
}

/// Policy-aware DATA BRIEF builder.
///
/// Under `BlindedToTensions`, all per-observable pull values are stripped and replaced with a
/// firewall notice. The proposer receives the count of significant tensions but no pull magnitudes
/// or signs — it cannot reverse-engineer which parameters to shift. Deterministic given policy.
pub(crate) fn build_data_brief_with_policy(
    observables: &[ObservableRecord],
    policy: ProposerBriefPolicy,
) -> String {
    match policy {
        ProposerBriefPolicy::FullPulls => build_data_brief(observables),
        ProposerBriefPolicy::SummaryOnly => {
            let model = BackgroundForwardModel;
            let ids: Vec<String> = observables
                .iter()
                .map(|o| o.observable_id.clone())
                .collect();
            let predicted: BTreeMap<String, f64> = model
                .predict(&CosmologyParams::planck_lcdm(), &ids)
                .map(|preds| {
                    preds
                        .into_iter()
                        .map(|p| (p.observable_id, p.value))
                        .collect()
                })
                .unwrap_or_default();
            let mut n_significant = 0usize;
            for o in observables {
                if let Some(&pred) = predicted.get(&o.observable_id) {
                    if o.uncertainty > 0.0 {
                        let pull = (o.value - pred) / o.uncertainty;
                        if pull.is_finite() && pull.abs() > 1.5 {
                            n_significant += 1;
                        }
                    }
                }
            }
            let mut out = String::new();
            out.push_str("## DATA BRIEF — summary mode (individual pulls withheld)\n");
            out.push_str(&format!(
                "  {}/{} observables show |pull| > 1.5σ vs ΛCDM\n",
                n_significant,
                observables.len()
            ));
            out.push_str(GOAL_LINE);
            out.push('\n');
            out
        }
        ProposerBriefPolicy::BlindedToTensions => {
            let mut out = String::new();
            out.push_str("## DATA BRIEF — BLINDED (S09 data-brief firewall active)\n");
            out.push_str(
                "  Per-observable pull values have been withheld to prevent post-hoc parameter\n",
            );
            out.push_str(
                "  fitting. Propose mechanisms from theoretical motivation alone. Do not attempt\n",
            );
            out.push_str("  to infer tension directions from observable names or ordering.\n");
            out.push_str(&format!("  N_observables: {}\n", observables.len()));
            out.push_str(GOAL_LINE);
            out.push('\n');
            out
        }
    }
}

/// The computed DATA BRIEF: predict the Planck-ΛCDM baseline on the actual observables and report
/// the real pulls, so the proposer sees exactly which tensions exist and their signs.
///
/// pull = (data − ΛCDM prediction) / data uncertainty; a non-positive (or non-finite) uncertainty
/// prints "n/a" instead of a fake pull. Observables the background model cannot predict are listed
/// as an explicit coverage gap, never faked. Compact 2-decimal values + 1-decimal pulls keep the
/// brief near ~1.2 KB for ~22 observables. Deterministic: two calls are byte-equal.
pub(crate) fn build_data_brief(observables: &[ObservableRecord]) -> String {
    let model = BackgroundForwardModel;
    let ids: Vec<String> = observables
        .iter()
        .map(|o| o.observable_id.clone())
        .collect();
    let predicted: BTreeMap<String, f64> = model
        .predict(&CosmologyParams::planck_lcdm(), &ids)
        .map(|preds| {
            preds
                .into_iter()
                .map(|p| (p.observable_id, p.value))
                .collect()
        })
        .unwrap_or_default();

    // pull for one record, or None when the uncertainty cannot support one.
    let pull_of = |o: &ObservableRecord, pred: f64| -> Option<f64> {
        if o.uncertainty > 0.0 {
            let pull = (o.value - pred) / o.uncertainty;
            pull.is_finite().then_some(pull)
        } else {
            None
        }
    };

    // First pass: find the max-|pull| observable so its line can be marked.
    let largest_idx: Option<usize> = observables
        .iter()
        .enumerate()
        .filter_map(|(i, o)| {
            let pred = predicted.get(&o.observable_id)?;
            pull_of(o, *pred).map(|p| (i, p.abs()))
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i);

    let mut out = String::new();
    out.push_str("## DATA BRIEF — real data vs the ΛCDM baseline (computed, not opinions)\n");
    let mut coverage_gaps: Vec<&str> = Vec::new();
    let mut growth_pull_negative = false;
    for (i, o) in observables.iter().enumerate() {
        let Some(pred) = predicted.get(&o.observable_id) else {
            coverage_gaps.push(&o.observable_id);
            continue;
        };
        let pull_txt = match pull_of(o, *pred) {
            Some(pull) => {
                if pull < 0.0 && is_growth_id(&o.observable_id) {
                    growth_pull_negative = true;
                }
                format!("{pull:+.1}σ")
            }
            None => "n/a".to_string(),
        };
        let marker = if largest_idx == Some(i) {
            " ← LARGEST TENSION"
        } else {
            ""
        };
        out.push_str(&format!(
            "  {}: data {:.2} ± {:.2} | LCDM {:.2} | pull {}{}\n",
            o.observable_id, o.value, o.uncertainty, pred, pull_txt, marker
        ));
    }
    if !coverage_gaps.is_empty() {
        out.push_str("(not predicted by the background model — coverage gap)\n");
        out.push_str(&format!("  {}\n", coverage_gaps.join(", ")));
    }
    out.push_str(GOAL_LINE);
    out.push('\n');
    if growth_pull_negative {
        out.push_str(GROWTH_NOTE_LINE);
        out.push('\n');
    }
    out
}

/// Cross-run memory assembled from the run ledgers: the deterministic facts a proposer should not
/// have to rediscover by burning oracle adjudications.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ProposerMemory {
    /// Up to 3 rendered top-scorer lines, best first:
    /// `"61.3 distinct=true openqg-v4-ndgp-vainshtein-component-tab-1 (gen 1320)"`.
    pub top: Vec<String>,
    /// Normalized kill classes with counts, most common first (ties broken by class name).
    pub kill_histogram: Vec<(String, usize)>,
    /// Positive directives (rendered with a `DO: ` prefix).
    pub do_lines: Vec<String>,
    /// Negative directives (rendered with a `DON'T: ` prefix).
    pub dont_lines: Vec<String>,
}

/// Normalize one raw kill-reason string to a stable class by substring, so the histogram survives
/// wording changes in the oracle's messages. Unrecognized reasons land in `other`.
fn normalize_kill_reason(reason: &str) -> &'static str {
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

/// All normalized kill classes carried by one ledger/attempt record. Kill reasons are read from a
/// top-level `kill_reasons` array, a nested `scorecard.kill_reasons` array, or a single
/// `kill_reason` string; an `outcome` of `"parse_error"` (an attempt whose LLM output was not
/// valid JSON) is its own class.
fn record_kill_classes(record: &Value) -> Vec<&'static str> {
    let mut classes = Vec::new();
    if record.get("outcome").and_then(Value::as_str) == Some("parse_error") {
        classes.push("parse_error");
    }
    let arrays = [
        record.get("kill_reasons"),
        record.get("scorecard").and_then(|s| s.get("kill_reasons")),
    ];
    for list in arrays.into_iter().flatten() {
        if let Some(reasons) = list.as_array() {
            classes.extend(
                reasons
                    .iter()
                    .filter_map(Value::as_str)
                    .map(normalize_kill_reason),
            );
        }
    }
    if let Some(single) = record.get("kill_reason").and_then(Value::as_str) {
        classes.push(normalize_kill_reason(single));
    }
    classes
}

/// A top-scorer entry from a proposal-ledger record: non-disqualified, with a finite total.
/// Returns `(total, rendered line, optional content hash for dedupe)`.
fn top_scorer_entry(record: &Value) -> Option<(f64, String, Option<String>)> {
    if record.get("disqualified").and_then(Value::as_bool) != Some(false) {
        return None;
    }
    let total = record.get("total").and_then(Value::as_f64)?;
    if !total.is_finite() {
        return None;
    }
    // V6 (review-10): memory must be PROMOTABLE-gated — a ledger entry scored under an older,
    // weaker rubric can only become a DO-example if its theory survives the CURRENT unified
    // physics gate. The V5 champions (bare screening labels, unexplained drift) die here.
    if let Some(theory_value) = record.pointer("/doc/theory") {
        match serde_json::from_value::<openqg_core::Theory>(theory_value.clone()) {
            Ok(theory) => {
                if !openqg_core::physics_kills(&theory).is_empty() {
                    return None;
                }
            }
            Err(_) => return None, // unparseable under the current schema ⇒ not promotable
        }
    }
    let theory_id = record.get("theory_id").and_then(Value::as_str)?;
    let generation = record
        .get("generation")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let distinct = record
        .get("distinct_from_baseline")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let line = format!("{total:.1} distinct={distinct} {theory_id} (gen {generation})");
    let sha = record
        .get("proposal_sha256")
        .and_then(Value::as_str)
        .map(str::to_string);
    Some((total, line, sha))
}

/// Read every JSON record from one `.jsonl` file; a missing file (e.g. a run that never wrote
/// `proposal-attempts.jsonl`) or an unparseable line is tolerated by skipping it.
fn read_jsonl_records(path: &Path, into: &mut Vec<Value>) {
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            into.push(value);
        }
    }
}

/// Coarse mechanism family of a ledger proposal (for the diversity histogram).
fn mechanism_family(record: &Value) -> &'static str {
    let bg = record.pointer("/doc/theory/background");
    let drag = bg
        .and_then(|b| b.get("drag_a"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    if drag > 0.0 {
        return "dark_scattering";
    }
    let mg = bg
        .and_then(|b| b.get("mg_family"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    match mg {
        "ndgp" => "ndgp",
        "fr_hu_sawicki" => "fr",
        _ => {
            let mu0 = bg
                .and_then(|b| b.get("mu0"))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if mu0 < 0.0 {
                "planck_mu0_suppressed"
            } else if mu0 > 0.0 {
                "planck_mu0_enhanced"
            } else {
                "lcdm_adjacent"
            }
        }
    }
}

/// Assemble cross-run memory from the ledger files under `runs_root` (every
/// `*/proposal-ledger.jsonl` and `*/proposal-attempts.jsonl`), plus the current run's records
/// passed as raw [`serde_json::Value`]s (so no type dependency on the writers). Pure given the
/// files: directories are scanned in sorted order, so the result is deterministic.
pub(crate) fn assemble_memory(runs_root: &Path, current_records: &[Value]) -> ProposerMemory {
    let mut records: Vec<Value> = Vec::new();
    if let Ok(entries) = fs::read_dir(runs_root) {
        let mut run_dirs: Vec<std::path::PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        run_dirs.sort();
        for dir in run_dirs {
            read_jsonl_records(&dir.join("proposal-ledger.jsonl"), &mut records);
            read_jsonl_records(&dir.join("proposal-attempts.jsonl"), &mut records);
        }
    }
    records.extend(current_records.iter().cloned());

    // Kill histogram over normalized classes.
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for record in &records {
        for class in record_kill_classes(record) {
            *counts.entry(class).or_insert(0) += 1;
        }
    }
    let mut kill_histogram: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(class, n)| (class.to_string(), n))
        .collect();
    kill_histogram.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Top scorers: best 3 non-disqualified ledger entries, deduped by content hash so a current
    // record already flushed to disk is not double-counted.
    let mut seen_sha: BTreeSet<String> = BTreeSet::new();
    let mut scorers: Vec<(f64, String)> = Vec::new();
    for record in &records {
        if let Some((total, line, sha)) = top_scorer_entry(record) {
            if let Some(sha) = sha {
                if !seen_sha.insert(sha) {
                    continue;
                }
            }
            scorers.push((total, line));
        }
    }
    scorers.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    let top: Vec<String> = scorers.into_iter().take(3).map(|(_, line)| line).collect();

    // V6 (review-10): mechanism-family histogram over promotable top scorers — the monoculture
    // diagnostic. 49/49 identical μ0 ideas in V5; the directive pushes the NEXT proposal out of
    // the dominant family.
    let mut families: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for r in &records {
        if r.get("disqualified").and_then(Value::as_bool) != Some(false) {
            continue;
        }
        let fam = mechanism_family(r);
        *families.entry(fam).or_insert(0) += 1;
    }

    // DO/DON'T synthesis: a static base set (lessons already paid for), plus histogram-driven
    // lines for the failure classes this corpus actually exhibits.
    let mut dont_lines = vec![
        "certify h0_from_h or flat_universe_omega_lambda for rigor — definitions earn 0 rigor \
         and no distinctness"
            .to_string(),
        "propose geff_over_g > 1 (enhanced growth) — it worsens the fσ8/S8 fit; the data prefer \
         suppression"
            .to_string(),
    ];
    let mut do_lines = vec![
        "supply the byte content of EVERY cited evidence path in the evidence map".to_string(),
        "make the background implement your certified modification (the engine binds and \
         computes it — conflicts are kills)"
            .to_string(),
    ];
    let has_class = |class: &str| kill_histogram.iter().any(|(c, _)| c == class);
    if has_class("unknown_relation") {
        dont_lines.push("invent relation names — only registry relations verify".to_string());
    }
    if has_class("parse_error") {
        do_lines.push("emit ONE valid JSON object, all magnitudes as numbers".to_string());
    }
    if has_class("failed_derivation_certificate") {
        dont_lines.push(
            "certify a relation unless your inputs reproduce the engine's calculation within \
             tolerance — the engine re-derives from your inputs and kills on mismatch"
                .to_string(),
        );
    }
    if has_class("fabricated_novel_prediction") {
        dont_lines.push(
            "claim a scored observable (fsigma8, S8, BAO, H0) as a novel pre-registered \
             prediction — novel predictions must reference the sealed forecast registry and \
             concern future data releases the engine has not yet seen"
                .to_string(),
        );
    }
    if has_class("data_fit_gate_failed") {
        dont_lines.push(
            "propose a theory that substantially worsens the data fit relative to ΛCDM — \
             the data-fit gate kills it before scoring; your mechanism must help at least one \
             observable without worsening the others"
                .to_string(),
        );
    }
    if has_class("screening_implausible") {
        dont_lines.push(
            "declare vainshtein screening without |alpha_B| >= 0.01 — use chameleon \
             (density-threshold) instead, which is always structurally plausible"
                .to_string(),
        );
    }
    if has_class("hidden_knob") {
        dont_lines.push(
            "change any background parameter (w0, mu0, omega_rc, fr_*, alpha_*, beta) without a \
             DerivedCertificate in a derived_certified parameter — every off-ΛCDM dial MUST be \
             wrapped in a certificate linking it to a registry relation"
                .to_string(),
        );
    }
    if has_class("unexplained_modification") {
        dont_lines.push(
            "set background MG fields (mu0, ndgp_omega_rc, fr_fR0, mg_family) directly in the \
             theory without a matching derived_certified parameter whose certificate input equals \
             that value — unexplained modifications are automatic kills"
                .to_string(),
        );
    }
    if has_class("conflicting_modification") {
        dont_lines.push(
            "declare a background value that conflicts with what the registry relation computes \
             from your certificate inputs — the engine re-derives and kills on mismatch; make \
             background values CONSISTENT with your certificate's expected output"
                .to_string(),
        );
    }
    if has_class("unimplemented_modification") {
        dont_lines.push(
            "certify a modification that the engine cannot integrate (e.g. ndgp_geff_over_g with \
             beta <= 0, or fr_largescale_geff_over_g with a non-binary regime, or any relation \
             with out-of-domain inputs) — check domain constraints in rule #2 before certifying"
                .to_string(),
        );
    }
    // The diversity directive: when one family dominates the promotable pool, push outward.
    if let Some((dominant, count)) = families.iter().max_by_key(|(_, c)| **c) {
        let total: usize = families.values().sum();
        if total >= 3 && *count * 2 > total {
            do_lines.push(format!(
                "propose a mechanism OUTSIDE the `{dominant}` family ({count}/{total} of \
                 promotable proposals already are `{dominant}`)"
            ));
        }
    }

    ProposerMemory {
        top,
        kill_histogram,
        do_lines,
        dont_lines,
    }
}

/// Render the memory as a prompt section capped at `budget` bytes. Truncation drops whole lines
/// from the end, never mid-line, so the rendered section is always valid prose. Sections in
/// order: TOP SCORERS, COMMON KILLS (top 6 with counts), DO, DON'T.
pub(crate) fn render_memory_section(memory: &ProposerMemory, budget: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("## MEMORY — prior oracle adjudications (deterministic facts)".to_string());
    if !memory.top.is_empty() {
        lines.push("TOP SCORERS:".to_string());
        for entry in &memory.top {
            lines.push(format!("  {entry}"));
        }
    }
    if !memory.kill_histogram.is_empty() {
        lines.push("COMMON KILLS:".to_string());
        for (class, n) in memory.kill_histogram.iter().take(6) {
            lines.push(format!("  {class} ×{n}"));
        }
    }
    for line in &memory.do_lines {
        lines.push(format!("DO: {line}"));
    }
    for line in &memory.dont_lines {
        lines.push(format!("DON'T: {line}"));
    }

    let mut out = String::new();
    for line in lines {
        if out.len() + line.len() + 1 > budget {
            break;
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record(id: &str, value: f64, uncertainty: f64, kind: &str) -> ObservableRecord {
        ObservableRecord {
            observable_id: id.into(),
            kind: kind.into(),
            value,
            uncertainty,
            unit: "dimensionless".into(),
            source: None,
        }
    }

    fn brief_observables() -> Vec<ObservableRecord> {
        vec![
            record("h0", 73.04, 1.04, "distance_ladder"),
            record("omega_m", 0.30, 0.01, "cosmology"),
            record("mystery_observable", 1.0, 0.1, "cosmology"),
        ]
    }

    #[test]
    fn data_brief_reports_real_pulls_and_marks_the_largest_tension() {
        let brief = build_data_brief(&brief_observables());
        assert!(brief.starts_with(
            "## DATA BRIEF — real data vs the ΛCDM baseline (computed, not opinions)"
        ));
        assert!(brief.contains("73.04"), "data value must appear: {brief}");
        // (73.04 - 67.40) / 1.04 = +5.4σ against the Planck baseline.
        assert!(brief.contains("+5.4σ"), "real H0 pull must appear: {brief}");
        let h0_line = brief
            .lines()
            .find(|l| l.trim_start().starts_with("h0:"))
            .expect("an h0 line");
        assert!(
            h0_line.contains("LARGEST TENSION"),
            "h0 is the largest pull here: {h0_line}"
        );
        // omega_m pulls the other way and is smaller — it must not carry the marker.
        let om_line = brief
            .lines()
            .find(|l| l.trim_start().starts_with("omega_m:"))
            .expect("an omega_m line");
        assert!(!om_line.contains("LARGEST TENSION"));
        assert!(om_line.contains("-1.5σ"), "{om_line}");
        // The unpredictable id is an explicit coverage gap, never a faked prediction.
        assert!(brief.contains("(not predicted by the background model — coverage gap)"));
        assert!(brief.contains("mystery_observable"));
        assert!(brief.contains(GOAL_LINE));
    }

    #[test]
    fn data_brief_is_deterministic_byte_equal() {
        let obs = brief_observables();
        assert_eq!(build_data_brief(&obs), build_data_brief(&obs));
    }

    #[test]
    fn data_brief_emits_the_growth_note_only_when_growth_pulls_are_negative() {
        // fσ8 data BELOW the ΛCDM prediction (the real S8-style tension) ⇒ NOTE present.
        let mut obs = brief_observables();
        obs.push(record("fsigma8@0.51", 0.40, 0.04, "growth"));
        let brief = build_data_brief(&obs);
        assert!(brief.contains(GROWTH_NOTE_LINE), "{brief}");

        // No growth observables ⇒ no NOTE.
        let plain = build_data_brief(&brief_observables());
        assert!(!plain.contains("NOTE:"), "{plain}");

        // Growth data ABOVE the prediction (positive pull) ⇒ no NOTE either.
        let mut high = brief_observables();
        high.push(record("fsigma8@0.51", 0.90, 0.04, "growth"));
        let brief_high = build_data_brief(&high);
        assert!(!brief_high.contains("NOTE:"), "{brief_high}");
    }

    #[test]
    fn data_brief_guards_a_non_positive_uncertainty_with_na() {
        let obs = vec![record("h0", 73.04, 0.0, "distance_ladder")];
        let brief = build_data_brief(&obs);
        assert!(brief.contains("pull n/a"), "{brief}");
        assert!(!brief.contains('σ'), "no fake pull may be printed: {brief}");
    }

    #[test]
    fn data_brief_stays_compact_for_a_realistic_observable_count() {
        // ~22 observables with realistic id lengths must stay near the ~1.2 KB target.
        let ids = [
            "h0",
            "omega_m",
            "omega_b_h2",
            "n_eff",
            "sum_mnu",
            "r_drag",
            "bbn_yp",
            "cmb_R",
            "cmb_lA",
            "cmb_omega_b_h2",
            "s8",
            "sigma8",
            "fsigma8@0.38",
            "fsigma8@0.51",
            "fsigma8@0.61",
            "dv_over_rd@0.15",
            "dm_over_rd@0.38",
            "dh_over_rd@0.38",
            "dm_over_rd@0.51",
            "dh_over_rd@0.51",
            "mu@0.30",
            "mu@0.50",
        ];
        let obs: Vec<ObservableRecord> = ids
            .iter()
            .enumerate()
            .map(|(i, id)| record(id, 1.0 + i as f64 * 0.1, 0.05, "cosmology"))
            .collect();
        assert_eq!(obs.len(), 22);
        let brief = build_data_brief(&obs);
        assert!(
            brief.len() < 1600,
            "brief must stay compact, got {} bytes",
            brief.len()
        );
    }

    // ---- memory assembly ----

    fn ledger_line(total: f64, disq: bool, theory_id: &str, generation: u64, sha: &str) -> String {
        json!({
            "generation": generation,
            "theory_id": theory_id,
            "proposal_sha256": sha,
            "disqualified": disq,
            "total": total,
            "distinct_from_baseline": true,
            "doc": {}
        })
        .to_string()
    }

    fn seeded_runs_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let run_a = dir.path().join("run-a");
        fs::create_dir_all(&run_a).unwrap();
        let ledger = [
            ledger_line(61.3, false, "openqg-v4-ndgp-tab-1", 1320, "sha-a"),
            ledger_line(55.0, false, "openqg-v4-fr-tab-2", 900, "sha-b"),
            ledger_line(40.0, false, "openqg-v4-mu0-tab-3", 12, "sha-c"),
            json!({
                "generation": 13,
                "theory_id": "openqg-v4-dead",
                "proposal_sha256": "sha-d",
                "disqualified": true,
                "total": 0.0,
                "distinct_from_baseline": false,
                "doc": {},
                "kill_reasons": ["certificate outcome UnknownRelation for foo_relation"]
            })
            .to_string(),
        ]
        .join("\n");
        fs::write(run_a.join("proposal-ledger.jsonl"), ledger).unwrap();
        // run-a deliberately has NO proposal-attempts.jsonl — that absence must be tolerated.

        let run_b = dir.path().join("run-b");
        fs::create_dir_all(&run_b).unwrap();
        let attempts = [
            json!({ "outcome": "parse_error", "generation": 5 }).to_string(),
            json!({
                "outcome": "killed",
                "kill_reasons": [
                    "evidence path derivations/x.txt not bound",
                    "claims unification but fails no_hidden_knob_test"
                ]
            })
            .to_string(),
        ]
        .join("\n");
        fs::write(run_b.join("proposal-attempts.jsonl"), attempts).unwrap();
        dir
    }

    fn current_records() -> Vec<Value> {
        vec![
            json!({
                "generation": 7,
                "theory_id": "openqg-v4-current-best",
                "proposal_sha256": "sha-live",
                "disqualified": false,
                "total": 70.0,
                "distinct_from_baseline": true,
                "doc": {}
            }),
            json!({
                "disqualified": true,
                "total": 0.0,
                "kill_reasons": ["veto FreeParameter: w_fit"]
            }),
        ]
    }

    #[test]
    fn memory_classifies_kills_and_ranks_the_top_three() {
        let root = seeded_runs_root();
        let memory = assemble_memory(root.path(), &current_records());

        // Top 3 by total, best first, mixing ledger + current records.
        assert_eq!(memory.top.len(), 3);
        assert_eq!(
            memory.top[0],
            "70.0 distinct=true openqg-v4-current-best (gen 7)"
        );
        assert_eq!(
            memory.top[1],
            "61.3 distinct=true openqg-v4-ndgp-tab-1 (gen 1320)"
        );
        assert!(memory.top[2].starts_with("55.0 "), "{:?}", memory.top);

        // Every seeded kill class lands in its stable bucket.
        let count = |class: &str| {
            memory
                .kill_histogram
                .iter()
                .find(|(c, _)| c == class)
                .map(|(_, n)| *n)
        };
        assert_eq!(count("unknown_relation"), Some(1));
        assert_eq!(count("parse_error"), Some(1));
        assert_eq!(count("unbound_evidence"), Some(1));
        assert_eq!(count("hidden_knob"), Some(1));
        assert_eq!(count("free_parameter"), Some(1));
    }

    #[test]
    fn memory_synthesizes_static_and_histogram_driven_directives() {
        let root = seeded_runs_root();
        let memory = assemble_memory(root.path(), &current_records());

        // Histogram-driven lines appear because their classes were seeded.
        assert!(memory
            .dont_lines
            .iter()
            .any(|l| l.contains("invent relation names")));
        assert!(memory
            .do_lines
            .iter()
            .any(|l| l.contains("ONE valid JSON object")));

        // The static base set is ALWAYS present — even from an empty corpus.
        let empty_root = tempfile::tempdir().unwrap();
        let empty = assemble_memory(empty_root.path(), &[]);
        assert!(empty.top.is_empty());
        assert!(empty.kill_histogram.is_empty());
        assert!(empty
            .dont_lines
            .iter()
            .any(|l| l.contains("h0_from_h or flat_universe_omega_lambda")));
        assert!(empty
            .dont_lines
            .iter()
            .any(|l| l.contains("geff_over_g > 1")));
        assert!(empty
            .do_lines
            .iter()
            .any(|l| l.contains("byte content of EVERY cited evidence path")));
        assert!(empty
            .do_lines
            .iter()
            .any(|l| l.contains("background implement your certified modification")));
        // ...but the histogram-driven lines do NOT appear without their classes.
        assert!(!empty
            .dont_lines
            .iter()
            .any(|l| l.contains("invent relation names")));
        assert!(!empty
            .do_lines
            .iter()
            .any(|l| l.contains("ONE valid JSON object")));
    }

    #[test]
    fn memory_assembly_is_deterministic() {
        let root = seeded_runs_root();
        let records = current_records();
        let a = assemble_memory(root.path(), &records);
        let b = assemble_memory(root.path(), &records);
        assert_eq!(a, b);
        assert_eq!(
            render_memory_section(&a, 4096),
            render_memory_section(&b, 4096)
        );
    }

    #[test]
    fn rendered_memory_respects_the_byte_budget_on_whole_lines() {
        let root = seeded_runs_root();
        let memory = assemble_memory(root.path(), &current_records());

        let full = render_memory_section(&memory, usize::MAX);
        assert!(full.starts_with("## MEMORY — prior oracle adjudications (deterministic facts)"));
        assert!(full.contains("TOP SCORERS:"));
        assert!(full.contains("COMMON KILLS:"));
        assert!(full.contains("DO: "));
        assert!(full.contains("DON'T: "));

        let capped = render_memory_section(&memory, 1536);
        assert!(capped.len() <= 1536, "got {} bytes", capped.len());

        // A tight budget still never cuts mid-line: every rendered line is a whole line of the
        // full render, and the output stays newline-terminated.
        let tight = render_memory_section(&memory, 200);
        assert!(tight.len() <= 200);
        assert!(tight.ends_with('\n'));
        for line in tight.lines() {
            assert!(
                full.lines().any(|f| f == line),
                "truncated mid-line: {line:?}"
            );
        }
    }

    #[test]
    fn blinded_policy_contains_no_pull_values() {
        let obs = brief_observables();
        let brief = build_data_brief_with_policy(&obs, ProposerBriefPolicy::BlindedToTensions);
        assert!(
            brief.contains("BLINDED"),
            "blinded brief must announce the firewall: {brief}"
        );
        assert!(
            !brief.contains("σ"),
            "no pull sigma markers may appear in blinded brief: {brief}"
        );
        assert!(
            !brief.contains("LARGEST TENSION"),
            "no tension markers in blinded brief: {brief}"
        );
        assert!(
            brief.contains("N_observables"),
            "count of observables must be reported: {brief}"
        );
    }

    #[test]
    fn summary_policy_reports_count_not_individual_pulls() {
        let obs = brief_observables();
        let brief = build_data_brief_with_policy(&obs, ProposerBriefPolicy::SummaryOnly);
        assert!(
            brief.contains("summary mode"),
            "summary brief header expected: {brief}"
        );
        assert!(
            !brief.contains("LARGEST TENSION"),
            "no individual tension markers in summary mode: {brief}"
        );
    }

    #[test]
    fn full_pulls_policy_matches_build_data_brief() {
        let obs = brief_observables();
        assert_eq!(
            build_data_brief_with_policy(&obs, ProposerBriefPolicy::FullPulls),
            build_data_brief(&obs),
            "FullPulls policy must be identical to build_data_brief"
        );
    }

    #[test]
    fn kill_histogram_is_sorted_by_count_then_class() {
        let dir = tempfile::tempdir().unwrap();
        let records = vec![
            json!({ "kill_reasons": ["UnknownRelation a", "UnknownRelation b"] }),
            json!({ "kill_reasons": ["veto FreeParameter: x"] }),
            json!({ "kill_reasons": ["something entirely new"] }),
        ];
        let memory = assemble_memory(dir.path(), &records);
        assert_eq!(
            memory.kill_histogram[0],
            ("unknown_relation".to_string(), 2)
        );
        // The 1-count classes tie and fall back to name order.
        assert_eq!(memory.kill_histogram[1].0, "free_parameter");
        assert_eq!(memory.kill_histogram[2].0, "other");
    }
}
