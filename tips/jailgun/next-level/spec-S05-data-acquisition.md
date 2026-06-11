# Spec S05 — Data acquisition and sealed verdict discipline for OpenQG/ZYAL V8

Review batch tab: 1. Lens: data critic / survey scientist. This spec is based on the attached source archive as extracted, with special attention to `data/fixtures/cosmology/`, `data/registry/`, `crates/openqg-core/src/scoring/covariance.rs`, `crates/openqg-core/src/cosmology/observables.rs`, `paper/main.tex`, and `docs/zyal-next-level-design.md`.

## Ranked backlog

1. **Build a sealed data registry and value firewall.** What: add a Rust-owned registry tier for `open_fit`, `sealed_kill`, and `future_sealed` datasets; proposer packets expose only observable names, redshift ranges, covariance rank, and mechanism semantics, never held-out numeric values. Why: all current and most proposed datasets are public, and an LLM may know headline values; honest generalization must therefore be defined operationally, not psychologically. Effort: **M**. Verification: a hostile test creates a sealed dataset with known canary values, runs the proposer/export pipeline, and asserts that no canary value, no exact uncertainty, and no hash preimage appears in prompts, ledgers, dashboards, or candidate certificates before adjudication.

2. **Install the growth verdict pack before making any claim about the V7 suppressed-growth / dark-scattering class.** What: ingest covariance-complete full-shape RSD and at least two independent lensing likelihoods, with ACT/Planck CMB lensing as a high-redshift cross-check. Why: the champion described in `paper/main.tex:561-573` lives or dies on growth. The current archive gives it five diagonal `fσ8` points and one scalar `s8`, which can only detect “growth seems low,” not identify a mechanism. Effort: **L**. Verification: the engine reproduces published reference likelihoods for SDSS/eBOSS DR16 full-shape, DESI DR1/DR2 full-shape when officially supported, DES Y6 or KiDS-Legacy/HSC Y3 shear, and ACT DR6 lensing, then reports a profile-fitted ΔlnZ/ΔAIC against ΛCDM, w0waCDM, and the suppressed-growth class on the same data.

3. **Upgrade DESI BAO from DR1 to DR2 and make DR1 a regression fixture.** What: ingest DESI DR2 BAO measurements and covariance from the DESI/Cobaya release, while preserving DR1 as a historical regression. Why: this is the cheapest geometry improvement and stabilizes the `w0 > -1` part of dark scattering, because the drag term in the paper is proportional to `(1+w) Ω_de`. Effort: **S**. Verification: a `cargo test` fixture loads DESI DR2 vectors and covariance, passes positive-definite checks, and reproduces DESI’s published BAO-only χ² at the fiducial point to a documented tolerance.

4. **Add Pantheon+ full stat+sys covariance as the primary SNe geometry dataset, with Union3 and DES-SN5YR as non-independent cross-check lanes.** What: ingest one SNe release as the scored primary and register the others as systematic challenge suites, not additive evidence. Why: SNe decide whether the dark-scattering background path is even geometrically allowed, but naïvely stacking Pantheon+, Union3, and DES-SN5YR double-counts supernovae, calibrators, and analysis choices. Effort: **M** for Pantheon+, **L** for DES-SN5YR simulations. Verification: the engine reproduces a published Pantheon+ ΛCDM/w0wa likelihood slice using the full covariance, and a double-count auditor refuses to score Pantheon+SH0ES plus scalar SH0ES `h0` as independent evidence.

5. **Create a covariance/likelihood group abstraction instead of forcing all new probes into `name@z`.** What: keep the canonical scalar/redshift grammar in `observables.rs` for BAO, SNe distance moduli, and compressed RSD, but add `likelihood_group` records for shear 2pt/SACC, CMB spectra, power-spectrum multipoles, lensing bandpowers, and GW posterior samples. Why: `observables.rs` deliberately supports only scalar names and five redshift families; trying to represent CMB `C_ell`, shear `ξ±(θ,z_i,z_j)`, or full-shape `P_ℓ(k,z)` as fake scalar observables will recreate the V6 instrument-error problem. Effort: **M**. Verification: fixture validation rejects `cl_tt@220` and `xi_plus@...` as scalar rows, but accepts them inside a registered likelihood group with its own likelihood adapter and provenance lock.

6. **Adopt a full-likelihood graduation rule for compressed CMB and lensing sectors.** What: if a candidate’s novelty witness or score improvement is carried by CMB, lensing, scale-dependent growth, early-universe physics, gravitational slip, or non-linear clustering, compressed vectors are smoke tests only; promotion requires a Boltzmann/nonlinear likelihood path. Why: `paper/main.tex:654-664` already admits fitting-formula backgrounds and compressed likelihoods; the V6 `ℓ_A` episode proves the compression pipeline itself can become the exploited instrument. Effort: **L**; see S08 for the backend, but the data gate must enforce the rule. Verification: a candidate that gains data-fit credit from `cmb_lA` alone receives no promotion credit until a full Planck/ACT likelihood residual audit passes.

7. **Install a double-counting and shared-calibration auditor.** What: every dataset manifest declares `volume_tags`, `object_tags`, `calibration_tags`, `analysis_tags`, and `derived_from`; the scorer refuses illegal combinations unless a joint covariance exists. Why: the current 23 observables mix DESI BAO, BOSS/eBOSS RSD, SH0ES, DES Y3 `S8`, BBN, and Planck distance priors without a machine-readable correlation audit. Effort: **M**. Verification: tests reject DESI DR1+DR2 same-tracer stacking, Pantheon+SH0ES+SH0ES scalar stacking, multiple overlapping SNe compilations as independent data, and BOSS/eBOSS full-shape plus compressed `fσ8` duplicates.

8. **Hash-lock every raw source, transform, normalized fixture, and covariance block.** What: extend `data/registry/*.yml` and `contracts/specs/dataset.yml` with raw archive hashes, normalized JSONL hashes, transform code revisions, and scorer receipt hashes. Why: the archive already has `CovarianceRegistry` health checks, but dataset provenance is not yet strong enough for hostile replication. Effort: **S/M**. Verification: modifying one byte in a raw data file, one covariance entry, or one transform script makes the run contract fail closed before scoring.

9. **Pre-register future kill datasets now.** What: add empty-value, hash-awaiting manifests for DESI DR3, Euclid DR1, Rubin/LSST DR1 shear+SNe, Roman SNe, LVK O5 sirens, and Simons Observatory/CMB-S4 public likelihoods. Why: future releases are the only cosmology data an LLM ensemble cannot have memorized at proposal time. Effort: **S**. Verification: a future-release entry can be referenced by mechanisms and adversarial tests today, but cannot contribute likelihood until a post-publication maintainer finalizes source hashes and opens the seal.

## Repo facts this spec responds to

The current evidence base is intentionally small. `data/fixtures/cosmology/tier1-multisector.jsonl` has 23 rows: 12 DESI DR1 BAO observables, 3 compressed Planck distance-prior observables, 5 diagonal `fσ8` rows, scalar `h0`, scalar `s8`, and scalar `bbn_yp`. The paper states the same table: H0 ladder 1 diagonal, S8 lensing 1 diagonal, RSD 5 diagonal, BAO 12 with 5 tracer blocks, CMB 3 with a 3x3 block, and BBN 1 diagonal (`paper/main.tex:341-359`).

The good news is that the code has the right spine. `scoring/covariance.rs` scores blocks as `-1/2 r^T C^-1 r`, marginalizes missing block members by submatrix, and fail-closes on non-positive-definite matrices. `data/fixtures/cosmology/covariance/README.md` already requires citations, ordered observable IDs, hashes, positive-definite checks, and condition numbers. `cosmology/observables.rs` already canonicalizes `fsigma8_z051` to `fsigma8@0.51`, folds `*_over_rs` to `*_over_rd`, and treats unknown grammar as unverifiable rather than crashing. Preserve all of that.

The bad news is also clear. The admitted growth evidence is not adequate for the standing champion. `paper/main.tex:527-539` says the V7 class gets zero data-fit credit because its raw improvement does not clear the evidence bar. `paper/main.tex:561-573` honestly identifies the mechanism as late-time suppression through Planck-style `μ(a)` and dark scattering drag, and says more growth data is the falsification axis. `docs/zyal-next-level-design.md:106-114` already warns that coverage was narrow and the discriminating data were absent. This spec goes beyond that warning by naming exactly which datasets must be acquired, how to represent them, and what verdict each can change.

## Acquisition table

Rows marked “score” are candidates for production scoring after validation. Rows marked “challenge” should be used for regression, systematic comparison, or sealed adjudication, not stacked blindly. Approximate sizes are intentionally conservative; the registry must store exact byte counts and SHA-256 hashes at ingestion time.

| Dataset | Exact source | Format | Covariance / likelihood | License and usage terms | Approx. size | Effort | Verdict changed for suppressed-growth / dark-scattering |
|---|---|---:|---|---|---:|---:|---|
| **Pantheon+ SNe, primary score** | GitHub `PantheonPlusSH0ES/DataRelease`, https://github.com/PantheonPlusSH0ES/DataRelease; Brout et al. arXiv:2202.04077; Scolnic et al. arXiv:2112.03863 | ASCII tables, covariance files, CosmoSIS inputs/chains | **Full stat+sys covariance** for distance-modulus likelihood; SH0ES-calibrated and uncalibrated modes must be separated | Public scientific release; cite Pantheon+ papers; do not redistribute raw archive in this repo unless license explicitly permits | ~10-100 MB | M | Tests the background half of dark scattering. If SNe+DESI DR2 do not support `w0 > -1` or require phantom behavior, the `(1+w)` drag mechanism loses its natural amplitude. Does not by itself test growth. |
| **Union3 SNe, challenge** | GitHub `rubind/union3_release`, https://github.com/rubind/union3_release; arXiv:2311.12098 | FITS compressed distance vector/inverse covariance; optional light-curve fit archive and Stan code | **Compressed full inverse covariance** over distance moduli / binned distances | Public repository; cite Union3; treat as cite-only unless repository license confirms redistribution | ~10-100 MB | M | Cross-checks whether any Pantheon+ verdict is compilation-specific. Never stack independently with Pantheon+ without overlap covariance. |
| **DES-SN5YR, challenge / later score** | Zenodo record https://zenodo.org/records/12720778; GitHub `des-science/DES-SN5YR`, https://github.com/des-science/DES-SN5YR; DES key cosmology arXiv:2401.02929; data-release arXiv:2406.05046 | SNANA-style files, simulations, binned results, chains, pipeline inputs | Published covariance products and simulation-derived systematics; full likelihood requires pipeline context | Zenodo/GitHub public release; cite DES-SN5YR data and cosmology papers; raw simulations should stay out of git | 0.1-10 GB depending products | L | Independent survey-systematics check on evolving-DE geometry. High value after Pantheon+, but dangerous as additive evidence because low-z anchors and some objects/calibrations overlap. |
| **DESI DR2 BAO, score** | DESI DR2 papers page https://data.desi.lbl.gov/doc/papers/dr2/; BAO paper arXiv:2503.14738, DOI 10.1103/tr6y-kpc6; Cobaya data https://github.com/CobayaSampler/bao_data | `.dat` vectors and covariance/inverse-covariance tables usable through Cobaya | **Full compressed BAO covariance** for published BAO vector | DESI public scientific data; cite collaboration and data release; store only normalized fixtures unless redistribution rights are explicit | <10 MB | S | Sharpens geometry and the allowed `w0` region. It can demote dark-scattering backgrounds that need geometry already excluded, but it cannot kill the growth mechanism alone. |
| **SDSS/BOSS/eBOSS DR16 full-shape BAO+RSD, score** | Cobaya BAO/RSD data https://github.com/CobayaSampler/bao_data; eBOSS DR16 consensus papers, e.g. arXiv:2007.08991 and DR16 likelihood files | Compressed `D_M/r_d`, `D_H/r_d`, `fσ8` vectors plus covariance; some full-shape grids | **Full compressed covariance** over geometry+growth per tracer | Public likelihood data in Cobaya ecosystem; cite SDSS/eBOSS DR16; no raw redistribution unless license checked | <10 MB for compressed; more for grids | S/M | Immediate growth verdict. Replaces five diagonal `fσ8` rows with correlated AP+growth constraints. Tests whether low `fσ8` survives covariance and refitted ΛCDM/w0wa baselines. |
| **DESI DR1 full-shape clustering, score after official likelihood lock** | DESI DR1 full-shape BAO clustering VAC https://data.desi.lbl.gov/doc/releases/dr1/vac/full-shape-bao-clustering/; DESI full-shape papers arXiv:2411.12021 and JCAP DESI 2024 VII | Power-spectrum/correlation-function multipoles, window matrices, covariance matrices; `desilike` likelihoods https://github.com/cosmodesi/desilike | **Full covariance / likelihood**, not just scalar `fσ8` | DESI data-access terms; cite DESI DR1 full-shape papers; normalized fixtures only | 0.1-5 GB if multipoles/windows/mocks included | L | Best near-term direct test of scale-dependent growth. A `μ0` scalar suppression that fits old fσ8 may fail full-shape AP, shape, bias, and RSD nuisance marginalization. |
| **DESI DR2 full-shape / RSD, future score** | DESI DR2 publication/data page as released; `desilike`/DESI likelihoods when official | Same as DR1, larger volume | **Full covariance / likelihood** required | DESI public release terms; pre-register now, score only after official files and hashes | likely GB scale | L | Required for strong exclusion if DR2 full-shape is public and validated. If it shows no late-time suppression after nuisance marginalization, current class should lose promotion eligibility. Confidence: medium on exact public likelihood timing. |
| **KiDS-Legacy weak lensing, score** | KiDS site https://kids.strw.leidenuniv.nl/; KiDS science data https://kids.strw.leidenuniv.nl/sciencedata.php; final KiDS-Legacy arXiv:2503.19442 | COSEBIs/2pt data vectors, covariance, redshift distributions, chains, configs | **Full covariance / likelihood**; scalar `S8` only smoke | Public data products; cite KiDS collaboration; observe site terms | tens to hundreds MB | M/L | Critical because final KiDS-Legacy appears more Planck-consistent than older low-S8 summaries. If the low-growth signal evaporates here, the champion’s motivating residual weakens. |
| **DES Y6 3x2pt weak lensing+clustering, score when data products available** | DES Y6 cosmology papers page https://www.darkenergysurvey.org/des-y6-cosmology-results-papers/; arXiv:2601.14559 | Likely SACC/twopoint vectors, covariances, n(z), masks, chains | **Full 3x2pt covariance / likelihood** | DES public release and collaboration citation terms; exact license must be captured in manifest | 0.1-5 GB | L | The strongest low-redshift growth/lensing adjudicator. If the published Y6 `S8`-region is only mildly low, scalar DES Y3 `s8` should not drive the champion. Confidence: high for paper, medium for exact public likelihood packaging as of 2026-06. |
| **HSC Y3 cosmic shear, score** | HSC weak-lensing Y3 page https://hsc-release.mtk.nao.ac.jp/doc/index.php/wly3/; CosmoSIS SACC likelihood docs; Phys. Rev. D 108, 123519, DOI 10.1103/PhysRevD.108.123519 | SACC data vector, covariance, redshift distributions, likelihood config | **Full covariance / likelihood** | Public release; cite HSC collaboration; respect site terms | tens to hundreds MB | M | Independent lensing systematics. If HSC, KiDS, and DES disagree, the right verdict is “systematics dominated,” not “new gravity found.” |
| **ACT DR6 lensing, score** | ACT lensing likelihood https://github.com/ACTCollaboration/act_dr6_lenslike; ACT DR6 data products https://act.princeton.edu/act-dr6-data-products; LAMBDA maps; arXiv:2304.05203 and 2304.05202 | Bandpowers, covariance, lensing likelihood; optional FITS maps | **Full lensing bandpower covariance / likelihood** | Public GitHub/NASA LAMBDA release; cite ACT papers; maps may be large external dependencies | likelihood <100 MB; maps GB | M/L | High-redshift growth cross-check. ACT lensing has tended to prefer high clustering amplitude; if validated, it can directly oppose late-time suppression tuned to low RSD/S8. |
| **ACT/unWISE or ACT×DESI lensing tomography, challenge then score** | ACTCollaboration `unwisexlens_lklh` GitHub and related papers | Cross-correlation bandpowers, covariances, redshift distributions, transfer functions | **Full covariance / likelihood** | BSD-2-Clause for at least the public code repository; cite papers | hundreds MB | L | Measures growth by redshift and galaxy sample. Decisive for whether suppression is late-time and smooth or an artifact of scalar `S8`. |
| **Planck PR4 / NPIPE CamSpec-lite or plik-lite equivalent, score only through full CMB path** | Planck Legacy Archive https://www.cosmos.esa.int/web/planck/pla; NPIPE products https://portal.nersc.gov/project/cmb/planck2020/; Cobaya Planck likelihood docs; CamSpec NPIPE-lite arXiv:2510.09430; Planck PR4 spectra arXiv:2205.10869 | Likelihood code, spectra, covariance, nuisance-marginalized compressed products | Full likelihood preferred; compressed vectors only smoke | ESA/NERSC/Cobaya public scientific software and data terms; cite exact likelihood | GB for full; MB for lite | L | Required to retire the `cmb_lA` fitting-formula surface. Suppressed growth affects CMB lensing/ISW and cannot be promoted from distance priors alone. Confidence: medium on the newest “lite” release packaging. |
| **BBN 2024 / PRyMordial-grade D/H + Yp likelihood, score** | PRyMordial arXiv:2307.07061; 2024 BBN baryon update arXiv:2401.15054; PDG BBN review 2025 | Code likelihood or small tables of D/H, Yp, nuclear-rate covariance/nuisance choices | Compressed likelihood with nuclear-rate systematics; not just scalar `Yp` | Public code/papers; cite PRyMordial and abundance measurements; exact code license in manifest | <100 MB | M | Mostly protects `r_d`, `N_eff`, and early-universe degrees of freedom. It will not kill late-time suppression, but it prevents candidates from buying DESI fit via unpriced sound-horizon/BBN drift. |
| **TDCOSMO time-delay lenses, challenge** | TDCOSMO public GitHub https://github.com/TDCOSMO; 2020 hierarchy repo; 2025 public repo; Cobaya adapter https://github.com/nataliehogg/tdcosmo_ext; TDCOSMO IV papers | Pickled processed likelihoods, notebooks, hierarchical lens samples | Hierarchical likelihood; covariance is not a simple dense block | Public analysis repositories; cite TDCOSMO/H0LiCOW; no raw redistribution without license | 10 MB-GB | L | Independent late-time distance/H0 check. Do not score until lens mass-profile, external convergence, and selection systematics are modeled; otherwise it is an H0 prior with hidden nuisance. |
| **GW standard sirens, challenge now / future sealed score** | GWOSC https://www.gw-openscience.org/; GWTC-3 PhysRevX 13, 041039; GWTC-4 Zenodo 17014085; GWTC-5 arXiv:2605.27225 and Zenodo 20276130 | HDF5 posterior samples, skymaps, selection functions, catalog tables | Posterior-sample likelihood, not Gaussian covariance | Open public LVK/GWOSC/Zenodo data; cite catalog and release DOI | many GB raw; 30-500 MB processed | M/L | Weak current H0/growth verdict, but important for modified GW propagation and future non-memorized holdouts. It should be sealed for O5/O6 standard-siren releases. |
| **Future Euclid DR1 / Rubin DR1 / DESI DR3 / Roman SNe, future sealed** | Euclid DR1 planned 2026-10-21 per ESA/Caltech timeline; Rubin DR1 depends on LSST survey start and first-year processing; DESI DR3 expected late 2026/early 2027; Roman public SNe TBD | To be determined: shear, clustering, BAO/RSD, SNe, maps | Must be full covariance / likelihood by sector | Register sources and usage once public; values absent until release | TB- to PB-scale raw; compact likelihoods smaller | L | These are the real generalization tests. Register names and schemas now; fill values only after publication by non-proposer data maintainers. |

## Priority and the kill question

The top three by verdict-sharpening per unit effort for the current champion are:

1. **SDSS/eBOSS DR16 full-shape BAO+RSD covariance pack.** This is the fastest way to replace the archive’s five diagonal `fσ8` points. It directly tests the residual the champion is chasing, and it comes with compressed correlated `D_M`, `D_H`, and `fσ8` vectors that the existing block-covariance machinery can almost represent today.

2. **DESI DR2 BAO.** This is geometry, not growth, but it is cheap and decisive for the dark-scattering background coupling. If DESI DR2 plus SNe leave no allowed `w0 > -1` region, a drag proportional to `(1+w)` has no honest amplitude source.

3. **Pantheon+ full covariance.** It is not a growth dataset, but it is the missing background lever arm. Without full-covariance SNe, the engine can confuse “growth mechanism” with “permitted late-time distance deformation.”

The **required exclusion set** for a defensible statement like “this suppressed-growth / dark-scattering mechanism class is ruled out at stated strength” is stricter: (a) SDSS/eBOSS plus DESI full-shape RSD with official covariance and AP-growth correlations; (b) at least two full weak-lensing likelihoods from DES Y6, KiDS-Legacy, and HSC Y3; (c) ACT DR6 or Planck PR4 CMB lensing as a high-redshift growth anchor; (d) DESI DR2 geometry plus Pantheon+ full covariance so the dark-energy equation-of-state side is priced; and (e) a full or validated emulator likelihood for any candidate with scale-dependent `μ(k,a)`, slip, or nonlinear power effects. A scalar `S8` and five diagonal `fσ8` rows can veto hype, not mechanisms.

The kill rule should be mechanistic, not rhetorical:

```text
For each candidate class C in {LCDM, w0waCDM, mu0_suppression, dark_scattering},
fit the same nuisance-complete data pack D under the same priors.
Promote C only if ΔlnZ(C - best_baseline) > +2 and every sealed kill block is non-worse
within its preregistered tolerance.
Exclude C at stated strength only if ΔlnZ(C - best_baseline) < -2 on the required pack,
its effect parameter is posterior-consistent with zero or the wrong sign,
and no predefined nuisance/systematics split rescues it.
Use |ΔlnZ| > 5 for public “strong” language.
```

A sharper version for the current class: if `μ0 < 0` or `A_drag > 0` improves old diagonal fσ8 but worsens full-shape RSD, DES/HSC/KiDS shear, or ACT lensing after refitting `σ8`, `Ωm`, neutrino mass, galaxy bias, intrinsic alignment, and baryon nuisance parameters, the verdict is not “needs more data.” It is “old scalar growth summaries were an attractive nuisance.”

## Ingestion interfaces

Extend `contracts/specs/dataset.yml` and each `data/registry/**/*.yml` entry with the following Rust-owned schema. TypeScript may render it in the dashboard, but policy, validation, tar checks, receipts, and run contracts belong in Rust.

```yaml
id: pantheonplus_v1_uncalibrated
title: Pantheon+ uncalibrated SNe Ia distance moduli
version: v1
release_date: 2022-02-08
source_url: https://github.com/PantheonPlusSH0ES/DataRelease
doi: null
citation:
  - arXiv:2202.04077
  - arXiv:2112.03863
license:
  kind: public_scientific_release
  redistribution: normalized_fixture_only
  required_citation: Pantheon+ collaboration papers
suite: cosmology
ingestion:
  effort: M
  raw_size_estimate: 100MB
  access_method: git_archive_or_release_tarball
  transform_script: tools/data_ingest/pantheonplus.py
  transform_git_rev: REQUIRED_AT_LOCK
provenance:
  raw_archive_sha256: REQUIRED_AT_LOCK
  normalized_fixture_sha256: REQUIRED_AT_LOCK
  covariance_sha256: REQUIRED_AT_LOCK
  lock_file: data/locks/pantheonplus_v1.sha256
observables:
  - family: mu
    canonical_pattern: mu@{z_cmb}
    unit: mag
    role: distance_modulus
covariance:
  kind: dense
  matrix_kind: covariance
  includes_stat: true
  includes_sys: true
  ordering: fixture_row_order
leakage_policy:
  tier: open_fit        # open_fit | sealed_kill | future_sealed
  proposer_visible: schema_only
  values_visible_to_proposer: false
double_count_tags:
  object_tags: [sne_ia_public_compilation]
  calibration_tags: [cepheid_if_sh0es_mode_enabled]
  analysis_tags: [pantheonplus_salt]
forward_requirements:
  min_model: flrw_background
  full_likelihood_required_for_promotion: false
validation:
  reproduce_reference_chi2: REQUIRED
  covariance_pd: REQUIRED
  canonical_ids: REQUIRED
```

Fixture JSONL remains small and normalized. Use canonical observable names when the existing grammar supports them:

```json
{"observable_id":"mu@0.01012","value":33.184,"uncertainty":0.142,"unit":"mag","source":"pantheonplus_v1_uncalibrated","covariance_id":"pantheonplus_v1_stat_sys","row_index":0,"provenance_sha256":"..."}
{"observable_id":"fsigma8@0.51","value":0.458,"uncertainty":0.038,"unit":"dimensionless","source":"sdss_dr16_lrg_fsbao","covariance_id":"sdss_dr16_lrg_dmdhfs8","row_index":4,"provenance_sha256":"..."}
```

Do **not** force full likelihoods into scalar rows. Add a sibling fixture type for grouped likelihoods:

```json
{"likelihood_group_id":"hsc_y3_shear_sacc","kind":"cosmic_shear_2pt","source":"hsc_y3","data_vector_uri":"external:data/hsc_y3.sacc","covariance_uri":"external:data/hsc_y3_cov.sacc","nuisance_schema":"contracts/specs/nuisance/shear_2pt.yml","provenance_sha256":"..."}
```

Covariance blocks should generalize the existing `data/fixtures/cosmology/covariance/*.json` format:

```json
{
  "id": "sdss_dr16_lrg_dmdhfs8",
  "format_version": "cosmo-covariance-block-v2",
  "source": "https://github.com/CobayaSampler/bao_data",
  "citation": ["arXiv:2007.08991"],
  "matrix_kind": "covariance",
  "observable_ids": ["dm_over_rd@0.38", "dh_over_rd@0.38", "fsigma8@0.38"],
  "matrix": [[...], [...], [...]],
  "units": ["dimensionless", "dimensionless", "dimensionless"],
  "raw_sha256": "...",
  "normalized_sha256": "...",
  "positive_definite": true,
  "condition_number": 1234.5,
  "condition_number_max": 1000000000000.0
}
```

Per-dataset validation tests must include:

* `all_rows_canonicalize_or_grouped`: every `observable_id` round-trips through `canonicalize_observable_id`; shear/CMB/full-shape exceptions must be `likelihood_group` records, not fake scalar IDs.
* `covariance_matches_rows`: covariance dimensions, row order, units, and diagonal uncertainties match fixtures; inverse covariances are inverted deterministically and stored as covariance unless `matrix_kind` says otherwise.
* `pd_no_jitter`: positive-definite validation cannot silently add diagonal jitter. If the public covariance is singular, the manifest must document a published compression or mode cut.
* `published_likelihood_replay`: each dataset has one reference cosmology or chain point whose χ²/loglike is reproduced within tolerance.
* `double_count_guard`: illegal combinations fail before scoring.
* `seal_guard`: sealed values are absent from proposer input, candidate JSON, progress ledgers, and dashboard payloads.
* `lock_guard`: raw hash, transform hash, normalized fixture hash, covariance hash, and scorer binary hash appear in the run receipt.

## Sealed holdouts and the contamination problem

Public data are contaminated for LLM-driven discovery in a way normal train/test language does not solve. The proposer may know “SH0ES is about 73” or “DES Y3 S8 is low” even if this run never prints the value. Honest generalization therefore means **the deterministic host can prove which values were available to the proposal channel and which values were opened only after candidate freezing**.

Define three data policies:

```rust
enum DatasetExposure {
    OpenFit,       // scorer can use values during search; proposer sees schema, not values
    SealedKill,    // values encrypted or physically absent until final adjudication
    FutureSealed,  // registry exists before publication; values and hashes absent until release
}
```

For `OpenFit`, the proposer still receives no numeric values; it receives only mechanism knowledge: “RSD constrains `fσ8(z)` at these redshifts,” “shear constrains projected matter clustering,” “Pantheon+ constrains relative luminosity distances.” Exact values, uncertainties, residuals, and per-block pulls are scorer-only. The proposer can know physics, not exploit numbers.

For `SealedKill`, the registry contains observable IDs, redshift bins, nuisance schema, expected covariance shape, source identity, and a hash commitment to an encrypted normalized bundle. The candidate is frozen before unsealing. The adjudicator then loads the sealed values, scores once, and writes a receipt containing the preimage hash. Any candidate text that contains exact sealed values before opening is a `ValueLeak` veto, not a clever prediction.

For `FutureSealed`, create manifests now for DESI DR3, Euclid DR1, Rubin DR1 shear/SNe, Roman SNe, LVK O5 sirens, and future Simons Observatory/CMB-S4 likelihoods. The manifest should declare expected observable families and acceptance tests, but values remain absent. Once released, a non-proposer data maintainer finalizes source URLs, hashes, and transforms. The first run that opens the values must be an adjudication-only run: no mutation, no proposer repair loop, no prompt regeneration.

Pseudocode for the run gate:

```text
build_proposer_packet(registry):
  for dataset in registry:
    emit dataset.id, physics_role, observable_names_or_group_schema,
         redshift_range, covariance_kind, nuisance_names
    redact values, uncertainties, residuals, row-level pulls, covariance entries

freeze_candidate(candidate):
  write canonical theory JSON
  write scorer binary hash and open-fit data locks
  ban further proposal calls

adjudicate(candidate, sealed_bundle):
  assert sha256(sealed_bundle) == registry.commitment
  load values and covariance
  score candidate and baselines with identical nuisance/profile rules
  write receipt with candidate hash, data hash, scorer hash, and verdict
```

Mechanism knowledge is allowed. Value knowledge is priced or banned. A theory may say “late-time drag should reduce `fσ8` at z < 1.” It may not say “fit `A_drag` so `fsigma8@0.61 = 0.413`” unless that value was open-fit and the parameter is charged through the model-selection machinery.

## Compressed versus full likelihood policy

Compressed vectors are adequate only when the compression is demonstrably sufficient for the model family being scored. Examples: DESI BAO ratios are acceptable for smooth late-time FLRW geometry when the sound horizon model is priced; Pantheon+ distance moduli are acceptable for background expansion; Planck distance priors are acceptable as a smoke test for late-time geometry with standard recombination and no perturbation novelty.

Compressed vectors are misleading when the candidate changes the physics that produced the compression. Modified gravity can alter CMB lensing, ISW, anisotropic stress, growth history, scale dependence, nonlinear matter power, neutrino-growth degeneracies, and galaxy bias inference. A distance prior cannot adjudicate those. The V6 8.4σ `ℓ_A` episode is the canonical failure mode: the compressed pipeline was not an innocent summary but the exploited instrument.

A sector must graduate to full likelihood when any of these is true:

* the candidate’s novelty witness is in that sector;
* more than 25% of data-fit credit or more than 2 nats of ΔlnL comes from a compressed sector;
* the candidate changes early-universe physics, recombination, `r_d`, CMB lensing, gravitational slip, or scale-dependent growth;
* a residual audit against a full likelihood differs by more than 0.2σ in any compressed coordinate or Δχ² > 1 at a reference candidate;
* a sealed kill dataset in the same sector is full-likelihood only.

Full likelihood requires a forward model that can generate the relevant observable: CLASS/CAMB/hi_class/MGCAMB/EFTCAMB or a validated emulator for CMB and matter power; nonlinear corrections, baryon feedback, intrinsic-alignment and photo-z nuisance models for lensing; galaxy bias/EFT nuisance and window functions for full-shape RSD; and posterior-sample population likelihoods for standard sirens. See S08 for backend architecture; this spec only sets the data gate.

## What not to ingest, and current double-counting audit

Do not ingest datasets just because they are famous.

* **Do not add more scalar `S8` values as final evidence.** Scalar `S8` is useful for smoke tests and plots, not verdicts. Lensing verdicts require the data vector, covariance, redshift distributions, intrinsic-alignment model, baryon nuisance, and shear calibration nuisance.
* **Do not stack SH0ES `h0` with Pantheon+SH0ES calibrated SNe as independent.** Either use uncalibrated Pantheon+ plus a separate H0 prior with its proper covariance, or use the combined Pantheon+SH0ES likelihood, not both.
* **Do not stack Pantheon+, Union3, and DES-SN5YR as independent evidence.** They share objects, low-z anchors, calibration assumptions, and light-curve standardization choices. Use one scored primary and the others as systematic challenge suites unless an overlap covariance exists.
* **Do not stack DESI DR1 and DR2 BAO for the same tracers.** DR2 supersedes DR1 for production scoring; DR1 stays as a regression test that guards reproducibility.
* **Do not add BOSS/eBOSS compressed `fσ8` and the same survey’s full-shape likelihood simultaneously.** The full-shape likelihood already contains the information.
* **Do not add cluster counts yet.** eROSITA, SPT, ACT, and optical cluster counts are valuable, but mass calibration, selection, miscentering, baryons, and survey masks are the likelihood. A scalar `S8`-like cluster constraint would be another attractive nuisance.
* **Do not add cosmic chronometers as high-weight evidence.** Stellar-population synthesis and age-model systematics are too easy to underprice. They can be a low-weight challenge suite only.
* **Do not use Planck distance priors as “CMB evidence” for MG.** They are background smoke tests. Promotion-grade MG claims need full CMB and lensing likelihoods.

Current 23-observable audit: DESI DR1 BAO intra-tracer covariance is now represented for five anisotropic tracers, but the fixture still has standalone isotropic BGS/QSO points and no global cross-survey covariance. The five RSD rows are diagonal and partly come from BOSS/eBOSS analyses whose geometry/growth covariance is absent. The DES Y3 scalar `s8` is a single compressed number and must not carry final growth verdicts. `h0` from SH0ES is independent of the current BAO/CMB/BBN rows, but it becomes non-independent the moment Pantheon+SH0ES calibration is ingested. Planck distance priors and BBN `Yp` are mostly independent in the current setup, but if D/H and `ω_b h²` BBN inference are added, the baryon-density and nuclear-rate correlations must be explicit.

## What we got wrong

1. **“The current champion has a clean sealed-holdout gap” is oversold.** `theory/holdout.rs` performs a deterministic split over known rows; that is not a value-sealed, future-release holdout. Check: insert a canary value in a held-out row and prove it never reaches proposer-visible artifacts. Until that passes, call it a split, not a sealed holdout.

2. **“Growth data support the mechanism” is too strong.** The archive’s growth evidence is five diagonal RSD points plus one scalar DES Y3 `S8`. That supports only the weaker statement “some compressed summaries are low.” Check: replace them with SDSS/eBOSS full-shape covariance plus DES/HSC/KiDS/ACT likelihoods and refit ΛCDM, w0waCDM, `μ0`, and dark scattering under identical nuisance rules.

3. **“Distance-prior CMB is an anchor” is true only for background smoke tests.** For MG, it is not a CMB likelihood. Check: run the same candidate through Planck PR4/CamSpec or plik-lite plus ACT/Planck lensing and compare Δχ² to the distance-prior score. Any discrepancy over the graduation threshold invalidates compressed-sector promotion credit.

4. **“SNe are Tier-0 and easy” is incomplete.** The forward model can compute `mu@z`, but the scientific object is the covariance-complete SNe likelihood with calibration modes and overlap tags. Check: reproduce Pantheon+ published ΛCDM/w0wa likelihood slices and prove SH0ES double-counting is impossible in the run contract.

5. **“More observables means more evidence” is false.** More rows can make the engine less honest if they are overlapping, scalar-compressed, or systematics-dominated. Check: every new row must have a manifest, covariance/likelihood, provenance lock, double-count tags, and a published replay test before it enters production scoring.

6. **“The data table is multisector enough for profound claims” is self-deceiving.** It is good enough to test machinery and to produce an honest negative. It is not enough to promote or rule out a modified-gravity growth mechanism. Check: no external claim may mention evidence for or against the current class until the required exclusion set above has been scored with profile-fitted baselines and sealed receipts.
