//! V4 M7: the TRUST GATE — the single artifact-checkable verdict that must pass before any
//! 1000–10000-generation campaign (M8).
//!
//! It composes the already-built, deterministic pieces into one report:
//! - **decoy calibration** (M4): every decoy is disqualified ⇒ `decoy_false_positive_rate == 0`;
//! - **human baselines survive** (M4): legitimate GR-recovering programs are not disqualified;
//! - **real evolution** (M5): a population run shows real per-generation progress
//!   (`population_progress_ok`) and a non-root champion after gen 1;
//! - **determinism**: the same seed reproduces the same champion, and a contender's scorecard
//!   replays bit-for-bit (no hidden nondeterminism in the score).
//!
//! Only when ALL pass is the score trustworthy enough to spend compute on a massive run. The gate is
//! fully deterministic (no LLM), so it is reproducible from artifacts.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::json;

use openqg_core::theory::{
    decoy_contenders, decoy_false_positive_rate, human_contenders, score_contender,
    scorecard_receipt, ExpectedVerdict, ScorecardV4,
};

use super::run_population::load_observables;
use super::theory_population::{evolve_population, population_progress_ok, EvolveConfig};

/// The trust-gate verdict + the checks behind it.
pub(crate) struct TrustGateReport {
    pub passed: bool,
    pub decoy_false_positive_rate: f64,
    pub humans_survive: bool,
    pub population_progress_ok: bool,
    pub champion_non_root: bool,
    pub evolution_deterministic: bool,
    pub scoring_deterministic: bool,
}

/// Evaluate the trust gate. Deterministic given `(config, observables)`.
pub(crate) fn evaluate_trust_gate(
    config: &EvolveConfig,
    observables_path: &Path,
) -> Result<TrustGateReport> {
    // --- M4 calibration: decoys must all die; humans must survive ---
    let decoys = decoy_contenders();
    let decoy_results: Vec<(ScorecardV4, ExpectedVerdict)> = decoys
        .entries
        .iter()
        .map(|(c, v)| (score_contender(c, &decoys.store), *v))
        .collect();
    let fpr = decoy_false_positive_rate(&decoy_results);

    let humans = human_contenders();
    let humans_survive = humans
        .entries
        .iter()
        .all(|(c, _)| !score_contender(c, &humans.store).disqualified);

    // --- scoring determinism: a contender's scorecard replays identically ---
    let scoring_deterministic = {
        let (c, _) = &humans.entries[0];
        let a = score_contender(c, &humans.store);
        let b = score_contender(c, &humans.store);
        let ra = scorecard_receipt(&a, "trust-gate-inputs");
        let rb = scorecard_receipt(&b, "trust-gate-inputs");
        a == b && ra == rb
    };

    // --- M5 real evolution: progress + non-root champion + determinism ---
    let observables = load_observables(observables_path)?;
    anyhow::ensure!(
        !observables.is_empty(),
        "no observables loaded from {}",
        observables_path.display()
    );
    let run = evolve_population(
        config,
        &observables,
        &[],
        None,
        &mut super::ledger_sink::NullSink,
    );
    let progress_ok = population_progress_ok(&run.progress);
    let champion_non_root = run
        .champions
        .iter()
        .filter(|c| c.generation > 1)
        .all(|c| !c.parent_ids.is_empty());

    let run2 = evolve_population(
        config,
        &observables,
        &[],
        None,
        &mut super::ledger_sink::NullSink,
    );
    let evolution_deterministic = run.best.as_ref().map(|b| &b.fingerprint)
        == run2.best.as_ref().map(|b| &b.fingerprint)
        && run.progress == run2.progress;

    let passed = fpr == 0.0
        && humans_survive
        && progress_ok
        && champion_non_root
        && evolution_deterministic
        && scoring_deterministic;

    Ok(TrustGateReport {
        passed,
        decoy_false_positive_rate: fpr,
        humans_survive,
        population_progress_ok: progress_ok,
        champion_non_root,
        evolution_deterministic,
        scoring_deterministic,
    })
}

/// Run the trust gate and write `trust-gate.json` under `<output_root>/runs/<run_id>/`. Returns the
/// path and whether it passed.
pub(crate) fn run_trust_gate(
    observables_path: &Path,
    output_root: &Path,
    config: EvolveConfig,
    run_id: &str,
) -> Result<(PathBuf, bool)> {
    let report = evaluate_trust_gate(&config, observables_path)?;
    let run_dir = output_root.join("runs").join(run_id);
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("create run dir {}", run_dir.display()))?;
    let value = json!({
        "record_kind": "trust_gate",
        "engine": "theory_population.v5",
        "config": { "population_size": config.population_size, "max_generations": config.max_generations, "seed": config.seed },
        "checks": {
            "decoy_false_positive_rate": report.decoy_false_positive_rate,
            "decoys_all_disqualified": report.decoy_false_positive_rate == 0.0,
            "humans_survive": report.humans_survive,
            "population_progress_ok": report.population_progress_ok,
            "champion_non_root_after_gen1": report.champion_non_root,
            "evolution_deterministic": report.evolution_deterministic,
            "scoring_deterministic": report.scoring_deterministic,
        },
        "passed": report.passed,
        "campaign_unblocked": report.passed,
    });
    fs::write(
        run_dir.join("trust-gate.json"),
        serde_json::to_string_pretty(&value)?,
    )?;
    Ok((run_dir, report.passed))
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
    fn the_trust_gate_passes_on_the_v4_engine() {
        let tmp = std::env::temp_dir().join(format!("openqg-v4-trustgate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let obs = write_obs(&tmp);
        let cfg = EvolveConfig {
            population_size: 9,
            max_generations: 5,
            seed: 2024,
        };
        let report = evaluate_trust_gate(&cfg, &obs).unwrap();
        assert_eq!(report.decoy_false_positive_rate, 0.0, "decoys must all die");
        assert!(report.humans_survive);
        assert!(report.population_progress_ok);
        assert!(report.champion_non_root);
        assert!(report.evolution_deterministic);
        assert!(report.scoring_deterministic);
        assert!(report.passed, "the v4 trust gate must pass");

        let (run_dir, passed) = run_trust_gate(&obs, &tmp, cfg, "trust-gate-test").unwrap();
        assert!(passed);
        assert!(run_dir.join("trust-gate.json").exists());
        let _ = fs::remove_dir_all(&tmp);
    }
}
