# S08 — Production Boltzmann + Emulator Forward Model for OpenQG/ZYAL

This review is based on the extracted `source.tar.gz` contents. The files that matter most are `docs/boltzmann-backend.md`, `crates/openqg-core/src/cosmology/{forward,background,growth,observables}.rs`, `crates/openqg-core/src/scoring/{covariance,likelihood}.rs`, `crates/openqg-core/src/theory/evaluate.rs`, `crates/openqg-bench/src/zyal_genome/physics_score.rs`, `paper/main.tex`, and `docs/zyal-next-level-design.md`. I am intentionally stricter than the repository’s own critique: a constant anchor correction for the CMB acoustic scale is not a V8 fix. It is a regression guard around one known wound.

## Ranked backlog

1. **Build a promotion-grade Boltzmann adjudicator now, not as an optional adapter.** What: add a deterministic external-solver service with warm worker pools, pinned CLASS/hi_class/CAMB/EFTCAMB/MGCAMB builds, immutable solver manifests, cache keys, and hard failure semantics. Why: the paper says the engine’s own fitting formula produced a `+0.755` acoustic-scale bias, `8.4σ`, harvested for roughly `35` nats of fake evidence before anchor calibration closed the valley (`paper/main.tex:42-47`, `paper/main.tex:498-507`). The current optional seam is a one-shot stdin/stdout adapter, not a production adjudicator (`docs/boltzmann-backend.md:10-19`, `docs/boltzmann-backend.md:72-75`). Effort: **L**. Verify: a clean-container replay of 50 candidate evaluations reproduces all promotion decisions; every ledger record includes solver, precision, executable, data, and envelope hashes; solver crash injection produces an infrastructure error, never a silent zero-likelihood or empty veto.

2. **Replace CMB anchor calibration with a searched-space calibration residual envelope.** What: sample fitting-formula-minus-Boltzmann residuals across the actual search distribution, not only at Planck ΛCDM, then add the residual covariance/floors to the likelihood. Why: `BackgroundForwardModel` now anchor-corrects `cmb_lA` and `cmb_R` at the Planck point (`forward.rs:91-108`, `forward.rs:145-149`), but the paper explicitly admits off-anchor residuals and derivatives remain exploitable (`paper/main.tex:654-659`). Effort: **M**. Verify: plant a `+0.755` `l_A` bias and run the V5/V6 valley samples; the envelope must either cover the residual at ≥95% searched-space quantile or mark the candidate `InstrumentRisk` and prevent promotion.

3. **Make fidelity tier explicit in the scorecard and league.** What: add `ForwardTier::{T0Formula,T1Emulator,T2Boltzmann,T3CrossSolver}` to each prediction, score, and league row. Why: `ForwardKind` currently distinguishes `Parametric`, `Derivation`, and `Boltzmann` only coarsely (`forward.rs:13-38`), while selection in `physics_score.rs` still instantiates `BackgroundForwardModel` directly and cannot express “triage score differs from adjudication score.” Effort: **M**. Verify: a candidate can have a high T0 score and still be blocked from champion status until T2/T3 adjudication passes; the ledger shows both scores and the consistency pull.

4. **Add model-error covariance support to `LikelihoodData`.** What: extend covariance scoring so `C_total = C_data + C_model`, with per-observable and cross-observable residual-envelope blocks. Why: `score_metrics_cov` currently supports data covariance blocks and diagonal fallback (`covariance.rs:144-149`) but has no model/systematic covariance. That is exactly why fitting-formula precision was able to masquerade as data precision. Effort: **M**. Verify: Planck distance-prior scoring with the residual envelope reduces the fake `l_A` win to <0.5 nat in the planted-bias test.

5. **Adopt hi_class as the first MG adjudicator and CLASS as the GR/ΛCDM/w0wa reference.** What: use CLASS/classy for GR, ΛCDM, w0waCDM, EDE-like standard-sector candidates; use hi_class for Horndeski/α-basis candidates; use MGCAMB/EFTCAMB as cross-solver referees for μ/Σ and EFT claims. Why: the repository’s current growth path is a 512-step RK4 ODE with scale-free `μ0` plus a small derived f(R)/nDGP `μ(a,k)` hook at one reference scale (`growth.rs:23-27`, `growth.rs:166-180`). It is not a Boltzmann perturbation hierarchy. Effort: **L**. Verify: published nDGP and Hu-Sawicki f(R) `fσ8(z)` curves match within stated tolerance; CLASS/CAMB ΛCDM cross-solver differences are below the envelope.

6. **Train emulators only after the Boltzmann corpus exists.** What: cache Boltzmann solves from actual search and use them to train theory-space emulators with uncertainty and out-of-hull rejection. Why: the loop needs `10^2-10^4` evaluations/generation, so full Boltzmann cannot replace triage. But an emulator trained only near ΛCDM would recreate the same instrument-bias problem in neural form. Effort: **M-L**. Verify: leave-one-region-out validation over champion-like theory neighborhoods; emulator residual covariance must be calibrated enough that 95% of held-out Boltzmann values fall inside predicted intervals.

7. **Separate candidate pathologies from solver infrastructure failures.** What: introduce `ForwardFailure::{Unsupported,DomainError,PrecisionFailure,Timeout,Crash,NondeterministicReplay}` and score them differently. Why: `evaluate_with_blocks` currently turns any forward-model error into `vetoed: true` with empty veto reasons (`evaluate.rs:126-138`). That hides whether a theory is unphysical or the instrument failed. Effort: **S-M**. Verify: unit tests for each failure class; infrastructure failures fail the run or trigger retry, not candidate demotion.

## Design constraints observed in the repository

The existing architecture has the right seam but the wrong weight class. `ForwardModel` promises that every prediction carries a `ForwardManifest` recording code/data provenance (`forward.rs:25-38`). `docs/boltzmann-backend.md` describes a subprocess adapter that receives `CosmologyParams` and observable IDs on stdin and returns `PredictionRecord`s on stdout, with crash/timeout treated as hard error and unsupported observables omitted rather than faked (`docs/boltzmann-backend.md:10-19`). That is a good prototype protocol. It is not enough for a final-phase V8 scientific instrument.

The pure-Rust engine is fitting-formula grade. `CosmologyParams` contains `h`, `omega_m`, `omega_b_h2`, `n_eff`, `sum_mnu`, `w0`, `wa`, `omega_k`, `sigma8`, `mu0`, `mg_family`, `fr_*`, `ndgp_*`, and `drag_a` (`background.rs:60-111`). Distances are Simpson-integrated backgrounds (`background.rs:236-243`); `r_drag` uses an Aubourg-style CAMB-calibrated formula; CMB distance priors are compressed quantities; growth is a late-time ODE for `fσ8(z)` and `S8` (`growth.rs:1-19`, `growth.rs:147-158`). This is a valuable triage model because it is fast and deterministic. It is not a reliable adjudicator for full CMB, CMB lensing, transfer functions, or modified-gravity perturbation observables.

The internal critique already says discriminating data are missing: growth, full CMB TT/TE/EE, lensing, SNe covariance, GW sirens, and real cross-domain likelihoods (`docs/zyal-next-level-design.md:106-115`, `docs/zyal-next-level-design.md:139-150`). It ranks Boltzmann as Tier 4, after SNe and growth. My disagreement is procedural: V8 can still add data tiers in that order, but champion promotion must pass through Boltzmann now because the engine has already demonstrated that the approximate CMB instrument is exploitable. “Later full CMB” is acceptable; “later Boltzmann adjudication of compressed CMB priors” is not.

## Options matrix

| Option | Accuracy on admitted/near-term observables | Warm latency target | MG coverage | Maintenance | Determinism | License / adoption note | Verdict |
|---|---:|---:|---|---|---|---|---|
| **CLASS + classy subprocess** (`class_public`, cite CLASS II arXiv:1104.2933; pin exact git SHA) | Excellent for ΛCDM/wCDM/w0wa backgrounds, CMB `C_l`, lensing, linear `P(k)`, distances; should make compressed `R,l_A,r_d` a derived output, not a fitting formula | ~0.5-3 s for distance+linear spectra; ~1-8 s for high-accuracy CMB likelihood profile depending precision | Standard gravity and standard dark-energy parameterizations; not arbitrary term algebra | Medium | Good if single-threaded BLAS/compiler/container pinned | Public research code; cite required; vendor only after legal review | **Primary reference for GR/ΛCDM/w0wa/EDE-like standard-sector candidates** |
| **CAMB + Python wrapper** (Lewis et al.; pin git/PyPI hash) | Excellent independent reference for CMB, lensing, matter spectra, distances | ~0.5-5 s | Standard gravity; extended by MGCAMB/EFTCAMB | Medium | Good with pinned Fortran compiler/BLAS | Source license must be recorded in `solvers.lock`; do not assume redistribution | **Cross-check CLASS; reference for Planck/CAMB ecosystem** |
| **hi_class** (Horndeski in CLASS; arXiv:1605.06102 / hi_class paper arXiv:1703.?? as in repo `hiclass2017`) | Full linear observables in seconds for Horndeski-like theories; can compute distances, CMB, matter spectra | ~1-10 s; unstable regions may cost more | Strong for α-basis/Horndeski, covariant and parameterized MG; maps naturally to S03 term-algebra outputs via α-functions | High | Good if pinned; stability failures must be explicit | CLASS-derived public code; cite; legal review before bundling | **Primary MG adjudicator for α-basis candidates** |
| **EFTCAMB / H-EFTCAMB** (EFT of dark energy; arXiv:1405.3590 and later) | Strong for EFT/α-basis with stability checks; useful for independent MG likelihoods | ~3-30 s | Broad single-scalar-field EFT; good stability machinery | High | Moderate; Fortran/CAMB stack plus EFT patches | License inherits CAMB plus EFTCAMB terms; record exact commit and license text | **Second-opinion referee for champion-level EFT claims** |
| **MGCAMB** (μ/γ, μ/Σ, Q/R phenomenology; arXiv:1106.4543, 1901.05956, 2305.05667, 2406.09204) | Good for phenomenological modified growth and CMB/LSS consistency tests | ~1-15 s | μ/Σ phenomenology and modified growth; less action-derived | Medium-high | Moderate | Patch to CAMB; license/provenance must be locked | **Referee for μ/Σ lanes, not the sole truth source for action claims** |
| **CosmoPower / CosmoPower-JAX** (arXiv:2106.03846 and successors) | Published errors can be tiny inside training hull for CMB/P(k); unusable outside hull | µs-ms batched | Whatever the network was trained on; most public models are near-standard cosmologies | Medium after training | Excellent inference determinism; training stochastic but lockable | Public repos commonly GPL/non-commercial; risky for product redistribution | **Use as internal emulator pattern; prefer self-trained weights** |
| **Capse.jl-style emulator** (arXiv:2307.14339) | µs predictions with sub-survey-error claims for trained CMB spectra | µs-ms | Training-set limited | Medium; Julia deployment cost | Good when model and BLAS pinned | MIT for Capse code; still validate dependencies | **Good design reference for in-house emulator** |
| **Native Rust mini-Boltzmann** | Could cover distances/growth; full CMB/lensing correctness would take years | Fast if built | Only what OpenQG implements | Very high | Excellent once correct | Project-owned | **Do not build first; use Rust for orchestration, schemas, cache, and replay** |

Recommended V8 configuration: **T0 pure Rust triage, T1 self-trained emulator/cached Boltzmann surrogate, T2 CLASS/hi_class adjudication, T3 cross-solver referee**. For GR, ΛCDM, w0waCDM, EDE-like standard-sector theories, CLASS is the default adjudicator and CAMB is cross-check. For Horndeski/α-basis and screened scalar-tensor candidates, hi_class is default and EFTCAMB or MGCAMB is cross-check, depending on whether the claim is action/EFT-derived or phenomenological μ/Σ. Native Rust remains the protocol and audit layer, not the physics solver.

## Calibration residual envelope

Define an envelope over **the searched theory space**, not over one canonical point. Let `F0(theta)` be the current Rust/fitting-formula prediction vector for a theory/parameter vector `theta`, and `FB(theta)` be the selected Boltzmann reference for the same observable block. For each envelope refresh `E`, construct a sample set:

- all league anchors and decoys: ΛCDM, w0waCDM, high-`h` valley candidates, `planck_mu0`, dark-scattering examples, f(R), nDGP, EDE/coupled-DE anchors once admitted;
- the top `N=256` unique candidates from the last `M=10` generations, fingerprint-deduplicated and parameter-jittered within their local mutation kernel;
- a Latin-hypercube prior over allowed `CosmologyParams` ranges, stratified by mechanism lane;
- adversarial probes around observable derivatives: high `h`, low `Ω_m`, `w0<-1`, nonzero `wa`, low `σ8`, `μ0<0`, nonzero `drag_a`, `fr_log10_fr0` in `[-6,-4]`, and nDGP `Ω_rc` in the linear-validity region.

For an observable block `b`, compute residual vectors

```text
delta_b(theta_s) = F0_b(theta_s) - FB_b(theta_s)
mu_b            = median_s(delta_b)
S_b             = shrink_cov_s(delta_b - mu_b)
floor_i         = quantile_0.95(|delta_i|) + eps_num_i
C_env_b         = S_b + diag(floor_i^2)
C_total_b       = C_data_b + C_env_b
```

Use the median only for diagnostics; do **not** blindly subtract `mu_b` as a correction unless a separate held-out validation proves the residual is stable and monotone. Treat model error as uncertainty unless corrected by a physically justified calibration. The known Planck anchor offset may remain a regression-calibrated constant for T0 continuity, but the likelihood must still receive `C_env` because off-anchor derivatives are the attack surface.

The envelope object is immutable:

```rust
pub struct ResidualEnvelopeManifest {
    pub envelope_id: String,                 // sha256 of canonical payload
    pub generated_at_utc: String,
    pub reference_solver: SolverManifest,
    pub approximate_model_manifest: ForwardManifest,
    pub theory_space_hash: String,           // grammar + lane + parameter bounds
    pub sample_manifest_hash: String,        // fingerprints and parameter vectors
    pub observable_blocks: Vec<EnvelopeBlock>,
    pub coverage_claim: EnvelopeCoverageClaim,
}

pub struct EnvelopeBlock {
    pub block_id: String,
    pub observable_ids: Vec<String>,
    pub covariance: Vec<Vec<f64>>,
    pub per_observable_floor: Vec<f64>,
    pub max_standardized_residual_on_holdout: f64,
}
```

Refresh policy: regenerate the envelope whenever the solver version, precision file, approximate formula, term grammar, admitted observable set, or champion lane changes. Nightly refresh is acceptable during active search; promotion requires a frozen envelope with a recorded `envelope_id`. A promoted candidate must have its own neighborhood included in the next refresh. If a candidate lives outside the envelope hull, it is not “rescored”; it is marked `InstrumentRisk` and cannot be champion until the envelope is expanded.

The `l_A` acceptance test is non-negotiable. Add a feature flag that injects `+0.755` into the approximate `cmb_lA` output while leaving the Boltzmann path untouched. Run the old high-`h` valley points from `paper/main.tex:486-507`. The test passes only if the residual floor/covariance either covers the bias at the Planck distance-prior precision scale or the tier-consistency gate blocks promotion. A hostile reviewer should see that the exact V6 exploit class cannot reappear under a different parameter drift.

## Deterministic external-solver architecture

The current adapter protocol should be replaced by a long-lived worker-pool protocol. Rust owns scheduling, canonicalization, cache keys, failure taxonomy, and ledger receipts. Python/Fortran/C solvers live behind a hermetic process boundary.

Request schema:

```json
{
  "schema_version": "openqg.forward.v2",
  "request_id": "uuid-v7",
  "theory_fingerprint": "sha256:...",
  "solver_theory_ir": {"class": "horndeski_alpha", "alpha_functions": "..."},
  "params": {"h":0.674, "omega_m":0.315, "omega_b_h2":0.02237, "...":"..."},
  "observables": [
    {"id":"cmb_lA", "kind":"distance_prior"},
    {"id":"fsigma8@0.510", "kind":"growth", "z":0.510, "k_h_mpc":0.10},
    {"id":"cl_tt", "kind":"cmb_cl", "ell_min":2, "ell_max":2508}
  ],
  "tier": "t2_boltzmann",
  "solver_selector": "class:gr-default",
  "precision_profile": "promotion-v1",
  "timeout_ms": 15000
}
```

Response schema:

```json
{
  "request_id": "uuid-v7",
  "status": "ok",
  "predictions": [
    {"observable_id":"cmb_lA", "value":301.4707, "uncertainty":0.001, "unit":"dimensionless", "validity":"inside"}
  ],
  "coverage": {"requested": 32, "returned": 32, "unsupported": []},
  "warnings": [],
  "timings_ms": {"background":4.1, "thermo":18.3, "perturb":612.0, "likelihood":91.0},
  "manifest": {"solver_name":"CLASS", "solver_git_sha":"...", "container_digest":"sha256:...", "precision_hash":"sha256:...", "settings_hash":"sha256:..."}
}
```

Workers start warm, load the solver once, complete a capability handshake, and then read length-prefixed JSON or CBOR messages. `K` workers are pinned by CPU socket; `OMP_NUM_THREADS=1`; BLAS vendor, compiler flags, Python version, NumPy/SciPy versions, Fortran compiler, solver git SHA, precision file hash, likelihood data hash, and container digest are recorded in `SolverManifest`. For production, use container images built from a `solvers.lock` file, not floating package installs. Recommended initial lock profile: Python 3.12, NumPy/SciPy pinned by hash, OpenBLAS pinned, CLASS/classy from exact SHA, CAMB from exact SHA/PyPI hash, hi_class from exact SHA, and Planck likelihood data mounted read-only with content hashes.

Cache key:

```text
sha256(
  schema_version || canonical_observable_specs || theory_fingerprint ||
  canonical_solver_theory_ir || ieee754_hex_parameter_vector ||
  solver_name || solver_git_sha || container_digest || precision_hash ||
  settings_hash || likelihood_data_hash || envelope_id
)
```

Use an on-disk content-addressed cache plus an in-memory LRU. Cache successful predictions permanently by key. Cache `Unsupported` diagnostics because they are deterministic. Do not cache `Crash`, `Timeout`, or `NondeterministicReplay` as candidate facts. The cache must store the full response manifest so replay can verify not only values but the instrument that produced them.

Determinism contract: pure-Rust T0 remains bitwise deterministic. External solvers are deterministic within a documented numerical tolerance. Define per-observable replay tolerances: distances and distance-prior values `≤1e-5` relative or `≤0.02σ_data`; CMB `C_l` vectors `≤1e-4` relative for well-conditioned multipoles or `≤0.02σ_band`; `Δχ²` replay `≤0.05` for compressed likelihoods and `≤0.5` for full high-ℓ likelihoods until Planck likelihood non-determinism is characterized. Replay failures are not candidate failures; they are instrument failures.

Failure semantics:

- `Unsupported`: solver honestly cannot compute an observable; coverage decreases; candidate may continue only if the adjudication suite does not require that observable for the claim.
- `DomainError`: theory maps to negative density, ghost/gradient instability, invalid recombination history, or solver-detected physical pathology; candidate is vetoed with explicit physics reason.
- `PrecisionFailure`: solver reaches precision guard, stiff integration failure, or inconsistent convergence; candidate is blocked as `InstrumentRisk` unless reproducibly caused by a physical singularity.
- `Crash`/`Timeout`: infrastructure failure; retry once on a fresh worker, then fail the run lane, not the candidate.
- `NondeterministicReplay`: instrument failure; freeze promotion until resolved.

This directly fixes the current empty-veto behavior in `evaluate_with_blocks` and prevents a future reviewer from asking whether a “bad theory” was merely a broken Python subprocess.

## Tiered fidelity and promotion pipeline

T0 remains the current Rust background/growth path, with residual-envelope covariance attached. It should handle population triage at microsecond-to-millisecond scale. T1 is a batched emulator trained on cached Boltzmann solves from the actual search distribution; it must emit uncertainty and reject out-of-hull points. T2 is the single-solver Boltzmann adjudicator. T3 is cross-solver referee mode for champion promotion and paper claims.

Pipeline:

1. Generation loop scores every surviving candidate through T0, recording approximate predictions, envelope ID, and cheap score.
2. Any candidate in the top 1-5%, any candidate with a novel mechanism claim, any candidate exploiting CMB/growth observables, and a random 1% audit sample are queued for T2.
3. T2 re-fits each model class and the ΛCDM baseline under the same data and complexity ledger. A challenger never compares against a fixed Planck baseline unless the data product explicitly says so; `docs/theory-league.md` already identifies fixed-baseline comparison as indefensible.
4. Tier consistency is computed:

```text
pull = (y_T0 - y_T2)^T (C_env + C_data)^-1 (y_T0 - y_T2)
score_gap = abs(DeltaLnZ_T0 - DeltaLnZ_T2)
if pull > chi2_quantile(df, 0.99) or score_gap > max(1.0 nat, 2*sigma_env_score):
    status = InstrumentRisk
```

5. `InstrumentRisk` candidates are not simply rescored downward. They trigger envelope refresh, emulator retraining, and a report explaining which observables paid the difference. This is how V8 turns instrument exploits into tests instead of quietly laundering them into “better fidelity.”
6. Champion promotion requires T2 pass, T3 cross-solver agreement on all headline observables, and a replay run in a clean container.

Cache economics make this feasible. Suppose a generation evaluates `10^4` candidates. T0 handles all. T2 audits 1-5%, or 100-500 solves. With warm CLASS/hi_class workers at 1-10 seconds and 32 workers, the wall time is minutes to a few hours, not days. Cache reuse across mutation neighborhoods should reach 60-90% once parameter vectors are canonicalized and league/profile-fit grids are memoized. Full T3 cross-solver is reserved for the top handful of candidates and nightly league runs.

## What the Boltzmann backend unlocks

**Full CMB TT/TE/EE or plik-lite.** Accuracy requirement: reproduce published Planck ΛCDM likelihood or plik-lite compressed `χ²` to the tolerance of the public likelihood product; for internal comparisons, `Δχ²≤1` versus a known Planck best-fit run is the acceptance bar. Runtime: 1-8 seconds per CLASS/CAMB high-accuracy solve plus likelihood overhead. Verdict impact: it destroys background-only H0 tricks, tests EDE/recombination-era claims, constrains ISW-sensitive MG behavior, and prevents compressed-prior derivatives from being exploited.

**CMB lensing `C_L^{φφ}`.** Accuracy requirement: lensing likelihood replay within `Δχ²≤0.5` for Planck lensing-lite or published reference examples; linear lensing spectra within ≤0.5% over the used multipole range after precision tuning. Runtime: 10-50% over a full CMB solve. Verdict impact: crucial for the V7 suppressed-growth class, because CMB lensing is a growth-sector observable integrated over redshift and scale. A late-time `μ0<0` or dark-scattering drag that helps low-redshift `fσ8` can still fail lensing amplitude or shape.

**Linear `P(k,z)` and full-shape RSD.** Accuracy requirement: linear `P(k)` ≤0.5% for `k<0.2 h/Mpc` and `z≤2` in ΛCDM/w0wa; for MG, validate against published f(R)/nDGP references before using nonlinear scales. Runtime: 0.5-5 seconds with spectra; nonlinear models 2-20 seconds depending add-ons. Verdict impact: this is where the suppressed-growth champion lives or dies. The current `fσ8(z)` scalar ODE can hide scale dependence and lets `σ8` absorb `μ0`; full-shape and scale-dependent growth break that degeneracy.

**Nonlinear/lensing power.** Accuracy requirement: do not use nonlinear MG predictions for champion promotion until screened nonlinear corrections are validated to a stated regime. For ΛCDM/w0wa, HMcode or emulator-backed nonlinear spectra can support weak-lensing `S8` blocks with 1-2% systematic covariance. Runtime: seconds to tens of seconds. Verdict impact: high; but use as a veto-risk flag until the MG nonlinear model is honest.

These data additions coordinate with S05 data acquisition, but S08 must own the prediction side. A dataset is not admitted until the forward path can emit its observable with a solver manifest and an error model.

## Acceptance tests

1. **Planck ΛCDM replay.** Given a locked CLASS/CAMB image and public Planck or plik-lite data, reproduce a published ΛCDM best-fit `χ²`/compressed likelihood within the documented product tolerance. Store the request, response, solver manifest, and replay tolerance in the ledger.

2. **CLASS-CAMB cross-solver smoke.** For Planck ΛCDM, w0waCDM, and a mild curved model, compare `r_drag`, `R`, `l_A`, selected TT/TE/EE bandpowers, `σ8`, and `P(k,z)` over a grid. Differences must be below the residual envelope or fail the solver profile.

3. **nDGP and f(R) growth regression.** For published benchmark parameters, match reference `fσ8(z)` or growth-ratio curves for nDGP and Hu-Sawicki f(R) to ≤1% in the linear regime and ≤0.5% for GR limiting cases. Use DGP/nDGP references (Dvali-Gabadadze-Porrati, Koyama-Maartens) and Hu-Sawicki f(R) arXiv:0705.1158. The test must include `k=0.1 h/Mpc` because the current Rust path uses that as a reference scale.

4. **Residual envelope catches planted `l_A` bias.** Inject `+0.755` into T0 `cmb_lA`, sample across valley-like high-`h` candidates, and prove that `C_env` or `InstrumentRisk` prevents the old fake evidence. This is the signature V8 test.

5. **End-to-end deterministic replay.** Run a fixed 50-candidate mixed population through T0/T2/T3 twice in clean containers. Candidate states, promotion flags, cache keys, solver manifests, and scores must match within the replay tolerance. Any nondeterministic solver output is a red build.

6. **Failure taxonomy.** Mock `Unsupported`, `DomainError`, `PrecisionFailure`, `Timeout`, `Crash`, and malformed JSON. Verify that only physical domain errors become candidate vetoes; infrastructure failures block the lane and appear in run health.

7. **Tier-consistency league test.** Construct a candidate whose T0 score is >2 nats better than T2 because of an approximate-formula residual. It must be flagged as `InstrumentRisk`; the league table must not list it as a champion with a quiet T2 replacement score.

## Sibling-spec interfaces

See **S03** for the term-algebra-to-solver interface: S08 needs a `SolverTheoryIR` containing α-functions, EFT functions, μ/Σ phenomenology, or named covariant models, but should not define the symbolic proof language. See **S05** for data acquisition: S08 defines the prediction and likelihood admission contract, not which external datasets are fetched first. See **S09/S10** if those own scorecard/governance: S08 requires that fidelity tier and instrument-risk status be first-class scorecard fields.

## What we got wrong

1. **“Anchor-calibrated `l_A` is safe enough.”** It is not. The paper itself says the constant offset is validated only at Planck and that off-anchor derivatives remain exploitable (`paper/main.tex:654-659`). Check: compare formula and Boltzmann `l_A`, `R`, and `r_d` over the last 10 generations plus valley probes; if derivative residuals exceed the envelope, no champion can use T0 CMB credit.

2. **“The current growth sector tests suppressed-growth physics.”** Only weakly. The current ODE tests a scale-free late-time `μ0` and a simple drag term (`growth.rs:43-76`), with limited derived f(R)/nDGP hooks. It does not test CMB lensing, transfer functions, ISW, or scale-dependent full-shape growth. Check: run the V7 `planck_mu0` candidates through hi_class/MGCAMB with CMB lensing and full-shape RSD; if the sign of the preference changes, the V7 conclusion was about the toy instrument, not the sky.

3. **“A subprocess seam equals a backend.”** No. The documented seam is a useful mockable boundary, but production needs warm workers, capability negotiation, cache, manifests, precision profiles, and replay tolerances. Check: run 1,000 mixed requests through the current one-shot adapter under crash/timeout/malformed-output injection; if run health cannot distinguish candidate pathology from infrastructure, it is not production-grade.

4. **“Emulators solve the speed problem by themselves.”** They solve only interpolation inside a known training hull. A neural emulator trained around ΛCDM can be an even more dangerous fitting formula: smooth, fast, and confidently wrong where evolution discovers incentives. Check: leave out each champion-like lane during emulator training and verify calibrated uncertainty on that lane before allowing emulator scores into promotion.

5. **“Native Rust Boltzmann is the cleanest deterministic answer.”** It is clean in provenance and bad in opportunity cost. Reimplementing recombination, perturbation hierarchies, neutrinos, lensing kernels, precision tuning, and Planck likelihood compatibility would consume the phase that should be used to falsify candidates. Check: estimate the implementation required to pass the Planck ΛCDM replay and f(R)/nDGP regression tests; if it is not measured in weeks, do not start with native Rust.

6. **“Solver failure can be a lethal candidate by default.”** Sometimes yes, often no. A negative-energy background or unstable perturbation is a candidate failure; a Python crash, BLAS nondeterminism, or timeout is an instrument failure. The current `evaluate_with_blocks` loses that distinction by returning an empty veto on any prediction error (`evaluate.rs:126-138`). Check: inject each failure class and require distinct ledger status.

7. **“Compressed likelihoods are good enough for discovery claims.”** They are good enough for triage and internal league pressure, not for external claims about CMB-era or MG physics. The repository already learned this with the Planck distance-prior bias. Check: any claim whose evidence depends on CMB priors, `σ8/S8`, or `fσ8` must be re-evaluated with Boltzmann-derived CMB/lensing/full-shape observables before it appears in a paper abstract.

## Minimal implementation milestone

The smallest defensible V8 milestone is not “full Planck + all MG.” It is: CLASS/CAMB locked containers; a Rust worker pool; residual envelope for current Tier-0 observables; explicit failure taxonomy; a promotion path that rescoring the top 1-5% through CLASS/hi_class; and the planted `l_A` exploit test. Once that passes, train emulators from the cached solves and add CMB lensing/full-shape data. The loop remains fast, but the final word moves from fitting formulas to a reproducible Boltzmann-grade instrument.
