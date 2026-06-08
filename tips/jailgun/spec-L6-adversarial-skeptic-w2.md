# OpenQG / ZYAL independent physicist red-team engineering spec

## Ranked backlog

### 1. Freeze or retract every stale headline claim about the `+36.7` champion

**What:** Replace `docs/production-run-1000.md` with a historical-failure note or hard-banner it as superseded. Its current headline says the champion “beats ΛCDM” by `+36.7` log-likelihood units and calls that a “real reflection” of DESI preference (`docs/production-run-1000.md:36-51`). The newer architecture and league docs correctly say that number was an artifact of fitting a candidate against a fixed Planck baseline under a diagonal likelihood (`docs/architecture.md:175-194`, `docs/theory-league.md:10-16`).

**Why it matters:** This is the easiest way to lose a serious physicist. A project cannot simultaneously advertise the refuted number and ask to be trusted for honest scoring. The correction is commendable; leaving the old claim live is not.

**Effort:** S.

**How to verify:** A grep for `+36.7`, `beats ΛCDM`, and `champion beats` must return only “historical invalid claim” language. The CI doc-lint should fail on raw Δlog-likelihood superiority claims unless paired with a re-fit baseline, covariance status, complexity penalty, and artifact hash.

### 2. Build a canonical degree-of-freedom registry; make “derived, not fit” true for every numeric value

**What:** Every numeric input that can affect a prediction must be represented once in a canonical parameter registry with role, prior/bounds, provenance, and either a fit label or a derivation certificate. Today the `Theory` type has `parameters: Vec<Parameter>` with `Provenance`, but it also has an unconstrained `background: CosmologyParams` containing `h, Ω_m, w0, wa, σ8, μ0` (`crates/openqg-core/src/theory/mod.rs:149-165`; `crates/openqg-core/src/cosmology/background.rs:35-63`). The veto cascade scans `theory.parameters`, not the background fields (`crates/openqg-core/src/theory/vetoes.rs:72-82`). The league then fits `CosmologyParams` directly as model-class free parameters (`crates/openqg-core/src/theory/league.rs:63-74`).

**Why it matters:** The central phrase “derived, not fit” currently binds only a subset of declared parameters and mostly the *form*, not the *values*. A model can tune `background.w0` or `background.mu0` while keeping the provenance vector clean. A reviewer will call this parameter fitting with nicer labels.

**Effort:** M/L.

**How to verify:** Add a decoy that changes `background.w0`, `background.wa`, `sigma8`, or `mu0` without a corresponding registry entry; it must fail. Add another decoy with a non-empty `Derived { mechanism }` string but no executable relation; it must be demoted or killed. Final artifacts must separate “fitted value inside an allowed model class” from “value derived from a certificate.”

### 3. Fix the growth/MG league plumbing before quoting any growth-sector result

**What:** Wire every declared `FreeParam` in `ModelClass` into `set_param` and make unknown free parameters a hard error. `ModelClass::lcdm_growth`, `w0wa_cdm_growth`, and `screened_mg` declare `sigma8` and `mu0` as free (`crates/openqg-core/src/theory/league.rs:124-155`), but `set_param` only recognizes `h`, `omega_m`, `omega_b_h2`, `n_eff`, `sum_mnu`, `w0`, `wa`, and `omega_k` (`crates/openqg-core/src/theory/league.rs:48-59`). The boolean return from `set_param` is ignored (`crates/openqg-core/src/theory/league.rs:158-164`).

**Why it matters:** This directly attacks the claim that `μ0` is degenerate with `σ8` or not independently favored (`docs/architecture.md:190-194`, `docs/theory-league.md:34-40`). As implemented in this snapshot, those “free” growth parameters are not actually free in the fit. Worse, they still count in `k`, so the model can be penalized for degrees of freedom the optimizer never moves.

**Effort:** S.

**How to verify:** Unit tests must fit a toy `S8` datum and recover the expected `sigma8`, fit a toy `fσ8` datum and move `mu0`, and fail if any `FreeParam.name` is not a known field. Golden league rows for `+growth` must be regenerated after the fix.

### 4. Make league coverage fail-closed

**What:** In the final league, a model class should either predict every observable in the scoring set or be marked “not scoreable on this dataset.” Current covariance scoring records missing predictions but does not subtract a likelihood penalty for them (`crates/openqg-core/src/scoring/covariance.rs:197-238`). It computes `coverage` (`crates/openqg-core/src/scoring/covariance.rs:242-248`), but `model_league` ranks by AIC derived from the partial log-likelihood (`crates/openqg-core/src/theory/league.rs:237-253`, `271-305`).

**Why it matters:** “The engine omits what it cannot derive” is good for coverage reporting; it is dangerous for model selection. A model that predicts only easy observables can avoid penalties from hard ones and still be ranked unless coverage is a hard eligibility condition.

**Effort:** M.

**How to verify:** Add a decoy forward model that predicts only `h0` or only one BAO point. It must not appear above a complete model in the league. The JSON artifact should have an explicit `scoreable: false` reason when coverage is below the dataset-required threshold.

### 5. Populate real covariance and data provenance before using words like “evidence”

**What:** Ship public-source manifests that reproduce each fixture row, plus external covariance files or deterministic download receipts. The docs admit committed fixtures are diagonal and real DESI/Planck covariance is not populated (`docs/architecture.md:269-271`; `docs/theory-league.md:83-86`). `tier0-combined.jsonl` is 15 scalar rows with source strings but no covariance, transform recipe, row/table provenance, or upstream checksum (`data/fixtures/cosmology/tier0-combined.jsonl:1-15`). The full-stack reproducer references `data/fixtures/cosmology/wl-s8.jsonl`, which is not present in this archive (`docs/architecture.md:290-296`; `docs/theory-league.md:42-54`).

**Why it matters:** Diagonal BAO/CMB-prior likelihoods are not a publication-grade adjudicator. Source labels are not data provenance. A hostile reviewer will not accept “raw data kept out of git” as a substitute for reproducible transforms.

**Effort:** M.

**How to verify:** A single `data audit` command should rebuild fixture JSONL and covariance blocks from versioned manifests, print upstream hashes, and prove that every docs reproducer references existing files.

### 6. Replace analytic “unification” placeholders with independent likelihoods

**What:** Keep the MIN-over-domains pattern, but replace the current stubs with data. The present channel checks `alpha_T`, a screening flag, BBN `Y_p`, and a toy GW-friction siren ratio (`crates/openqg-core/src/theory/unification.rs:64-133`). The docs also admit this is analytic self-consistency, not independent data (`docs/architecture.md:272-274`).

**Why it matters:** The word “unification” is over-strong for checks a candidate can pass by setting `alpha_T = 0` and writing `screening: "vainshtein"`. It is a useful structural prefilter, not evidence that gravity, BBN, sirens, and lab tests have been jointly explained.

**Effort:** M/L.

**How to verify:** Add Cassini/MICROSCOPE/Eöt-Wash likelihood records, GW170817 speed/timing constraints, LVK siren posteriors where appropriate, and BBN D/H + helium with a real BBN-network or accepted emulator. The unification report should cite independent data IDs and log-likelihood contributions, not only notes.

### 7. Implement a Boltzmann-backed final adjudicator and restrict compressed CMB priors to their validity domain

**What:** Keep the pure-Rust background+growth backend for fast CI, but make final claims go through CLASS/hi_class or CAMB-class machinery. The architecture names no full `C_ℓ`, no lensing, no nonlinear `P(k)` as the largest fidelity gap (`docs/architecture.md:265-268`). Yet the model classes being discussed include EDE, MG, and coupled dark energy, where compressed Planck distance priors can be invalid or insufficient.

**Why it matters:** The most interesting physics lives in perturbations and recombination-era signatures. Background distances plus two CMB priors cannot adjudicate α-basis gravity, EDE, or coupled sectors.

**Effort:** L.

**How to verify:** A `physics-stack` league must reproduce a standard ΛCDM Planck-lite baseline, then compare ΛCDM/w0wa/EDE/MG with full or approved compressed likelihoods appropriate to each model. Any result produced only by the pure-Rust backend must be labeled “screening/triage, not final evidence.”

### 8. Replace metadata-based vetoes with derived structural checks

**What:** Infer dimensions, Lorentz scalar status, stability, and screening from a typed action grammar or model-family implementation. Today `Term` carries user-supplied `mass_dimension` and `free_lorentz_indices` (`crates/openqg-core/src/theory/mod.rs:79-87`), and `Stability` is also supplied as coefficients (`crates/openqg-core/src/theory/mod.rs:122-145`). The vetoes check the metadata, not the Lagrangian (`crates/openqg-core/src/theory/vetoes.rs:51-121`).

**Why it matters:** A plausible but wrong theory can pass by declaring healthy metadata. This is not a small edge case; it is exactly how an LLM would launder a bad proposal through a structural gate.

**Effort:** L.

**How to verify:** For at least `f(R)`, nDGP, quintic Galileon decoy, coupled quintessence, and EDE, the engine must compute α-functions/stability/screening relations from model-family fields. Hand-entered stability values should be banned in final scoring.

### 9. Relabel the current adversary or make it genuinely co-evolving

**What:** The latest `evolve_run` does make frontier pressure affect breeding and final champion eligibility (`crates/openqg-core/src/theory/evolve.rs:196-207`, `250-273`). That is better than pure telemetry. But `Adversary` itself is still a scalar margin with rollback (`crates/openqg-core/src/theory/adversary.rs:21-77`), not an opponent that generates attacks, decoys, held-out regimes, or new falsifiers.

**Why it matters:** “Co-evolving adversary” still sounds like theatre unless it creates new tests the champion did not anticipate. A scalar threshold is a useful pressure schedule; call it that.

**Effort:** M.

**How to verify:** Each generation should emit at least one adversary-authored decoy or held-out observable request, then record whether deterministic gates killed the decoy and whether the champion survived out-of-sample. The run should change if adversary generation is disabled.

### 10. Build the five-theory league as the flagship artifact

**What:** Encode and fairly score ΛCDM, w0waCDM, scalar-field EDE, screened scalar-tensor MG, and coupled/interacting dark energy. The internal review already identifies this as the compelling artifact (`docs/zyal-next-level-design.md:154-200`).

**Why it matters:** A single evolved near-ΛCDM champion is not persuasive. A pre-registered league of known serious alternatives, each treated fairly and killed where appropriate, would be.

**Effort:** L.

**How to verify:** Publish a league table with priors, nuisance parameters, covariance status, coverage, ΔAIC, ΔBIC, nested-sampling evidence for the finalists, and posterior predictive checks. The table must be reproducible from a clean checkout.

---

## Data sources

The current data story is promising but not yet defensible. The architecture is honest that raw datasets are not shipped and that the committed fixtures are previews (`docs/architecture.md:225-256`). The Tier-0 fixture contains 12 DESI DR1 BAO ratios plus BBN helium and two Planck distance priors (`data/fixtures/cosmology/tier0-combined.jsonl:1-15`). Growth has five `fσ8` rows (`data/fixtures/cosmology/growth-rsd.jsonl:1-5`), and SH0ES is one H0 prior (`data/fixtures/cosmology/sh0es-h0.jsonl:1`). That is useful smoke data. It is not enough for an expert-facing evidence engine.

The minimum fix is not to commit raw bulk data; the project’s policy against raw dumps is reasonable (`README.md:68-72`). The minimum fix is to commit reproducible provenance: upstream URL/DOI/arXiv or collaboration release, table/row mapping, unit transform, version date, hash of the downloaded source, hash of the transformed fixture, and the covariance block that belongs to each group. DESI BAO points should not be treated as independent scalar rows. Planck distance priors should come with the full covariance and a validity note saying which model classes may use them. Pantheon+ cannot be “just μ(z)” without nuisance handling and covariance. RSD and weak-lensing constraints need correlation and scale cuts. BBN should include D/H, baryon-density dependence, and uncertainty propagation, not only a linearized helium anchor.

The archive also has a plain reproducibility fault: the full-stack commands reference `data/fixtures/cosmology/wl-s8.jsonl` (`docs/architecture.md:290-296`; `docs/theory-league.md:48-54`), but that file is not in this source snapshot. Either ship it, remove it from reproducers, or mark the command as aspirational. A physicist who cannot run the stated command will stop reading.

Verification target: `openqg data audit --rebuild-fixtures --check-covariance --strict-doc-links` should rebuild every fixture, validate every covariance block is positive-definite, and fail if any Markdown command references a missing data file.

## Forward-model fidelity

The pure-Rust background model is a credible CI triage layer. It computes FLRW distances, CPL dark energy, sound-horizon quantities, BAO ratios, CMB compressed priors, BBN helium, and now linear growth observables (`docs/architecture.md:118-151`; `crates/openqg-core/src/cosmology/forward.rs:83-110`). That is a real forward map, not the old identity map. This is one of the project’s genuine strengths.

The problem is scope control. A background+growth model cannot carry the rhetorical load of “quantum gravity / unification meets data.” It can test late-time expansion fits and a leading-order growth handle. It cannot test EDE recombination physics, CMB damping, lensing, scale-dependent modified gravity, nonlinear `P(k)`, or galactic dynamics. The architecture admits no full CMB `C_ℓ`, no lensing, no nonlinear `P(k)` (`docs/architecture.md:149-151`, `265-268`). That limitation should be promoted from “open problem” to “final-claim gate”: no final physics claim should pass without an appropriate Boltzmann likelihood.

There are also internal consistency issues. `docs/zyal-engine-rebuild.md` still says the background model does not compute `σ8` or `fσ8` (`docs/zyal-engine-rebuild.md:369-378`), while `BackgroundForwardModel` now returns `s8`, `sigma8`, and `fsigma8@z` (`crates/openqg-core/src/cosmology/forward.rs:94-108`). The docs are evolving faster than the code. For a science engine, stale architecture docs are not harmless; they create ambiguity about what was actually scored.

The deterministic veto cascade is useful as a cheap necessary-condition filter, but it is not yet a physics proof. `alpha_T` is allowed up to `1e-2` (`crates/openqg-core/src/theory/vetoes.rs:17`, `87-90`; `docs/architecture.md:98-104`), which is far too loose for a GW170817-labeled tensor-speed gate. Screening is just `Option<String>` and a declaration (`crates/openqg-core/src/theory/mod.rs:161-163`; `crates/openqg-core/src/theory/vetoes.rs:112-121`). Stability is supplied, not derived. A bad theory can pass by saying: `alpha_T = 0`, `screening = "vainshtein"`, `stability = healthy`, all terms have dimension 4, and every fitted value is “fundamental.” This is the plausible-wrong theory that slips through: a cosmetically screened Horndeski-like CPL background with arbitrary `w0`, `wa`, `α_K`, and `μ0`, no actual Lagrangian screening calculation, and no CMB/growth scale test.

The fix is to split the engine into two modes. **Triage mode** accepts metadata and reports “necessary gates passed.” **Adjudication mode** accepts only model-family implementations that compute their own α-functions, stability, screening recovery, and observables. Anything else is not scoreable as physics.

## Statistical rigor

The new `theory league` is the right direction. It profile-fits the baseline and challengers, then reports AIC/BIC/Schwarz-style evidence instead of raw Δlog-likelihood (`docs/theory-league.md:18-28`; `crates/openqg-core/src/theory/league.rs:1-19`). The correction of the `+36.7` episode is exactly the kind of self-audit that makes the project more credible.

But the implementation must harden before the numbers can be trusted. The growth/MG free-parameter bug is the most concrete: `sigma8` and `mu0` are declared as free but ignored by `set_param`. This invalidates any growth-sector league result until fixed. More broadly, `set_param` returns `false` on unknown names but the caller ignores it; a scientific optimizer must never silently ignore a declared degree of freedom.

The second statistical issue is coverage. The docs say missing observables are omitted, never faked (`docs/architecture.md:111-116`). Good. But omitted observables cannot simply disappear from the likelihood in model selection. Current scoring records missing predictions as findings and lowers `coverage`, but AIC/BIC are still computed from the partial log-likelihood. A final league should use one of three strict policies: require full coverage; compare only on a named common subset and label it explicitly; or integrate missing observables through a principled predictive distribution. “Did not predict the hard data” must never improve rank.

The third issue is likelihood normalization and covariance. Dropping the multivariate Gaussian normalization is acceptable only for fixed data, fixed covariance, and equal coverage; the comment says it cancels across models (`crates/openqg-core/src/scoring/covariance.rs:9-18`). Once coverage varies or covariance submatrices are used, the log determinant no longer cancels. For the final league, include the full Gaussian log-likelihood or restrict comparisons to complete, identical observable vectors.

Finally, the search itself induces a trials factor. MAP-Elites/evolution can examine many candidates. AIC/BIC on the final selected family is not automatically a correction for search over many generated forms. The solution is a pre-registered model-class league for headline claims, with held-out posterior predictive checks for anything discovered by evolution. Evolution can propose; the league adjudicates.

## Theory coverage and the most valuable unified-physics target

The project should resist trying to score “quantum gravity” directly. The right target is a low-energy effective handle whose predictions can be computed and killed by existing data. I would prioritize **scalar-field Early Dark Energy as the first flagship target**, followed by screened scalar-tensor MG.

EDE is worth chasing not because it is the most likely winner, but because it is the cleanest honesty test. If the engine accepts a free `f_ede` knob, it fails its mission. If it encodes scalar-field dynamics so `f_ede`, `z_c`, and the sound-horizon shift are computed from a potential and initial conditions, then scores that model against CMB+BAO+SNe+SH0ES with fair priors and full likelihoods, it demonstrates the distinction between “fit knob” and “derived sector.” The internal review already frames EDE this way (`docs/zyal-next-level-design.md:172-179`).

Concrete falsifiable EDE program:

1. Implement an EDE model family with a scalar potential, field equations, initial-condition policy, and derived background contribution. Do not allow a direct fitted `f_ede` field.
2. Add a Boltzmann-backed likelihood because compressed CMB distance priors are insufficient for EDE.
3. Pre-register priors and compare ΛCDM, w0waCDM, and EDE on the same datasets.
4. Require EDE to improve a full evidence metric, not merely H0, without unacceptable CMB residuals or growth degradation.
5. Publish posterior predictive residual plots and a failed-decoy report showing that a free-`f_ede` impostor is killed.

For screened scalar-tensor MG, do not fit constant `α_i` values. Encode `f(R)`, nDGP, and one Galileon decoy as model families. Derive `α_i(a)`, `G_eff(a,k)`, screening scale, and stability from their parameters. Then use growth, lensing, GW speed, and solar-system constraints. The current α-basis metadata is a staging representation, not a derived theory.

## ZYAL multi-agent design

The load-bearing invariant is correct: LLMs may propose and critique, but deterministic oracles judge (`docs/ZYAL.md:13-23`, `134-136`). Keep that. Also keep the host-owned runbook philosophy: budgets, stop conditions, evidence gates, permissions, and explicit arming (`docs/ZYAL.md:50-95`).

The danger is claiming more agentic science than exists. In this source snapshot, the new symbolic engine is primarily deterministic. The ZYAL docs describe jnoccio and jailgun routes (`docs/ZYAL.md:115-136`) and a legacy genome workspace (`docs/ZYAL.md:140-207`), while the architecture says the live LLM proposer is an optional hook and the old float-vector engine is on a retirement path (`docs/ZYAL.md:227-255`). That is fine if described as orchestration infrastructure. It is not yet evidence that multi-agent reasoning discovered physics.

A credible multi-agent design should have typed roles:

- **Proposer agents** emit strict `Theory` ASTs or model-family patches, never scores.
- **Derivation agents** emit derivation sketches that a deterministic checker can execute or reject.
- **Skeptic agents** generate decoys, missing-regime challenges, and falsification attempts.
- **Data-curator agents** propose new public constraints, but a deterministic data-audit lane must build the fixture.
- **Judge/oracle** remains Rust-owned: schema validation, parameter registry, vetoes, forward models, likelihoods, coverage eligibility, and receipts.

The adversary should become a real opponent. The current frontier margin is a pressure schedule; it is not “co-evolution” in the scientific sense. A real adversary produces a new decoy or held-out observable each generation, and the run’s result changes if those attacks are disabled. That would make the ZYAL story worth presenting.

## Software and reproducibility

The repository has good instincts: policy forbids raw data dumps and undocumented knobs (`MISSION.md:9-13`), generated outputs have ownership boundaries (`GOVERNANCE.md:12-19`), and the architecture insists on deterministic replay. The missing piece is a strict proof lane that a hostile reviewer can run from a clean checkout.

Required proof lane:

1. `cargo test -p openqg-core` or an equivalent selected-source test command must compile the exact modules cited in the docs.
2. `theory league --observables tier0` must regenerate the honest geometry-only table and prove ΛCDM wins under the current fixture.
3. `theory league --full-stack` must either run successfully with all referenced data files or refuse with a clear “dataset unavailable” error.
4. Golden tests must pin the growth/MG parameter plumbing: `sigma8` and `mu0` move when fitted.
5. A doc-link lint must fail on missing files such as `wl-s8.jsonl` or stale claims such as the unqualified `+36.7` headline.
6. A decoy suite must include: background knob without provenance; derived string with no executable relation; fake screening string; too-loose `alpha_T`; subset-predictor; non-PD covariance; unknown `FreeParam`; and a model with healthy hand-entered stability but an invalid action.

Jankurai boundary: Rust should own durable policy, config, tar validation, run contracts, parameter registry, receipts, and scientific gates. TypeScript should remain dashboard/browser surface only. Do not commit secrets, real prompts, browser profiles, logs, receipts, runtime state, downloaded archives, or private local paths. The `docs/ZYAL.md` references local endpoints and paths for jnoccio/jailgun (`docs/ZYAL.md:120-130`); public artifacts should turn those into configurable interface contracts, not environment-specific proof.

## Single thing that would sink the project in a seminar

A sharp questioner will ask: “Show me where the value of `w0`, `wa`, `σ8`, or `μ0` is derived rather than fit.” In this snapshot, the honest answer is: it mostly is not. The engine has a good structural veto for explicitly declared free parameters, but the physics-driving background values are still continuous fit parameters, and some are not even wired correctly in the league. That does not make the project bad; it makes the current claim narrower:

> OpenQG is a promising deterministic framework for fitting and comparing restricted cosmological model classes with structural vetoes and improving provenance discipline. It is not yet a derived-theory discovery engine.

What would change my mind: a clean, reproducible five-theory league in which every numeric degree of freedom is either declared fitted or certified derived; growth/MG parameters actually fit; real covariance and coverage gates are enforced; EDE is implemented as a scalar-field sector rather than a knob; and the old `+36.7` story appears only as a documented failure that the system learned not to repeat.
