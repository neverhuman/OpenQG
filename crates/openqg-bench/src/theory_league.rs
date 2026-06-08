//! CLI handler: the fair model-selection **league**. Where `theory evolve` is a search heuristic,
//! this command is the rigorous adjudicator — it profile-fits every model class to the *same* data
//! (the baseline ΛCDM re-fit, not held fixed) under the covariance-aware likelihood, and ranks them
//! by ΔAIC / Δln-evidence with a complexity penalty. This is what replaces the indefensible
//! "+36.7 log-L beats ΛCDM" comparison with a number a referee accepts
//! (see `docs/zyal-next-level-design.md` §1.4, §3).

use anyhow::{Context, Result};
use openqg_core::cosmology::BackgroundForwardModel;
use openqg_core::scoring::{CovarianceBlock, LikelihoodData};
use openqg_core::theory::{model_league, LeagueRow, ModelClass};
use openqg_core::ObservableRecord;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// The model classes scoreable on the given data. When growth observables (fσ8 / S8) are present
/// the growth-aware variants are used — σ8 becomes a fitted parameter and the screened modified-
/// gravity class (free μ0) joins the league. On geometry-only data the background variants are
/// used so σ8 is not an unconstrained nuisance inflating the parameter count. The early-dark-energy
/// and coupled-dark-energy classes (`docs/zyal-next-level-design.md` §3.3, §3.5) await the
/// scalar-field / coupling sectors and are deliberately NOT faked here.
fn available_models(has_growth: bool) -> Vec<ModelClass> {
    if has_growth {
        // v3.0.0 M4: the pre-registered growth-scoreable league. lcdm/w0waCDM are the standard
        // null + CPL; screened_mg is the leading-order μ0 handle; f_r and nDGP are the genuinely
        // DERIVED modified-gravity families (scale-dependent μ(a,k) from one fundamental parameter).
        // EDE and coupled-DE remain triage-only (await the scalar-field / Boltzmann sectors in v3.1).
        vec![
            ModelClass::lcdm_growth(),
            ModelClass::w0wa_cdm_growth(),
            ModelClass::screened_mg(),
            ModelClass::f_r(),
            ModelClass::ndgp(),
        ]
    } else {
        vec![ModelClass::lcdm(), ModelClass::w_cdm(), ModelClass::w0wa_cdm()]
    }
}

/// Does the dataset contain growth-of-structure observables (fσ8 / S8 / σ8)?
fn has_growth_observables(obs: &[ObservableRecord]) -> bool {
    obs.iter().any(|o| {
        let id = o.observable_id.as_str();
        id.starts_with("fsigma8") || id == "s8" || id == "S8" || id == "sigma8"
    })
}

fn load_observables(paths: &[std::path::PathBuf]) -> Result<Vec<ObservableRecord>> {
    let mut out = Vec::new();
    for path in paths {
        let text = fs::read_to_string(path)
            .with_context(|| format!("read observables {}", path.display()))?;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let rec: ObservableRecord = serde_json::from_str(line)
                .with_context(|| format!("parse observable in {}", path.display()))?;
            out.push(rec);
        }
    }
    Ok(out)
}

fn load_covariance(path: Option<&Path>) -> Result<Vec<CovarianceBlock>> {
    match path {
        None => Ok(Vec::new()),
        Some(p) => {
            let text =
                fs::read_to_string(p).with_context(|| format!("read covariance {}", p.display()))?;
            let blocks: Vec<CovarianceBlock> =
                serde_json::from_str(&text).with_context(|| format!("parse covariance {}", p.display()))?;
            Ok(blocks)
        }
    }
}

fn row_json(r: &LeagueRow) -> Value {
    json!({
        "model_id": r.fit.model_id,
        "description": r.fit.description,
        "k": r.fit.k,
        "n_data": r.fit.n_data,
        "chi2": r.fit.chi2,
        "log_likelihood": r.fit.log_likelihood,
        "aic": r.fit.aic,
        "bic": r.fit.bic,
        "coverage": r.fit.coverage,
        // v3.0.0 M2: a best-fit parameter pinned to its prior bound (interval/evidence suspect).
        "boundary_hit": r.fit.boundary_hit,
        "eligible": r.eligible,
        "delta_aic": r.delta_aic,
        "delta_bic": r.delta_bic,
        "delta_ln_evidence": r.delta_ln_evidence,
        "best_params": r.fit.best_params.iter()
            .map(|(n, v)| json!({"name": n, "value": v})).collect::<Vec<_>>(),
    })
}

/// Run the model-selection league and write a JSON + Markdown artifact.
#[allow(clippy::too_many_arguments)]
pub fn run_league(
    observables: &[std::path::PathBuf],
    covariance: Option<&Path>,
    reference: &str,
    output: &Path,
) -> Result<()> {
    let obs = load_observables(observables)?;
    if obs.is_empty() {
        anyhow::bail!("no observables loaded");
    }
    let blocks = load_covariance(covariance)?;
    let data = LikelihoodData {
        observables: obs,
        blocks,
    };

    let models = available_models(has_growth_observables(&data.observables));
    let rows = model_league(&models, &data, &BackgroundForwardModel, reference);

    // Console summary.
    println!(
        "fair model-selection league  (n={} observables, reference={}, {} blocks)",
        data.observables.len(),
        reference,
        data.blocks.len()
    );
    println!(
        "{:<11} {:>2} {:>9} {:>9} {:>9} {:>9} {:>9}  {}",
        "model", "k", "chi2", "AIC", "dAIC", "dBIC", "dlnZ", "elig"
    );
    for r in &rows {
        println!(
            "{:<11} {:>2} {:>9.3} {:>9.3} {:>+9.3} {:>+9.3} {:>+9.3}  {}",
            r.fit.model_id, r.fit.k, r.fit.chi2, r.fit.aic, r.delta_aic, r.delta_bic,
            r.delta_ln_evidence,
            if r.eligible { "yes" } else { "NO (coverage)" }
        );
    }

    let artifact = json!({
        "engine": "openqg-core/theory-league",
        "reference_model": reference,
        "observable_count": data.observables.len(),
        "covariance_blocks": data.blocks.len(),
        "observable_sources": observables.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "ranking": rows.iter().map(row_json).collect::<Vec<_>>(),
    });

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(output, serde_json::to_string_pretty(&artifact)?)
        .with_context(|| format!("write league artifact {}", output.display()))?;
    println!("-> {}", output.display());
    Ok(())
}
