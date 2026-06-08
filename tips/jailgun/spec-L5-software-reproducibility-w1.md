# Ranked backlog — highest leverage first

This review is based on the extracted source snapshot, not filenames or prior project memory. The source-only archive contains the 31 selected files, not a runnable full workspace; I therefore reviewed the actual docs and Rust modules but did not independently execute the claimed 172-test lane. Treat the existing 172-pass statement in `docs/architecture.md:288-298` as a claim that now needs an attached replay receipt.

1. **Fix the growth/MG league parameter setter before quoting any growth result.**  
   **What:** Extend `set_param()` in `crates/openqg-core/src/theory/league.rs:48-59` to apply `sigma8` and `mu0`, or replace the string setter with a typed enum that cannot omit fields. `ModelClass::lcdm_growth()`, `w0wa_cdm_growth()`, and `screened_mg()` advertise free `sigma8` and `mu0` in `league.rs:124-154`, but those names are currently ignored by the setter.  
   **Why it matters:** The docs say growth data makes `sigma8` free and tests modified growth through `mu0` (`docs/theory-league.md:20-26`, `:34-36`, `:78-82`). As written, the optimizer can vary coordinates that never reach the forward model. That invalidates any growth/MG league interpretation.  
   **Effort:** S.  
   **How to verify:** Add a synthetic `fsigma8`/`S8` dataset generated with known `sigma8` and known `mu0`; assert `fit_model()` recovers both within tolerance and that `screened_mg` changes log-likelihood when `mu0` changes. Add a regression test that fails if every `FreeParam.name` is not handled.

2. **Create a one-command independent replay package for the headline league.**  
   **What:** Ship `replay.sh`, `Cargo.lock`, source commit, compiler/toolchain lock, data lockfile, covariance files or hashes, exact model list, expected JSON outputs, and a CI job that re-derives the published league numbers.  
   **Why it matters:** The docs ask reviewers to reproduce `theory league` (`docs/architecture.md:288-297`, `docs/theory-league.md:42-56`), but the command references `data/fixtures/cosmology/wl-s8.jsonl`, which is absent from the snapshot, and the 1000-gen runbook points to `data/fixtures/tension/observables.jsonl` (`ZYAL/runs/run-hybrid-1000-v2.zyal:27-31`), also absent here. A referee cannot accept a result whose replay inputs are not sealed.  
   **Effort:** M.  
   **How to verify:** Fresh checkout plus `./replay.sh` produces byte-stable or tolerance-stable `league.json`; CI fails if row count, source hash, covariance hash, best-fit parameters, χ², ΔAIC, or ΔlnZ drift outside declared tolerances.

3. **Build a solver-truth comparison suite against CLASS/CAMB/hi_class.**  
   **What:** For a grid of ΛCDM, wCDM, w0waCDM, curvature, neutrino, and MG-like parameters, compare OpenQG predictions for distances, `r_drag`, `z_*`, `R`, `l_A`, `fσ8`, and `S8` against reference solvers.  
   **Why it matters:** `BackgroundForwardModel` is intentionally pure Rust and deterministic (`docs/architecture.md:111-151`), but a referee will not trust new cosmology numbers without independent solver agreement.  
   **Effort:** M for background/growth; L once hi_class MG and full CMB are included.  
   **How to verify:** Golden tables with per-observable tolerances, signed reference input decks, and CI comparing every supported observable. Publish both absolute and fractional residuals.

4. **Attach numerical convergence/error budgets to Simpson and RK4 outputs.**  
   **What:** Replace fixed, undocumented numerical-error floors with measured convergence estimates. `comoving_distance()` uses fixed 2048-panel Simpson integration (`background.rs:131-138`); `sound_horizon()` uses 8192 panels (`background.rs:187-198`); growth uses fixed 512-step RK4 (`growth.rs:23-27`, `:68-119`).  
   **Why it matters:** The code claims sub-percent or sub-0.1% behavior, but model-selection differences are often at the Δχ²~few level. Numerical error must be quantified and propagated.  
   **Effort:** M.  
   **How to verify:** Run step-doubling convergence tests, analytic-limit tests, and error propagation into likelihood. CI should fail if a prediction’s numerical error exceeds a registered fraction of the observational uncertainty.

5. **Populate real covariance matrices and make covariance provenance first-class.**  
   **What:** Add DESI intra-tracer BAO, Planck distance-prior, Pantheon+/SH0ES, and growth covariance blocks with source manifests. Validate symmetry, dimensions, positive definiteness, condition number, units, and observable order.  
   **Why it matters:** The code has a Cholesky block likelihood (`scoring/covariance.rs:78-149`), but the committed cosmology fixtures are diagonal (`data/fixtures/cosmology/*.jsonl`). The docs correctly admit diagonal runs (`docs/architecture.md:269-271`, `docs/theory-league.md:83-86`); until fixed, quoted Δχ² is provisional.  
   **Effort:** M.  
   **How to verify:** Unit tests compare `rᵀC⁻¹r` against a trusted numerical package for random SPD matrices; fixture tests validate every real covariance block and prove diagonal/block equivalence on diagonal covariances.

6. **Make coverage a league eligibility rule, not a soft side effect.**  
   **What:** In `score_metrics_cov()`, missing predictions inside covariance blocks are marginalized (`covariance.rs:180-218`) and missing independent predictions only reduce coverage/findings (`covariance.rs:221-260`). Define comparison classes: either all league models predict the same registered observable set, or a lower-coverage model is ineligible for the same AIC/BIC table.  
   **Why it matters:** “Omit what you cannot derive” is honest for reporting. It is not automatically fair model selection: a model could avoid a difficult observable and pay only a weak coverage penalty while AIC/BIC is computed over `data.observables.len()` (`league.rs:233-252`).  
   **Effort:** S-M.  
   **How to verify:** Add tests where a model omits a high-tension point and prove it cannot outrank a full-coverage model in the same league unless explicitly placed in a lower-coverage table.

7. **Add analytic-limit physics tests.**  
   **What:** Test Einstein-de Sitter growth (`D∝a`, `f=1`), de Sitter late-time suppression, flat-ΛCDM distance identities, curvature small-Ωk expansions, high-z radiation behavior, and physical rejection of negative `E²`.  
   **Why it matters:** Existing tests mostly check broad observed bands (`background.rs:330-386`, `growth.rs:170-224`). Band tests catch gross mistakes but not subtle sign, normalization, or derivative errors.  
   **Effort:** S-M.  
   **How to verify:** Exact or high-precision analytic fixtures with tight tolerances; property tests over safe parameter ranges.

8. **Implement the optional Boltzmann backend without weakening the pure-Rust default.**  
   **What:** Add `BoltzmannForwardModel` behind an explicit feature flag and external-tool manifest, not as a default dependency. It should consume sealed input decks and return normalized predictions through the existing `ForwardModel` trait.  
   **Why it matters:** Full CMB `C_ℓ`, lensing, nonlinear/linear `P(k)`, and scale-dependent MG cannot be adjudicated by the current background+growth model (`docs/architecture.md:265-268`).  
   **Effort:** L.  
   **How to verify:** Pure-Rust CI remains green with no external solver; physics-stack CI runs in a locked container/image with pinned CLASS/CAMB/hi_class commits and hashes every input/output.

9. **Replace analytic “unification” stubs with real cross-domain likelihoods.**  
   **What:** Convert `unification_report()` from closed-form placeholders over `alpha_T`, screening flags, `Y_p`, and a loose siren ratio (`unification.rs:64-133`) into data-backed likelihood blocks for GW170817, LVK sirens, Cassini, MICROSCOPE/Eöt-Wash, BBN D/H+Yp, and screening calculations.  
   **Why it matters:** The current MIN-over-domain design is good, but the domains are mostly flags or duplicate information (`docs/zyal-next-level-design.md:97-105`). “Unified” must mean independent data consistency, not self-consistency.  
   **Effort:** M-L.  
   **How to verify:** Each domain has an observable record, covariance/limit policy, source manifest, and decoy that must fail.

10. **Build a value-level derivation oracle.**  
    **What:** A `Derived{mechanism}` parameter must carry machine-checkable inputs and a formula/certificate, and the code must verify the numeric value.  
    **Why it matters:** `Provenance::Derived` currently checks only non-empty text (`mod.rs:59-67`; vetoed only if empty in `vetoes.rs:71-84`). The docs admit that derived-not-fit binds form, not value (`docs/architecture.md:278-280`; `docs/zyal-next-level-design.md:53-74`).  
    **Effort:** L.  
    **How to verify:** Decoys with arbitrary values and plausible prose are demoted to `Free`; certified relations such as `alpha_M=alpha_B` in a specific f(R) limit pass.

11. **Retire or quarantine stale legacy/flagship claims.**  
    **What:** Mark `docs/production-run-1000.md` as superseded or rewrite it around the fair league. It still says the champion “beats ΛCDM” by `ε=+36.7` and describes that as recovered derived physics (`production-run-1000.md:36-64`), while `docs/theory-league.md:10-16` correctly says that number was wrong.  
    **Why it matters:** Contradictory docs are enough for a referee to stop trusting the project.  
    **Effort:** S.  
    **How to verify:** A docs-lint check forbids the old headline except in a clearly labeled “superseded result” context.

12. **Make ZYAL’s agent layer auditable as scientific infrastructure.**  
    **What:** Keep the invariant “LLMs propose, host judges” (`docs/ZYAL.md:215-221`), but require structured AST outputs, prompt-template hashes, model/provider IDs, deterministic oracle receipts, red-team decoy receipts, and held-out-data decisions.  
    **Why it matters:** Browser/LLM routing is not reproducible science unless every nondeterministic input is quarantined from judging and every accepted artifact is replayable by the deterministic host.  
    **Effort:** M-L.  
    **How to verify:** Disable all LLMs and replay accepted theories through the oracle; results and league standings must match the published artifact.

---

## Data sources and data policy

The data surface is promising but not yet referee-grade. The committed Tier-0 file contains 15 diagonal records: 12 DESI-style BAO ratios, one BBN helium datum, and two Planck compressed CMB priors (`data/fixtures/cosmology/tier0-combined.jsonl:1-15`). The growth file adds five RSD `fσ8` points (`growth-rsd.jsonl:1-5`), and the ladder file adds one SH0ES `h0` prior (`sh0es-h0.jsonl:1`). The presence of `source` fields is good: every row has a provenance label rather than anonymous numbers.

The missing layer is a data lockfile and transform ledger. A serious replay package should include, for every observable block: source citation, retrieval URL or DOI, download date, upstream checksum, local transform code hash, unit normalization, row count, covariance availability, and exact observable ID mapping. Raw data need not be committed, but the *manifest of how to reproduce the fixture* must be committed. The current README says public data is tracked through manifests rather than raw dumps (`README.md:10-15`, `:68-72`), but the curated archive does not include `data/registry/`, so the claim cannot be audited here.

The immediate data hygiene failures are mechanical and fixable. The official reproduction commands cite `wl-s8.jsonl` (`docs/architecture.md:293-296`, `docs/theory-league.md:49-53`), but that file is absent from this archive. The hybrid 1000 runbook names `data/fixtures/tension/observables.jsonl` (`ZYAL/runs/run-hybrid-1000-v2.zyal:27-31`), also absent. Do not make a referee hunt for run inputs. A release that claims a headline league must include a self-contained replay subset: observables, covariance blocks, model definitions, expected output JSON, and a script.

The next dataset priority should be: first real covariance for the existing Tier-0 data; then Pantheon+/SH0ES with full covariance; then growth/lensing with covariances; then full CMB-lite/plik-lite through the Boltzmann backend. A diagonal Tier-0 league is acceptable as a smoke test, not as a physics result.

## Forward-model fidelity

The strongest engineering choice is that the forward model derives observables instead of copying genes. `BackgroundForwardModel` returns predictions only when it can compute them (`cosmology/forward.rs:49-56`, `:73-114`). The background module computes FLRW expansion, distances, `r_drag`, CMB distance priors, and BBN helium from `CosmologyParams` (`background.rs:116-276`). The growth module integrates the standard linear growth equation by RK4 and exposes `fσ8` and `S8` (`growth.rs:68-132`). This is a real step beyond the old identity-map engine described in `docs/zyal-engine-rebuild.md:13-55`.

The main weakness is not that the formulas are ridiculous; it is that their error budgets are not yet engineered. Fixed Simpson panel counts and fixed RK4 step counts can be fine, but only if the code measures convergence. A referee will ask: what is the error in `D_M/r_d` at every BAO redshift? What is the change in ΔAIC if distance integrals are recomputed with 4096/8192 panels? What is the difference between 512, 1024, and 2048 RK4 growth steps? Which observational points are sensitive to the Aubourg `r_drag` fit, the Hu-Sugiyama `z_*` fit, and the linearized `Y_p` fit? The docs acknowledge fit-formula fidelity limits (`docs/architecture.md:265-280`), but that admission must become a machine-readable model-error ledger.

Several implementation details deserve hardening. `e_of_z()` clamps negative total density with `.max(0.0).sqrt()` (`background.rs:116-124`). That prevents NaNs but can silently turn an unphysical background into a zero expansion rate. Replace this with a typed invalid-background error and make the forward model omit or veto the candidate with an explicit finding. The growth solver computes `dlnE/dN` by finite difference (`growth.rs:58-66`); add an analytic derivative for CPL backgrounds and keep the finite difference as a cross-check. `growth_fsigma8()` recomputes the whole growth history per observable (`growth.rs:121-127`); cache one history per parameter vector in a league fit so convergence tests and profiling are tractable.

The optional Boltzmann backend should not contaminate the default. Keep `BackgroundForwardModel` as the pure-Rust, no-network, deterministic baseline. Add `BoltzmannForwardModel` only behind a feature flag such as `physics-stack`, with an external manifest carrying solver name, version/commit, container image digest, input parameter file hash, output file hash, and postprocessing code hash. The `ForwardManifest` currently returns an empty provenance hash for the in-repo model (`forward.rs:140-147`). Even pure Rust predictions should include source commit, crate version, feature set, compiler version, and data-lock hash; otherwise two “same model_id/version” runs may not be comparable.

## Statistical rigor and fair model selection

The theory league is the right architectural correction. The docs correctly demote the old `+36.7` result: it compared a fitted candidate to a fixed baseline, under a diagonal likelihood, without a complexity penalty (`docs/theory-league.md:10-16`). The league instead fits every model class, including ΛCDM, and reports AIC/BIC/Schwarz evidence approximations (`league.rs:191-253`; `docs/theory-league.md:18-28`). This is exactly the direction a referee would demand.

But the current implementation has three trust blockers. First, the `sigma8`/`mu0` setter bug discussed above makes growth/MG league rows suspect. Second, coverage is not integrated into model-selection eligibility. AIC and BIC are only comparable for models evaluated on the same data with the same likelihood normalization. The covariance module intentionally marginalizes missing predictions (`covariance.rs:15-18`, `:180-218`), which is honest for a prediction report but dangerous for a league table. A model that cannot predict a difficult point must not get an easier AIC. Establish named leagues by coverage: “geometry-only,” “geometry+growth,” “full-stack.” Within a league, require coverage 1.0 or mark the row ineligible.

Third, the likelihood drops the multivariate Gaussian log-determinant because it cancels on fixed data (`covariance.rs:9-13`). That cancellation is valid only when every model is scored against the same covariance block and same subset of components. Once missing predictions are marginalized, the dimension and determinant change. Include the normalization term in the stored absolute likelihood and then compute deltas; at minimum, prove by test when cancellation is allowed.

The optimizer also needs more diagnostics. Deterministic Nelder-Mead with one initial simplex (`league.rs:217-227`, `:308-410`) is reproducible, but not automatically reliable in bounded, degenerate cosmology fits. Add deterministic multistart from Latin-hypercube or grid seeds, record boundary hits, compare with a grid/profile scan for low-dimensional models, and estimate a Hessian or profile interval at the optimum. For finalists, AIC/BIC is not enough: report AICc for small samples, BIC/Schwarz as an approximation, and eventually nested-sampling or thermodynamic-integration evidence using declared priors.

## Theory coverage and the most valuable near-term physics target

The project is right not to pretend that string theory, loop quantum gravity, CDT, or asymptotic safety are directly scoreable in this engine unless they yield a concrete low-energy handle (`docs/zyal-next-level-design.md:154-162`). The most worthwhile attainable target is therefore not “quantum gravity” in the abstract. It is **a derived, screened scalar-tensor modified-gravity family with a small number of physical parameters**, benchmarked against background, growth, lensing, GW speed, and solar-system constraints.

Concretely, start with Hu-Sawicki f(R) or nDGP, not a constant `mu0` toy. The model must derive `α_i(a)` or `μ(a,k)` and any screening behavior from one physical parameter (`f_R0` or crossover scale), not fit `α_M`, `α_B`, `α_K`, and `mu0` independently. The current `Theory` type has an α-basis and screening slot (`mod.rs:89-165`), so the schema is close, but it needs functions of time/scale and real screening checks rather than constants and a string. The falsifiable program is:

1. Pre-register ΛCDM, w0waCDM, and one screened-MG family with explicit priors and parameter bounds.  
2. Score geometry-only first; require no MG claim from geometry-only data.  
3. Add RSD `fσ8`, weak-lensing `S8`, and eventually scale-dependent growth/lensing through hi_class.  
4. Add cross-domain constraints: GW170817 tensor speed, Cassini/PPN recovery, fifth-force/EP limits, and LVK siren consistency.  
5. Declare success only if the MG family beats ΛCDM and w0waCDM by a pre-registered threshold, for example ΔAIC < −6 and ΔlnZ > +3, with coverage 1.0, no domain failure, and stable ranking under covariance/holdout perturbations. Otherwise, publish the failure.

Early Dark Energy is the second target, not the first. It is attractive because it attacks the H0 tension, and the docs correctly say an `f_ede` knob should be killed unless it is computed from a scalar-field sector (`docs/zyal-next-level-design.md:172-179`). But EDE is CMB-era physics; without a Boltzmann backend and recombination/CMB likelihood, it will be under-tested. Build it after the solver-truth and Boltzmann path exist.

## ZYAL multi-agent design

The best invariant in ZYAL is “LLM proposes; deterministic host judges” (`docs/architecture.md:213-221`, `docs/ZYAL.md:215-221`). Preserve it ruthlessly. No agent output should ever update a score directly. Agents may propose a `Theory` AST, propose a derivation sketch, generate decoys, choose a held-out test, or summarize a literature constraint. The host must validate schemas, run vetoes, run forward models, run likelihoods, and write receipts.

The current source shows a transition in progress. The new `evolve_run()` now uses the adversary margin to filter breeding elites and gate final champions (`evolve.rs:196-252`, `:271-279`), improving on the older internal critique that the adversary was telemetry-only (`docs/zyal-next-level-design.md:22-38`). That is a good fix. But the adversary remains a scalar frontier, not yet a scientific red team. The next version should require each adversary agent to emit a structured decoy and a proposed held-out observable/regime. The deterministic oracle then adjudicates whether the decoy dies and whether the champion survives the held-out test. That would make “adversarial” mean more than a rising threshold.

ZYAL runbooks also need exact implementation binding. `run-hybrid-1000-v2.zyal` sets `adversary.escalation_step: 0.6` (`ZYAL/runs/run-hybrid-1000-v2.zyal:38-41`), while `Adversary` hardcodes `ESCALATION_STEP = 0.02` (`adversary.rs:14-19`). If that runbook is legacy-only, label it. If it is meant to configure the new engine, wire the config and test that the runtime receipt records the applied value. Scientific automation fails when YAML says one thing and Rust does another.

Agent memory should be scientific memory: derivation lemmas, failed regions, data-source updates, and decoy outcomes. Do not store private prompts or browser profiles; store prompt-template hashes, model/provider IDs, schema versions, and deterministic oracle receipts. A replay with LLM calls disabled must be able to re-score all accepted theories and reproduce every published ranking.

## Software, verification, and reproducibility

OpenQG’s credibility will be won or lost in CI. The pure-Rust deterministic default is a strength, but “deterministic” needs a precise contract. The code uses floating-point functions such as `powf`, `exp`, `ln`, and `sqrt` throughout `background.rs` and `growth.rs`; bit-for-bit equality across CPU/libm/compiler targets is not guaranteed. Define whether the project promises bit-stable output on a pinned target or tolerance-stable output across supported targets. Then test exactly that.

A referee-grade CI matrix should include: unit tests; property tests over physical parameter ranges; analytic-limit tests; solver-truth golden tests; covariance linear-algebra tests; league replay tests; data-lock validation; docs command validation; and a “no silent regression” gate that recomputes headline artifacts. The fair Tier-0 result already has a regression guard (`league.rs:518-576`), but it covers only geometry and diagonal data. Add analogous guards for growth, covariance, SH0ES, and the bug-fixed `sigma8`/`mu0` dimensions.

Veto thresholds need unit-level validation. `ALPHA_T_TOLERANCE = 1e-2` is described as a structural tolerance despite GW170817 being far tighter (`vetoes.rs:14-17`). That may be defensible as a coarse kill threshold during search, but it is not a final likelihood. Separate “cheap search veto” from “physical constraint likelihood,” and make the final league use the physical data-backed constraint. Likewise, screening should not pass merely because `screening: Some("vainshtein")` exists (`vetoes.rs:112-118`; `unification.rs:76-104`). Require a screening model to compute a recovery scale and compare to PPN/fifth-force limits.

The project also needs a source manifest for the code itself. `ForwardManifest` should include hashes of the Rust source modules, Cargo.lock, data lockfile, and feature flags. Every league row should record the forward manifest, likelihood manifest, optimizer manifest, data manifest, and model-class manifest. When a number changes, the manifest diff should explain whether the cause was code, data, covariance, optimizer, or model definition.

Finally, reconcile documentation. `docs/architecture.md` and `docs/theory-league.md` are appropriately cautious. `docs/production-run-1000.md` still reads like a flagship claim. Either quarantine it as historical, or rewrite it so the first sentence says the `+36.7` number is superseded. Trustworthy science is not only correct code; it is stale-claim removal.

## Release acceptance gate

Do not ask a serious physicist to evaluate another flagship until these gates pass:

- Growth/MG setter bug fixed and covered by synthetic-recovery tests.
- `theory league` replay package re-derives geometry, geometry+growth, and full-stack tables from sealed data.
- Real covariance blocks loaded and validated, or every headline explicitly labeled diagonal-smoke.
- Simpson/RK4 convergence budgets emitted and propagated to model uncertainty.
- At least one external-solver comparison table published for the supported background/growth observables.
- All stale `+36.7 beats ΛCDM` claims quarantined.
- ZYAL accepted-theory artifacts replay with LLMs disabled.

After that, the project becomes much more interesting: not because it has found new physics, but because it would have become an unusually honest machine for discovering which proposed physics fails.
