# Concrete file-level engineering spec for V6

1. `crates/openqg-core/src/theory/vetoes.rs`: add `pub fn physics_kills(theory: &Theory) -> Vec<VetoReason>` that concatenates `run_veto_cascade(theory)` and `adjudicate(theory)` without duplicates. Keep `run_veto_cascade` as cheap triage for diagnostics, but make every scoring/promotability caller use `physics_kills`.

2. `crates/openqg-core/src/theory/evaluate.rs`: replace the `run_veto_cascade(theory)` call with `physics_kills(theory)`. Replace the `score_metrics` call with covariance-capable scoring over `LikelihoodData`. If keeping a compatibility overload, name it `evaluate_diagonal_for_tests` so production code cannot accidentally use it.

3. `crates/openqg-bench/src/zyal_genome/physics_score.rs`: change `baseline_log_likelihood` and `score_candidate` to accept `&LikelihoodData`. Compute the Planck baseline with `score_metrics_cov`. Store `likelihood_mode`, `covariance_block_count`, and `effective_observable_count` in `DataFitOutcome` or a companion receipt.

4. `crates/openqg-core/src/theory/scorecard.rs`: modify `free_dof` to count non-baseline background coordinates. Add `background_dof(theory)` that checks `h`, `omega_m`, `omega_b_h2`, `n_eff`, `sum_mnu`, `w0`, `wa`, `omega_k`, `sigma8`, `mu0`, `mg_family`, `fr_n`, `fr_log10_fr0`, and `ndgp_omega_rc`. A coordinate is free unless it is synchronized to a provenanced `Parameter` or bound by a verified relation. Use `free + background_free` for parsimony.

5. `crates/openqg-core/src/theory/proposal.rs`: remove the split-brain between `parameters` and `background`. When `BackgroundProposal` overrides a baseline value, insert or update a corresponding `Parameter` with provenance. Reject a proposal where `Parameter {symbol:"H0"}` disagrees with `background.h * 100` or `Omega_m` disagrees with `background.omega_m`.

6. `crates/openqg-core/src/theory/mutation.rs`: when mutating `background.h`, `omega_m`, or `w0`, also mutate the corresponding parameter/provenance or mark it as free. Better: mutate through a new `set_background_param(theory, symbol, value, provenance)` helper that preserves ledger consistency.

7. `crates/openqg-core/src/theory/binding.rs`: in every relation that carries background inputs, verify those inputs against `theory.background`. For `ndgp_beta_from_omega_rc`, reject certificates whose `omega_m`, `w0`, or `wa` inputs disagree with the actual background. For `fr_alpha_m`, remove `.max(1e-9)` masking on dark energy; domain errors should be `UnimplementedModification`.

8. `crates/openqg-core/src/theory/obligation.rs`: split `NovelPredictionWitness.min_detectable` into `min_detectable` and optional `honesty_tolerance`. Add a default engine tolerance. Add a verdict enum for novel audits.

9. `crates/openqg-core/src/theory/scorecard.rs`: change novelty scoring: no prediction witness -> 0 points; all uncomputable -> 0 points; computable dishonest beyond hard bound -> disqualification; computable honest distinct on a holdout observable -> full points. Remove the alpha-jitter half-credit.

10. `crates/openqg-core/src/scoring/covariance.rs`: expose validation errors as typed errors, not string findings only. Add tests for Planck 3x3, BAO 2x2, missing prediction marginalization, non-PD rejection, and equality with diagonal scoring when blocks are diagonal.

11. `crates/openqg-core/src/validation/manifest/dataset.rs`: add covariance metadata fields. Validate matrix dimensions, observable order, units, and data-role labels (`fit`, `validation`, `holdout`, `adjudication`).

12. `crates/openqg-data/src/registry.rs`: add `load_likelihood_data(root, manifest)` returning `LikelihoodData`. Implement JSON/CSV matrix loading and hash-lock covariance files in `DataLock`.

13. `crates/openqg-bench/src/zyal_genome/proposer_memory.rs`: require `promotable=true` before top scorers enter memory. Add family-diversity counts and covariance-aware data brief rendering.

14. `crates/openqg-bench/src/zyal_genome/proposer_jekko.rs`: stamp prompt hash, config, quality band, samples, repair budget, and scorer version into `ProposalAttemptRecord`. Split parse/evidence repair from physics repair and reduce kill-threshold leakage in physics repair prompts.

15. `crates/openqg-bench/src/zyal_genome/theory_population.rs`: implement re-clothing only as a new candidate constructor that produces a fresh `ProposalDoc` and runs `score_proposal`. Add fields `donor_doc_sha`, `descendant_fingerprint`, `dirty_symbols`, and `reverified_obligations` to the ledger.

16. `run-data` generation code in `crates/openqg-bench/src/zyal_genome/whitepaper.rs`: fix the misleading parameter table. Print both `Theory.parameters` and `Theory.background`, and flag disagreements such as `H0=67.4` while `background.h=0.718`.

17. `crates/openqg-core/src/theory/fingerprint.rs`: include a normalized "scored physics fingerprint" that hashes the bound background, alpha fields, screening recovery, claim graph digest, and likelihood-data hash. The current claim fingerprint can remain for lineage diversity, but promotion should compare the scored physics fingerprint so cosmetic claim changes cannot hide identical physics.

18. `crates/openqg-bench/src/zyal_genome/tests/`: add replay fixtures derived from `run-data/v5-chunk-1-white-paper.json` and a clean suppressed-growth proposal. Tests should assert the former loses promotability under V6 gates and the latter still earns data credit when its witness is honest.

19. `crates/openqg-core/src/theory/scorecard.rs`: add `scoring_version` and `gate_version` fields to `ScorecardV4`. V5 ledgers replay under V5 semantics; V6 ledgers must not be silently compared against V5 scorecards. This is especially important once covariance and adjudication change totals.

20. `crates/openqg-bench/src/zyal_genome/run_summary.rs`: include counters for covariance blocks used, candidates killed by adjudication, candidates penalized for background drift, and novelty witnesses by verdict. These counters should appear in every white paper before the champion section.
