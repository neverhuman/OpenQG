# Concrete file-level V7 engineering spec

V7 should be scoped as a hardening release. The minimum file-level changes are below.

1. Replace substring term policy with typed term capabilities. Modify `crates/openqg-core/src/theory/mod.rs`: extend `Term` with `kind: TermKind` and add `TermKind` enum. Keep `name` for display/back-compat. Add capability methods, for example `TermKind::generates_ndgp()`, `generates_fr()`, `generates_dynamic_w()`, `generates_drag()`, and `generates_alpha()`. Update `Theory::baseline_lcdm()` construction to set `EinsteinHilbert` and `CosmologicalConstant`.

2. Move structural requirements into binding. Modify `crates/openqg-core/src/theory/binding.rs`: extend `FieldBinding` with `required_terms: Vec<TermKind>` or a string-stable equivalent if serialization is easier. When binding `ndgp_geff_over_g` or `ndgp_beta_from_omega_rc`, require `DgpBrane`. When binding `fr_alpha_m` or `fr_largescale_geff_over_g`, require `FRHuSawicki`. When binding `dark_scattering_growth_drag`, require `DarkSectorMomentumExchange`. When binding `coupled_de_geff_over_g`, require `CoupledScalarDarkMatter`.

3. Replace `StructurallyUngenerated` substring checks. Modify `crates/openqg-core/src/theory/vetoes.rs::adjudicate`: remove the raw `names.iter().any(|n| n.contains(...))` policy. Check typed capabilities. Also inspect `bind_modified_background(theory).report.bindings` and kill when required terms are missing. Add regression tests for the survivor shape, fake substring terms, f(R) without f(R) term, and drag without drag term.

4. Close direct fundamental MG binding. Modify `binding.rs::bind_modified_background`: do not copy `Provenance::Fundamental` parameters named `mu0`, `ndgp_omega_rc`, `fr_log10_fr0`, or `fr_n` unless the symbol is registered in a new allowed-fundamentals registry. Add a `VetoReason::UnregisteredFundamentalModification`. Add tests replacing `direct_fundamental_omega_rc_binds_without_relation` with a failing test unless an explicit term and registry entry exist.

5. Make fit-set novelty non-scoring. Modify `crates/openqg-core/src/theory/scorecard.rs::score_with_observables`: change the fit-set branch from `0.25` to `0.0`. Add a `fit_explanation` boolean to `NovelPredictionAudit` or a separate metadata section. Reduce engine-refreshed novelty to zero unless a new `pre_registered` flag is true.

6. Add prediction registry. Create `crates/openqg-core/src/theory/prediction_registry.rs` with `PredictionRegistry`, `RegisteredPrediction`, and `RegistrationStatus`. Fields: `observable_id`, `mechanism_field`, `registered_before_run`, `measurement_sigma`, `fit_set_allowed`, `source`. Update `score_with_observables` signature or add `score_with_context` to accept it. Use registry sigmas for all witnesses, not only fit observables.

7. Replace scalar BIC in campaign scoring. Modify `crates/openqg-bench/src/zyal_genome/physics_score.rs::score_candidate`: move Occam computation to a new helper returning `EvidenceMethod::{BicApprox, Laplace, Grid}` and `n_eff`. Use `crates/openqg-core/src/scoring/evidence.rs::laplace_log_evidence` for one- and two-dimensional profiled model classes. Keep BIC only as fallback and stamp the method in `DataFitOutcome`.

8. Add effective mode accounting. Modify `crates/openqg-core/src/scoring/covariance.rs`: add `effective_modes(data: &LikelihoodData) -> f64`. Initial conservative implementation: each unblocked observable counts one; each covariance block counts the number of positive eigenvalues above a tolerance, capped by block size. Use it in BIC fallback instead of `observables.len()`.

9. Make V6.1 rescores reproducible. Modify `crates/openqg-bench/src/zyal_genome/whitepaper.rs`: include `rubric_version`, `scorecard_receipt`, `forward_manifest`, `covariance_fixture_hashes`, and `calibration_version` in `white-paper.json`. Add a new CLI command in `crates/openqg-bench/src/cli/zyal.rs` or `zyal_genome/mod.rs`: `rescore-whitepaper --input run-data/... --out ...` that extracts the champion theory/proposal and writes a current scorecard artifact.

10. Save full campaign ledgers. The archive must include the proposal ledger or a compressed reproducibility bundle. Modify `run_population.rs` and `ledger_sink.rs` so white-paper artifacts reference ledger hashes and campaign reports include enough files to replay all 754 calls without LLM calls.

11. Harden proposer memory. Modify `crates/openqg-bench/src/zyal_genome/proposer_memory.rs::top_scorer_entry`: require `active_rubric_version`, current `scorecard_sha256`, and a current score above threshold. Reject entries with no V6.1/V7 receipt. Add lane-family diversity fields to `ProposerMemory`.

12. Separate oracle repair from authorship. Modify `crates/openqg-bench/src/zyal_genome/proposer_router.rs`: if a proposal needed oracle repair, mark `oracle_repaired = true` in `ProposalDoc` metadata. `score_with_observables` should use that flag to deny novelty for repaired witness values.

13. Add Boltzmann promotion hook. Modify `crates/openqg-core/src/cosmology/forward.rs`: add a `BoltzmannForwardModel` trait adapter behind a feature flag, with `ForwardManifest { kind: ForwardKind::Boltzmann }`. Add a fail-closed promotion rule: no external-candidate report if the final scorecard lacks a Boltzmann or calibrated-envelope receipt for CMB-sensitive observables.

14. Tests to add: survivor DQ under structural term grammar; constant anchor plus derivative grid; fit-set witness zero; direct fundamental MG kill; out-of-fit witness without registry zero; V6 white-paper rescore writes current scorecard; covariance `n_eff` differs from raw record count.

Acceptance rule: do not start a V7 live campaign until these tests pass and the generated campaign bundle includes all input data manifests. The first V7 run should be a replay-only audit over the V6 champions, not a fresh proposer campaign. Only after every V6 champion either disqualifies or has a reproducible current score should the router be allowed to spend new calls.
