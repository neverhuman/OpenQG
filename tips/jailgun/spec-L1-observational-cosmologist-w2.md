# Ranked backlog — observational-cosmology hardening specification

**Batch tab:** 1.  
**Source-read status:** blocking intake failure. I attempted to locate and extract `source.tar.gz` in the code sandbox as requested. The archive was not present in `/mnt/data`, the invocation root, `/tmp`, or the accessible filesystem search paths, and searches of the available File Library, Google Drive, and installed GitHub connectors did not expose the requested source tarball or the requested source tree. Therefore this is **not** a completed source-only review of the actual curated archive. It is a fail-closed engineering specification based only on the user-provided project description and selected path list. Any acceptance of scientific claims must wait until `source.tar.gz` is delivered and the named files are inspected directly.

The highest-leverage changes below are still useful because the brief itself identifies the core risk surface: a background+growth cosmology engine, diagonal/partial covariance likelihoods, a theory league using AIC/BIC/evidence, and a ZYAL layer where LLMs propose while a deterministic host judges. The first backlog item is therefore non-negotiable: make the review and run contracts impossible to fake.

## 1. Make source ingestion and review provenance fail closed

**What:** Add an invocation contract that requires `source.tar.gz` to exist, records its SHA-256, validates that it contains only allowed paths, extracts it to a disposable directory, and emits a receipt listing every file read with hash, byte count, and timestamp. For reviews, `engineering-spec.md` must include a machine-generated “source-read receipt” table. If the archive is absent, the review must produce only an intake-failure artifact, not a scientific review.

**Why it matters:** The present invocation demonstrates the most dangerous failure mode for a trust engine: a plausible-looking review can be produced without source access. For a project claiming “whitebox” evidence gating, inability to prove which bytes were reviewed is fatal.

**Effort:** S.

**How to verify:** Run a fixture with no archive and assert that the review says “archive unavailable” and contains no source claims. Run a fixture with an archive and assert that `docs/architecture.md`, `docs/theory-league.md`, `docs/zyal-next-level-design.md`, `docs/ZYAL.md`, and the listed `crates/openqg-core/src/...` files appear in the receipt with stable hashes. Add a negative test for path traversal and tarbombs.

## 2. Replace diagonal cosmology likelihoods with block covariance first, full covariance second

**What:** Implement covariance-aware likelihoods for BAO, supernovae, and compressed CMB. Start with block covariance matrices by dataset family: DESI BAO per tracer/redshift block, Pantheon+ statistical+systematic covariance, and a Planck distance-prior covariance over `(R, l_A, omega_b h^2)` or equivalent. Keep diagonal likelihoods only as an explicitly labelled toy mode.

**Why it matters:** The user brief identifies diagonal-vs-covariance likelihoods as a known limitation. In modern cosmology, covariances are not a refinement; they are part of the measurement. A diagonal likelihood will misstate the relative weight of nearby supernova bins, coupled BAO transverse/radial distances, and CMB compressed priors. It will overconfidently rank extensions when residuals share calibration, reconstruction, or nuisance-parameter systematics.

**Effort:** M.

**How to verify:** For every dataset fixture, require a `covariance_source`, dimension check, positive-semidefinite check or documented regularization, and a golden chi-square value against a public reference likelihood where available. Compare diagonal vs covariance posteriors on LCDM, wCDM, and CPL; fail CI if the diagonal mode is used in any “claim-grade” league run.

## 3. Add DESI BAO as the next geometry anchor, but ingest the covariance, not just points

**What:** Add DESI BAO measurements in the order: DESI Year-1 BAO as a stable baseline, then DESI DR2 BAO when the project can ingest its official covariance and tracer metadata. The observable layer must support `D_M/r_d`, `D_H/r_d`, `D_V/r_d`, effective redshift, tracer class, and correlated `D_M`–`D_H` blocks.

**Why it matters:** BAO is the cleanest immediate stress test for the current background engine. It sharpens `Omega_m`, `H0*r_d`, curvature/dark-energy degeneracies, and time-varying equation-of-state claims while requiring less forward-model complexity than CMB spectra or 3x2pt weak lensing. The DESI 2024 BAO results report robust distance/rate measurements from more than 6 million extragalactic objects over `0.1 < z < 4.2`, and combinations with CMB/SNe are precisely where dynamical dark-energy hints arise. External reference: DESI 2024 VI, arXiv:2404.03002.

**Effort:** M.

**How to verify:** Reproduce DESI-only LCDM constraints to within documented tolerance using the official likelihood products. Verify that profile fits can recover the published qualitative behavior: BAO-only is compatible with flat LCDM, while BAO+CMB+SN can prefer CPL-like evolution depending on SN compilation. The league must print “geometry-only conclusion” separately from “geometry+CMB/SN conclusion.”

## 4. Add Pantheon+ with its full covariance before adding more supernova compilations

**What:** Implement Pantheon+ SNe Ia as a proper covariance likelihood. The data interface must include distance moduli, redshifts, Cepheid-host treatment when SH0ES is enabled, nuisance/marginalization strategy for absolute magnitude, and the full statistical+systematic covariance.

**Why it matters:** The user brief asks where the diagonal approximation most distorts inference. Pantheon+ is one of the worst places to be diagonal because calibration, selection, light-curve, peculiar-velocity, and host-mass systematics couple many supernovae. A diagonal Pantheon+ toy likelihood can produce a false sense that time-varying dark energy or SH0ES-driven extensions are cleanly favored. Pantheon+ reports 1701 light curves of 1550 SNe Ia from `z=0.001` to `2.26` and explicitly treats systematic uncertainties and Cepheid-host covariance in the cosmology analysis. External reference: Brout et al. 2022, arXiv:2202.04077.

**Effort:** M.

**How to verify:** Reproduce Pantheon+ alone constraints for flat LCDM and flat wCDM within tolerance. Run with and without Cepheid-host distances and require the league to label those as different likelihoods: “unanchored SN shape” versus “SN+SH0ES ladder.”

## 5. Implement a defensible CMB-lite lane and ban CMB overclaims until Boltzmann support exists

**What:** Add a compressed Planck distance-prior likelihood as a limited CMB-lite mode, including covariance over the compressed parameters and model-validity guardrails. In parallel, define a full-CMB milestone requiring CLASS/CAMB-backed spectra, Planck TT/TE/EE/lowE/lensing likelihood integration, nuisance parameters, and emulator tests.

**Why it matters:** The current brief admits no Boltzmann CMB. That is acceptable for a distance-prior scout, not for cosmological claims about early-universe physics, neutrino mass, `N_eff`, EDE, primordial spectra, or modified recombination. Planck 2018 full-mission results constrain the six-parameter LCDM model using CMB anisotropy spectra and lensing, with the acoustic scale measured to about 0.03%; a distance prior intentionally discards most of the spectral information. External references: Planck 2018 VI, arXiv:1807.06209; Planck distance priors, arXiv:1808.05724.

**Effort:** CMB-lite M; full Boltzmann L.

**How to verify:** For CMB-lite, reproduce published distance-prior constraints in LCDM, wCDM, and CPL within tolerance. Add runtime guards: if a theory changes recombination, `r_s`, radiation density, early dark energy, neutrino sector, or gravitational-wave propagation in a way that invalidates the compressed prior, the league must reject the CMB-lite likelihood and report “full Boltzmann required.” For full CMB, compare spectra and likelihoods against CLASS/CAMB reference chains.

## 6. Split the league into data-slices with explicit conclusion provenance

**What:** The theory league must report independent scorecards for geometry-only, geometry+SH0ES, geometry+CMB-lite, growth-only, all-late-universe, and claim-grade combined data. Each conclusion must say which likelihoods caused the model ordering.

**Why it matters:** The user brief already notes the key behavior: geometry-only favors LCDM; adding SH0ES can favor a dark-energy extension. That is not a nuisance; it is the scientific result. A serious physicist will attack any single “winner” that hides data dependence.

**Effort:** M.

**How to verify:** Golden tests should include a synthetic case where LCDM wins BAO-only, wCDM wins BAO+SH0ES, and LCDM returns when SH0ES is removed. The report must not state “model X wins” without a data-slice qualifier.

## 7. Put parameter accounting on trial: evidence/AIC/BIC must know which parameters are derived, fixed, shared, or profiled

**What:** Extend the model descriptor so every parameter has a provenance class: fixed physical constant, derived from lower-level theory, externally calibrated nuisance, profiled cosmological parameter, prior-constrained parameter, or free phenomenological knob. AIC/BIC/evidence must use the effective number of parameters appropriate to the run.

**Why it matters:** The project’s “derived-not-fit” claim is valuable only if it survives adversarial accounting. Binding the functional form while fitting the same number of cosmological parameters is not a unification win. Conversely, a real theory-derived relation should earn parsimony credit only if the code prevents hidden calibration.

**Effort:** M.

**How to verify:** Create adversarial candidates that rename fitted parameters as “derived” and assert veto. Create a true fixed-parameter baseline and assert that the league parameter count changes. Compare AIC/BIC and evidence ordering under explicit parameter-count perturbations.

## 8. Add growth data only after the growth model can represent the observable and covariance honestly

**What:** Growth/RSD fixtures should support `f sigma_8(z)`, Alcock-Paczynski coupling, survey covariance, and consistency with the same background parameters used for BAO. Weak-lensing 3x2pt should not be added as a single `S8` scalar except in a clearly labelled smoke lane.

**Why it matters:** A background+growth engine can score simple RSD approximations, but DES-Y3/KiDS-1000 3x2pt needs non-linear matter power, intrinsic alignments, baryonic feedback, photometric-redshift nuisance parameters, shear calibration, scale cuts, and covariance. Compressing all of that to `S8 +/- sigma` is useful for a dashboard but not for model selection. DES Y3 + KiDS-1000 found a combined cosmic-shear `S8` around 0.79 and emphasized analysis choices, baryon feedback, IA, priors, samplers, and non-linear power modeling. External reference: DES Y3 + KiDS-1000, arXiv:2305.17173.

**Effort:** RSD covariance M; 3x2pt L.

**How to verify:** Reproduce a public RSD likelihood before claiming growth support. For DES/KiDS, start with an ingest-only fixture that refuses claim-grade scoring until a real 3x2pt likelihood and nuisance model exists.

## 9. Add BBN D/H as a cheap, high-value early-universe anchor

**What:** Add a BBN likelihood for primordial deuterium and helium with an explicit nuclear-rate uncertainty model and a theory hook for `omega_b h^2`, `N_eff`, and expansion-rate modifications.

**Why it matters:** BBN is much cheaper than full CMB spectra and directly attacks early-universe modifications that try to change `r_d` or ease H0 tension. It is also an excellent falsification surface for unification proposals that touch radiation, particle content, or gravitational strength before recombination. The deuterium abundance is especially sensitive to the baryon-to-photon ratio; precision D/H measurements are a standard baryon-density check. External reference: Cooke, Pettini & Steidel 2018, “One Percent Determination of the Primordial Deuterium Abundance.”

**Effort:** S/M.

**How to verify:** Reproduce standard BBN baryon-density constraints and reject models that change pre-recombination physics while attempting to use a fixed Planck distance prior.

## 10. Treat LVK standard sirens as a late-universe independence test, not a precision anchor yet

**What:** Add a standard-siren lane after BAO/SNe/CMB-lite. Support bright sirens separately from dark sirens and require host-catalog, selection-function, and population-model metadata.

**Why it matters:** Standard sirens are independent of the distance ladder and CMB, which makes them conceptually valuable for H0 adjudication. But current constraints are broad and population-systematics-sensitive; using them as a high-weight likelihood today would be performative. GW170817 gave `H0 = 70.0^{+12.0}_{-8.0}` km/s/Mpc, and dark-siren combinations remain much less constraining than SH0ES or Planck. External references: Abbott et al. 2017, arXiv:1710.05835; Palmese et al. 2021, arXiv:2111.06445.

**Effort:** M.

**How to verify:** Reproduce GW170817 and a dark-siren literature result with broad posteriors. Assert that the league cannot let standard sirens dominate combined evidence unless their covariance/selection metadata says they should.

## 11. Harden ZYAL against “plausible physics prose”

**What:** Require every LLM-proposed theory mutation to ship with a typed observable contract, parameter-provenance table, forbidden-calibration declaration, likelihood-validity declaration, and deterministic replay receipt. The host should reject proposals that lack executable predictions for supported observables.

**Why it matters:** An orchestration layer is useful only if it increases adversarial coverage rather than generating impressive but untestable theory language. “Analytic-only unification” is currently a danger zone: it can look deep while never entering the likelihood.

**Effort:** M.

**How to verify:** Add fake LLM proposals: one with elegant prose but no observables, one with copied fixture values, one with hidden fitted constants, and one with a narrow honest prediction. Only the narrow honest prediction should pass to scoring.

## 12. Reproducibility: make every league result rebuildable from a manifest

**What:** Emit a run manifest containing git/source hash, dataset hashes, covariance hashes, model descriptors, priors, optimizer settings, random seeds, and exact league settings. Store results in append-only JSONL plus a compact human report.

**Why it matters:** Cosmology is hyper-sensitive to priors, nuisance handling, and dataset combinations. Without a manifest, the project cannot defend any model-ranking claim.

**Effort:** S/M.

**How to verify:** A `cargo test` or mapped proof lane should rebuild a small league table bit-for-bit from checked-in fixtures. A deliberate one-byte change in a covariance file must change the manifest hash and invalidate cached evidence.

---

# Data sources and ingestion order

The correct order is not “largest first”; it is “maximal scientific leverage per modeling risk.” For the current background+growth engine described in the brief, the next data should be:

1. **DESI BAO with covariance.** This is the best immediate addition because it lives mostly in the background-distance layer that the project already claims to model. It tests `D_M(z)`, `D_H(z)`, `D_V(z)`, `r_d`, curvature, and dark-energy histories. The key implementation detail is to preserve tracer/redshift blocks and `D_M`–`D_H` covariance. Do not ingest DESI as independent scalar points.

2. **Pantheon+ SNe with full covariance.** SNe add relative distance-redshift shape, which is exactly what distinguishes many late-time dark-energy histories. They are also where diagonal likelihoods most visibly manufacture overconfidence. The first accepted implementation must include the systematic covariance and the SH0ES/Cepheid-host covariance switch as a separate likelihood option.

3. **Planck compressed distance prior, only as CMB-lite.** This is defensible for late-time dark-energy background scouts under LCDM-like early-universe assumptions. It is not defensible for claims about `N_eff`, EDE, recombination, neutrino masses, primordial spectra, or modified gravity affecting perturbations. Every report using CMB-lite must print a warning: “valid only for models that preserve standard early-universe CMB physics at the compression level.”

4. **BBN D/H.** Add before full CMB because it is cheap, high-signal, and catches early-universe cheating around `r_d`, `omega_b h^2`, and radiation content.

5. **RSD/growth with covariance.** Keep this to RSD-style `f sigma_8` and growth-index smoke tests until the engine has a verified matter-power pipeline.

6. **DES-Y3/KiDS-1000 3x2pt.** This is scientifically powerful but should wait for a real likelihood or a deliberate interface to an external likelihood engine. A scalar `S8` fixture belongs in dashboard mode, not model selection.

7. **LVK standard sirens.** Add as an independence and future-readiness lane; do not let it decide current evidence unless selection and population systematics are modeled.

# Forward-model fidelity

A background+growth engine is a legitimate scout. It is not a cosmology engine that can adjudicate frontier claims without guardrails. The forward model should therefore be tiered.

**Tier 0: background geometry.** Support LCDM, wCDM, CPL, curvature, and simple derived quantities: `H(z)`, `D_M`, `D_A`, `D_L`, `D_H`, `D_V`, distance modulus, sound horizon handling, and age. This tier is enough for BAO+SNe+H0 scouts. Verification is reference values against Astropy/CLASS/CAMB for a small grid.

**Tier 1: linear growth approximations.** Support growth ODEs, `D(a)`, `f(a)`, and `f sigma_8(z)` under GR with smooth dark energy. This tier can score RSD-like fixtures, but every model must declare whether the growth equation is valid. Modified gravity cannot silently reuse GR growth.

**Tier 2: CMB-lite and BBN.** Add compressed Planck priors and BBN constraints with validity guards. The system must know when a theory invalidates a compressed prior.

**Tier 3: Boltzmann-backed spectra.** Required for claim-grade CMB, EDE, neutrino, modified-recombination, radiation-sector, inflationary, or unification claims. This likely means integrating CLASS/CAMB or building a validated bridge rather than reimplementing everything in Rust immediately.

**Tier 4: non-linear structure/3x2pt.** Required before DES/KiDS/LSST-style weak-lensing likelihoods become claim-grade. This involves non-linear `P(k,z)`, intrinsic alignments, baryonic feedback, photo-z, shear calibration, survey masks/scale cuts, and covariance.

The review target files, once available, should be checked against this tiering: `crates/openqg-core/src/cosmology/background.rs`, `growth.rs`, `forward.rs`, `scoring/likelihood.rs`, and `scoring/covariance.rs`. Any function that returns a prediction must carry a validity class and fail closed when asked to score outside that class.

# Statistical rigor

The league must be treated as a scientific instrument. AIC/BIC/evidence are not decorative leaderboard columns; they encode assumptions.

First, covariance handling must be first-class. The likelihood should support `chi2 = r^T C^{-1} r`, log determinant terms where needed, covariance provenance, and nuisance marginalization. Positive-semidefinite validation and condition-number reporting should be mandatory. Diagonal mode can stay only for unit tests and pedagogical previews.

Second, parameter accounting must be adversarial. A “derived” relation must be auditable to a source-independent derivation or upstream parameter. If a proposal chooses a coefficient after seeing data, it is fitted. If it fixes a number because it improves the score, it is fitted. If it inherits a reference predictor calibrated on the fixture, it is leakage. The files to inspect later are `crates/openqg-core/src/theory/unification.rs`, `vetoes.rs`, `assessment.rs`, `evaluate.rs`, and `league.rs`.

Third, evidence estimates need calibration. If the league computes Bayesian evidence, it must state the sampler/integrator, priors, convergence diagnostics, and stability under prior widening. AIC/BIC are acceptable fast approximations, but the report must not blur them with actual marginal likelihood.

Fourth, tensions must be represented as data-conditioned statements. H0 and S8 are not single rows to be “explained”; they are stress tests of consistency among likelihoods. SH0ES should be a switchable local-distance-ladder likelihood, not a default prior. S8 should be reported as a derived comparison until real 3x2pt/RSD likelihoods are claim-grade. The league should expose posterior predictive checks and suspicious pulls by dataset.

# Theory coverage and a falsifiable unification program

The most attainable unified-physics target on current data is **a constrained early-to-late dark-sector/radiation-sector model that predicts a limited deformation of `r_d`, late-time `w(a)`, and linear growth while preserving BBN and CMB-lite validity tests where applicable**. This is more realistic than jumping directly to quantum gravity. It touches the frontier because H0, S8, BAO+CMB+SN dark-energy hints, neutrino/radiation content, and growth are all connected; it is attainable because the first scoring surfaces are BAO, SNe, CMB-lite, BBN, and RSD.

The concrete program:

1. Define a minimal theory family with an action or effective equations, not just a CPL wrapper. Require a parameter-provenance table.
2. Derive background equations, sound-horizon consequences, BBN expansion-rate effects, and GR or modified growth equations.
3. Score in staged gates: BAO-only, BAO+SNe, BAO+SNe+SH0ES, BAO+SNe+CMB-lite, BBN consistency, then RSD.
4. Reject if it improves H0 by breaking BBN or using an invalid CMB compression.
5. Promote to full Boltzmann only if it survives late-universe and BBN gates without hidden calibration.
6. Publish a falsification target: parameter regions that solve H0 must predict specific changes in DESI high-z BAO, BBN D/H, and growth; if those are absent, the model is disfavored.

This is honest because it does not claim quantum gravity. It asks whether a structurally constrained theory can survive increasingly adversarial data slices without adding knobs.

# ZYAL multi-agent design

ZYAL should behave like a hostile lab manager, not a brainstorming circle. LLM agents may propose theories, dataset ingests, or likelihood repairs, but the host must decide using typed contracts and deterministic evidence.

Required hardening:

- Every proposal must include observables, equations, parameters, priors, calibration declarations, validity domain, and exact files it expects to change.
- The host must run independent validators for tar hygiene, source hashes, dataset provenance, covariance shape, likelihood golden tests, and parameter leakage.
- Multi-agent disagreement should be structured: one agent proposes, one attacks data validity, one attacks parameter accounting, one attacks software reproducibility. The deterministic host records only the checks, not confidence theater.
- Failed proposals should become negative examples. ZYAL should learn “why rejected” categories: no observable, invalid likelihood, hidden fit, unsupported physics, covariance missing, reproducibility failure.
- Browser/dashboard TypeScript surfaces should remain read-only views over Rust-owned receipts, not sources of truth.

The source files to verify later include `docs/ZYAL.md`, `docs/zyal-next-level-design.md`, `docs/zyal-engine-rebuild.md`, `ZYAL/README.md`, `ZYAL/runs/run-hybrid-1000-v2.zyal`, and `ZYAL/stages/00-atlas/stage.yml`.

# Software and reproducibility

The Rust core should own durable policy, configuration, tar validation, receipts, and run contracts. TypeScript should own only browser/dashboard surfaces. This matches the user’s Jankurai outline and is the right boundary.

Required software contracts:

- **Dataset manifest:** every fixture row points to a source, version, covariance block, units, transformation, and license/provenance note.
- **Likelihood registry:** every likelihood declares whether it is toy, scout, or claim-grade.
- **Run manifest:** source hash, data hashes, covariance hashes, model hash, priors, optimizer settings, random seeds, and output hashes.
- **Golden tests:** LCDM reference predictions, likelihood chi-square values, league ordering on synthetic cases, and source-ingestion failure cases.
- **No private state:** do not commit real prompts, logs, receipts, local paths, browser profiles, downloaded archives, runtime state, or secrets.
- **Proof lane:** a small deterministic proof lane must remain runnable without network access and must prove diagonal toy mode, covariance claim mode, source-ingestion failure, and one league replay.

# Acceptance criteria for the next real review

When the archive is available, a valid source-only review must begin by extracting `source.tar.gz`, reading the ordered files requested in the prompt, and producing a receipt. The review must cite actual passages or symbols from `docs/architecture.md`, `docs/theory-league.md`, `docs/zyal-next-level-design.md`, `docs/ZYAL.md`, and relevant Rust modules. It must distinguish “observed in source” from “recommended.” Any artifact that lacks source hashes or cites only filenames should be rejected.

