# OpenQG / ZYAL engineering specification — observational-cosmology review

**Batch tab:** 1  
**Review lens:** observational cosmologist: data vectors, covariance, likelihoods, survey realism, and claim hygiene.  
**Source-access note:** the runtime did not expose the promised source tarball bytes in `/mnt/data`. This review is therefore grounded in the path-level claims in the invocation, the accessible public OpenQG surfaces (`README.md`, `MISSION.md`, `docs/MOONSHOT.md`, `GOVERNANCE.md`, `docs/architecture.md`, and `crates/openqg-core/src/scoring/likelihood.rs`), and the selected path list. I am not going to pretend I line-read unavailable archive files. Every code-level claim below that relies on the selected-but-unmounted archive should be treated as a required verification target during replay.

## Ranked backlog

### 1. Replace scalar/diagonal Gaussian scoring with named covariance-block likelihoods

**What:** Introduce a `DataVector`/`CovarianceBlock` abstraction in `crates/openqg-core/src/scoring/covariance.rs` and wire it into `crates/openqg-core/src/scoring/likelihood.rs`. The current accessible scoring surface evaluates each observable independently and uses `sigma = max(observed.uncertainty, prediction.uncertainty)` before adding scalar Gaussian terms. That is acceptable only for smoke fixtures, not cosmology. Add dense and sparse covariance support, block identifiers, Cholesky/eigendecomposition checks, nuisance-parameter projection hooks, and failure modes for non-positive-definite matrices.

**Why it matters:** The fastest way for a referee to dismiss the present engine is to say its likelihood is not the survey likelihood. BAO, SNe, CMB priors, weak lensing, and RSD are dominated by correlated errors. A diagonal approximation can change the sign and apparent significance of a dark-energy preference by misrepresenting the tilted directions in parameter space.

**Effort:** M.

**Verify:** Reproduce a toy correlated Gaussian exactly; verify `chi2 = r^T C^{-1} r` against NumPy/Julia/R for DESI-like 2D blocks; fail tests on diagonal fallback unless the dataset manifest explicitly declares `covariance_status: diagonal_smoke_only`.

### 2. Add DESI DR2 BAO as the first real Tier-1 dataset

**What:** Add DESI DR2 BAO with the published `D_M/r_d`, `D_H/r_d`, and/or `D_V/r_d` data vector and its covariance. Do not ingest it as independent scalar rows. Model each redshift/tracer block with the published anisotropic covariance and preserve the distinction between galaxy BAO and Lyα BAO. Selected paths to touch conceptually: `data/fixtures/cosmology/tier0-combined.jsonl`, `crates/openqg-core/src/cosmology/forward.rs`, and `crates/openqg-core/src/scoring/covariance.rs`.

**Why it matters:** DESI DR2 is the sharpest near-term geometry stress test for a background+growth engine. DESI DR2 reported BAO measurements from more than 14 million galaxies and quasars over three years, with CMB+BAO combinations challenging flat ΛCDM in some extensions; the engine should be able to reproduce the collaboration-level geometry conclusions before touching speculative theory.

**Effort:** M.

**Verify:** Reproduce the DESI DR2 ΛCDM and w0wa distance-only contours to a predeclared tolerance using the same priors and covariance. Add a golden test showing that diagonalizing the DESI covariance changes the inferred w0/wa ellipse; this makes the danger visible.

### 3. Add Pantheon+ SNe with the full systematic covariance before adding more H0 tension knobs

**What:** Ingest Pantheon+ distance moduli and full covariance. Keep uncalibrated SNe, Cepheid-calibrated SH0ES anchors, and external H0 priors as separate likelihood modules with explicit calibration flags. Never let `data/fixtures/cosmology/sh0es-h0.jsonl` silently stand in for the Pantheon+/SH0ES joint likelihood.

**Why it matters:** Supernovae are not a set of independent candles. Calibration, selection, peculiar velocity, intrinsic scatter, and survey cross-calibration produce strong off-diagonal covariance. Pantheon+ contains 1701 light curves of 1550 spectroscopically confirmed SNe Ia and explicitly improved the treatment of systematics. Treating these rows diagonally is not a harmless approximation; it can overstate evidence for late-time dark-energy structure or wash it out, depending on redshift structure.

**Effort:** M/L.

**Verify:** Reproduce Pantheon+ ΩM for flat ΛCDM from SNe alone and the published w0/w0wa checks within tolerance. Verify that enabling SH0ES changes only the calibrated likelihood branch and that the league report labels it as a local-distance-ladder prior, not as SNe-only evidence.

### 4. Implement a Planck 2018 CMB-lite distance-prior module, then stop calling it Planck

**What:** Add a Planck distance-prior likelihood using `(R, l_A, omega_b)` or `(l_A, R, omega_b h^2, n_s)` exactly as published by the chosen prior paper, with the full 3x3 or 4x4 covariance. In the UI and league reports, call this `Planck distance prior`, not `Planck TT/TE/EE` or `Planck likelihood`.

**Why it matters:** A background+growth code can defensibly use CMB distance priors for late-time geometry triage. It cannot make full CMB-spectrum claims, infer early-dark-energy viability, or score models that alter recombination, perturbations, lensing, N_eff, or primordial spectra. Planck 2018 measures the acoustic scale to about 0.03%, which is exactly why a cavalier distance prior will dominate the fit if misused.

**Effort:** M.

**Verify:** Reproduce Planck-prior ΛCDM constraints for Ωm/H0/r_d at the level expected from the published compressed prior. Add hard vetoes: EDE, varying N_eff, modified recombination, nonstandard perturbations, and early-universe unification candidates must not receive `full_cmb_supported=true` until a Boltzmann likelihood exists.

### 5. Add BBN D/H and helium as an independent sound-horizon/baryon-density anchor

**What:** Add a BBN likelihood for D/H and optionally Yp, with a transparent nuclear-rate/systematic model and a derived prior on Ωb h² when the model is standard. Keep it independent of Planck.

**Why it matters:** Geometry-only BAO constraints are often really constraints on ratios to `r_d`. If OpenQG wants to adjudicate late-time versus early-time resolutions of H0 tension, it needs a non-CMB baryon-density anchor. BBN is one of the few early-universe constraints that a background code can use responsibly without pretending to fit TT/TE/EE.

**Effort:** S/M.

**Verify:** Reproduce a standard BBN Ωb h² constraint and show how DESI BAO + BBN constrains H0/r_d differently from DESI BAO + Planck distance prior.

### 6. Add weak-lensing/growth only after the covariance engine and nuisance system exist

**What:** Defer DES-Y3 / KiDS-1000 3x2pt until there is a proper nuisance-parameter layer: shear calibration, photo-z shifts, intrinsic alignments, baryonic feedback, galaxy bias, nonlinear matter power, scale cuts, and covariance. If a compressed S8 datum is added earlier, label it `compressed_lensing_demo`, not a 3x2pt likelihood.

**Why it matters:** Growth is where modified gravity and unification claims become interesting, but weak-lensing likelihoods are nuisance-heavy. A single S8 Gaussian erases most of the information and all of the risk. The current selected fixture `growth-rsd.jsonl` is a useful smoke target; it is not a lensing survey.

**Effort:** L.

**Verify:** First reproduce DES-Y3 or KiDS-1000 published ΛCDM S8 constraints in a frozen pipeline. Then run posterior predictive checks and a scale-cut stress test. Reject any theory-ranking conclusion that flips when reasonable baryonic-feedback priors change.

### 7. Split the league into claim classes: smoke, geometry, calibrated-local, growth, and full-cosmology

**What:** Update `docs/theory-league.md` and `crates/openqg-core/src/theory/league.rs` so every ranking states its evidence class. A model can win `geometry_only_no_cmb_spectrum`, lose `growth`, and be `not_scoreable_full_cmb`. Do not produce one global leaderboard that hides the data combination.

**Why it matters:** The invocation says the league already exhibits the important behavior: geometry-only favors ΛCDM; adding SH0ES favors a dark-energy extension. That is not a bug; it is exactly the data-dependence the report must expose. The danger is compressing it into a single champion narrative.

**Effort:** M.

**Verify:** Golden reports must include a per-dataset Δχ²/ΔlogL table, a data-combination transition diagram, and a statement of which conclusion is licensed by which data. Add a regression test where SH0ES toggling cannot alter the geometry-only conclusion.

### 8. Add profile-fit and posterior/evidence backends before advertising model selection

**What:** Keep AIC/BIC as quick diagnostics, but add real profile likelihoods and nested-sampling or thermodynamic-integration evidence for serious model comparison. Priors must be declared in theory manifests. Parameter bounds must be physically motivated and receipted.

**Why it matters:** AIC/BIC are cheap approximations and can be actively misleading in nonlinear cosmological models with curved degeneracies, bounded priors, and non-Gaussian posteriors. A referee will not accept “BIC says this wins” as evidence for new physics.

**Effort:** L.

**Verify:** Reproduce known ΛCDM vs wCDM vs w0wa evidence rankings on a simple public dataset combination. Add simulation-based calibration on mock data where ΛCDM is true; the false-positive rate for extensions must be measured.

### 9. Make derived-not-fit binding mean something operational

**What:** In `crates/openqg-core/src/theory/assessment.rs`, `unification.rs`, `vetoes.rs`, and `anchors.rs`, distinguish three levels: (a) parameterized phenomenology, (b) derived functional form with free physical parameters, and (c) derived parameter values. The current “derived-not-fit binds form but not parameter values” limitation is honest, but the league must penalize it explicitly.

**Why it matters:** “Derived-not-fit” can become a loophole: the LLM proposes a beautiful derivation, then the engine fits enough free knobs to win. That is not discovery. It is dressed-up phenomenology unless the parameter count, priors, and derivation boundary are enforced.

**Effort:** M.

**Verify:** Every candidate must emit a parameter ledger: free, fixed-by-external-data, derived-from-theory, nuisance, and post-hoc. The score report must show AIC/BIC/evidence using only free physical + nuisance parameters that actually entered the likelihood.

### 10. Add a full Boltzmann adapter path, but keep it boxed behind deterministic contracts

**What:** Add a CLASS/CAMB adapter lane for full CMB spectra, matter power spectra, lensing, and EDE/coupled-DE scoring. Rust owns manifests, validation, run contracts, hashes, and receipt policy; the numerical Boltzmann engine can be an exception-only external adapter with locked versions and cached outputs.

**Why it matters:** Without Boltzmann fidelity, OpenQG must not make cosmological statements about EDE, nonstandard radiation, early modified gravity, primordial spectra, or Planck TT/TE/EE. A CMB-lite path is fine for late-time triage, not for frontier claims.

**Effort:** L.

**Verify:** Reproduce Planck-lite first, then a public CLASS/CAMB ΛCDM power-spectrum benchmark, then a CMB likelihood smoke test. Add a hard run-contract flag: no full-CMB claim without a Boltzmann receipt.

### 11. Harden ZYAL as a proposal generator, not a scientific judge

**What:** In `docs/ZYAL.md`, `docs/zyal-next-level-design.md`, and ZYAL run/stage files, require every agent output to be a typed claim card with data scope, admissible evidence, mutation boundary, falsifier, and deterministic validator. The host must own policy and scoring.

**Why it matters:** The ZYAL concept — LLM proposes, deterministic host judges — is the right architecture. But a reviewer will attack any place where prose evaluation, hidden prompts, or self-graded agents affect scientific ranking.

**Effort:** M.

**Verify:** Replay a run with network/model access disabled using stored candidate manifests and receipts. The league score must be reproducible from deterministic artifacts alone.

### 12. Build release-grade reproducibility before adding more theory families

**What:** Add a `release-cosmology-smoke` proof lane: exact dependency lock, dataset hashes, covariance matrix hashes, seed registry, model manifests, run receipts, and archived score reports. The public README says OpenQG is evidence-gated and receipt-oriented; make that true for cosmology, not just packaging.

**Why it matters:** The project’s cultural claim is trustworthiness. In cosmology, trust begins with re-running the exact likelihood.

**Effort:** M.

**Verify:** Fresh checkout + fixture download + `just cosmology-smoke` reproduces all benchmark numbers and writes a no-private-path receipt. CI fails if runtime state, prompts, logs, browser profiles, or downloaded archives are committed.

## Data sources and likelihood priorities

The data plan should be ruthless: add datasets in the order that most improves falsifiability while staying within the current forward model.

**First: DESI DR2 BAO.** DESI DR2 is the natural next target because the current engine is described as background+growth, and BAO is the cleanest high-precision background observable. The data vector must preserve anisotropy: `D_M/r_d` and `D_H/r_d` are correlated within tracer/redshift bins. The Lyα point at high redshift is especially valuable because it tests the redshift lever arm, but it comes with its own systematics and should be a separate block. Diagonalizing DESI throws away the ellipse orientation and can distort w0-wa inference. The verification target is not “loads rows”; it is “reproduces the published DESI DR2 distance-only contours.” Use DESI Collaboration DR2 Results II (arXiv:2503.14738) and the Lyα companion (arXiv:2503.14739) as the primary implementation references.

**Second: Pantheon+ SNe with covariance.** Add Pantheon+ before additional ad-hoc H0 priors. It is the indispensable redshift-distance bridge between BAO and local calibration, and the systematic covariance is the dataset. SNe rows are not independent. The covariance contains calibration and survey structure; ignoring it is most damaging for exactly the late-time dark-energy claims OpenQG will be tempted to rank. Keep three modes: uncalibrated Pantheon+ for shape, Pantheon+SH0ES for calibrated local distance ladder, and a standalone H0 prior only for controlled sensitivity tests. Use Scolnic et al. full dataset (arXiv:2112.03863) and Brout et al. cosmological constraints (arXiv:2202.04077).

**Third: Planck distance prior and BBN.** For the current forward model, a compressed CMB distance prior is defensible only as a distance-prior module. It is not a full Planck likelihood and must not be advertised as one. The Planck 2018 paper reports the acoustic scale at about 0.03% precision; that makes the prior powerful and dangerous. Pair it with BBN D/H so the engine can test BAO+BBN and BAO+CMB-prior separately. This distinction matters for H0 tension: an early-universe anchor and a local ladder prior pull through different assumptions.

**Fourth: RSD/growth smoke, then weak-lensing 3x2pt.** The selected fixture `growth-rsd.jsonl` is a useful seed. RSD fσ8 points can test growth code at modest effort, but real RSD covariances still matter. DES-Y3/KiDS-1000 3x2pt should wait until the nuisance framework exists. If added too early as a single S8 Gaussian, it will create false confidence and hide the hard parts: shear calibration, redshift distributions, intrinsic alignments, baryons, nonlinear P(k), and scale cuts. The DES-Y3/KiDS-1000 joint cosmic-shear literature is useful as a compressed validation target, not a substitute for a full likelihood.

**Fifth: LVK standard sirens.** Standard sirens are scientifically attractive but currently lower leverage for this engine than BAO/SNe/CMB-lite/BBN. They are broad H0 constraints and require host-galaxy catalog/selection modeling for dark sirens. Add them as an independent H0 likelihood after the SH0ES path is clean. Their value is as a calibration cross-check, not as the primary adjudicator of dark energy.

The covariance population order is therefore: DESI intra-tracer `D_M-D_H` and cross-bin covariance; Pantheon+ full systematic covariance; Planck distance-prior 3x3/4x4 covariance; RSD/growth covariance; then weak-lensing 3x2pt covariance with nuisance parameters. The diagonal likelihood approximation most distorts inference for Pantheon+ and DESI anisotropic BAO. It is also unacceptable for Planck priors, because the acoustic scale, shift parameter, and baryon density are deliberately compressed into correlated combinations.

## Forward-model fidelity

A background+growth model can compute H(z), comoving/angular/luminosity distances, BAO ratios, distance moduli, and approximate linear growth. That is enough to triage ΛCDM, Ωk, wCDM, CPL w0wa, simple late-time scalar-field effective histories, and maybe phenomenological modified-growth functions. It is not enough for full CMB, EDE, coupled dark energy, neutrino-sector claims, primordial-spectrum features, or serious modified gravity.

The engine should enforce this at the type level. Every theory candidate should declare which observable families it can honestly predict: `background`, `linear_growth`, `linear_matter_power`, `cmb_distance_prior_safe`, `cmb_spectra`, `nonlinear_lensing`, `standard_siren`, `bbn`. A candidate that only implements background equations cannot be ranked on TT/TE/EE, S8, or lensing. A candidate that changes pre-recombination physics cannot be scored with a Planck distance prior unless that prior was derived for that model class, which it usually was not.

The most urgent forward-model additions are not exotic. Add curvature, massive-neutrino approximations, radiation/N_eff bookkeeping, sound-horizon calculation with clear validity flags, and a robust growth integrator with tests against known ΛCDM results. Then add a Boltzmann adapter. The adapter should not compromise the Jankurai boundary: Rust owns durable policy, manifests, tar validation, receipts, run contracts, and proof lanes. CLASS/CAMB can run behind a locked numerical service or subprocess with exact version, input hash, output hash, and no authority to write source policy.

## Statistical rigor and league behavior

The league must be an adjudicator of claims, not a leaderboard generator. A fair model-selection report should contain: maximum likelihood, profile likelihood intervals, posterior summaries under declared priors, AIC/BIC as approximations, evidence when computed, parameter count, coverage, failed predictions, and per-dataset contribution. It should also explicitly say which model family was not scoreable.

AIC/BIC alone are not enough. BIC assumes regular models, large-sample behavior, and a simple relation between sample size and independent information. Cosmological posteriors often violate that through degeneracies, hard priors, and non-Gaussian covariance. Evidence is prior-sensitive, so the answer is not to hide priors; the answer is to declare them and run sensitivity checks. Profile fits are valuable because they expose whether an apparent win is driven by one nuisance corner.

The H0 and S8 tensions need disciplined handling. SH0ES should be a separate local-calibration likelihood. It should never be silently combined into “SNe” or treated as an independent copy of Pantheon+ information when the covariance is shared. The league should show the transition: BAO-only, BAO+SNe, BAO+CMB-lite, BAO+SNe+CMB-lite, then plus SH0ES. If a DE extension wins only after SH0ES is added, the report should say exactly that. That is not embarrassing; it is honest. Similarly, S8 should not be represented by one magic number unless the claim is explicitly compressed. Growth and lensing must be separated from geometry so the engine can catch theories that fit distances by breaking structure.

Add posterior predictive checks and mock-data false-positive tests. Generate ΛCDM mock data with DESI/Pantheon+/Planck-prior covariances and run the league. Measure how often w0wa or a unification-inspired extension falsely wins. If that rate is not reported, the engine is not trustworthy enough for frontier claims.

## Theory coverage and a falsifiable unification program

The most worth-chasing target on attainable data is not “quantum gravity” directly. It is a tightly constrained late-time effective theory that connects a theoretically motivated scalar sector to distances and growth: for example, a stable scalar-tensor/dynamical-dark-energy family with a derived w(a) form, sound speed/stability constraints, gravitational-wave speed compatibility, and a modified-growth signature. DESI DR2 + Pantheon+ + CMB distance prior + BBN + RSD can actually falsify parts of that space. Full CMB and weak lensing can come next.

A concrete program:

1. Define a minimal theory manifest with a derived functional form for H(z) or w(a), not just free bins. Declare whether parameters are derived, free physical, or nuisance.
2. Add physical vetoes: no ghost/gradient instability in the declared regime; positive expansion; acceptable early density fraction; GW speed constraints where applicable; BBN-safe early behavior; no negative sound horizon; no hidden post-hoc calibration.
3. Score in stages: background smoke, DESI BAO, DESI+Pantheon+, add CMB distance prior, add BBN, then add growth/RSD.
4. Require falsifiers: a candidate fails if it cannot reproduce Planck θ* distance-prior consistency, if it requires an H0 prior to beat ΛCDM, if it worsens growth beyond tolerance, or if its fitted parameters violate the derivation ledger.
5. Publish the data-dependence plot. If the theory wins geometry but loses growth, that is a scientific result.

The analytic-only unification lane is currently too soft. It may be a useful idea generator, but it is not a scoring target until it maps to observables. The engine should treat unification prose as `hypothesis_source`, not `evidence`. To become scoreable, a unification proposal must emit an effective action or phenomenological equations, a parameter ledger, a validity domain, and a deterministic forward-model adapter. Otherwise it belongs in an idea atlas, not the league.

## ZYAL multi-agent design

The right high-level split is: LLMs propose; deterministic host judges. Preserve it fiercely. ZYAL should generate candidate manifests, repair suggestions, criticism cards, and experiment plans. It should not grade its own physics or mutate policy. The host should validate schemas, run proof lanes, calculate likelihoods, enforce protected paths, and write receipts.

Each ZYAL stage should produce small typed artifacts: `claim`, `evidence_scope`, `source_paths`, `mutation_boundary`, `falsifier`, `validator`, and `expected_receipts`. For cosmology, add `data_combination`, `covariance_status`, `forward_model_validity`, and `claim_class`. The selected docs `docs/ZYAL.md` and `docs/zyal-next-level-design.md` should be revised so the failure mode is explicit: an agent can be eloquent and still scientifically irrelevant unless the host can reproduce the score.

Multi-agent debate is useful only if it leaves adversarial evidence. Add a “hostile cosmologist” role whose only job is to find dataset double counting, diagonal covariance misuse, prior sensitivity, and unsupported CMB claims. Add a “software auditor” role whose only job is to check that no prompts, logs, private paths, downloaded archives, secrets, or browser state entered the artifact. The run should fail if either role emits a blocking finding that lacks a deterministic disposition.

## Software and reproducibility

The public OpenQG surfaces already point in the right direction: evidence-gated benchmark/control plane, typed manifests, reproducible smoke data, explicit scores, auditable release receipts, public data through manifests rather than raw dumps, and candidate theories with named physical parameters. The governance surface says ownership boundaries and generated-output directories matter. Now the cosmology engine has to earn those words.

Concrete software requirements:

- Add dataset manifests with source DOI/arXiv, version, file hash, covariance hash, license, citation, and allowed claim classes.
- Make every likelihood module deterministic from a manifest and a data-vector file.
- Ban silent diagonal fallback. If covariance is missing, the run may be `smoke_only` but not `science_score`.
- Add unit-aware prediction matching; current unit mismatch findings are good but should become score blockers for physical observables.
- Store all priors and parameter bounds in theory manifests.
- Emit receipts with git SHA, dataset hashes, covariance hashes, parameter ledger, optimizer settings, random seeds, and environment.
- Add no-private-path checks to release tar validation.
- Keep Rust in charge of durable policy and run contracts; TypeScript owns dashboards; Python/external services are exception-only numerical adapters.
- Maintain fakeable shell/browser/remote boundaries. The proof lane must run without live browsing or mutable external state.

The minimum release criterion should be: a fresh checkout can reproduce ΛCDM on DESI DR2 + Pantheon+ + Planck-distance-prior smoke data, produce a ranked league report that says exactly which data support which conclusion, and fail loudly if any covariance is missing or diagonalized without an explicit smoke label.

## Stern bottom line

OpenQG is aiming at the right trust target, but the current cosmology credibility bottleneck is not theory generation. It is likelihood realism. Add fewer theories and better data. The first serious milestone is not “find new physics”; it is “reproduce the public ΛCDM and w0wa behavior of DESI DR2, Pantheon+, and a Planck distance prior with the right covariances and honest claim labels.” Only after that should ZYAL be allowed to spend cycles on unification candidates.

A referee will forgive a small, conservative engine. They will not forgive a misleading likelihood, double-counted H0 information, Planck-lite advertised as Planck, or an LLM-generated theory whose fitted knobs are laundered as derivations. Fix those, and the project becomes worth a serious physicist’s time.

## External implementation references

- DESI Collaboration, “DESI DR2 Results II: Measurements of Baryon Acoustic Oscillations and Cosmological Constraints,” arXiv:2503.14738.
- DESI Collaboration, “DESI DR2 Results I: Baryon Acoustic Oscillations from the Lyman Alpha Forest,” arXiv:2503.14739.
- Scolnic et al., “The Pantheon+ Analysis: The Full Dataset and Light-Curve Release,” arXiv:2112.03863.
- Brout et al., “The Pantheon+ Analysis: Cosmological Constraints,” arXiv:2202.04077.
- Planck Collaboration, “Planck 2018 results. VI. Cosmological parameters,” arXiv:1807.06209.
- DES/KiDS collaboration literature for DES-Y3 and KiDS-1000 cosmic shear / 3x2pt validation targets, including the joint DES Y3 + KiDS-1000 analysis arXiv:2305.17173.
- LVK/GWTC standard-siren literature for later independent H0 cross-checks, starting with GWTC-3 constraints and O4/GWTC-4 updates as they become release-grade.
