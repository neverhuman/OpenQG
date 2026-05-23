# Unified Theory Agent Brief

This brief is the operating contract for agents proposing stronger unified
theories in OpenQG. It is not a benchmark contract, schema, source pack, or
generated artifact.

## Mandatory Order Of Work

Agents MUST complete the work in this order:

1. Write a complete derivation dossier before adapter implementation.
2. Build a constants and structural-target ledger before benchmark scoring.
3. Map each derivation to fixture families, observable IDs, prediction row
   metadata, and uncertainty propagation.
4. Grade the proposal with the pre-submission scorecard in this brief.
5. Implement an adapter only for rows that have fixture-independent
   derivations.
6. Run the declared proof lanes and universal lanes only after the dossier and
   scorecard are complete.

Theories scoring below 85/100 on the pre-submission scorecard MUST NOT be shown
for review as candidate unified theories. They MAY remain private research
notes or explicitly marked incomplete drafts.

## Derivation First Gate

Before any adapter code, benchmark score, report, or review request, the agent
MUST submit a derivation dossier containing:

- A section-by-section mathematical derivation for every required domain listed
  in this brief.
- A constants ledger that distinguishes metrology anchors from dimensionless
  physical targets and structural integers.
- A fixture extraction map that names the exact source families and observable
  prefixes each rule can emit.
- A row-status plan for every selected fixture row: `predicted`,
  `not_implemented`, or `invalid_domain`.
- A self-grade with unresolved gaps, failure modes, and automatic-failure
  checks.

The derivation dossier MUST be complete enough that an auditor can reconstruct
every emitted number without reading adapter code. Adapter code is an executable
translation of the dossier, not a place to invent missing theory.

FAILS REVIEW IF:

- A `predicted` row appears before its derivation, assumptions, constants,
  provenance, fixture mapping, and uncertainty propagation are documented.
- A theory uses fixture values, validation `source_dataset_id` values, benchmark
  scoring feedback, or reference predictor outputs as calibration input.
- A theory fills unsupported domains with placeholders, empirical surrogates,
  neural regressors, fitted coefficients, copied source rows, or post-hoc
  tuning.
- A theory hides free parameters, re-labels empirical constants as derived
  constants, or omits unresolved gaps from the self-grade.

## Mission

Produce a runnable, source-backed unified theory artifact that can target 100%
coverage across all committed universal phenomena. The artifact MUST be
executable, auditable, fixture-independent, and honest about unsupported
domains.

The target is not empirical curve fitting. The target is a theory whose
mathematical structure derives or reproduces observable values through declared
rules with named provenance. Every numeric `predicted` row MUST come from a
derivation independent of the target fixture value and backed by named theory
sources or admitted local source-pack provenance.

## Automatic Failure Conditions

Any of the following conditions redline the proposal and FAIL REVIEW:

- Leakage from observable fixture values, validation rows, fixture locks,
  benchmark reports, or universal score feedback into a prediction rule.
- Copied fixture values, copied reference-predictor values, or row-wise
  memorization.
- Reuse of the calibrated `sm-gr-lcdm-eft-calibrated` reference predictor as a
  theory engine.
- Benchmark calibration, micro/mini holdout tuning, post-hoc parameter
  adjustment, or selecting rules because they improve the score.
- Neural regressors, empirical surrogates, lookup tables, interpolators, or
  fitted coefficients unless the coefficients are derived by the submitted
  theory without target data.
- Nonzero `calibration_data_used` for a strict theory `predicted` row.
- Missing `metadata.no_calibration_data_used=true` for a strict theory
  `predicted` row.
- Silent numeric placeholders for domains without audited derivations.
- Undeclared free parameters, unstated priors, missing unit conversions, or
  missing uncertainty propagation.

Unsupported rows are acceptable. Dishonest numeric rows are not theory evidence.

## Evaluation Surfaces

OpenQG universal reports separate strict ranking from partial theory coverage:

- Strict `openqg_score` remains the official candidate redline. Partial
  candidates can remain at `0.000` when unsupported rows dominate the official
  scorecard.
- `non_empirical_theory_score` is a separate coverage lane for honest strict
  theory predictions:
  `clamp(100 * coverage - 2 * parameter_count_penalty - leakage_penalty - empirical_penalty, 0, 100)`.
- Unsupported rows reduce only coverage in the theory score lane.
- Rows that reuse the calibrated reference predictor, train on benchmark
  fixtures, touch validation source IDs as calibration data, or copy observable
  values are redlined from this lane.

The calibrated `sm-gr-lcdm-eft-calibrated` reference predictor can report high
holdout coverage. It is a baseline, not a unification score.

## Current Baseline

The current worked strict-theory example is `spectral-fixed-point-unification`.

Its admitted source-backed rules are deliberately narrow:

- LIGO/Virgo/KAGRA GR null-test rows: predict zero deviation from the IR
  Einstein-Hilbert/GR limit.
- MICROSCOPE weak-equivalence-principle rows: predict zero Eotvos deviation
  from the IR GR/WEP limit.
- SME Lorentz/CPT bound rows: predict zero coefficient magnitude beneath the
  published upper bound.

Current scores:

| tier | valid/total | coverage | strict score | theory score |
| --- | ---: | ---: | ---: | ---: |
| micro-v0.2 | 12/64 | 0.1875 | 0.000 | 18.750 |
| mini-v0.2 | 96/512 | 0.1875 | 0.000 | 18.750 |

Domain status:

| domain | baseline status |
| --- | --- |
| `equivalence_principle` | covered by WEP null-limit rows |
| `lorentz_cpt` | covered by SME null-coefficient rows |
| `gravity` | partial: LIGO GR null tests covered; GWOSC catalog rows unsupported |
| `atomic` | unsupported |
| `black_hole` | unsupported |
| `constants` | unsupported |
| `cosmology` | unsupported |
| `galaxy_dynamics` | unsupported |
| `neutrino` | unsupported |
| `particle` | unsupported |
| `solar_system` | unsupported |

The baseline proves the expected behavior: a partial candidate can emit
source-backed rows, stay redlined in strict `openqg_score`, and still receive
limited non-empirical coverage credit.

## Mathematical Derivation Requirements

Every required derivation section MUST include:

- Equations: the governing action, constraints, field equations, spectra,
  transport equations, renormalization relations, or extraction equations
  needed for the target domain.
- Assumptions: symmetry assumptions, boundary conditions, vacuum choice,
  topology, dimensionality, state preparation, approximations, and ignored
  operators.
- Parameter origins: whether each symbol is derived, fixed by structural
  consistency, a metrology anchor, a declared free parameter, or unsupported.
- Dimensional analysis: units, natural-unit conventions, conversion factors,
  dimensionless ratios, and checks that the predicted row unit is correct.
- Limiting cases: Standard Model, GR, LCDM, Newtonian, QED, QCD, weak-field,
  nonrelativistic, high-energy, low-energy, or other applicable limits.
- Source references: named literature or local source-pack provenance used to
  justify the rule, without using target row values as calibration data.
- Fixture mapping: source family, fixture prefix, domain, observable shape,
  selected rule ID, and emitted status for every covered or refused row.
- Uncertainty propagation: analytic, interval, covariance, prior, truncation,
  numerical, or declared zero uncertainty with justification.
- Executable rule metadata: `metadata.rule_id`, `metadata.rule_version`,
  `metadata.theory_basis`, `metadata.source_references`,
  `metadata.no_calibration_data_used`, `metadata.prediction_kind`,
  `metadata.calibration_status`, and `metadata.observable_shape_match`.

If any item is missing for a row, that row MUST be `not_implemented` or
`invalid_domain`.

## Required Dossier Sections

The derivation dossier MUST grade each section below as `complete`, `partial`,
or `missing`. A `predicted` row is allowed only when its section is complete for
that observable family.

| section | required derivation work |
| --- | --- |
| Foundations | State the ontology, degrees of freedom, action or generating principle, Hilbert/path-integral/statistical structure, gauge or diffeomorphism symmetries, constraints, quantization rule, and consistency conditions. |
| Low-energy limit | Derive the Standard Model, GR, LCDM, QED, QCD, weak interaction, Newtonian, and atomic limits required by fixture domains, or mark the missing limits. |
| Constants | Derive or reproduce the constants ledger targets and identify which quantities are metrology anchors, structural integers, free parameters, or failed derivations. |
| Particles | Derive particle spectra, masses, widths, lifetimes, charges, spins, and any PDG/HEPData observable extraction maps. |
| Interactions | Derive gauge interactions, coupling normalization, running couplings, electroweak symmetry breaking, scattering amplitudes, decay rules, and precision electroweak observables. |
| Flavor | Derive Yukawa structure, quark and lepton mass hierarchies, CKM, PMNS, CP phases, neutrino masses, oscillation probabilities, and NuFIT mappings. |
| QCD and nuclear physics | Derive confinement scale, hadronization assumptions, proton/neutron properties, nuclear binding approximations, strong CP treatment, and QCD uncertainty limits. |
| Cosmology | Derive background expansion, perturbations, primordial parameters, baryon asymmetry, dark matter, dark energy or vacuum energy, distance ladders, BAO, CMB, and supernova mappings. |
| Gravity | Derive field equations, equivalence principle behavior, gravitational waves, post-Newtonian limits, compact-object dynamics, and GWOSC/LIGO null-test extraction. |
| Black holes | Derive horizon structure, shadow/angular-scale predictions, spin and mass dependence, accretion or emission assumptions, and EHT observable extraction. |
| Atomic physics | Derive bound-state spectra, QED/QCD/nuclear corrections, transition energies, isotope/hyperfine assumptions, and NIST ASD extraction. |
| Solar system | Derive ephemeris observables, orbital elements, relativistic corrections, coordinate conventions, and JPL Horizons extraction. |
| Galaxy dynamics | Derive rotation curves, baryonic distribution handling, dark matter or modified-gravity profile rules, scaling relations, and SPARC extraction. |
| Lorentz/CPT | Derive exact symmetry, controlled breaking, SME coefficient maps, null predictions, and published-bound comparisons. |
| Observable extraction | Derive the row-by-row transformation from theory quantities to OpenQG prediction JSONL, including units, uncertainty, status, and metadata. |

Partial derivations MUST remain explicit research gaps. They MUST NOT be
converted into numeric predictions to improve coverage.

## Universal Constants And Structural Targets

The theory MUST maintain a constants ledger with one row per target. The ledger
MUST state: symbol, physical role, dimension, observed or structural target,
derivation status, origin in the theory, uncertainty propagation, fixture/source
mapping, and row status.

Required ledger coverage:

| target | required accounting |
| --- | --- |
| `c` | Identify as an SI-defining metrology anchor and as the invariant causal speed in the theory. Explain whether the invariant speed is derived structurally or assumed. |
| `hbar` | Identify as the quantum action scale derived from the SI-defining Planck constant `h`. Explain whether quantization normalizes it, derives dimensionless ratios independent of it, or treats it as a unit convention. |
| `G` and `M_Pl` | State that `G` is not an SI-defining constant. Derive gravitational coupling normalization or the Planck mass relation, and state whether `G` is an effective low-energy coupling, an assumed input, or a free parameter. |
| `k_B` | Identify as an SI-defining metrology anchor and thermodynamic conversion factor. Derive dimensionless thermodynamic predictions separately from unit choice. |
| charge quantization | Derive electric charge unit, representation assignments, anomaly consistency, and allowed charges. |
| `alpha` | Derive or reproduce the fine-structure constant as a dimensionless target; listing CODATA is insufficient. |
| gauge couplings | Derive normalization and running of `g1`, `g2`, `g3`, unification or non-unification behavior, and scale conventions. |
| Yukawas | Derive quark and lepton Yukawa matrices or mark them as unresolved free structure. |
| Higgs/electroweak scale | Derive or account for `v`, Higgs mass, electroweak symmetry breaking, hierarchy assumptions, and precision electroweak mappings. |
| CKM | Derive quark mixing angles and CP phase, including uncertainty propagation and PDG mappings. |
| PMNS | Derive lepton mixing, neutrino mass ordering, CP phase assumptions, and NuFIT mappings. |
| `Lambda_QCD` | Derive dimensional transmutation, scheme/scale convention, hadron mass connection, and QCD uncertainty. |
| strong CP | Derive theta handling, axion or symmetry mechanism if present, and predicted EDM implications if mapped. |
| proton/electron ratio | Derive `m_p / m_e` or explain which mass derivations are missing. |
| vacuum energy | Derive vacuum energy or cosmological constant scale; separate absolute energy density from unit conventions. |
| baryon asymmetry | Derive baryogenesis/leptogenesis mechanism, CP violation, out-of-equilibrium conditions, and observed ratio mapping. |
| dark matter | Derive candidate, abundance, interaction scale, structure effects, or mark the target unsupported. |
| primordial parameters | Derive scalar amplitude, spectral index, tensor ratio, non-Gaussianity assumptions, and Planck mappings. |
| dimensions | Derive spacetime dimension count, compact dimensions if any, and low-energy dimensional reduction. |
| gauge group | Derive `SU(3) x SU(2) x U(1)` or alternative low-energy group and map to observed interactions. |
| colors | Derive three QCD colors or mark color count as assumed. |
| generations | Derive three fermion generations or mark generation count as assumed. |
| anomaly cancellation | Prove gauge, mixed, and gravitational anomaly cancellation for chiral matter content. |
| chiral weak structure | Derive left-handed weak doublets, right-handed singlets, parity violation, and weak current structure. |

### Constants Policy

SI-defining constants MAY be discussed, but they do not by themselves earn
unification credit. Grading emphasizes:

- Dimensionless ratios and couplings: `alpha`, mass ratios, mixing angles,
  CP phases, density fractions, spectral indices, and amplitude ratios.
- Couplings and masses relative to `M_Pl`, `v`, or another justified theory
  scale.
- Structural integers: dimensions, gauge group, colors, generations,
  representation assignments, anomaly cancellation, and chiral structure.
- Cosmological state parameters: baryon asymmetry, dark matter abundance,
  vacuum energy, primordial perturbation parameters, and background expansion.

A theory MUST NOT present a table of measured constants as a derivation. It MUST
explain which constants are derived, which are assumed as unit conventions,
which are free parameters, and which remain unsupported.

## Data Surfaces

Committed universal fixtures live at:

- `data/fixtures/micro-v0.2/observables.jsonl`, 64 rows.
- `data/fixtures/mini-v0.2/observables.jsonl`, 512 rows.

Each candidate MUST emit exactly one prediction row for each observable in the
selected fixture. Use `predicted` only for fixture-independent derivations. Use
`not_implemented` or `invalid_domain` for rows that are not honestly derived.

Representative micro fixture samples:

| source family | domain | sample observable | value +/- uncertainty | unit |
| --- | --- | --- | ---: | --- |
| NIST ASD | `atomic` | `asd_001`, normalized ASD energy term bin 1 | 1.0 +/- 0.0004 | eV |
| NIST CODATA | `constants` | `codata_001`, normalized CODATA constant bin 1 | 0.00729735256 +/- 1.1E-10 | dimensionless |
| PDG | `particle` | `pdg_001`, normalized PDG particle property bin 1 | 0.00051099895 +/- 1.5E-10 | GeV |
| HEPData | `particle` | `hepdata_001`, normalized electroweak term bin 1 | 80.377 +/- 0.012 | GeV |
| NuFIT | `neutrino` | `nufit_001`, normalized oscillation summary bin 1 | 0.307 +/- 0.012 | dimensionless |
| DESI | `cosmology` | `desi_001`, normalized BAO distance term bin 1 | 147.1 +/- 1.8 | Mpc |
| BOSS/eBOSS | `cosmology` | `boss_001`, normalized BAO term bin 1 | 150.0 +/- 2.5 | Mpc |
| Planck | `cosmology` | `planck_001`, normalized legacy parameter bin 1 | 0.315 +/- 0.007 | dimensionless |
| Pantheon+ | `cosmology` | `pantheon_001`, normalized distance modulus bin 1 | 34.2 +/- 0.12 | mag |
| GWOSC | `gravity` | `gwosc_001`, normalized compact-binary catalog bin 1 | 8.2 +/- 1.1 | solar_mass |
| LIGO GR null tests | `gravity` | `ligogr_001`, normalized tests-of-GR deviation bin 1 | 0.0 +/- 0.08 | dimensionless |
| EHT | `black_hole` | `eht_001`, normalized angular scale term bin 1 | 42.0 +/- 3.0 | microarcsecond |
| MICROSCOPE | `equivalence_principle` | `microscope_001`, normalized Eotvos term bin 1 | 0.0 +/- 9E-15 | dimensionless |
| SME bounds | `lorentz_cpt` | `sme_001`, normalized SME coefficient bound bin 1 | 1E-20 +/- 1E-20 | GeV |
| JPL Horizons | `solar_system` | `horizons_001`, normalized orbital term bin 1 | 0.387 +/- 0.000001 | au |
| SPARC | `galaxy_dynamics` | `sparc_001`, normalized velocity term bin 1 | 42.0 +/- 3.0 | km s^-1 |

Mini uses the same source families with 32 rows per source family.

## Source Catalog

Use the local source packs and registry manifests as the provenance catalog:

- Source packs: `data/source-packs/micro-v0.2.yml`,
  `data/source-packs/mini-v0.2.yml`, and opt-in
  `data/source-packs/full-v0.2.yml`.
- Fixture locks: `data/fixtures/micro-v0.2/source-fixture-lock.json` and
  `data/fixtures/mini-v0.2/source-fixture-lock.json`.
- Registry manifests: `data/registry/constants/nist-codata.yml`,
  `data/registry/particle/pdg-api.yml`, `data/registry/literature/hepdata.yml`,
  `data/registry/cosmology/planck-legacy.yml`,
  `data/registry/gravity/gwosc.yml`,
  `data/registry/dark-energy/desi-dr1.yml`, and
  `data/registry/dark-matter/sparc.yml`.

Compact source-pack IDs in the committed universal fixtures:

| source ID | fixture prefix | domain coverage |
| --- | --- | --- |
| `nist-codata-2022` | `codata_*` | constants |
| `pdg-2025-api` | `pdg_*` | particle masses and widths |
| `hepdata-electroweak` | `hepdata_*` | particle and electroweak literature tables |
| `nufit-6.0` | `nufit_*` | neutrino oscillation summaries |
| `desi-y1-bao` | `desi_*` | cosmology BAO distances |
| `boss-eboss-bao` | `boss_*` | cosmology BAO/RSD products |
| `planck-legacy` | `planck_*` | CMB summary parameters |
| `pantheon-plus` | `pantheon_*` | supernova distance moduli |
| `gwosc-gwtc` | `gwosc_*` | compact-binary catalog quantities |
| `ligo-gr-null-tests` | `ligogr_*` | tests-of-GR null deviations |
| `eht-public-products` | `eht_*` | black-hole angular scales |
| `microscope-final-wep` | `microscope_*` | weak-equivalence-principle Eotvos rows |
| `sme-lorentz-cpt` | `sme_*` | Lorentz/CPT SME bounds |
| `nist-asd-v5.12` | `asd_*` | atomic spectra |
| `jpl-horizons` | `horizons_*` | solar-system ephemeris quantities |
| `sparc-rotcurves` | `sparc_*` | galaxy rotation curves |

Do not invent extra provenance. If a derivation needs a source not represented
locally, add it through the normal source-pack and fixture process before using
it as benchmark evidence.

## Artifact Shape

Create or update a theory artifact with these surfaces only after the
Derivation First Gate is satisfied:

- `theories/<theory-id>/manifest.yml` with a stable ID, name, status,
  `benchmark_suite`, citations, observables, named parameters, and adapter
  command.
- `examples/theory-adapters/<theory-id>/adapter.py`, compatible with the
  existing adapter pattern.
- One prediction JSONL row per observable for the selected fixture.

For every `predicted` row, include at least:

- `observable_id`, `status`, `value`, `uncertainty`, `unit`, and `theory_id`.
- `distribution`, `prediction_uncertainty`, and `domain_validity`.
- `free_parameters_used` and `calibration_data_used`.
- `metadata.rule_id` and `metadata.rule_version`.
- `metadata.theory_basis`.
- `metadata.source_references`.
- `metadata.no_calibration_data_used`.
- `metadata.prediction_kind`.
- `metadata.calibration_status`.
- Enough `metadata.observable_shape_match` detail to prove which source-backed
  rule selected the row.

For unsupported rows, emit `not_implemented` when the domain is plausibly in
scope but the derivation is missing. Emit `invalid_domain` when the observable
is outside the theory's declared domain. Include `domain_validity` and metadata
that makes the refusal auditable. Unsupported rows MUST NOT include numeric
prediction fields that imply a derived value.

## Observable Extraction Rules

The adapter MUST be a direct executable form of the dossier. For each observable
family, document and implement:

- `rule_id` and `rule_version`.
- Fixture prefix and source ID.
- Observable selection predicate.
- Input theory quantities.
- Unit conversion and dimensional check.
- Value extraction equation.
- Prediction uncertainty equation.
- `distribution` selection.
- `domain_validity` statement.
- Free parameters used, if any.
- Calibration data declaration, which MUST be empty for strict theory rows.
- Unsupported-row refusal reason.

The adapter MUST emit exactly one row per input observable. It MUST preserve
fixture-independent behavior if observable row order changes or if unrelated
fixture rows are added.

## Pre-Submission Scorecard

Before requesting review, agents MUST grade themselves out of 100 and include
evidence for every item. Scores below 85 MUST NOT be submitted as candidate
unified theories.

| category | points | grading requirement |
| --- | ---: | --- |
| Mathematical derivation completeness | 25 | Award full credit only if every required dossier section has equations, assumptions, parameter origins, dimensional analysis, limiting cases, sources, fixture mapping, uncertainty propagation, and executable rule metadata. |
| Constants and structural target accounting | 20 | Award full credit only if the constants ledger covers all required targets and distinguishes metrology anchors, dimensionless targets, structural integers, free parameters, and unsupported quantities. |
| Observable extraction maps | 15 | Award full credit only if every `predicted` row has a row-level extraction map and every unsupported row has an auditable refusal. |
| Source-backed non-empirical provenance | 15 | Award full credit only if all derivations use named theory sources or admitted local source-pack provenance without fixture leakage or benchmark calibration. |
| Executable adapter readiness | 10 | Award full credit only if the adapter plan maps one-to-one to documented rules and can emit required JSONL fields for all fixture rows. |
| Falsifiability, unsupported-row honesty, and failure modes | 10 | Award full credit only if the theory states falsifiable predictions, unsupported domains, assumptions that could fail, and automatic-failure checks. |
| Clarity and auditability | 5 | Award full credit only if an auditor can trace each number from equation to prediction row without hidden steps. |

The submission MUST include a section-by-section grade:

| section | status | points kept/lost | unresolved gaps |
| --- | --- | ---: | --- |
| Foundations |  |  |  |
| Low-energy limit |  |  |  |
| Constants |  |  |  |
| Particles |  |  |  |
| Interactions |  |  |  |
| Flavor |  |  |  |
| QCD and nuclear physics |  |  |  |
| Cosmology |  |  |  |
| Gravity |  |  |  |
| Black holes |  |  |  |
| Atomic physics |  |  |  |
| Solar system |  |  |  |
| Galaxy dynamics |  |  |  |
| Lorentz/CPT |  |  |  |
| Observable extraction |  |  |  |

## Pre-Submission Checklist

Before review, the agent MUST confirm:

- Derivation dossier is complete before adapter code was written.
- Constants ledger covers all required constants and structural targets.
- SI-defining constants are treated as metrology anchors unless the theory
  derives their structural role independently of units.
- Dimensionless ratios, couplings, mass ratios, mixing parameters, structural
  integers, and cosmological state parameters are explicitly derived,
  reproduced, or marked unsupported.
- Every `predicted` row has a derivation, source references, uncertainty
  propagation, fixture mapping, and executable metadata.
- Every unsupported row is `not_implemented` or `invalid_domain`.
- `calibration_data_used` is empty for strict theory `predicted` rows.
- `metadata.no_calibration_data_used` is true for strict theory `predicted`
  rows.
- No neural regressors, empirical surrogates, reference predictor outputs,
  copied fixture values, benchmark calibration, or post-hoc fitting are used.
- The self-grade is at least 85/100, with unresolved gaps included.
- Generated reports are regenerated only through declared commands.

## Required Commands

For this documentation workflow, run:

```bash
rtk just fast
```

For universal scoring work, use the declared lanes that regenerate auditor
outputs:

```bash
rtk just universal-micro
rtk just universal-mini
```

Do not hand-edit generated reports under `reports/universal-theory/` or release
evidence under `reports/releases/draft/`.
