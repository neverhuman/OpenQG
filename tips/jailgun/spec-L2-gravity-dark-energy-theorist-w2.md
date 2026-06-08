# OpenQG / ZYAL hardening review — engineering specification

Reviewer stance: the archive is directionally serious and unusually honest about limitations, but it is not yet a physicist-grade theory-discovery engine. It is a defensible FLRW+CPL+linear-growth scorer wrapped in a promising governance/orchestration shell. The next work is not cosmetic. The highest-risk gaps are concrete and fixable: the current growth/MG league has a parameter-application bug; the theory genome is too low-dimensional for modified gravity; the veto cascade accepts declarations where it must require derived checks; the data stack is still mostly diagonal, compressed, and pre-Boltzmann; and ZYAL agents do not yet create falsifiable structural theory space.

Source citations below use archive paths and line ranges from the extracted `source.tar.gz` snapshot.

## Ranked backlog

| Rank | Change | Why it matters | Effort | Verification |
|---:|---|---|---|---|
| 1 | **Fix `ModelClass` parameter plumbing for growth/MG. Add `sigma8` and `mu0` to `set_param()` and make unknown free parameters fatal.** | `screened_mg()`, `lcdm_growth()`, and `w0wa_cdm_growth()` declare `sigma8`/`mu0` free, but `set_param()` handles only `h, omega_m, omega_b_h2, n_eff, sum_mnu, w0, wa, omega_k` (`crates/openqg-core/src/theory/league.rs:48-59, 124-155`). That means current growth/MG league fits are partly no-ops while still paying a complexity penalty. | S | Unit test: fit synthetic `fsigma8`/`S8` data generated with `sigma8=0.75, mu0=-0.3`; recover both within tolerance. Add `debug_assert!(set_param(...))` or return `Result` so an unknown field fails the model definition. |
| 2 | **Promote theory families to first-class derived sectors, not constant `mu0` handles.** | The docs correctly identify α-basis + CPL + late-time `mu0` as a narrow scoreable approximation (`docs/architecture.md:73-85, 133-147`; `docs/theory-league.md:64-70`), but a modified-gravity theorist will not accept a constant late-time `mu(a)` as the genome. The engine should evolve model classes whose full `alpha_i(a[,k])`, screening, and background are derived from action-level parameters. | L | Add `ModelClass::{HuSawickiFR, NDGP, NoSlip, CoupledQuintessence, EDEScalar}` with snapshot tests against published reference curves for `H(a)`, `D(a)`, `mu(a,k)`, `Sigma(a,k)`, and any `alpha_i(a)`. |
| 3 | **Replace declaration-based vetoes with derived physical vetoes on a redshift/scale grid.** | The current cascade is a useful scaffold, but `alpha_T` tolerance is `1e-2` despite comments acknowledging the measured bound is `~1e-15` (`crates/openqg-core/src/theory/vetoes.rs:14-20`), stability coefficients are candidate-supplied fields (`theory/mod.rs:122-135`), and PPN screening passes on `screening.is_some()` (`vetoes.rs:112-118`). These are not physics checks; they are labels. | M-L | Veto report must include computed `M_*^2(a)>0`, `Q_T>0`, `c_T^2(a)`, `Q_S(a)>0`, `c_s^2(a)>0`, strong-coupling margins, Cassini/EP likelihoods, and screening radii/thin-shell factors for at least f(R), nDGP, Galileon/no-slip anchors. |
| 4 | **Use real covariances by default and make diagonal-only runs visibly non-promotable.** | Covariance code exists (`crates/openqg-core/src/scoring/covariance.rs:144-149`), but the shipped fixtures are diagonal (`data/fixtures/cosmology/tier0-combined.jsonl:1-15`; docs admit this at `docs/architecture.md:269-271`). The old `+36.7` problem was partly diagonal-likelihood inflation (`docs/theory-league.md:10-16`). | M | A promotion artifact must include DESI BAO block covariances, CMB distance-prior covariance including `omega_b_h2`, Pantheon+/DES-SN covariance, and a provenance hash. CI should fail `production` labels when `LikelihoodData.blocks.is_empty()` for correlated sources. |
| 5 | **Wire a Boltzmann/hi_class lane behind stable receipts.** | Background+growth cannot score CMB damping tail, lensing, ISW, `P(k)`, EDE, or real Horndeski perturbations. The project names this as the largest fidelity gap (`docs/architecture.md:149-151, 265-268`). | L | `ForwardKind::Boltzmann` manifest with solver version, ini hash, data hash, and reproducible compressed likelihood tests. Compare ΛCDM TT/TE/EE/lensing and BAO outputs to Planck/CLASS references within published tolerances. |
| 6 | **Replace “coverage” with observable-family capability accounting.** | Current coverage can be `1.0` over a deliberately narrow Tier-0 set (`docs/production-run-1000.md:15-16, 53-60`), so it overstates physics coverage. | S-M | Coverage report grouped by `background`, `growth`, `lensing`, `CMB_primary`, `CMB_lensing`, `local_gravity`, `GW`, `BBN`, `galactic`; a model cannot claim modified-gravity credibility without nonzero growth+lensing+local-gravity coverage. |
| 7 | **Make ZYAL agents propose structured theory edits and adversarial tests, not score numbers.** | The invariant “LLM proposes; deterministic host judges” is right (`docs/ZYAL.md:215-220`), but the archive lacks the full proposer/mutation modules and the docs describe a legacy/live split not yet retired (`docs/ZYAL.md:227-237`; missing modules referenced in `theory/mod.rs:19-24`). | M-L | Agent output schema must be a typed AST diff: action terms, fundamental parameters, derivation sketches, claimed observables, and falsifiers. Every accepted proposal has a receipt: parsed AST, derivation-oracle verdict, veto report, forward manifest, likelihood artifact. |
| 8 | **Turn unification from analytic self-consistency into independent likelihoods.** | `unification.rs` uses `alpha_T`, screening flag, a duplicated `Y_p` comparison, and a simple siren ratio (`unification.rs:58-125`). The docs admit this is not independent data (`docs/architecture.md:272-274`; `docs/zyal-next-level-design.md:97-105`). | M | Add likelihood blocks for GW170817 speed, LVK bright/dark sirens, Cassini, MICROSCOPE/Eöt-Wash, BBN D/H+Yp covariance, and local `dot G/G`. Report a vector of log-likelihood contributions, not just a min-score. |
| 9 | **Separate discovery search from adjudication in UI and reports.** | The code now has `theory league` as rigorous adjudicator (`docs/theory-league.md:18-28`), but production docs still foreground a “1000 generations” narrative (`docs/production-run-1000.md:1-5, 36-51`). A referee will attack any headline not from the league. | S | Run reports must have two sections: “search heuristic output” and “league result.” Only league ΔAIC, ΔBIC, profile likelihood, or nested evidence may appear in executive summaries. |
| 10 | **Ship a runnable proof lane or state that the archive is a curated source slice.** | `theory/mod.rs` references missing `holdout`, `mutation`, `pareto`, `proposal`, and `robustness` modules (`theory/mod.rs:19-24`), and there is no root `Cargo.toml` in this archive. A reviewer cannot compile this snapshot as-is. | S | Add a minimal workspace or a `SNAPSHOT.md` saying compile is out of scope. Better: include enough files for `cargo test -p openqg-core theory::league` to pass in the proof lane. |

## 1. Data sources and data policy

The current Tier-0 fixture is useful but too small and too compressed: 12 DESI DR1 BAO ratios, Aver-style helium, and two Planck distance priors (`data/fixtures/cosmology/tier0-combined.jsonl:1-15`; described in `docs/architecture.md:225-239`). Tier-1 growth is five RSD `fσ8` points (`growth-rsd.jsonl:1-5`), and Tier-2 local ladder is one SH0ES H0 point (`sh0es-h0.jsonl:1`). That is adequate for smoke tests. It is not adequate for claims about quantum gravity, unification, or the viability of Horndeski-family models.

First, update public cosmology fixtures to the present comparison set: DESI DR2 BAO, Pantheon+ or DES-SN5YR with full covariance, Planck/ACT compressed or full likelihoods, DES/KiDS/HSC weak-lensing summaries, and recent LVK standard-siren likelihoods. The archive still cites DESI DR1 as the geometry base (`data/fixtures/cosmology/tier0-combined.jsonl:1-12`) while the field has moved to DR2-scale BAO. This matters because the target use case is not “does a toy engine reproduce a 2024 fixture,” but “does a serious engine respond honestly when current high-precision surveys move the posterior.” The data loader should support versioned data bundles outside git with explicit source manifests and hashes; the docs already say raw bulk data is not committed (`docs/architecture.md:254-256`), so the missing piece is a locked, reproducible external-data contract.

Second, the fixture format needs covariance and likelihood-family metadata. `ObservableRecord` rows are too atomistic for correlated BAO, SN, and CMB-prior sources. Add `dataset_id`, `block_id`, `vector_order`, `likelihood_kind`, `systematic_family`, and `provenance_hash`. A DESI tracer block should enter as a vector with a covariance matrix, not as independent JSONL scalars. The covariance implementation can marginalize missing members (`scoring/covariance.rs:144-149`), which is good; now make it the normal path, not a follow-up caveat.

Third, data tiers should be tied to theory claims. A model that modifies gravity cannot be promoted using only distance ratios and helium. Promotion requirements should be claim-sensitive: EDE requires CMB primary spectra or a validated compressed early-universe likelihood; f(R)/nDGP/no-slip require growth + lensing + local gravity; interacting dark energy requires background + growth + CMB stability priors; galactic modified gravity requires SPARC/cluster/Bullet-style sectors. “Coverage 1.0” over Tier-0 must not be shown next to a modified-gravity champion without a red warning that growth/lensing/CMB/local-gravity coverage is absent.

## 2. Forward-model fidelity

The background model is the strongest part of the archive. It integrates CPL FLRW distances, BAO ratios, CMB distance priors, and a linearized BBN helium fit (`cosmology/background.rs:109-124, 131-170, 200-270`). The forward seam is also well designed: `ForwardModel` omits observables it cannot compute rather than faking them (`cosmology/forward.rs:49-56, 176-186`). Preserve that design.

The weak point is not the FLRW quadrature; it is the theory space projected onto it. The engine uses `CosmologyParams {h, omega_m, omega_b_h2, n_eff, sum_mnu, w0, wa, omega_k, sigma8, mu0}` (`background.rs:35-64`) and a linear growth ODE with `mu(a)=1+mu0 Ω_DE(a)/Ω_DE0` (`growth.rs:42-56, 68-127`). That is a reasonable phenomenological extension, but it is not a derived modified-gravity model. It has no scale dependence, no slip `η=Φ/Ψ`, no lensing response `Σ`, no time-dependent `alpha_i(a)`, no quasi-static validity flag, and no mapping from action terms to perturbation coefficients.

The next forward-model object should be a `PerturbationModel` returned by theory sectors:

```text
H(a), M_*^2(a), alpha_K(a), alpha_B(a), alpha_M(a), alpha_T(a),
mu(a,k), Sigma(a,k), eta(a,k), c_s^2(a), Q_s(a), Q_T(a), screening_profile(r,a,environment)
```

For sub-horizon growth, integrate either the quasi-static system or a hi_class/H-EFTCAMB-backed full perturbation calculation. The minimal quasi-static lane should evolve

```text
D'' + [2 + H'/H]D' - 3/2 Omega_m(a) mu(a,k) D = 0,
fσ8(z,k) = σ8 D(z,k)/D(0,k) d ln D / d ln a,
Σ(a,k) controls lensing: k^2(Φ+Ψ) = -8πG a^2 Σ(a,k) ρ_m Δ_m.
```

Do not collapse `mu` and `Sigma` into one knob. RSD constrains matter clustering through `μ`; weak lensing constrains `Σ`; ISW and CMB lensing constrain time derivatives and line-of-sight potentials. The current `mu0` is degenerate with `σ8` because `fσ8` is an amplitude observable: increasing `μ` changes the growth normalization and rate, while lowering `σ8` can compensate in `σ8·D·f`. With only scale-independent `fσ8(z)` plus one `S8`, the model can trade growth strength against initial amplitude. The degeneracy breaks when the engine sees (a) scale dependence in `P(k)`/RSD, (b) lensing-vs-clustering slip through `Σ` and `E_G`, (c) ISW/CMB lensing sensitivity to potential time evolution, and (d) external amplitude priors from `A_s e^{-2τ}` or full CMB.

## 3. Statistical rigor

The project correctly corrected its worst headline: the old `+36.7` was a fitted candidate against a fixed baseline under a diagonal likelihood (`docs/theory-league.md:10-16`; `docs/production-run-1000.md:48-51`). The league design is the right response: profile-fit every model, refit ΛCDM, then rank by AIC/BIC/Schwarz evidence (`docs/theory-league.md:18-28`; `theory/league.rs:1-19`). Keep this as the only publishable adjudicator.

But fix the implementation before trusting growth/MG rows. As noted in backlog rank 1, `sigma8` and `mu0` are declared free (`theory/league.rs:124-155`) but cannot be set (`league.rs:48-59`). Worse, `params_for()` ignores `set_param()` failure (`league.rs:158-164`), so the model definition can silently lie. This is a serious correctness bug. Any reported conclusion that `mu0` is not favored needs rerun after this fix; the physical degeneracy may remain, but the current code cannot establish it.

The league should also separate parameter counting by model semantics. `k` should include only parameters actually optimized and physically active in predicted observables. If a model has unpredicted sectors, do not count them as fitted unless their predictions enter the likelihood; alternatively, report “inactive parameters” as a warning. For Bayesian evidence, BIC is acceptable as a sprint-level approximation (`docs/theory-league.md:87-88`), but serious publication should add nested sampling or at least profile-likelihood intervals and prior-volume sensitivity. EDE, f(R), nDGP, Galileon, and coupled dark energy are not Gaussian-linear enough for BIC alone to settle.

Scoring should also penalize missing predictions more formally. The current covariance scorer records missing block members while marginalizing over present ones (`covariance.rs:180-218`) and computes coverage (`covariance.rs:242-259`). That is good for honest partial predictions, but model comparison can become unfair if one theory avoids difficult observables. For final league tables, require a common observable set for every model in a claim family, or rank models in separate “coverage-compatible” leagues. Otherwise a background-only model can be compared against a perturbation-capable model on a dataset where it omits the very observables that would kill it.

## 4. Theory coverage and encoding

### 4.1 Is α-basis + CPL + late-time μ0 the right genome?

As a low-dimensional smoke-test genome, yes. As a discovery genome for modified gravity and dark energy, no. The Bellini-Sawicki α-basis is the right lingua franca for linear Horndeski phenomenology (`docs/architecture.md:77-79`; `theory/mod.rs:89-100`), but the engine stores α-values evaluated today, not α-functions derived from a Lagrangian. CPL is a useful background phenomenology, but it cannot represent EDE, thawing/freezing scalar fields with physical potentials, interacting dark sectors, or screening. A constant late-time `mu0` is too coarse for f(R), nDGP, Galileon, no-slip gravity, or DHOST.

Recommendation: keep α-basis as an interchange layer, not as the primary genome. The primary genome should be a `TheorySector` enum with action/sector parameters and a deterministic `derive()` method that emits background and perturbation functions. For example:

```text
enum ModelClass {
  LCDM,
  CPL,
  ScalarFieldEDE { V0, n, theta_i, f_axion_or_m, initial_conditions },
  CoupledQuintessence { V(phi), beta_c(phi), disformal_D(phi) },
  HuSawickiFR { n, f_R0 },
  NDGP { r_c, branch, brane_tension },
  CovariantGalileon { c2,c3,c4,c5 with luminal-safe restrictions },
  NoSlipGravity { M_*^2(a) ansatz satisfying alpha_B=-2 alpha_M },
  EFTAlphaFunctions { basis, coefficients, priors, validity_window },
  DHOST { class, degeneracy_conditions, c_T_certificate }
}
```

The `EFTAlphaFunctions` fallback is acceptable for phenomenology, but any “derived-not-fit” claim must prefer sector-specific derivations. Do not let a proposer set arbitrary α-functions and call them fundamental.

### 4.2 Veto cascade critique

The current cascade is valuable as a cheap first pass, but physically incomplete (`docs/architecture.md:93-107`; `vetoes.rs:46-120`). Specific fixes:

* **GW speed:** `|alpha_T| <= 1e-2` is not a credible bound for late-time Horndeski after GW170817. Use a redshift-aware evaluation at the GW170817 epoch and require `c_T/c-1` at the observational bound unless the theory carries an explicit EFT validity/frequency-screening certificate. A 1% tolerance should be an exploratory warning, not a survival gate.
* **Tensor sector:** add `M_*^2(a)>0`, `Q_T>0`, and `c_T^2(a)>0` across the validity range.
* **Scalar stability:** derive `Q_S` and `c_s^2` from the α-functions and background over a grid. Candidate-supplied `q_s` and `sound_speed_sq` are not evidence. Penalize near-zero `c_s^2`/`Q_S` for strong coupling, not merely negative values.
* **DHOST/Ostrogradsky:** `has_nondegenerate_higher_derivatives` is a label. For DHOST, encode the degeneracy class and verify algebraic degeneracy conditions.
* **PPN/screening:** `screening: Some("vainshtein")` cannot pass Cassini. Compute Vainshtein radius, chameleon thin-shell parameter, symmetron/k-mouflage environmental profile, and resulting `γ_PPN-1`, fifth-force strength, and local `dot G/G`.
* **Other missing constraints:** gravitational Cherenkov bounds for subluminal tensor modes, binary-pulsar damping where relevant, Lunar Laser Ranging/local `G_eff`, equivalence-principle composition dependence for conformal couplings, BBN/CMB variation of effective Planck mass, and positivity/unitarity priors for EFT coefficients.

### 4.3 Which theories to chase and how to encode them as derived `ModelClass`es

**Highest attainable unified-physics target:** screened scalar-tensor gravity with derived α-functions, especially Hu-Sawicki f(R), nDGP, and no-slip gravity, scored against growth+lensing+local-gravity data. This is the best near-term target because the archive already has α-basis concepts, growth integration, screening hooks, and a league. It is more scoreable on attainable data than string/LQG/asymptotic-safety claims, and more discriminating than CPL alone.

**EDE:** encode an axion-like scalar sector, not an `f_ede` knob. Integrate

```text
phi¨ + 3H phi˙ + dV/dphi = 0,
rho_phi = phi˙²/2 + V(phi), p_phi = phi˙²/2 - V(phi),
H² = (8πG/3)(rho_m+rho_r+rho_phi+...),
f_ede = max_a Omega_phi(a), z_c = argmax transition.
```

`f_ede` is then computed from initial displacement, potential scale, exponent, and mass/critical redshift parameters. It must modify `r_drag`, equality, early ISW, and CMB peaks; without Boltzmann/CMB it should not be promoted.

**Coupled/interacting dark energy:** encode conformal/disformal coupling, e.g. `A(phi)=exp(beta phi/M_Pl)` and optional `D(phi)`. Derive

```text
rho_c' + 3H rho_c = Q phi',
phi'' + (3+H'/H)phi' + V_,phi/H² = -Q/H²,
Q ∝ beta(phi) rho_c / M_Pl
```

and perturbation modifications to friction and effective gravity. `beta` may be fundamental, but `Q`, background dilution, fifth-force strength, and growth suppression must be derived. EP/local constraints must be part of the unification likelihood.

**nDGP:** one parameter `r_c` derives the expansion modification and Vainshtein-screened scalar force. In the normal branch with Λ, derive the brane-bending mode contribution to `G_eff` and growth, including Vainshtein screening for local tests.

**Hu-Sawicki f(R):** parameters `{n, f_R0}` derive scalaron mass, Compton wavelength, scale-dependent `mu(a,k)` and `Sigma(a,k)`, chameleon screening, and effective α-relations. Score on RSD, lensing, clusters, and local bounds.

**Covariant Galileon:** only admit luminal-safe subsets after GW170817 unless a valid decoupling certificate exists. Derive Vainshtein screening and stability; most naive covariant Galileon choices should die.

**No-slip gravity:** encode `alpha_B = -2 alpha_M` with `alpha_T=0`, so `mu` and `Sigma` relations are constrained rather than free. This is a good derived α-function test because it directly links GW friction, lensing, and growth.

## 5. ZYAL multi-agent design

The ZYAL principle is correct: LLMs propose/critique, deterministic code judges (`docs/ZYAL.md:200-203, 283-284`). The problem is that the physics generator is not yet a structural theory generator. The docs themselves say the old live/legacy machinery still exists and the new symbolic engine is the one to build on (`docs/ZYAL.md:227-237`). The archive also contains only a source slice: `theory/mod.rs` references non-included `proposal`, `mutation`, `pareto`, `holdout`, and `robustness` modules (`theory/mod.rs:19-24`), so an outside reviewer cannot validate the claimed loop from this tarball.

ZYAL should be reorganized into five roles with hard interfaces:

1. **Proposer agents** emit typed sector/action diffs, not prose and not free parameter vectors.
2. **Derivation agents** emit algebraic sketches and reference equations; they do not receive fitness.
3. **Skeptic agents** try to refute derivations and identify missing observables.
4. **Adversary agents** generate decoys and held-out tests that should kill overfit theories.
5. **Deterministic host** parses, demotes, vetoes, integrates, scores, and writes receipts.

The host must treat every unverified derivation as `Free`. A valuable ZYAL contribution is not “a better score”; it is a new falsifiable sector with equations, validity domain, and a prediction set. Memory should store certified derivation lemmas, killed regions, data-release provenance, and decoy families. Do not let long-running browser/jailgun state enter physics receipts; per the Jankurai outline, durable policy/config/tar validation/receipts/run contracts belong in Rust, while TypeScript owns browser/dashboard surfaces.

## 6. Software and reproducibility

The pure-Rust deterministic forward model and manifests are a good foundation (`cosmology/forward.rs:25-38, 140-148`). Now enforce them everywhere. Every score artifact should contain: git/source hash, data bundle hash, covariance hash, forward manifest, solver version, feature flags, model class schema version, parameter priors, optimizer settings, random seed, veto report, coverage matrix, and league rows. If Boltzmann solvers are used, include container image hash and external executable checksums.

The curated archive cannot currently be used as a runnable proof lane because it lacks a root workspace and references missing modules. That may be intentional for the review bundle, but then it should say so explicitly. If this is meant to be extracted over a repo, add a `proof-lane.md` that lists exact commands expected to pass after overlay. If it is meant to stand alone, include a minimal Cargo workspace and the referenced modules or remove those exports from `mod.rs`.

Testing priorities:

* Golden-data tests for `D_M/r_d`, `D_H/r_d`, `R`, `l_A`, `fσ8`, and `S8` against known ΛCDM references.
* Synthetic recovery tests for each league model, especially `sigma8`/`mu0` after the plumbing fix.
* Red-team tests where fake `screening: "vainshtein"` with no screening calculation is killed.
* Covariance tests using a nontrivial DESI-like block, not only toy matrices.
* Cross-model fairness tests proving every model in a league sees the same observable set or is labeled coverage-incompatible.

## 7. Concrete falsifiable program

Sprint 1 should produce a corrected league over ΛCDM, wCDM, w0waCDM, and the repaired scale-independent `mu0` model using real covariance and DESI DR2/Pantheon+/Planck-distance/growth/S8 data. The output is not “new physics”; it is a sanity-calibrated baseline.

Sprint 2 should add two derived MG classes: Hu-Sawicki f(R) and nDGP. Each gets a `derive()` method producing `H(a)`, `mu(a,k)`, `Sigma(a,k)`, screening diagnostics, and stability/local-gravity vetoes. Score them against growth+lensing+local constraints. If the engine cannot distinguish f(R) scale dependence from `sigma8`, that is a failed sprint and a useful diagnosis.

Sprint 3 should add EDE as a scalar-field sector and require a Boltzmann/CMB lane before any EDE promotion. The success criterion is not that EDE wins; it is that a free `f_ede` knob dies, while a scalar sector can be scored honestly and falsified by CMB+BAO+SN.

Sprint 4 should run ZYAL proposer/skeptic/adversary loops only after the deterministic sectors above exist. Agents propose action-level variations; the host produces a league table and a killed-region map. The publication-grade artifact is a five-theory league table with covariance, coverage, stability/local-gravity verdicts, and profile/evidence comparisons. Anything less is an internal demo.

