# Spec S06 — KPI anti-cheating metrology for OpenQG/ZYAL V8

Scope: this spec responds to the extracted archive, especially `crates/openqg-core/src/theory/scorecard.rs`, `vetoes.rs`, `league.rs`, `evidence.rs`, `crates/openqg-core/src/scoring/evidence.rs`, `paper/main.tex`, `docs/zyal-next-level-design.md`, and `docs/V6-ACCEPTANCE.md`. The current design is unusually honest, but it still measures a blend of discovery, fit, self-consistency, and evaluator health as one number. V8 should split those quantities before another champion learns to arbitrage the blend.

## Ranked backlog

1. **Replace fit-set data points with an evidence gate and a post-registration forecast score.** Why: `RubricV4` still assigns 20/100 to `data_fit` (`scorecard.rs:79-88`, `539-550`), even though every historical exploit found free value in the fitted data (`paper/main.tex:408-416`). Effort: **M**. Verification: replay V4-V7 champions and prove the fit-set contribution is exactly zero points; a model can pass/fail the evidence gate but cannot raise its theory score using observables admitted before proposal timestamp.

2. **Add a first-class `PricingLedger` and fail closed on unpriced choices.** Why: current `k` prices free parameters, background drift, and proposer-chosen certificate inputs (`scorecard.rs:220-313`), but the paper admits witness tolerances and observable choice were monetized (`paper/main.tex:690-694`). Effort: **M**. Verification: fuzz proposals over tolerance, lane, witness observable, prompt template, covariance mode, prior bounds, optimizer budget, and dataset subset; every perturbation either has a ledger entry with a complexity/multiplicity charge or is proven invariant by a test.

3. **Implement a signed prediction registry with later-data adjudication.** Why: V7 novelty is still scored from witnesses present in the same campaign object, capped for fit-set and engine-refreshed cases (`scorecard.rs:552-580`); this is better than V6 but still not a forecast. Effort: **M**. Verification: create a `prediction.lock.jsonl`, freeze it before adding a synthetic future dataset, then confirm that only registry entries signed before the dataset hash date can earn forecast points.

4. **Move unification credit off shared-parameter self-consistency.** Why: `unification.rs` has real null-test blocks for GW170817, sirens, Cassini, MICROSCOPE, and D/H (`unification.rs:1-24`), but scorecard unification still awards 15 points from `shared_parameter_audit` plus `no_hidden_knob_test` (`scorecard.rs:583-594`). Effort: **M**. Verification: a theory with shared `H0` but no independent cross-domain effect earns zero unification credit; a theory that passes null tests is eligible but gets no frontier credit unless it predicts a signed, non-null, mechanism-backed cross-domain deviation.

5. **Publish engine KPIs beside theory KPIs.** Why: the paper says the audit cascade is the product and proposes time-to-invalidate (`paper/main.tex:712-717`), but no data structure makes it a first-class metric. Effort: **M**. Verification: every run emits `engine_kpis.json` with time-to-invalidate, exploit discovery rate, regression growth, search-volume covered, exclusion certificates, reproducibility cost, and cost-normalized rates; values reproduce from ledgers only.

6. **Add held-out exploit classes and a rubric-overfit statistic.** Why: `paper/main.tex:670-673` names oracle overfitting but treats rapid rubric turnover as the defense. That is training on known exploits. Effort: **M**. Verification: maintain public exploit tests and a sealed reserve; block releases when reserve kill-rate lags public kill-rate beyond a pre-registered z-test threshold.

7. **Replace index-based holdout with cryptographic, release-time holdouts.** Why: `alternating_holdout(n)` is predictable (`holdout.rs:106-109`) and `held_out_evaluate` uses the same diagonal path as the legacy scorer (`holdout.rs:44-60`). Effort: **S-M**. Verification: split assignment is generated from a sealed salt; no proposer prompt contains T5 evidence (`evidence.rs:17-29`, `55-62`); heldout scoring uses the same covariance and baseline-refit path as promotion scoring.

8. **Wire exact marginal evidence into promotion-grade scoring.** Why: `league.rs` reports `ΔlnZ ≈ -0.5 ΔBIC` (`league.rs:392-441`) while `scoring/evidence.rs` already warns BIC can mis-rank degenerate extensions and provides Laplace/grid estimators (`scoring/evidence.rs:1-34`). Effort: **L**. Verification: for all 1-3 parameter leagues, BIC, Laplace, and grid/nested estimates are reported; if they disagree by >1 nat, promotion uses the more conservative value and flags the model.

9. **Define and enforce the profound-vs-incremental line.** Why: a one-glance outside-physicist scoreboard needs thresholds, not vibes. Effort: **S**. Verification: a release cannot label a result “profound” unless it satisfies one of the registered numerical criteria below.

10. **Generate decoys by exploit taxonomy, not only hand fixtures.** Why: fixed decoys become training data for the oracle. Effort: **M**. Verification: the decoy generator produces instrument, laundering, specification, coherence, economics, and friction variants; each new accepted exploit adds a generator rule and a regression seed.

## Current metrology: what the repo actually measures

The scorecard is veto-first: evidence materialization, physics kills, derivation obligations, and hidden-knob checks run before any positive credit (`scorecard.rs:390-429`). That structure should remain. The current components are `derivation_rigor` 20, `data_fit` 20, `novel_prediction` 20, `unification` 15, `robustness_under_judge` 13, and `parsimony` 12 (`scorecard.rs:79-88`). Data fit is one-sided, so ties with ΛCDM earn zero (`scorecard.rs:138-145`, `539-550`). Novelty is audited against machine-computed predictions, with fit-set witnesses capped at 0.25 and engine-refreshed witnesses at 0.5 (`scorecard.rs:552-580`). Parsimony now counts ordinary free parameters, drifted background coordinates, and selected certificate inputs (`scorecard.rs:220-313`). Those are real improvements.

The remaining problem is semantic: the 100-point score still mixes *fit quality*, *forecast quality*, *derivation quality*, *null-test consistency*, *audit survivability*, and *engine operating health*. A candidate can therefore improve the headline by optimizing the easiest remaining proxy. V8 should publish three linked but separate artifacts:

```rust
pub struct V8ReleaseVerdict {
    pub gates: GateVerdict,                 // pass/fail, no points
    pub theory_score: TheoryDiscoveryScore, // 0..100, no fit-set points
    pub engine_score: EngineKpiSnapshot,    // operational health, not physics truth
    pub pricing_ledger: Vec<PricingLedgerEntry>,
    pub compatibility: LegacyScoreCrosswalk,
}
```

The old `total` remains in `LegacyScoreCrosswalk` for V4-V7 comparability, but it must not be the promotion number.

## Dimension-by-dimension gaming audit

| Dimension | Known/could exploit | Audit taxonomy | Current fix | Structural V8 fix |
|---|---|---|---|---|
| Derivation rigor | A fitted value wears a `Derived` label; literature-equivalence earns rigor without executable dynamics; a certificate verifies a parameter but not the term that generates it. | laundering, specification | certificate registry verifies closed-form values (`certificate.rs:1-14`); failed certs kill; `relation_rigor_weight` demotes definitions (`certificate.rs:137-163`). | Require a `GeneratingMechanism` object for every non-GR dial: action term, EOM hook, background/growth binding, and proof obligation. Rigor points are multiplicative: `value_certified * dynamics_generated * bound_to_forward_model`. Text-only derived values are not diagnostics; they are zero-rigor and non-promotable. |
| Data fit | Fitted candidate compared to fixed baseline; diagonal likelihood; arbitrary prior boxes; BIC surrogate; cherry-picked dataset tier. | instrument, economics, specification | `league.rs` profile-fits baseline and candidate with covariance (`league.rs:1-16`), coverage fail-closed (`386-390`). | Fit-set evidence becomes a gate. Prior boxes, covariance blocks, dataset tier, optimizer budget, and evidence estimator are pre-registered and priced. Promotion uses Laplace/grid/nested evidence, not BIC alone, when available. |
| Novel prediction | Witness on fit-set data; witness tolerance chosen after seeing residuals; engine refresh manufactures “honesty”; observable selected from a large menu. | laundering, specification, economics | computed-honest-distinct audit; fit-set and engine-refresh caps (`scorecard.rs:552-580`). | Only time-stamped predictions registered before data release earn forecast points. Global look-elsewhere correction prices number of eligible observables, transformations, and tolerances. |
| Unification | Shared parameter label or duplicated BBN point masquerades as cross-domain unification. | laundering, coherence | `unification.rs` adds independent null constraints, but scorecard still awards shared-parameter credit. | Split “cross-domain consistency gate” from “unification discovery credit.” Passing nulls prevents DQ; it does not score. Positive unification credit requires a derived non-null deviation in another domain. |
| Robustness | Alternating holdout is predictable; negative gaps can be hidden; adversary can be telemetry, not selection pressure. | specification, economics | paper claims sealed gaps; internal critique says adversary was inert (`docs/zyal-next-level-design.md:22-38`); holdout code is index-based. | Cryptographic T5 splits, fresh red-team decoys, and champion eligibility based on adversary survival. Publish time-to-invalidate and reserve-exploit generalization gap. |
| Parsimony | Marginal drift cheap under harmonic penalty; tolerance/lane/prompt/observable choice unpriced. | economics | linear `1-k/4` and post-search input dof (`scorecard.rs:605-610`, `220-240`). | One ledger for all freedoms: numeric parameters, priors, tolerances, model-class selection, prompt families, lanes, observable menu size, data transformations, optimizer restarts, and calibration/nuisance terms. |

## V8 scoring: evidence as gate, forecasts as points

Ship this rubric:

```rust
pub struct GateVerdict {
    pub physics_vetoes_pass: bool,
    pub evidence_materialized: bool,
    pub derivation_minimum_pass: bool,       // rigor_raw >= 0.60 for promotable claims
    pub fit_set_evidence_gate: EvidenceGate, // no points
    pub cross_domain_nulls_pass: bool,
    pub replay_pass: bool,
    pub unpriced_dof_count: u32,             // must be 0
}

pub struct TheoryDiscoveryScore {
    pub mechanism_derivation: f64,       // 0..25
    pub post_release_forecast: f64,      // 0..30
    pub cross_domain_unification: f64,   // 0..15
    pub adversarial_robustness: f64,     // 0..10
    pub parsimony_integrity: f64,        // 0..10
    pub reproducibility_metrology: f64,  // 0..10
    pub total: f64,
    pub cap_reasons: Vec<String>,
}
```

Fit-set evidence gate: pass if (a) full-coverage covariance likelihood, (b) refit ΛCDM, (c) registered prior boxes, (d) conservative marginal evidence `ΔlnZ_fit >= 0` for “not worse than null” and `>= +5` for “positive promoted candidate,” (e) no single observable block contributes more than 60% of the positive evidence unless the mechanism explicitly predicts that block and a mechanism-off twin removes the gain. A failed gate can still produce a useful exclusion result, but not a discovery champion.

Forecast points replace data-fit points. A `PredictionRegistryEntry` is append-only:

```rust
pub struct PredictionRegistryEntry {
    pub prediction_id: String,
    pub theory_digest: String,
    pub created_at_utc: String,
    pub proposer_seen_data_watermark: Vec<DataReleaseId>,
    pub target_observable_id: String,
    pub release_expected_after: String,
    pub predicted_distribution: DistributionSpec,
    pub baseline_distribution: DistributionSpec,
    pub mechanism_off_distribution: DistributionSpec,
    pub tolerance_policy_id: String,
    pub eligible_observable_menu_hash: String,
    pub signature: String,
}
```

When new data land, compute local and global evidence:

```text
local_log_bayes = log p(data_new | theory_prediction) - log p(data_new | baseline)
look_elsewhere_M = count(eligible observables, transforms, signs, windows)
global_p = min(1, local_p * look_elsewhere_M)
forecast_raw = clamp01((local_log_bayes - 1.0) / 9.0) * confirmation_gate
confirmation_gate = 1 if global_sigma >= 3 and sign/mechanism match else 0
```

A post-release forecast confirmed at global `>=3σ` earns serious credit; `>=5σ` and `ΔlnZ_new >= +5` unlocks the “profound” label. A fit-set explanatory witness earns zero forecast points even if it remains useful in the paper narrative.

To enforce underived-fit earns nothing, apply caps: total score cannot exceed 20 if `mechanism_derivation < 0.6`; cannot exceed 35 without a registered out-of-fit-set forecast; cannot exceed 50 unless all unpriced DOFs are zero; cannot exceed 70 unless at least one independent cross-domain prediction or exclusion certificate exists.

Migration: for one release, emit both `ScorecardV4` and `ScorecardV8`. Publish a crosswalk table: old total, old data-fit points, old novelty points, V8 evidence gate, V8 forecast points, and cap reason. V4-V7 historical scores stay as archaeology; the honest comparison is “how much of each old score survives V8.”

## Completeness of the pricing ledger

Add `PricingLedgerEntry` to every scorecard receipt:

```rust
pub enum DofKind {
    PhysicalParameter, BackgroundDrift, CertificateInput, PriorBox,
    Tolerance, ObservableChoice, DatasetChoice, CovarianceMode,
    PromptFamily, MechanismLane, RepairFeedback, OptimizerBudget,
    CalibrationNuisance, TransformationChoice,
}

pub struct PricingLedgerEntry {
    pub kind: DofKind,
    pub name: String,
    pub chosen_value: String,
    pub allowed_set_hash: String,
    pub chosen_before_data_hash: Option<String>,
    pub complexity_charge: f64,
    pub multiplicity_count: u64,
    pub harmless_proof: Option<String>,
}
```

Pricing rules:

* **Tolerances.** Certificate tolerances, novelty tolerances, finite-difference steps, covariance condition-number cuts, and acceptance epsilons must be registry-defined. If proposer-chosen, charge `log2(range/default_bin)` as multiplicity and cap forecast credit unless the tolerance was registered before the target data release.
* **Lane selection.** Lanes are search trials. They do not make a theory less parsimonious, but they do reduce discovery surprise. Charge them in global forecast/exclusion look-elsewhere: `M *= number_of_active_lanes * best_of_k_samples * repair_attempts`.
* **Prompt content.** Prompt templates and few-shot examples are engine DOFs. Record prompt hash, template family, redaction class, and examples. If a prompt contains a target observable, tolerance, or mechanism prior not in the public registry, either price it as a prior or mark the forecast ineligible.
* **Novelty observable choice.** The choice of which observable to predict is a lottery ticket. Register the complete eligible menu hash and use it in global significance. If the witness observable was added after seeing residuals, forecast credit is zero.
* **Prior bounds.** A narrower prior box can inflate evidence. Prior boxes require a citation or pre-run registration. Boundary-hit (`league.rs:253-258`, `305-311`) becomes a gate warning: no promotion until the prior is justified or widened and replayed.
* **Dataset/covariance mode.** Dataset inclusion is not a theory DOF, but it is an engine choice. Every release must carry a data suite hash and covariance registry hash. Switching to a favorable subset produces a separate, non-comparable leaderboard.
* **Optimizer budget.** Restarts and resolution are search effort. Charge in engine KPIs and freeze before campaigns; if increased after a champion appears, re-run all contenders and nulls.

Acceptance test: `pricing_ledger_is_complete` mutates each serialized scorecard by deleting one ledger entry at a time and must make `unpriced_dof_count > 0`, `total=0`, or an explicit `harmless_proof` test fail.

## Engine-level KPIs and dashboard

The product is the engine. Publish `engine_kpis.json` per campaign:

```rust
pub struct EngineKpiSnapshot {
    pub campaign_id: String,
    pub champion_promoted_at: String,
    pub time_to_invalidate_hours: Option<f64>,
    pub exploit_discovery_rate_per_1000_attempts: f64,
    pub regression_corpus_growth: RegressionStats,
    pub search_volume: SearchVolumeStats,
    pub exclusion_certificates: Vec<ExclusionCertificate>,
    pub reproducibility_cost: ReproCost,
    pub cost_normalized: CostKpis,
}
```

**Time-to-invalidate (TTI).** Start at champion promotion receipt. Stop at first accepted exploit that either disqualifies the champion or reduces its V8 score by at least 10 points and lands as a regression test. Report min/median/p90 by champion class. “Not yet invalidated” is right-censored, not infinite.

**Exploit discovery rate.** `verified_new_exploit_classes / adversary_hours` and `/1000 proposals`. A new exploit class is not a duplicate stack trace; it is a new taxonomy label or a new generator rule that kills at least one prior survivor.

**Regression growth.** Count tests by exploit class and by gate touched. Publish the fraction of tests that are generative/fuzzed versus fixed fixtures. A healthy corpus grows in variants, not just hand examples.

**Search-volume covered.** Define a declared theory space as cells:

```text
cell = hash(mechanism_family, term_multiset, relation_names,
            sector_set, parameter_role_vector,
            prior_hyperbox_bin, observable_capability_set)
cell_volume = prior_mass * grammar_prior * sector_weight
covered_volume = sum(cell_volume for cells with >=N valid attempts and oracle-complete receipts)
```

Use MAP-Elites for exploration, but publish volume over a meaningful grammar, not just the current <=36 descriptor cells criticized in `docs/zyal-next-level-design.md:40-51`. Report covered volume, frontier volume, killed volume, and unvisited high-prior volume.

**Exclusion strength.** An exclusion certificate says: within declared class `C`, prior volume hash `V`, data suite `D`, and oracle version `O`, no candidate in covered volume passes the evidence gate; posterior mass of passing region is `<α` or Bayes factor against class is `<-B` nats. Suggested thresholds: publish “weak exclusion” at 80% covered volume and `α<0.10`; “strong exclusion” at 95% covered and `α<0.05`; “decisive exclusion” at 99% covered and `α<0.01` or `ΔlnZ_class<-10`.

**Reproducibility cost.** Record CPU model, wall time, deterministic command, peak memory, and CPU-hours to replay from ledgers with no network. The cost of belief is part of the result.

Dashboard panels: (1) gates, (2) theory score decomposition, (3) forecast registry, (4) engine health KPIs, (5) search volume heatmap, (6) exclusion certificates, (7) pricing ledger diff, (8) cost-normalized performance. TypeScript owns the browser surface; Rust owns receipts and schemas.

## Profound vs incremental line

A result may be labeled **profound** only if one of these is true:

1. **Forecast discovery:** a prediction registered before data release is confirmed at `global >=5σ`, `ΔlnZ_new >= +5`, sign matches, and a mechanism-off twin loses the effect by at least 70%.
2. **Mechanism exclusion:** a named mechanism class is excluded over `>=95%` declared prior volume with `posterior_mass_passing <5%`, full replay, and no reserve exploit failure.
3. **Instrument discovery:** the engine finds an evaluator/data bug at `>5σ` or `>10` nats impact, the fix reverses or materially changes a headline result, and the bug becomes a regression plus calibration envelope.
4. **Cross-domain unification:** one mechanism-derived parameter predicts a non-null shift in at least two independent domains, both confirmed post-registration with combined `ΔlnZ>=+10`, while all null constraints pass.

Everything else is “incremental,” “negative,” or “instrumental.” Those are not failures. The V7 43-class suppressed-growth result is a useful negative/incremental candidate: right direction, insufficient evidence. The scoreboard should make that legible in one glance.

## Oracle overfitting protocol

Create three exploit corpora:

* **Public regression:** all known V4-V7 exploits, visible to developers and proposers.
* **Development red-team:** fresh LLM/human decoys used during a release cycle.
* **Sealed reserve:** exploit classes held by a custodian and never used for tuning; only aggregate results are released until retirement.

For every rubric change, run public and reserve tests. Compute:

```text
overfit_gap = kill_rate_public - kill_rate_reserve
z = overfit_gap / sqrt(p_pool*(1-p_pool)*(1/n_public + 1/n_reserve))
```

Block release if `z > 2` and reserve kill-rate is below the pre-registered floor. Also run leave-one-exploit-class-out validation: tune on five taxonomy classes, evaluate on the sixth. If the held-out class survives, the fix is a patch, not structural immunity.

Periodic campaigns: quarterly external red-team, one human physicist plus one ML reward-hacking reviewer plus LLM agents. Their findings must be reproduced as deterministic failing tests before they count. This preserves the project invariant: LLMs critique; the host judges.

## Cost-normalized KPIs

Assuming S12 provides raw token telemetry, define:

```text
score_per_million_tokens = max(0, V8_theory_score_delta) / prompt_completion_tokens_millions
kills_per_million_tokens = oracle_kills / tokens_millions
verified_exploits_per_million_tokens = new_exploit_classes / tokens_millions
regressions_per_million_tokens = new_regression_tests / tokens_millions
forecast_points_per_million_tokens = post_release_forecast_points / tokens_millions
cpu_hours_per_promotable_candidate = replay_cpu_hours / promoted_candidate_count
```

Do not optimize only score-per-token; that would reintroduce proxy gaming. Use it as an operating metric beside quality gates. A run with low score but high exploit discovery can be successful engine work.

## Implementation notes

Hard requirements: Rust schemas under `contracts/specs/metrology.yml`, `contracts/specs/pricing-ledger.yml`, and `contracts/specs/prediction-registry.yml`; serde structs in `openqg-core`; deterministic JSON canonicalization and SHA-256 receipts; CLI subcommands `zyal metrology emit`, `zyal registry preregister`, `zyal registry adjudicate`, `zyal kpi dashboard-json`, and `zyal exploit-reserve evaluate`. For marginal evidence, use existing deterministic Laplace/grid code for low-dimensional classes and add nested sampling for higher dimensions behind a feature flag; cite Trotta 2008 (arXiv:0803.4089), MultiNest (arXiv:0809.3437), PolyChord (arXiv:1506.00171), CLASS (arXiv:1104.2933), hi_class (arXiv:1605.06102), DESI DR1 BAO (arXiv:2404.03002), Planck 2018 (arXiv:1807.06209), SH0ES (arXiv:2112.04510), KiDS-1000 (arXiv:2007.15632), GW170817 constraints (arXiv:1710.05835, arXiv:1710.05877), reward gaming (arXiv:2209.13085), and AI safety reward misspecification (arXiv:1606.06565, arXiv:2201.03544).

## What we got wrong

1. **“Every V7 point is defensible” is too strong.** Check: replace scorecard unification with `unification_report` null blocks and replay V7. If shared-parameter claims still provide the 15-point lift, the score is partly self-consistency, not cross-domain unification.

2. **“Data fit deserves 20 points if one-sided” is still a proxy error.** Check: rerun V6/V7 with data fit as a gate and compare champion ordering. If ordering changes, the old score was still rewarding fit-set optimization.

3. **“Out-of-fit-set novelty” is not the same as post-release prediction.** Check: add a synthetic future data release with a later timestamp. Existing witnesses must earn zero forecast credit unless they were signed before the release hash.

4. **“The cascade itself defends against oracle overfitting” is self-deceiving.** Check: maintain a sealed reserve of exploit classes. If public exploit kill-rate exceeds reserve kill-rate by `z>2`, the rubric has overfit its audit history.

5. **“BIC-style ΔlnZ is promotion-grade evidence” is not settled.** Check: compare BIC, Laplace, grid, and nested evidence on the same low-dimensional model classes. If disagreement exceeds 1 nat, quote the conservative estimator and stop calling the BIC proxy definitive.

6. **“Prompt redaction prevents numerical leakage” ignores prompt/lane search freedom.** Check: run identical theory grammar with different prompt templates and lanes; if success probabilities differ materially, prompt/lane choices belong in the search multiplicity ledger.

7. **“Coverage 1.0 means cosmology coverage” remains misleading.** Check: publish coverage by sector and theory claim. If a modified-gravity claim is scored only on background distances while growth/CMB/lensing are absent, mark it “claim-uncovered” even when record-level coverage is 1.0.
