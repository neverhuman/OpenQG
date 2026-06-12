//! A [`ForwardModel`] that delegates to an external subprocess — the seam through which the
//! optional Boltzmann backend (CLASS / CAMB / hi_class, wrapped in a Python script) plugs in.
//!
//! The engine writes `{ "params": <CosmologyParams>, "observables": [<id>, ...] }` as JSON to the
//! command's stdin and reads back a JSON array of `PredictionRecord`s on stdout. This keeps the
//! heavy Python/C physics stack *out* of the Rust build (the default engine stays pure-Rust and
//! deterministic); the script is "just a command you pass," exactly like the LLM proposer hook.
//! A reference adapter lives at `tools/boltzmann_adapter.py`.
//!
//! Reproducibility: a crash/timeout/non-zero exit is an error (a pathological cosmology that makes
//! the solver fail is a lethal candidate, never a silent default) — see
//! `docs/research/forward-model-and-unification.md` §5.

use super::forward::{ForwardFailure, ForwardKind, ForwardManifest, ForwardModel};
use super::CosmologyParams;
use crate::types::PredictionRecord;
use std::io::Write;
use std::process::{Command, Stdio};

/// A forward model backed by an external command (run via `sh -c`).
pub struct SubprocessForwardModel {
    command: String,
    model_id: String,
    version: String,
}

impl SubprocessForwardModel {
    /// Build a subprocess model from a shell command whose stdin is the JSON request and whose
    /// stdout is the JSON `PredictionRecord` array.
    pub fn new(command: impl Into<String>) -> Self {
        SubprocessForwardModel {
            command: command.into(),
            model_id: "subprocess-boltzmann".into(),
            version: "0.1.0".into(),
        }
    }

    pub fn with_id(mut self, model_id: impl Into<String>, version: impl Into<String>) -> Self {
        self.model_id = model_id.into();
        self.version = version.into();
        self
    }
}

impl ForwardModel for SubprocessForwardModel {
    type Theory = CosmologyParams;

    fn predict(
        &self,
        theory: &CosmologyParams,
        observable_ids: &[String],
    ) -> Result<Vec<PredictionRecord>, ForwardFailure> {
        let request = serde_json::json!({
            "params": theory,
            "observables": observable_ids,
        })
        .to_string();

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ForwardFailure::Crash)?;
        child
            .stdin
            .take()
            .ok_or(ForwardFailure::Crash)?
            .write_all(request.as_bytes())
            .map_err(|_| ForwardFailure::Crash)?;
        let output = child
            .wait_with_output()
            .map_err(|_| ForwardFailure::Crash)?;
        if !output.status.success() {
            return Err(ForwardFailure::Crash);
        }
        serde_json::from_slice(&output.stdout).map_err(|_| ForwardFailure::Crash)
    }

    fn manifest(&self) -> ForwardManifest {
        ForwardManifest {
            model_id: self.model_id.clone(),
            version: self.version.clone(),
            kind: ForwardKind::Boltzmann,
            // External Boltzmann solver backend → T2Boltzmann (promotion-grade).
            tier: super::ForwardTier::T2Boltzmann,
            // The provenance hash should be filled by the adapter (code+data versions); empty here.
            provenance_hash: String::new(),
            // The concrete adapter should inject a SolverManifest before the score is accepted.
            // Left None here so the adapter can fill it after obtaining the binary SHA-256.
            solver_manifest: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegates_to_an_external_command_and_parses_predictions() {
        // A mock backend: ignore stdin, echo a fixed prediction array (stands in for a CLASS run).
        let mock = "cat >/dev/null; printf '[{\"observable_id\":\"h0\",\"value\":67.4,\"uncertainty\":0.5,\"unit\":\"km s^-1 Mpc^-1\"}]'";
        let model = SubprocessForwardModel::new(mock);
        let preds = model
            .predict(&CosmologyParams::planck_lcdm(), &["h0".to_string()])
            .expect("predict");
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].observable_id, "h0");
        assert!((preds[0].value - 67.4).abs() < 1e-9);
    }

    #[test]
    fn a_failing_backend_is_an_error_not_a_silent_default() {
        let model = SubprocessForwardModel::new("exit 3");
        let r = model.predict(&CosmologyParams::planck_lcdm(), &["h0".to_string()]);
        assert_eq!(r, Err(ForwardFailure::Crash));
    }

    #[test]
    fn manifest_marks_a_boltzmann_backend() {
        let m = SubprocessForwardModel::new("true").manifest();
        assert_eq!(m.kind, ForwardKind::Boltzmann);
    }
}
