//! V4 M8: the white-paper generator.
//!
//! Turns a completed population run into a defensible report on the top candidate: the champion
//! theory and its parameters, its full per-dimension critic-proofness scorecard, its ranking against
//! the human-baseline contenders on the *identical* rubric, its lineage, the trust-gate status, and
//! an explicit honesty section. Deterministic and reproducible from the seed (no LLM). Emits
//! `white-paper.md` (human) + `white-paper.json` (machine).

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Value};

use openqg_core::theory::{human_contenders, score_contender, Provenance, ScorecardV4};

use super::physics_score::final_score_unit;
use super::run_population::load_observables;
use super::theory_population::{evolve_population, EvolveConfig, Individual};
use super::trust_gate::evaluate_trust_gate;

fn provenance_label(p: &Provenance) -> &'static str {
    match p {
        Provenance::Fundamental => "fundamental",
        Provenance::Derived {
            certificate: Some(_),
            ..
        } => "derived (certified)",
        Provenance::Derived { .. } => "derived (uncertified)",
        Provenance::Free => "free",
    }
}

fn component_points(sc: &ScorecardV4, name: &str) -> f64 {
    sc.components
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.points)
        .unwrap_or(0.0)
}

/// One row of the same-rubric ranking.
struct RankRow {
    entrant: String,
    total: f64,
    disqualified: bool,
    distinct: bool,
    derivation: f64,
    data_fit: f64,
    novelty: f64,
    unification: f64,
    robustness: f64,
    parsimony: f64,
}

fn rank_row(entrant: &str, sc: &ScorecardV4) -> RankRow {
    RankRow {
        entrant: entrant.to_string(),
        total: sc.total,
        disqualified: sc.disqualified,
        distinct: sc.distinct_from_baseline,
        derivation: component_points(sc, "derivation_rigor"),
        data_fit: component_points(sc, "data_fit"),
        novelty: component_points(sc, "novel_prediction"),
        unification: component_points(sc, "unification"),
        robustness: component_points(sc, "robustness_under_judge"),
        parsimony: component_points(sc, "parsimony"),
    }
}

/// Generate the white paper for the champion of a population run. Returns the run dir.
pub(crate) fn generate_whitepaper(
    observables_path: &Path,
    output_root: &Path,
    config: EvolveConfig,
    run_id: &str,
    proposer: Option<&dyn super::proposer::Proposer>,
) -> Result<PathBuf> {
    let observables = load_observables(observables_path)?;
    anyhow::ensure!(!observables.is_empty(), "no observables loaded");
    let n_obs = observables.len();
    let used_proposer = proposer.is_some();

    let run_dir_early = output_root.join("runs").join(run_id);
    let mut sink = super::ledger_sink::RunDirSink::create(&run_dir_early, 25)?;
    let run = evolve_population(&config, &observables, proposer, &mut sink);
    drop(sink);
    let champion: Individual = run
        .best
        .clone()
        .context("no non-disqualified champion produced; cannot write a white paper")?;
    let sc = champion.scorecard.clone();

    // Same-rubric ranking: champion + human contenders.
    let mut rows = vec![rank_row("openqg-v4 (evolved champion)", &sc)];
    let humans = human_contenders();
    for (c, _) in &humans.entries {
        let hsc = score_contender(c, &humans.store);
        rows.push(rank_row(&format!("human: {}", c.name), &hsc));
    }
    rows.sort_by(|a, b| {
        b.total
            .partial_cmp(&a.total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let trust = evaluate_trust_gate(&config, observables_path)?;

    // --- white-paper.md ---
    let mut md = String::new();
    let _ = writeln!(md, "# OpenQG · ZYAL V4 — Candidate Theory Report\n");
    let _ = writeln!(
        md,
        "**Engine:** `theory_population.v4` · deterministic · seed `{}` · {} generations · population {}\n",
        config.seed, config.max_generations, config.population_size
    );
    let _ = writeln!(
        md,
        "**Trust gate:** {} (decoy false-positive rate {:.3}; humans survive {}; progress {}; deterministic {}/{})\n",
        if trust.passed { "**PASSED** — campaign unblocked" } else { "FAILED — campaign blocked" },
        trust.decoy_false_positive_rate,
        trust.humans_survive,
        trust.population_progress_ok,
        trust.evolution_deterministic,
        trust.scoring_deterministic,
    );

    let _ = writeln!(md, "## Champion — `{}`\n", champion.theory.id);
    let _ = writeln!(
        md,
        "- **Critic-proofness score:** {:.1} / 100  (band {:.1}–{:.1})",
        sc.total, sc.total_band.0, sc.total_band.1
    );
    let _ = writeln!(md, "- **Disqualified:** {}", sc.disqualified);
    let _ = writeln!(md, "- **Claim fingerprint:** `{}`", champion.fingerprint);
    let _ = writeln!(
        md,
        "- **Lineage:** generation {}, island `{}`, parents `{:?}`\n",
        champion.generation, champion.island, champion.parent_ids
    );

    let _ = writeln!(md, "### Per-dimension scorecard\n");
    let _ = writeln!(md, "| Dimension | Weight | Points |");
    let _ = writeln!(md, "|---|---:|---:|");
    for c in &sc.components {
        let _ = writeln!(md, "| {} | {:.0} | {:.1} |", c.name, c.weight, c.points);
    }
    let _ = writeln!(md);

    let _ = writeln!(md, "### Theory parameters\n");
    let _ = writeln!(md, "| Symbol | Value | Provenance | Meaning |");
    let _ = writeln!(md, "|---|---:|---|---|");
    for p in &champion.theory.parameters {
        let _ = writeln!(
            md,
            "| `{}` | {} | {} | {} |",
            p.symbol,
            p.value,
            provenance_label(&p.provenance),
            p.physical_meaning
        );
    }
    let _ = writeln!(md);

    let _ = writeln!(
        md,
        "## Ranking vs contenders (identical rubric)\n\n_The human entrants are **ΛCDM-recovering \
         baselines** (string/M-theory, LQG, … make no distinct low-energy prediction); \
         `real_modification_program` is a genuine nDGP modification. `Dist` = makes a physical \
         departure from ΛCDM._\n"
    );
    let _ = writeln!(md, "| Rank | Entrant | Total | Dist | Derivation | DataFit | Novelty | Unification | Robustness | Parsimony | DQ |");
    let _ = writeln!(
        md,
        "|---:|---|---:|:--:|---:|---:|---:|---:|---:|---:|:--:|"
    );
    for (i, r) in rows.iter().enumerate() {
        let _ = writeln!(
            md,
            "| {} | {} | {:.1} | {} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} | {} |",
            i + 1,
            r.entrant,
            r.total,
            if r.distinct { "✓" } else { "—" },
            r.derivation,
            r.data_fit,
            r.novelty,
            r.unification,
            r.robustness,
            r.parsimony,
            if r.disqualified { "✗" } else { "" }
        );
    }
    let _ = writeln!(md);

    let _ = writeln!(md, "## Honesty & reproducibility\n");
    if used_proposer {
        let _ = writeln!(
            md,
            "- **Replayable (engine + scoring deterministic; proposals content-pinned).** The LLM \
             proposals are non-deterministic, so they are recorded verbatim and hashed in \
             `proposal-ledger.jsonl`; the run re-scores from that ledger **without re-calling the LLM** \
             (`zyal genome replay --ledger proposal-ledger.jsonl`). This is NOT a from-seed replay — \
             a fresh LLM call would propose something different."
        );
    } else {
        let _ = writeln!(
            md,
            "- Deterministic: same seed reproduces this champion; the score replays without the LLM."
        );
    }
    let _ = writeln!(
        md,
        "- Distinct from baseline: **{}** (`false` ⇒ observationally a ΛCDM rediscovery, however \
         well-certified). Champion derivations/evidence are materialized under `evidence/`.",
        sc.distinct_from_baseline
    );
    let _ = writeln!(
        md,
        "- Data fit computed on {n_obs} observables from `{}`.",
        observables_path.display()
    );
    if used_proposer {
        let _ = writeln!(
            md,
            "- This run used the **LLM-proposer path**: the champion may carry verified derivations \
             + a unification claim, so `derivation_rigor`/`unification` reflect *proven* claims \
             (every derivation was re-checked by the deterministic oracle — the LLM never scored)."
        );
    } else {
        let _ = writeln!(
            md,
            "- This run used **pure parameter evolution** (no LLM-attached derivations), so \
             `derivation_rigor` and `unification` reflect that — those dimensions are earned only by \
             verified derivations, which the proposer path supplies. The champion's merit here is \
             data fit + parsimony + physical sanity; it is **not** yet a critic-proof unified theory."
        );
    }
    let _ = writeln!(
        md,
        "- A candidate is only critic-proof when it clears all gates AND earns derivation + \
         unification credit; this report states exactly where the champion stands on each dimension."
    );

    // --- write artifacts ---
    let run_dir = output_root.join("runs").join(run_id);
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("create run dir {}", run_dir.display()))?;
    fs::write(run_dir.join("white-paper.md"), &md)?;

    // (proposal-ledger.jsonl and progress-ledger.jsonl are STREAMED by the RunDirSink above.)
    // Materialize the champion's cited evidence to disk (path-hygiene: relative, no `..`).
    if let Some(champ) = run
        .live_proposals
        .iter()
        .find(|r| r.generation == champion.generation && r.theory_id == champion.theory.id)
    {
        if let Some(ev) = champ.doc.get("evidence").and_then(Value::as_object) {
            for (path, content) in ev {
                if path.contains("..") || std::path::Path::new(path).is_absolute() {
                    continue;
                }
                let dest = run_dir.join("evidence").join(path);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(dest, content.as_str().unwrap_or_default())?;
            }
        }
    }

    let json_doc = json!({
        "record_kind": "white_paper",
        "engine": "theory_population.v5",
        "config": { "seed": config.seed, "max_generations": config.max_generations, "population_size": config.population_size },
        "trust_gate": {
            "passed": trust.passed,
            "decoy_false_positive_rate": trust.decoy_false_positive_rate,
            "humans_survive": trust.humans_survive,
            "population_progress_ok": trust.population_progress_ok,
            "evolution_deterministic": trust.evolution_deterministic,
            "scoring_deterministic": trust.scoring_deterministic,
        },
        "champion": {
            "id": champion.theory.id,
            "fingerprint": champion.fingerprint,
            "final_score_unit": final_score_unit(&sc),
            "distinct_from_baseline": sc.distinct_from_baseline,
            "generation": champion.generation,
            "parent_ids": champion.parent_ids,
            "theory": serde_json::to_value(&champion.theory).unwrap_or(Value::Null),
            "scorecard": serde_json::to_value(&sc).unwrap_or(Value::Null),
        },
        "live_proposal_count": run.live_proposals.len(),
        "audit_trail": {
            "proposal_ledger": "proposal-ledger.jsonl",
            "progress_ledger": "progress-ledger.jsonl",
            "evidence_dir": "evidence/",
            "replay": "zyal genome replay --ledger proposal-ledger.jsonl --observables <obs>",
        },
        "ranking": rows.iter().map(|r| json!({
            "entrant": r.entrant, "total": r.total, "disqualified": r.disqualified,
            "distinct_from_baseline": r.distinct,
            "derivation_rigor": r.derivation, "data_fit": r.data_fit,
            "novel_prediction": r.novelty, "unification": r.unification,
            "robustness_under_judge": r.robustness, "parsimony": r.parsimony,
        })).collect::<Vec<_>>(),
        "observables_count": n_obs,
    });
    fs::write(
        run_dir.join("white-paper.json"),
        serde_json::to_string_pretty(&json_doc)?,
    )?;

    Ok(run_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_obs(dir: &Path) -> PathBuf {
        let path = dir.join("obs.jsonl");
        let lines = [
            r#"{"observable_id":"bao_dv_z038","kind":"cosmology","value":1.0,"uncertainty":0.05,"unit":"x"}"#,
            r#"{"observable_id":"bao_dv_z051","kind":"cosmology","value":1.1,"uncertainty":0.05,"unit":"x"}"#,
            r#"{"observable_id":"fsigma8_z038","kind":"cosmology","value":0.45,"uncertainty":0.03,"unit":"x"}"#,
            r#"{"observable_id":"fsigma8_z051","kind":"cosmology","value":0.46,"uncertainty":0.03,"unit":"x"}"#,
        ];
        fs::write(&path, lines.join("\n")).unwrap();
        path
    }

    #[test]
    fn whitepaper_is_generated_with_champion_and_ranking() {
        let tmp = std::env::temp_dir().join(format!("openqg-v4-wp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let obs = write_obs(&tmp);
        let cfg = EvolveConfig {
            population_size: 9,
            max_generations: 5,
            seed: 555,
        };
        let run_dir = generate_whitepaper(&obs, &tmp, cfg, "wp-test", None).unwrap();

        let md = fs::read_to_string(run_dir.join("white-paper.md")).unwrap();
        assert!(md.contains("Candidate Theory Report"));
        assert!(md.contains("Per-dimension scorecard"));
        assert!(md.contains("Ranking vs contenders"));
        assert!(
            md.contains("Novelty"),
            "ranking must show the novelty dimension"
        );
        assert!(md.contains("evolved champion"));
        assert!(
            md.contains("pure parameter evolution"),
            "must include the honesty caveat"
        );

        let doc: Value =
            serde_json::from_str(&fs::read_to_string(run_dir.join("white-paper.json")).unwrap())
                .unwrap();
        assert_eq!(doc["record_kind"], "white_paper");
        assert!(doc["champion"]["scorecard"]["total"].is_number());
        assert!(doc["ranking"].as_array().unwrap().len() >= 2); // champion + >=1 human
        assert_eq!(doc["trust_gate"]["passed"], Value::Bool(true));

        let _ = fs::remove_dir_all(&tmp);
    }
}
