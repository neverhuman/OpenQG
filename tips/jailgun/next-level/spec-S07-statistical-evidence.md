# S07 — Statistical Evidence Hardening Spec

Invocation: batch tab 1 of 1. Scope: OpenQG / ZYAL final-phase V8 statistics review. Deliverable is a root-level specification file, not a source patch.

## Ranked backlog

### 1. Make nested sampling the only promotion-grade evidence engine

**What.** Replace `ΔlnZ ≈ -0.5·ΔBIC` as a promotion statistic with a nested-sampling `EvidenceReceipt` containing log evidence, numerical error, priors, sampler diagnostics, posterior samples, goodness-of-fit summaries, and reproducibility checks. Keep the existing BIC/Schwarz output only as a cheap diagnostic.

**Why it matters.** The current league path advertises a “Schwarz/Laplace evidence approximation” in `crates/openqg-core/src/theory/league.rs:1-19` and computes `delta_ln_evidence: -0.5 * delta_bic` in `league.rs:398-440`. That is not referee-proof at `n=23` and `k≤4`. At `n=23`, one extra parameter costs only `0.5 ln(23)=1.57` nats under Schwarz. Widening a single flat prior by a factor of 10 changes real evidence by `-ln(10)=-2.30` nats while leaving BIC unchanged. For the actual V7 suppressed-growth/dark-scattering champion class, the extra `mu0` or dark-scattering dial lives exactly on this surface: one plausible prior-width choice can dominate the reported Bayes factor. The production artifact also reports a raw post-search log-likelihood improvement of `epsilon_delta_log_likelihood = 36.7399195` on only 15 observables (`ops/jailgun/.../prod-1000-champion.json:10-15`), while the fair-model-selection regression says that after refitting ΛCDM the apparent CPL gain collapses to “only a few” and Schwarz prefers ΛCDM (`league.rs:987-1043`). That discrepancy is not a detail; it is the statistical heart of the project.

**Effort.** L.

**Verification.** A hostile reviewer must be able to run `openqg evidence --model screened_mg --data tier1-multisector --seed 1 --sampler ultranest` and receive a JSON receipt whose `logz`, `logz_err`, `prior_hash`, `data_hash`, `likelihood_hash`, `sampler_version`, `ncall`, and posterior sample hash reproduce within two reported standard errors across two machines. A regression test must demonstrate that changing the `mu0` prior width by 10× shifts logZ by about `ln(10)` while BIC stays fixed; the CLI must refuse to call either value “promotion-grade” unless the prior was pre-registered.

### 2. Add a prior registry and anti-gaming prior discipline

**What.** Introduce a durable prior registry owned by the deterministic host, not by the LLM proposer. Priors must be pre-registered by mechanism class, parameter transform, support, density, rationale, source, and hash before any candidate in that class touches the data.

**Why it matters.** Auto-generated theories create an anti-cheating surface: a proposer can win evidence by narrowing a prior around a post-hoc optimum, or lose unfairly because a broad unphysical prior was assigned after the fact. The implemented `laplace_log_evidence` already proves prior-volume sensitivity in tests (`scoring/evidence.rs:362-372`), but the production path does not make prior ownership or prior volume a first-class receipt.

**Effort.** M.

**Verification.** A test suite must show that two candidates with identical likelihoods and different prior volumes receive different logZ values and identical profile likelihoods. The receipt must expose `log_prior_volume` and fail closed if any prior has `owner = LlmGenerated` or lacks a locked hash.

### 3. Run profile likelihood beside Bayesian evidence

**What.** Add a profile-likelihood lane that reports best-fit `χ²`, `Δχ²`, boundary-aware intervals, and sector pulls under the same forward model and covariance data. Use it as a mandatory companion, not as a replacement for evidence.

**Why it matters.** Cosmology referees split between Bayesian evidence and frequentist/profile likelihood. The current engine already does deterministic Nelder-Mead profile fits (`league.rs:262-336`) but then maps the fitted likelihood to BIC and calls it evidence. Boundary flags exist (`FitResult.boundary_hit`, `league.rs:249-258,307-336`), and tests confirm they fire when parameters pin to bounds (`league.rs:878-910`), but `scorecard.rs:535-545` only widens the data-fit score band; it does not invalidate the statistical claim. Worse, evolved theories set `boundary_hit: false` because their values are fixed, not fitted (`openqg-bench/src/zyal_genome/physics_score.rs:112-114`). A fixed post-search value can still be a boundary exploit.

**Effort.** M.

**Verification.** Add a synthetic one-sided problem where the true optimum is at `mu0=0`, with support `mu0≤0`. The profile-likelihood p-value must use a Chernoff/Self-Liang mixture or simulation, not Wilks’ interior `χ²_1`. The promotion gate must label any evidence/profile disagreement as `statistically_unstable` until resolved.

### 4. Install absolute goodness-of-fit gates before “beats ΛCDM” is reportable

**What.** Add GoF gates: covariance-respected `χ²/dof`, global p-value, posterior-predictive p-values, per-sector pulls, and covariance-block residual diagnostics. If the absolute fit is bad, `ΔlnZ` may be stored internally but cannot be used in paper text, leaderboard copy, or release receipts.

**Why it matters.** A candidate can beat ΛCDM while both models fit terribly. The current `DataFitOutcome` has `delta_aic`, `delta_lnz`, coverage, boundary, likelihood mode, and covariance block count (`scorecard.rs:42-59`), but no absolute-fit p-value or posterior-predictive check. The paper says the likelihood is covariance-aware for DESI/Planck blocks and diagonal otherwise (`paper/main.tex:291-299`); that is necessary but not sufficient.

**Effort.** M.

**Verification.** A candidate with `ΔlnZ=+6` and global `p<0.01` must be marked `fit_failed_not_reportable`. A candidate with one sector pull above `3σ` must carry a sector-specific warning in the receipt and fail promotion unless the pull is a pre-registered prediction tested on sealed data.

### 5. Replace alternating holdout with sealed replication and rank-stability tests

**What.** Retire `alternating_holdout(n)` as a scientific generalization statistic. Use it only as a smoke test. Add sealed data tiers, sector jackknife, block bootstrap, and post-search rank-stability receipts.

**Why it matters.** The holdout code selects odd indices deterministically (`theory/holdout.rs:106-109`) and evaluates held-out likelihood with the diagonal scorer (`holdout.rs:44-60`). With `n=23`, alternating records can split correlated survey blocks, leak BAO/CMB structure, and produce a noisy per-point train/test gap. It is not a meaningful estimate of out-of-sample cosmological generalization.

**Effort.** M.

**Verification.** A rank-stability test must resample by survey sector/block, not by individual rows. A champion is stable only if it remains rank 1 or statistically tied for rank 1 in at least 80% of sector jackknifes and keeps the same sign of `ΔlnZ` in at least 90% of bootstrap replicates.

### 6. Add look-elsewhere and search-trials accounting

**What.** Treat the champion’s statistic as the maximum of a search over hundreds or thousands of proposals. Maintain a `SearchLedger` with every scored and vetoed candidate, structural fingerprint, prior hash, data version, and score. Estimate an effective-trials correction from null simulations and structural clustering.

**Why it matters.** The honest project history says every era’s champion was audited to destruction. That history is evidence of heavy adaptive search. The current production artifacts include 1000 generations and a population of 48 (`prod-1000-champion.json:89-105`), but the reported number is not adjusted for the fact that the champion is selected after a search.

**Effort.** L.

**Verification.** Under synthetic ΛCDM data generated from the exact covariance registry, the full proposer/evaluator/selector loop must be replayed at least 200 times. The reported post-search p-value is `Pr(max ΔlnZ_sim ≥ observed ΔlnZ)`. A candidate cannot claim “discovery” or “exclusion” until the post-search statistic, not the per-candidate statistic, passes the gate.

### 7. Finish `laplace_log_evidence` as a diagnostic, not a headline engine

**What.** Keep `scoring/evidence.rs` but change its public contract. It may provide a fast analytic diagnostic and preflight check; it must not supersede nested sampling for quoted evidence. Wire it into receipts as `LaplaceDiagnostic`, with explicit validity flags.

**Why it matters.** The file says the module “supersedes BIC for any quoted model evidence” (`scoring/evidence.rs:10-18`), but `rg` finds `laplace_log_evidence` only in that file and tests, not in the production league. Its own comments correctly note BIC drops the Occam prefactor and mis-ranks near-degenerate extensions. The right verdict is **finish, but demote**. Deleting it would lose useful Gaussian sanity tests; promoting it would reproduce the same asymptotic confidence problem under a different name.

**Effort.** S.

**Verification.** Analytic Gaussian tests must pass within `0.1` nat. Banana-shaped, multimodal, and boundary-pinned posteriors must set `laplace_valid=false`. If `|logZ_laplace - logZ_nested| > 1.0` nat, any headline evidence claim must cite nested sampling only and include the disagreement as a diagnostic warning.

### 8. State the n=23 claim boundary in paper and receipts

**What.** Add explicit claim classes: `triage`, `interesting_fit`, `promotion_candidate`, `discovery_claim`, and `exclusion_claim`. At `n=23`, the default allowed class is `triage` or `interesting_fit`; `discovery` and broad `exclusion` require sealed replication and larger likelihoods.

**Why it matters.** `data/fixtures/cosmology/tier1-multisector.jsonl` has 23 rows; `tier0-combined.jsonl` has 15. The paper already admits compressed likelihoods and missing full Boltzmann/CMB/SNe machinery in the limitations section (`paper/main.tex:652-665`), but several scoring and narrative claims still dress these numbers as stronger than the data scale supports.

**Effort.** S.

**Verification.** Paper build tests must fail if a `ΔlnZ` table row lacks claim class, evidence method, GoF status, trials correction, and data tier. V8 release receipts must refuse the words “discovery”, “excluded”, or “decisive” unless the class-specific gates pass.

## Current path audit: where a hostile referee will attack

The current code base has made real progress compared with the internal critique. `docs/zyal-next-level-design.md:76-95` correctly attacks the old diagonal, fixed-baseline, no-complexity-penalty score. The present source now contains covariance-aware Gaussian scoring (`scoring/covariance.rs:1-18,150-245`), DESI/Planck covariance registry checks (`covariance.rs:261-270`), a fairer model league (`theory/league.rs:1-19`), a `DataFitOutcome` with likelihood mode and covariance block count (`scorecard.rs:42-59`), and an effective-mode count (`covariance.rs:489-515`). Those are good engineering moves. They are not yet enough.

The central failure is that a profile best fit is still being turned into evidence with an asymptotic proxy. In `model_league`, every model is optimized by multistart Nelder-Mead and then compared by AIC, BIC, and `delta_ln_evidence = -0.5 * delta_bic` (`league.rs:262-336,408-440`). Schwarz/BIC assumes regular interior parameters, large-sample asymptotics, and a local Gaussian posterior whose prior density is tame near the maximum. The OpenQG problem violates all three: `n=23`, posteriors can be non-Gaussian and correlated through compressed cosmological observables, and the most relevant new dials are often bounded physical effects (`mu0`, screening strength, dark scattering, `f_R0`, `Ω_rc`).

Quantitatively, the Schwarz penalty at the actual data scale is weak and prior-blind. With 23 observables, a one-parameter extension is penalized by only 1.57 nats relative to the same likelihood. A real evidence calculation would penalize the prior volume; expanding one flat prior by 10× costs 2.30 nats. Therefore, for the suppressed-growth/dark-scattering champion class, the prior volume of the additional mechanism parameter is already larger than the Schwarz model penalty. This is not a philosophical objection. It means a hostile referee can change a defensible prior width and flip a marginal Bayes factor without changing any prediction. The current receipts do not prove that this did not happen.

The second failure is boundary treatment. The code recognizes boundary-pinned optima (`boundary_hit`) but does not make them fatal. A boundary optimum is not a minor display issue. Standard likelihood-ratio asymptotics can fail; intervals and p-values need mixture distributions or simulation. This is familiar in cosmology and astroparticle pipelines: see Chernoff (1954), Self & Liang (1987), Protassov et al. (2002, ApJ 571, 545), and the recent cosmology profile-likelihood review by Herold, Ferreira, and collaborators, “Profile Likelihoods in Cosmology: When, Why and How” (arXiv:2408.07700). If a candidate’s improvement exists only because `mu0`, `f_R0`, or a screening parameter sits on a prior boundary, V8 must classify it as unresolved rather than promoted.

The third failure is search selection. A champion is the maximum of a long adaptive process, not a pre-specified one-off model. The evidence should be reported as a post-search statistic over the selected search volume. `prod-1000` explicitly records 1000 generations and 48 population members; the V7 smoke ledger records a suppressed-growth `planck_mu0` proposal that is disqualified but structurally representative of the standing champion class. The current scoring does not estimate the null distribution of the maximum score after this adaptive loop. That omission makes any “best so far” headline optimistic.

The fourth failure is GoF. A relative Bayes factor can be positive for the less-bad of two bad models. `DataFitOutcome` stores deltas, not absolute acceptability. The paper’s limitations already admit compressed likelihoods, no Boltzmann backend, and missing full survey covariance. V8 must refuse to publish “beats ΛCDM” unless the candidate passes absolute-fit gates under the exact covariance structure.

## Evidence backbone: nested sampling by process boundary

### Recommended backend choice

Default promotion backend: **UltraNest 4.5.x**. UltraNest’s reactive nested sampling is the best default here because OpenQG has low dimension (`k≤4` or modestly higher after Boltzmann integration), potentially curved degeneracies, and a need for robust diagnostic output and resumption. Cite Buchner (2021), “UltraNest — a robust, general purpose Bayesian inference engine” (arXiv:2101.09604).

Secondary cross-check backend: **dynesty 3.0.0**. Dynesty is pure Python, widely used, and fast for small-dimensional problems. It is excellent for CI, developer checks, and independent confirmation of UltraNest receipts. Cite Speagle (2020), “dynesty: a dynamic nested sampling package for estimating Bayesian posteriors and evidences” (arXiv:1904.02180).

High-dimensional / slow-likelihood backend: **PolyChord / PolyChordLite**. PolyChord’s slice-sampling nested sampler is appropriate when V8 connects a Boltzmann backend or higher-dimensional nuisance hierarchy. Cite Handley, Hobson, and Lasenby (2015), arXiv:1502.01856 and arXiv:1506.00171.

Pure-Rust option: **not credible as promotion-grade in V8** unless the team is willing to build and validate a nested sampler as a research project. Rust should own durable policy, configs, data validation, and receipts; Python/Fortran can own the sampler behind a sealed process boundary.

### Process contract

Rust creates an `EvidenceRequest` JSON file and launches a pinned sampler environment with no network and a fixed seed. The child process calls back into a deterministic likelihood service or loads a compiled forward-model binary. The child returns `EvidenceReceipt` plus posterior samples. Rust validates hashes, sampler diagnostics, and GoF gates, then signs the receipt.

```rust
#[derive(Serialize, Deserialize)]
pub struct EvidenceRequest {
    pub run_id: String,
    pub model_id: String,
    pub model_fingerprint: String,
    pub data_manifest_hash: String,
    pub likelihood_manifest_hash: String,
    pub prior_registry_hash: String,
    pub parameters: Vec<ParameterSpec>,
    pub priors: Vec<PriorSpec>,
    pub sampler: SamplerSpec,
    pub rng_seed: u64,
    pub max_likelihood_calls: u64,
    pub walltime_seconds: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PriorSpec {
    pub name: String,
    pub transform: PriorTransform,       // Linear, Log10Abs, LogitUnit, DerivedFixed
    pub support: [f64; 2],
    pub density: PriorDensity,           // Uniform, LogUniform, GaussianExternal, Mixture
    pub owner: PriorOwner,               // HumanPreRegistered, RegistryExternal, LegacyFrozen
    pub rationale: String,
    pub citations: Vec<String>,
    pub locked_sha256: String,
}

#[derive(Serialize, Deserialize)]
pub struct EvidenceReceipt {
    pub model_id: String,
    pub data_tier: String,
    pub logz: f64,
    pub logz_err: f64,
    pub delta_logz_vs_lcdm: f64,
    pub delta_logz_err: f64,
    pub information_h: f64,
    pub ncall: u64,
    pub nlive: u64,
    pub posterior_ess: f64,
    pub boundary_mass: Vec<BoundaryMass>,
    pub gof: GoodnessOfFitReceipt,
    pub sector_pulls: Vec<SectorPull>,
    pub sampler: SamplerSpec,
    pub reproducibility: ReproducibilityReceipt,
    pub warnings: Vec<EvidenceWarning>,
}
```

Acceptance defaults: `nlive >= max(400, 100*k)`, `dlogz_target <= 0.1` for finalists, `posterior_ess >= 2000`, two independent seeds agree within `2 sqrt(err_1^2 + err_2^2)` and within an absolute 0.5 nat, and no single covariance block contributes an unexplained catastrophic pull. Development runs can use looser settings but must be marked `diagnostic_only`.

Runtime budget for today’s cheap FLRW forward model should be small: target 10,000–50,000 likelihood calls per model, less than two minutes per model on a workstation and less than one hour for a full finalist league. When S05/S08 add Boltzmann likelihoods, the same interface supports cached emulators or PolyChord with speed hierarchies. The sampler process must emit incremental JSONL telemetry so stalled likelihoods do not silently hang ZYAL.

## Prior discipline and anti-cheating policy

A prior is part of the scientific claim. For auto-generated theories, the LLM may propose a parameter, but it may not set its own prior. Priors must be assigned by deterministic policy from a registry keyed by mechanism class.

Mechanism-class examples:

```yaml
class_id: screened_mu0_growth_v1
parameters:
  - name: mu0
    transform: linear
    support: [-0.5, 0.5]
    density: uniform
    owner: HumanPreRegistered
    rationale: "Phenomenological Planck-mu0 growth modification; broad enough to include no-effect and current viable suppression/enhancement."
    citations: ["Planck 2018 modified gravity", "arXiv:1504.05481"]
```

The registry must distinguish structural parameters from nuisance/background parameters. If a background coordinate is refit for every model (`h`, `Ω_m`, `σ8`), the baseline must receive the same opportunity. If a generated theory claims a parameter is “derived,” the evidence path must either fix it from a verified certificate independent of the fit data or count the upstream free parameter that determines it. This plugs the historical V6 survivor exploit: a costless certified β with no generating brane term.

Anti-gaming gates:

1. A candidate cannot narrow a prior after seeing data.
2. A prior update creates a new league epoch; old and new evidence numbers are not comparable without re-running all baselines.
3. Evidence receipts must print `log_prior_volume` by parameter and total.
4. Derived-fixed parameters require a `DerivationReceipt` and an upstream free-DOF accounting record.
5. Any prior whose lower or upper bound is hit by more than 1% posterior mass triggers `boundary_mass_warning`.

## Profile likelihood lane

Run profile likelihood because it answers a different question: “Can this model fit better at any point in its allowed space?” Evidence answers: “Does the model class predict the data after paying prior volume?” Both are useful; neither should impersonate the other.

Profile receipt:

```rust
pub struct ProfileLikelihoodReceipt {
    pub chi2_min: f64,
    pub delta_chi2_vs_lcdm: f64,
    pub dof_nominal: i32,
    pub optimizer: OptimizerSpec,
    pub best_fit: BTreeMap<String, f64>,
    pub boundary_hit: Vec<String>,
    pub intervals: Vec<ProfileInterval>,
    pub asymptotic_validity: ValidityFlag,
    pub simulation_calibrated_p: Option<f64>,
}
```

Use the current deterministic multistart Nelder-Mead as the minimum viable optimizer, but add cross-checks with `iminuit`/Minuit2 for smooth problems and differential evolution for rough surfaces. If any parameter touches a bound, the receipt must not use Wilks’ theorem by default. Use Chernoff mixtures for simple one-sided parameters and parametric bootstrap for complex cosmology boundaries.

Promotion rule: Bayesian evidence gates model-class promotion; profile likelihood gates “interesting-fit” claims. If profile improves strongly but evidence is negative, the candidate is an over-flexible or prior-volume-expensive fit and may be archived as `interesting_fit_not_promoted`. If evidence is positive but profile does not show a stable best-fit improvement, the receipt is suspicious and requires sampler diagnostics or likelihood-bug review.

## Goodness-of-fit battery

The absolute-fit gate must run before any relative statement is reportable.

Global gate:

- `coverage == 1.0` for all observables in the declared tier.
- Covariance blocks must be positive-definite and used; fallback to diagonal downgrades the claim class.
- `χ²/dof <= 1.5` and global `p >= 0.01` for promotion.
- Posterior-predictive global p-value in `[0.01, 0.99]`.

Sector gate:

- Sectors: BAO, CMB distance priors, RSD growth, SH0ES H0, weak-lensing S8, BBN.
- Compute sector `χ²`, sector p-value, largest standardized residual, and signed pull.
- Any sector p-value below `0.003` fails promotion unless that sector was sealed and the discrepancy is the pre-registered novel prediction.

Pseudocode:

```text
for posterior_sample theta_s:
    y_rep_s ~ Normal(pred(theta_s), C)
    T_rep_s = block_chi2(y_rep_s, pred(theta_s), C)
    T_obs_s = block_chi2(y_obs, pred(theta_s), C)
ppp_global = mean(T_rep_s >= T_obs_s)
ppp_sector[j] = mean(T_rep_s[j] >= T_obs_s[j])
```

The receipt must make the covariance structure visible: DESI anisotropic BAO blocks and Planck distance-prior blocks are not cosmetic. If a model only wins by exploiting one diagonal fallback sector, it cannot be promoted.

## Selection effects, bootstrap stability, and sealed replication

Every scored proposal must enter a search ledger:

```rust
pub struct SearchLedgerRow {
    pub generation: u64,
    pub candidate_id: String,
    pub structural_fingerprint: String,
    pub mechanism_class: String,
    pub prior_hash: String,
    pub data_hash: String,
    pub status: CandidateStatus,       // Vetoed, FitFailed, Diagnostic, EvidenceRun, Promoted
    pub delta_logz: Option<f64>,
    pub delta_logz_err: Option<f64>,
    pub gof_pass: bool,
}
```

The trials correction must be empirical. Generate synthetic data under the registered ΛCDM baseline using the same covariance blocks. Run the same proposer/evaluator/selector budget, including vetoes and prior policy, on each simulation. The post-search p-value is the fraction whose maximum promotion statistic exceeds the real champion. If full LLM proposal replay is too expensive, use the archived structural fingerprints and mutation operators to replay a fixed search skeleton; do not pretend a single-candidate Bayes factor is post-search evidence.

Rank stability must be block-aware. Do not resample individual rows from the 23-vector. Use sector jackknife and block bootstrap: leave out BAO, CMB priors, RSD, H0, S8, or BBN; resample DESI tracer blocks as units; resample posterior samples inside each evidence receipt. A champion that only wins when SH0ES is included, or only wins when S8 is included, is not a general cosmology champion. It is a tension-targeting phenomenology candidate and must be labeled that way.

Sealed replication protocol:

1. Discovery tier is frozen: model grammar, priors, sampler, and champion fingerprint locked.
2. The LLM may no longer mutate the champion.
3. A new data tier is unsealed and loaded through the same registry path.
4. Evidence and GoF are recomputed once, with no prior tuning.
5. The replication receipt, not the discovery receipt, controls paper claims.

The existing `alternating_holdout` should remain as a regression smoke test only. It is deterministic, small, and diagonal; it should not appear in scientific text as a generalization guarantee.

## The n=23 reality check

At 23 compressed observables, OpenQG can honestly do three things: triage mechanisms, find suspicious failure modes, and generate candidates for sealed replication. It cannot honestly claim a decisive discovery from the current league alone. It also cannot broadly exclude a physical mechanism class unless the class is very narrowly defined and the exclusion is conditioned on the exact searched prior volume.

Published or draft numbers that need downgrade language:

- Any table entry that reports `ΔlnZ` from `-0.5 ΔBIC` should be labeled `Schwarz diagnostic`, not evidence.
- Any claim that a V7/V8 champion “beats ΛCDM” must include whether ΛCDM was refit with the same nuisance/background freedom.
- Any robustness claim based on alternating holdout should be replaced with sector jackknife, block bootstrap, and sealed replication.
- Any “coverage 1.0” claim must say coverage of which observables, not cosmology coverage. The internal critique already notes that narrow FLRW coverage is not full physical coverage (`docs/zyal-next-level-design.md:106-114`).

Minimum data scale for desired claims, to feed S05: full Pantheon+/Union-style SNe likelihoods with covariance, DESI DR2 or later BAO/full-shape/RSD likelihoods, Planck/ACT/SPT high-ℓ or compressed likelihoods with documented covariance, weak-lensing two-point likelihoods, CMB lensing, and independent sealed data not touched by proposal search. The data count should be hundreds to thousands of correlated modes, with nuisance parameters and covariance handled explicitly. Even then, auto-generated-theory search needs trials correction.

## Exclusion statement machinery

The desired sentence is not “we exclude this mechanism class” in the abstract. The defensible sentence is:

> Under mechanism class `C`, grammar `G`, prior measure `π`, data tier `D`, forward backend `F`, and search budget `B`, no candidate achieved GoF pass and post-search `ΔlogZ` above threshold `τ`; injection tests show power `≥q` to detect effects with strength `s` over volume `V`. Therefore OpenQG excludes improvements of that strength in that searched volume, subject to the listed caveats.

Required components:

1. **Class definition.** A machine-readable grammar of allowed terms, parameters, transforms, screening rules, stability constraints, and forward-model obligations.
2. **Search volume measure.** Discrete structural prior over grammar choices times continuous parameter priors. The measure must be fixed before search.
3. **Strength statistic.** For suppressed growth, use a physical effect statistic such as `max_z∈[0,1.5] |Δfσ8(z)|/σ_obs(z)` or `max_z |G_eff/G - 1|`, not just parameter magnitude. For dark scattering, define a drag-rate or growth-suppression envelope.
4. **Detection power.** Inject mechanisms at strength `s` into synthetic data, run the full search, and estimate the probability that the pipeline recovers a GoF-passing, trials-corrected evidence signal.
5. **Caveats.** Condition on the compressed data tier, prior measure, forward backend, and finite search budget.

Without injection power, the strongest honest null is: “No promotable candidate was found in the searched volume.” That is valuable. It is not an exclusion.

## Implementation acceptance lane

Add `openqg evidence-league` with three modes:

- `--mode diagnostic`: BIC, Laplace diagnostic, current profile fit; CI-friendly.
- `--mode promotion`: UltraNest evidence, GoF battery, profile receipt, two-seed reproducibility.
- `--mode replication`: sealed data only; mutation/proposal disabled.

Golden tests:

1. Analytic Gaussian evidence agrees with closed form and prior widening shifts logZ by `ln(width_ratio)`.
2. Banana posterior makes Laplace invalid but nested sampling stable.
3. Boundary optimum invalidates Wilks p-values and sets `boundary_mass_warning`.
4. ΛCDM synthetic replay yields uniform posterior-predictive p-values and calibrated post-search p-values.
5. The V7 suppressed-growth smoke class cannot be promoted without a registered `mu0` prior and a nested evidence receipt.
6. The `prod-1000` artifact’s raw `epsilon` cannot be displayed as evidence; the CLI must call it a raw post-search likelihood improvement unless a fair baseline refit and trials correction exist.

## Literature references to cite in the code and paper

- Schwarz, G. 1978, “Estimating the Dimension of a Model,” Annals of Statistics.
- Skilling, J. 2004/2006, nested sampling.
- Trotta, R. 2008, “Bayes in the sky,” Contemporary Physics 49, 71, arXiv:0803.4089.
- Speagle, J. 2020, “dynesty,” MNRAS, arXiv:1904.02180.
- Buchner, J. 2021, “UltraNest,” JOSS, arXiv:2101.09604.
- Handley, W., Hobson, M., Lasenby, A. 2015, PolyChord, arXiv:1502.01856 and arXiv:1506.00171.
- Higson et al. 2018, nested-sampling diagnostics / `nestcheck`, arXiv:1804.06406.
- Chernoff, H. 1954; Self & Liang 1987; Protassov et al. 2002, boundary likelihood-ratio failures.
- Herold et al. 2025, “Profile Likelihoods in Cosmology: When, Why and How,” arXiv:2408.07700.
- Gelman, Meng, and Stern 1996, posterior predictive assessment.
- Gross and Vitells 2010, look-elsewhere effect, Eur. Phys. J. C 70, 525.

## What we got wrong

1. **We treated a Schwarz proxy as evidence.** The code names `ΔlnZ ≈ -0.5·ΔBIC` as evidence, but at `n=23` it is a diagnostic. The check that settles it is a nested-sampling evidence receipt for every finalist and the ΛCDM baseline, under pre-registered priors.

2. **We underpriced prior choice.** A 10× prior-width change moves real logZ by 2.30 nats and BIC by zero. The check that settles it is a prior-sensitivity table for `mu0`, dark-scattering drag, `log10|f_R0|`, and `Ω_rc`, with prior hashes and rationale.

3. **We made boundary flags too soft.** `boundary_hit` currently widens a score band; it should block promotion-grade evidence until boundary asymptotics are handled. The check that settles it is a boundary-calibrated profile-likelihood simulation.

4. **We confused post-search best fit with pre-specified evidence.** A champion selected after 1000 generations is a maximum statistic. The check that settles it is a null replay of the same search budget and a post-search p-value.

5. **We overread alternating holdout.** Alternating 23 rows, evaluated diagonally, is not a cosmology holdout. The check that settles it is sealed replication plus sector jackknife and block bootstrap rank stability.

6. **We let relative fit speak before absolute fit.** A positive `ΔlnZ` is meaningless if both models fail GoF. The check that settles it is a GoF receipt with global and sector posterior-predictive p-values.

7. **We overstated what 23 observables can prove.** The current data tier can prioritize and falsify obvious cheats, not establish discovery or broad exclusion. The check that settles it is replication on new, larger likelihoods with full covariance and nuisance handling.

8. **We used “exclusion” before defining a searched volume.** An honest null is not automatically an exclusion. The check that settles it is a class grammar, prior/search-volume measure, effect-strength statistic, and injection-tested power curve.

9. **We left `laplace_log_evidence` in a misleading middle state.** The code says it supersedes BIC, but production does not use it. The check that settles it is to wire it as a diagnostic with validity flags and require nested sampling for public evidence.

The V8 statistical standard should be simple: no prior hash, no evidence; no GoF pass, no “beats ΛCDM”; no trials correction, no champion claim; no sealed replication, no discovery; no class volume and injection power, no exclusion.
