# S04 — Gap-directed decomposition for OpenQG/ZYAL

## Ranked backlog

| Rank | Change | Why it matters | Effort | Hostile-review verification |
|---:|---|---|---:|---|
| 1 | Add a durable **Component Graph + Component Ledger** to Rust, emitted for every champion and candidate promoted above a low threshold. | Today `ClaimGraph` proves claim/obligation structure and `ScorecardV4` reports rubric dimensions, but nothing answers “which relation/binding/term moved `fsigma8@0.51`, cost novelty, or leaned on `cmb_lA`.” Without a durable component identity, focused re-proposal is slogan-level. | M | Given the V7 `planck_mu0-suppressed-growth` fixture, a test can locate stable component IDs for `relation:planck_mu0_geff`, `param:geff_over_g`, `bg:mu0`, `term:planck_mu_parametrization`, and `claim:claim-suppressed-growth`, and can reproduce the same ledger digest under reordered JSON fields/claims. |
| 2 | Implement a deterministic **ablation engine** with two neutralization modes: `mechanism_off_keep_cost` and `remove_component_reprice`. | Component attribution must distinguish physics effect from bookkeeping credit. Turning off `mu0` while keeping its cost tells whether it helps evidence; deleting the obligation tells whether it was only earning rigor/novelty. | L | On the V7 fixture, `mechanism_off_keep_cost(relation:planck_mu0_geff)` resets the bound MG effect to GR and moves `fsigma8` predictions back toward the LCDM values in `paper/data/predictions.txt`, while `remove_component_reprice` also removes its rigor/novelty contribution. |
| 3 | Emit a per-observable **EvaluationTrace** from the forward model and likelihood, including residuals, pulls, covariance block contributions, and binding provenance. | The scorecard’s `DataFitOutcome` has only aggregate `delta_lnz`, coverage, and covariance count. A gap-directed engine needs signed, per-observable pull deltas and per-block likelihood deltas. | M | For all observables in `data/fixtures/cosmology/tier1-multisector.jsonl`, the trace records prediction, observed value, uncertainty or covariance block, pull, and log-likelihood contribution; summing the trace reproduces the existing `DataFitOutcome` within `1e-9`. |
| 4 | Build the **Gap Dashboard** as a Rust artifact first, TypeScript surface second. | Routing attention by dashboard widgets alone is a laundering risk. The ranking metric must be deterministic, content-bound, and replayable; TypeScript should render immutable JSON. | M | A planted weakened dark-scattering fixture is ranked top evidence gap for the drag component, not for `H0`, `w0`, or an unrelated claim. Re-running with shuffled component order returns byte-identical `component_ledger.json`. |
| 5 | Define a strict **FocusedPatchSketch** protocol and merge-back verifier. | Current proposer sketches can describe whole theories. Current `reclothe_candidate` grafts donor structure onto descendants. Neither enforces “freeze everything except one weak component.” | M/L | A patch that changes only `cert.inputs[a_drag]` and associated obligations is allowed; the same patch plus an unapproved `background.w0` edit is killed with `FrozenDigestChanged` before physics scoring. |
| 6 | Add **anti-laundering invariants** for component freezing and job shifting. | Freezing creates a new exploit: make a target look repaired by moving its job to a sibling relation, background field, or oracle tolerance. V6.1 claim/physics coherence is the right prior art but too narrow. | M | A synthetic patch that leaves the target component weak but increases sibling `sigma8` or `drag_a` influence is killed as `JobShiftToSibling`; a patch that creates a hidden certificate input is killed as `DofLaundering`. |
| 7 | Add **bandit credit assignment** over `(agent, lane, operator, component_class, gap_class)` with a replayable reward ledger. | V7’s lanes are static prompt categories. The campaign should learn which proposer/operator combinations repair growth relations, novelty witnesses, or rigor gaps. | M | In a deterministic simulation where one lane has higher planted success on `growth_relation` gaps, discounted UCB allocates at least 70% of non-reserved budget to that arm by 100 pulls while preserving the configured exploration/null quotas. |
| 8 | Treat **oracle approximations as components** and run sensitivity audits as standing jobs. | The V6 `l_A` failure was found because the optimizer leaned on the oracle. Make that an instrumented sensitivity ledger: which approximation can flip a champion’s score band? | L | Inject a known `cmb_lA` offset into the forward model; the oracle ledger ranks `oracle:cmb_lA_anchor_calibration` top by score leverage and blocks promotion when the sign of evidence changes. |
| 9 | Promote the V7 relation input-key contract from prompt prose into a typed registry used by schema, verifier, prompts, neutralizers, and patches. | `proposer_sketch.rs` hard-codes exact input keys in lane prompts and schema-enums relation names. That is not enough for component-local prompting or automated constraint surfaces. | S/M | Adding/removing an input key in the relation registry changes generated JSON Schema, prompt snippets, ablation neutralizers, and focused-patch validation in one golden test. |

## Source basis

_Source basis._ I extracted and read the attached 97-file source archive before writing this spec. The most important files for this review were `crates/openqg-core/src/theory/claim_graph.rs`, `crates/openqg-bench/src/zyal_genome/proposer_sketch.rs`, `crates/openqg-core/src/theory/mutation.rs`, `crates/openqg-core/src/theory/scorecard.rs`, `crates/openqg-bench/src/zyal_genome/mod.rs`, `crates/openqg-bench/src/zyal_genome/theory_population.rs`, `crates/openqg-core/src/theory/binding.rs`, `crates/openqg-core/src/cosmology/forward.rs`, `docs/zyal-next-level-design.md`, and the limitations/discussion sections of `paper/main.tex`. The extracted archive root tree digest I reviewed was `12baf95b796e7fed119837bee2e54d0d1086cfb05f56a8caa98f1dd54d1e6957`.

## Diagnosis grounded in the current code

The project already has strong gating pieces, but their granularity stops one layer too high for S04. `ClaimGraph` is a deterministic DAG of claims with sectors, obligations, evidence, dependencies, order-independent digest, and no-hidden-knob unification checks (`claim_graph.rs`). `ScorecardV4` enforces the six 100-point rubric dimensions, one-sided data fit, free-DOF pricing, binding reports, and novelty audits (`scorecard.rs`). `binding.rs` truth-binds relation certificates into actual background fields: for example `planck_mu0_geff` binds `mu0`, while `dark_scattering_growth_drag` binds `drag_a` after checking `w0` and `omega_de0` consistency. `proposer_sketch.rs` is the strongest V7 seed for this work: `ProposalSketch` already contains parameters, relation names, per-parameter inputs, claims, obligations, and lane-specific exact input-key instructions such as `planck_mu0_geff` needs `[mu0]` and `dark_scattering_growth_drag` needs `[a_drag, w0, omega_de0]`.

What is missing is not another scorecard dimension. It is causal accounting. The paper’s discussion explicitly says the evaluator needs attribution: “which observables, which formula, which dial paid out.” The limitations section admits the current forward path is fitting-formula grade, the term registry is an allowlist not a term algebra, and oracle overfitting remains a live threat. `docs/zyal-next-level-design.md` already calls for discriminating data, a real generator, a real adversary, and fair model selection. S04 should not repeat those asks. It should give the existing gates the machinery to answer: “for this champion, what exact component helped, hurt, remained weak, or leaned on an oracle approximation?”

The current `mutation.rs::recombine` and `theory_population.rs::reclothe_candidate` show the risk. They can merge or graft theory structure, parameters, obligations, terms, and refreshed witnesses, but they do not preserve a frozen component boundary. Once focused re-proposal exists, this becomes a laundering channel unless the merge-back verifier is stricter than the current whole-theory recombination path.

## 1. Component Graph and Component Ledger

Add `crates/openqg-core/src/theory/components.rs` and re-export from `theory/mod.rs`. Rust owns durable policy, receipts, schemas, and run contracts; TypeScript only renders the resulting JSON.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    Relation, Parameter, CertificateInput, BackgroundField, ActionTerm,
    Binding, Claim, Obligation, UnificationSharedParam, Observable,
    LikelihoodBlock, ScoreDimension, OracleApproximation,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ComponentId {
    pub kind: ComponentKind,
    pub stable_key: String,      // e.g. relation:planck_mu0_geff:param:geff_over_g
    pub content_digest: String,  // sha256 over canonical component payload
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TheoryComponent {
    pub id: ComponentId,
    pub owner_path: String,      // JSON pointer into Theory / ClaimGraph / obligations
    pub sector: Option<Sector>,
    pub inputs: Vec<String>,     // relation input keys, upstream component ids, or fields
    pub outputs: Vec<String>,    // bound fields, observables, score dimensions
    pub upstream: Vec<ComponentId>,
    pub downstream: Vec<ComponentId>,
    pub claim_ids: Vec<String>,
    pub obligation_ids: Vec<String>,
    pub costed_dof: f64,
    pub neutralizer: NeutralizerSpec,
}
```

Component identity must be path-stable and content-stable. Examples:

* `relation:planck_mu0_geff:param:geff_over_g`
* `input:planck_mu0_geff.mu0`
* `binding:bg.mu0<-relation:planck_mu0_geff`
* `term:planck_mu_parametrization`
* `claim:claim-suppressed-growth`
* `observable:fsigma8@0.51`
* `oracle:cmb_lA_anchor_calibration`

The graph builder should consume the same objects the scorecard already uses: `Theory`, `ClaimGraph`, obligations, `UnificationClaim`, `BindingReport`, `ForwardModel::manifest`, observables, covariance blocks, and scorecard. It must create explicit edges:

`certificate input -> relation -> derived parameter -> binding -> background field -> forward observable -> likelihood block -> data_fit`

and separately:

`claim -> obligation -> derivation_rigor`, `novel witness -> novelty`, `shared param -> unification`, `costed input/background drift -> parsimony and Occam k`.

The output is the `ComponentLedger`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentLedger {
    pub schema_version: u32,
    pub run_id: String,
    pub champion_id: String,
    pub champion_digest: String,
    pub source_tree_digest: String,
    pub full_trace: EvaluationTrace,
    pub components: Vec<TheoryComponent>,
    pub ablations: Vec<ComponentAttribution>,
    pub pairwise_interactions: Vec<PairwiseInteraction>,
    pub shapley_estimates: Vec<ShapleyEstimate>,
    pub gaps: Vec<GapRecord>,
    pub oracle_sensitivities: Vec<OracleSensitivity>,
    pub receipts: Vec<ReceiptRef>,
}
```

Use `serde` 1.x, `serde_json` 1.x, `schemars` 0.8 for schema generation, `sha2` 0.10 for digests, and `rayon` 1.10 for parallel ablation execution. The ledger must be JSONL-friendly for campaign logs and also emitted as canonical JSON for per-champion audit.

## 2. EvaluationTrace: per-observable accounting

Add `EvaluationTrace` as a scorecard-side artifact, not a dashboard-only postprocess:

```rust
pub struct EvaluationTrace {
    pub theory_id: String,
    pub theory_digest: String,
    pub scorecard: ScorecardV4,
    pub binding: BindingReport,
    pub forward_manifest: String,
    pub observables: Vec<ObservableTrace>,
    pub likelihood_blocks: Vec<LikelihoodBlockTrace>,
    pub dimension_points: BTreeMap<String, f64>,
    pub free_dof: u32,
}

pub struct ObservableTrace {
    pub observable_id: String,
    pub kind: String,
    pub observed: f64,
    pub uncertainty: f64,
    pub predicted: Option<f64>,
    pub baseline_predicted: Option<f64>,
    pub residual: Option<f64>,      // predicted - observed
    pub pull_sigma: Option<f64>,    // whitened when covariance block exists
    pub loglike_contribution: Option<f64>,
    pub covariance_block_id: Option<String>,
    pub upstream_components: Vec<ComponentId>,
}
```

`scorecard.rs::score_with_observables` should keep returning `ScorecardV4` for compatibility, but add `score_trace_with_observables` that returns `(ScorecardV4, EvaluationTrace)`. The covariance path in `crates/openqg-core/src/scoring/covariance.rs` and `likelihood.rs` must report whitened residuals and block contributions. The invariant is simple: summing `LikelihoodBlockTrace.delta_loglike` and applying the same Occam penalty must reproduce `DataFitOutcome.delta_lnz`; summing `dimension_points` must reproduce `ScorecardV4.total`.

No fake coverage remains mandatory. `ForwardModel::predict` already omits observables the model cannot compute. The trace should preserve omissions as `predicted: null` with a reason from the forward manifest, not impute a value.

## 3. Ablation engine

Add `crates/openqg-core/src/theory/ablation.rs` plus a bench CLI command, for example:

```text
zyal genome component-ledger \
  --champion ops/.../v7-smoke-proposal-ledger.jsonl:planck_mu0-suppressed-growth \
  --observables data/fixtures/cosmology/tier1-multisector.jsonl \
  --covariance data/fixtures/cosmology/covariance \
  --out component-ledger.json
```

Neutralization has two modes:

1. `mechanism_off_keep_cost`: remove the physical influence but keep the cost, claims, and bookkeeping. This answers “does the mechanism move evidence in the right direction?” Examples: set `mu0=0`, `drag_a=0`, `mg_family=none`, `fr_log10_fr0=-30`, or replace `G_eff/G` by the GR value `1.0`, while preserving the certificate input as a charged post-search choice.
2. `remove_component_reprice`: delete the component and recompute claims, obligations, novelty, unification, parsimony, and data fit. This answers “was the component only earning scorecard credit or reducing score through complexity?”

Neutralizers must be relation-aware and generated from a typed relation registry. For `planck_mu0_geff`, neutralize output to `G_eff/G=1` and bound `mu0=0`. For `dark_scattering_growth_drag`, neutralize `drag_a=0` while keeping the certified `w0`/`omega_de0` consistency checks. For a claim, remove only the claim and its downstream obligations in `remove_component_reprice`; in `mechanism_off_keep_cost`, claims remain so that physics effect is isolated.

Attribution sign convention:

```text
score_delta(c) = full_score - ablated_score
points_delta_dim(c,d) = full_points[d] - ablated_points[d]
pull_help(c,o) = abs(pull_ablated[o]) - abs(pull_full[o])
prediction_effect(c,o) = pred_full[o] - pred_ablated[o]
```

Positive `score_delta` means the component helps total score. Positive `pull_help` means the component reduced that observable’s absolute pull. Negative values are harmful.

Pseudocode:

```rust
fn component_ledger(input: ChampionInput, budget: AblationBudget) -> ComponentLedger {
    let full_trace = evaluate_trace(&input.theory, &input.context);
    let graph = build_component_graph(&input, &full_trace);
    let mut ablations = vec![];

    for c in graph.components.iter().filter(|c| c.neutralizer.is_supported()) {
        for mode in [MechanismOffKeepCost, RemoveComponentReprice] {
            let t2 = apply_neutralizer(&input, c, mode)?;
            let trace2 = evaluate_trace(&t2, &input.context);
            ablations.push(diff_component(c, mode, &full_trace, &trace2));
        }
    }

    let suspicious = rank_interaction_candidates(&ablations, budget.top_k_pairs);
    let pairwise = run_pairwise_ablations(&input, &graph, suspicious);
    let shapley = estimate_group_shapley(&input, &graph, budget.shapley_samples);
    let gaps = compute_gaps(&graph, &full_trace, &ablations, &pairwise, &shapley);
    ComponentLedger { full_trace, components: graph.components, ablations, pairwise_interactions: pairwise, shapley_estimates: shapley, gaps, .. }
}
```

Interaction effects must not be hand-waved. Pairwise interaction is:

```text
I(a,b) = score_delta({a,b}) - score_delta(a) - score_delta(b)
```

Flag `InteractionRisk` when `|I(a,b)| > 0.5 points` or `|I(a,b)| > 0.2 * max(|score_delta(a)|, |score_delta(b)|)`. Exact Shapley attribution is only required for grouped component sets of size `N <= 8`; otherwise use 64–256 deterministic permutation samples over component groups, not raw leaves. This follows the SHAP/Shapley attribution literature (Lundberg and Lee, 2017, arXiv:1705.07874) but must be represented as estimated cooperative-game attribution, not causal truth.

Compute cost: if a candidate has `N` components, single ablations cost `2N` full evaluations. Pairwise over top `K` components costs `K(K-1)/2`. Shapley over `G` groups and `M` permutations costs `M*G`. For V8, default to `N<=80`, `K=12`, `G<=12`, `M=128`; parallelize but require deterministic ordering and seed.

## 4. Gap dashboard and metrics

The dashboard is an attention router, not a score. It ranks “what to work on next.” Each `GapRecord` is content-bound and includes local context for focused re-proposal.

```rust
pub struct GapRecord {
    pub gap_id: String,
    pub component_id: ComponentId,
    pub gap_class: GapClass, // evidence, rigor, novelty, oracle, interaction, anti_gaming
    pub priority: f64,
    pub evidence_shortfall: f64,
    pub rigor_shortfall: f64,
    pub novelty_shortfall: f64,
    pub interaction_risk: f64,
    pub affected_observables: Vec<ObservableGap>,
    pub local_constraint_surface: ConstraintSurface,
    pub failed_attempt_refs: Vec<String>,
    pub recommended_patch_scope: PatchScope,
}
```

For observable `o`, define residual gap:

```text
residual_gap(o) = max(0, abs(pull_full(o)) - target_sigma(o))
```

with `target_sigma=1.0` by default, tightened for high-leverage CMB priors only after see S01 full-Boltzmann validation. For component `c`, use the ablation effect to decide whether the component helps or hurts:

```text
help(c,o) = abs(pull_ablated(c,o)) - abs(pull_full(o))
harm(c,o) = max(0, -help(c,o))
insufficiency(c,o) = max(0, residual_gap(o) - max(0, help(c,o)))
```

Evidence shortfall is assigned only to observables in the component’s declared downstream cone:

```text
EvidenceGap(c) = Σ_o cone_weight(c,o) * residual_gap(o) * [0.7*insufficiency_norm(c,o) + 0.3*harm_norm(c,o)]
```

`cone_weight` is derived from graph paths and normalized finite-difference sensitivity; direct binding-to-observable paths receive higher weight than vague sector claims. This prevents ranking `H0` as responsible for a growth residual simply because both live in the background object.

Rigor shortfall:

```text
RigorGap(c) = centrality(c) * (1 - verified_rigor(c))
```

`verified_rigor` is `1.0` for registry-verified numeric certificates with matching binding and action term, `0.5` for engine-refreshed but non-generating witnesses, `0.3` for weak certified phenomenology like the current V7 derivation ceiling, and `0.0` for prose/unobligated physics. `centrality` is downstream score weight plus observable count. A weak obligation that gates a high-impact relation outranks a weak claim that moves nothing.

Novelty shortfall:

```text
NoveltyGap(c) = distinct_effect(c) * witness_deficit(c)
```

`witness_deficit=1.0` when a component drives fit but has no computed, honest, out-of-fit witness; `0.75` when all witnesses are in the fit set; `0.5` for engine-refreshed-only witnesses; `0.0` for proposer-authored, computed, honest, out-of-fit witnesses. Existing V6 caps in `scorecard.rs` remain the scoring authority.

Overall dashboard priority:

```text
GapPriority(c) = 0.45*EvidenceGap(c) + 0.25*RigorGap(c)
               + 0.20*NoveltyGap(c) + 0.10*InteractionRisk(c)
```

The default weights intentionally overweight evidence because V7’s honest negative is an evidence-bar failure, but they do not replace the 100-point rubric.

## 5. Focused re-proposal protocol

Extend `proposer_sketch.rs` with a separate strict schema, not an overloaded whole-theory sketch:

```rust
pub struct FocusedPatchSketch {
    pub base_theory_digest: String,
    pub target_component_id: ComponentId,
    pub target_gap_id: String,
    pub frozen_digest: String,
    pub allowed_paths: Vec<String>,
    pub relation_signature: Option<RelationSignature>,
    pub local_context_digest: String,
    pub replacement: ComponentPatch,
    pub claims_delta: Vec<Claim>,
    pub obligations_delta: Vec<DerivationObligation>,
    pub evidence_delta: BTreeMap<String, String>,
    pub expected_observable_moves: Vec<ExpectedMove>,
    pub failed_attempts_seen: Vec<String>,
    pub no_changes_outside_allowed_paths: bool,
}
```

The prompt contract should include only local information:

* target component and stable digest;
* exact relation input-key contract from the typed registry;
* local upstream inputs and downstream observables;
* current residuals/pulls and finite-difference constraint surface;
* known failed attempts and their kill reasons;
* frozen sibling summaries, not the whole champion;
* strict JSON patch schema and the explicit warning that changing frozen paths is an automatic deterministic kill.

Example local objective for a growth relation:

```text
Target: relation:dark_scattering_growth_drag:param:Gamma0
Allowed outputs: cert.inputs[a_drag], obligation ob-drag-witness, novel witness for one non-fit growth observable.
Do not edit: background.w0, background.omega_m, background.h, sigma8, any sibling relation, any covariance/score config.
Constraint surface: increasing a_drag by +0.5 lowers fsigma8@0.51 by 0.006±0.001 but worsens cmb_lA by 0.15σ through current fitting formula.
Goal: reduce EvidenceGap without increasing OracleSensitivity(cmb_lA) or free_dof.
```

Merge-back path:

1. Load base champion by digest.
2. Canonicalize and hash all frozen paths.
3. Apply patch only to `allowed_paths`.
4. Verify frozen digests byte-identically.
5. Rebuild `ClaimGraph`; require DAG and no duplicate/renamed claim IDs unless declared in `claims_delta`.
6. Re-run all obligations that are in the target dependency closure plus all binding-relevant obligations.
7. Re-run full veto cascade, truth-binding, scorecard, novelty audit, evaluation trace, and local ablations.
8. Accept only if `GapPriority(target)` decreases by at least `δ_gap=0.25`, total score lower band does not fall, no sibling gap increases by more than `0.5`, and no anti-laundering kill fires.

Convergence criterion: per gap, stop after `W=24` failed focused attempts, `R=4` distinct accepted patches without further `δ_gap` improvement, or when the component is no longer in the top-10 dashboard gaps. For high-cost oracle or Boltzmann-backed components, use successive halving budgets rather than full scoring on every patch; final acceptance still requires full replay.

## 6. Campaign credit assignment and routing

Extend `ProposalAttemptRecord` in `theory_population.rs`:

```rust
pub struct ComponentAttemptCredit {
    pub gap_id: String,
    pub target_component_id: ComponentId,
    pub component_class: String,       // relation, binding, novelty_witness, oracle, etc.
    pub gap_class: GapClass,
    pub agent_id: String,
    pub mechanism_lane: String,
    pub mutation_operator: String,
    pub pre_gap_priority: f64,
    pub post_gap_priority: Option<f64>,
    pub pre_total: f64,
    pub post_total: Option<f64>,
    pub accepted: bool,
    pub kill_reasons: Vec<String>,
    pub ledger_digest: String,
}
```

Arms are tuples `(agent_or_quality_band, mechanism_lane, mutation_operator, component_class, gap_class)`. Reward:

```text
reward = clip(pre_gap - post_gap, -2, +2)
       + 0.25 * accepted
       + 0.10 * max(0, post_total - pre_total)
       - 0.50 * laundering_kill
       - 0.10 * parse_or_schema_failure
```

Use discounted UCB as the default router:

```text
ucb_arm = mean_reward_arm + c * sqrt(ln(total_pulls + 1) / pulls_arm)
```

with exponential decay `0.97` per generation for non-stationarity. Reserve budget: 60% top dashboard gaps by priority, 25% UCB exploration over under-sampled arms, 10% adversarial/null diagnostic lanes, 5% random mutation for escape. For many candidate patches on one gap, use successive halving/Hyperband: cheap schema+local-neutralizer checks first, medium full scorecard second, expensive pairwise/Shapley/oracle audit only for finalists. Cite Auer, Cesa-Bianchi, and Fischer (2002) for UCB; Jamieson and Talwalkar’s successive halving work (arXiv:1502.07943); Li et al. Hyperband (arXiv:1603.06560). If the evolutionary population still uses islands/MAP-Elites style diversity, keep that as an orthogonal diversity reservoir (Mouret and Clune, arXiv:1504.04909); do not let bandit exploitation collapse fingerprint diversity.

## 7. Anti-gaming kill rules and merge-back invariants

New vetoes should live beside `vetoes.rs` and merge-back verifier code, not in prompt prose.

* `FrozenDigestChanged`: any JSON pointer outside `allowed_paths` differs from the base digest.
* `OutputConeViolation`: target patch changes an observable outside its declared downstream cone beyond `max(0.1σ, 1e-6 absolute)` without a declared dependency update and full reviewer/auditor path.
* `JobShiftToSibling`: target gap improves only because a sibling component’s `score_delta` or `pull_help` changes by more than 50% while the target’s own neutralized effect remains weak.
* `DofLaundering`: new certificate input, background drift, witness tolerance, or post-search choice appears without increasing the same DOF ledger used by parsimony and Occam k.
* `ClaimGraphLaundering`: obligations/evidence are copied to new IDs to evade failed-attempt memory, or a target claim is removed while its score credit survives through a renamed sibling.
* `TermBindingMismatch`: a relation/binding earns credit without a generating term matching the registry; this generalizes the V6 survivor’s missing brane term.
* `OracleLeakage`: patch value matches redacted held-out oracle outputs or fixture constants beyond allowed local context precision.
* `ConstraintSurfaceOverfit`: patch improves only on ablation sample points but fails fresh local perturbations or holdout observables.
* `MechanismOffNoveltyCollapse`: novelty witness disappears under the mechanism-off twin, or is only an in-fit explanation already capped by `scorecard.rs`.

Required invariants: exact frozen path hash; single owner for each bound background field; exact relation input keys; no background-field alias mismatch; all affected obligations reverified; no decrease in recorded failed-attempt coverage; ledger digest included in the attempt receipt. These are Rust policy. TypeScript must not be able to waive them.

## 8. Decomposition as oracle audit

Oracle approximations are components too. Add `OracleComponent` variants for:

* CMB distance-prior fitting formulas and `cmb_lA` anchor calibration;
* `r_drag` approximations;
* growth ODE step size, `k_ref`, and MG response branches;
* covariance block choices and effective mode count in the Occam term;
* novelty tolerance clamps and fit-set membership;
* observable registry values and uncertainties.

For each champion, run deterministic perturbations:

```text
sensitivity(q) = max_delta |Score(full, q+δ) - Score(full, q)|
observable_leverage(q,o) = max_delta |pull_q+δ(o) - pull_q(o)|
band_flip(q) = total_band crosses promotion or rejection threshold under q perturbation
```

Store `OracleSensitivity { oracle_component_id, perturbation, score_delta, dimension_delta, observable_pull_delta, band_flip, support_level }` inside the same component ledger. This generalizes the V6 `l_A` failure into a standing audit job. Promotion should be blocked when any unsupported oracle approximation can change total score by more than `2.0` points, flip the sign of `delta_lnz`, or dominate the claimed component improvement. Full Boltzmann replacement is outside S04; see S01. S04’s hard requirement is the interface and CI audit now, so the full backend can plug in later as another oracle component with lower approximation risk.

## 9. Acceptance tests

Add these as Rust tests and replay fixtures. They must run under the mapped proof lane.

1. `ledger_reproduces_v7_planck_mu0_hand_attribution`: load the `planck_mu0-suppressed-growth` record from `ops/jailgun/next-level-payload/results/v7-smoke-proposal-ledger.jsonl`. The ledger must identify the relation, parameter, binding, term, claim, and novelty witness components. Mechanism-off ablation of `planck_mu0_geff` must move `fsigma8@0.51` from the `mu0` prediction toward the LCDM prediction in `paper/data/predictions.txt` (`0.469139 -> 0.474112` in the checked artifact) and report positive `pull_help` only where suppression helps the observed growth value.
2. `weakened_dark_scattering_ranked_top_gap`: create a fixture from the dark-scattering class but weaken `a_drag` by 90%. The dashboard must rank the drag relation/input component as the top evidence gap for `fsigma8@0.51`/growth residuals, while not blaming unrelated `H0` or CMB-only components.
3. `focused_patch_recovers_planted_synthetic_fix`: generate synthetic observables from `dark_scattering_growth_drag` with planted `a_drag=2.0`, seed a champion with `a_drag=0.2`, and provide only local context. A focused patch produced by a fake proposer must recover `a_drag` within 5% or reduce target `GapPriority` by at least 80%, with frozen digests unchanged.
4. `frozen_laundering_killed`: the same patch also edits `background.w0` or `sigma8`. The merge verifier must kill before scoring with `FrozenDigestChanged` and no ledger mutation.
5. `job_shift_to_sibling_killed`: target `mu0` remains neutralized but a sibling parameter changes to mimic its growth effect. Pairwise attribution must flag `JobShiftToSibling`.
6. `oracle_lA_bias_ranked_top`: inject a constant `cmb_lA` offset comparable to the historic V6 bias. Oracle audit must rank `oracle:cmb_lA_anchor_calibration` top and block promotion if data-fit sign or score band flips.
7. `bandit_routes_to_high_yield_component_class`: deterministic fake lanes repair known classes at known rates. Discounted UCB must route most budget to the high-yield arm while preserving exploration and null quotas.
8. `schema_relation_keys_single_source`: relation input keys are defined once in the registry. The proposer schema, prompt snippet, neutralizer, and focused patch verifier all update together in a golden snapshot.

## Implementation order

Milestone A: relation registry hardening, `ComponentId`, graph builder, `EvaluationTrace`, schema snapshots. Milestone B: single-component ablations and gap ranking. Milestone C: focused patch schema and frozen merge-back verifier. Milestone D: pairwise/Shapley interactions, anti-laundering tests, bandit routing. Milestone E: oracle perturbation audit and TypeScript dashboard renderer. Do not start dashboard UI until Milestone B produces a canonical ledger that a hostile reviewer can replay.

## What we got wrong

1. **“We already have decomposition” is oversold.** The current code has claim graphs and rubric components, not component-level causal attribution. Check: before this work, ask the engine which exact component caused the remaining `fsigma8@0.51` pull and what happens if only that relation is neutralized. It cannot answer from durable artifacts. Fix: Component Ledger plus EvaluationTrace; verification: V7 hand-attribution test.

2. **The V7 input-key contract is still partly prompt prose.** `proposer_sketch.rs` names exact keys in lane prompts and JSON Schema relation enums, but S04 needs typed relation signatures shared by prompts, certificates, neutralizers, and patches. Check: change `dark_scattering_growth_drag` input keys in one place and see whether every consumer updates. Fix: single `RelationSignature` registry; verification: golden schema/prompt/neutralizer test.

3. **Focused freezing is not enforceable with current recombination.** `mutation.rs::recombine` and `theory_population.rs::reclothe_candidate` are whole-theory operations. They are useful for search, but not safe for “repair one component.” Check: patch a target relation while also altering `w0`; current machinery would only catch this if some downstream physics veto happens. Fix: frozen JSON-pointer digests and merge-back verifier; verification: `FrozenDigestChanged` test.

4. **The term registry is a fraud firewall, not a derivation engine.** The paper admits the V7 term registry is an allowlist, and the V6 survivor’s missing brane term proves the danger. Check: remove or rename a generating term while leaving a passing certificate and see whether the dynamics are still formally generated. Fix for S04: term-binding mismatch as a kill and attribution to term components. Full term algebra/proof assistant is outside S04; see S02.

5. **The `cmb_lA` anchor may still be a brittle local patch.** The limitations section says the anchor offset is validated at the Planck point, not off-anchor derivatives. Check: perturb `h`, `omega_m`, and `w0` around the champion and compare `cmb_lA` pulls against a Boltzmann backend. Fix for S04: oracle approximation sensitivity ledger and promotion block on band flips. Full backend integration: see S01.

6. **V7’s “directionally right” growth story could be a data-corpus artifact.** `paper/data/predictions.txt` shows dark drag improves `fsigma8@0.51` but also shifts compressed CMB/BAO quantities; current growth data are sparse. Check: leave-one-growth-point-out and covariance-block perturbations must keep the same component ranked as a genuine gap/help, not flip randomly. Fix: per-observable attribution, holdout-aware gap ranking, oracle sensitivity, and bandit rewards based on post-gap replay rather than headline score.

7. **Agent credit is currently too coarse to learn from failures.** Existing attempt records capture source/model/lane/outcome, but not which component class improved or failed. Check: after 100 V7-style attempts, ask which model/operator is best at repairing novelty witnesses versus growth relations. The ledger cannot answer. Fix: `ComponentAttemptCredit` and UCB/successive-halving router; verification: deterministic routing simulation.
