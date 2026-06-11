# SYNTHESIS — The V8 Program, distilled from 12 external engineering specs

Source: the 2026-06-11 jailgun "next-level" review event. 12 specs (S01–S12), each 3,100–4,900
words, produced by independent frontier-model reviewers against the full V7 source payload
(97 files, 959 KB). All 12 passed the harvest quality gate; see `INDEX.md`. This file is the
team's distillation: the ranked V8 backlog, the convergent findings, the conflicts to resolve,
and the paper-gating decisions.

---

## 1. The headline: what 12 independent reviewers converged on

Where reviewers with different lenses independently demand the same thing, treat it as settled:

1. **The pre-registered prediction registry is THE move.** Demanded independently by S05
   (future-sealed data tier), S06 (cryptographically time-locked forecast points), S10 (sealed
   registry for DESI DR3 / Euclid DR1 / Rubin), and S11 (the 12-month headline experiment).
   Rationale, in every case: it is the **only test an LLM-driven engine cannot have memorized**.
   Hash-sealed, third-party timestamped (OpenTimestamps + Zenodo/OSF + signed git tag),
   registered before collaboration release dates.
2. **The Boltzmann adjudicator is the universal prerequisite.** S08 owns it; S03, S04, S05, S07,
   S10, S11 each independently gate their own deliverable on it. The 8.4σ ℓ_A episode is cited
   across specs as proof that the fitting-formula instrument cannot carry promotion-grade claims:
   the constant Planck-point anchor is "a brittle local patch", and off-anchor
   residuals/derivatives remain exploitable.
3. **"Derived, not fit" is currently value-level vulnerable.** S01, S02, S09, S11 all found the
   same hole from different directions: a fitted posterior median can flow into a verified
   relation and collect derivation credit. The fix is S01's anti-laundering provenance rule
   (`scored_data_estimate` reaching a derived target = hard kill) + S09's
   `LiteratureValueLaundering` kill + S11's table-labeling discipline.
4. **The fit-set must stop paying points.** S06 (evidence gate, fit-set = 0 points, forecast
   points 0–30), S07 (claim classes; at n=23 the default cap is triage/interesting_fit), S10
   (sector-wise tie-scores-zero), S11 (no novelty credit without a hash-sealed forecast) — four
   independent designs of the same principle.
5. **The current "sealed holdout" is not sealed.** S05 and S07 both: `theory/holdout.rs` is a
   deterministic split over known rows; alternating 23 correlated rows is not a generalization
   estimate. Rename it a *split* until a canary leak test passes; real sealing = data tiers +
   block jackknife + the registry in (1).
6. **Evidence must graduate from the Schwarz proxy.** S07 (UltraNest nested sampling as the only
   promotion-grade engine; a 10× prior widening moves real lnZ by ~2.3 nats and BIC by zero),
   S06 (gate not score), S11 (audited marginal likelihood as the default quoted comparison).

---

## 2. Ranked V8 backlog (cross-spec, deduplicated)

| # | Item | Owner | Effort | Verification (compressed) |
|---|------|-------|--------|---------------------------|
| 1 | **Token/cost ledger**: `LlmCallReceipt` on every jnoccio+jailgun call (exact `usage` or flagged tokenizer estimate), `token-ledger.jsonl` events, SQLite rollups, `zyal tokens` CLI | S12 | M | Campaign report with `unattributed_calls=0`; replay unaffected with observability off |
| 2 | **Pre-registered prediction registry**: hash-sealed forecast entries for DESI DR3 (primary), Euclid DR1 (aux) — observable definition, frozen params, frozen code hash, signed timestamp BEFORE release dates | S06/S10/S11 | M | Post-hash entries earn zero; three independent timestamp surfaces verify |
| 3 | **Scorecard V5 restructure**: split into gates / theory score / engine KPIs / pricing ledger; fit-set data → evidence GATE (0 pts); forecast points (0–30) only from (2); `PricingLedger` fails closed on unpriced choices | S06 | M | Replay V4–V7: champions' relative ordering reproducible; fuzz every DOF type → ledger entry or harmless-proof |
| 4 | **Nested-sampling evidence engine**: UltraNest 4.5.x primary (dynesty cross-check), `EvidenceReceipt` (logz±err, prior/data hashes, diagnostics), deterministic-host-owned **prior registry** (LLM may not set priors), profile-likelihood companion lane | S07 | L | Receipt reproduces within 2 SE across machines; 10× prior widening shifts logZ ~ln(10) |
| 5 | **GoF + trials gates**: absolute χ²/dof + posterior-predictive gates before any "beats ΛCDM" is reportable; SearchLedger + ≥200 null-replay trials correction; sector jackknife + block bootstrap rank stability | S07 | M/L | ΔlnZ=+6 with global p<0.01 → `fit_failed_not_reportable`; champion rank-1 in ≥80% jackknifes |
| 6 | **Boltzmann adjudication service**: pinned CLASS (GR-sector) + hi_class (MG) worker pools, `SolverManifest` hashes, `ForwardTier::{T0Formula,T1Emulator,T2Boltzmann,T3CrossSolver}` on every score, tier-consistency `InstrumentRisk` gate | S08 | L | Planted +0.755 ℓ_A bias reduced to <0.5 nat or promotion blocked; nDGP/Hu-Sawicki fσ8 match published curves |
| 7 | **Calibration residual envelope**: fitting-formula−Boltzmann residuals sampled across the SEARCHED space (not just ΛCDM), folded in as `C_total = C_data + C_env` | S08 | M | Envelope covers ≥95% searched-space quantile; V5/V6 valley samples covered |
| 8 | **Growth verdict pack** (before ANY suppressed-growth claim): full-shape RSD (eBOSS DR16 + DESI), ≥2 independent lensing likelihoods (KiDS-Legacy / DES Y6 / HSC Y3), ACT DR6 CMB lensing; plus DESI DR2 BAO (S), Pantheon+ full covariance as sole scored SNe (M) | S05 | L | Reproduce each published likelihood at fiducial; per-dataset survival table before scoring |
| 9 | **Sealed data tiers + value firewall + double-count auditor**: `open_fit`/`sealed_kill`/`future_sealed`; proposer packets carry names/ranges/semantics, never held-out values; manifests with volume/object/calibration tags fail closed on overlap (Pantheon+SH0ES + scalar H0, DR1+DR2 stacking) | S05 | M | Hostile canary test: zero sealed values in prompts/ledgers/certificates pre-adjudication |
| 10 | **DerivationTrace sandbox**: replace value-only certificates with machine-checkable traces (typed expression DAG, step kinds, assumption-strength enum); `openqg-deriv-sandbox` isolated checker (no network, seccomp, content-addressed receipts); checker stack = SymPy/Symbolica + egg/egglog + Arb intervals + Schwartz–Zippel + optional Lean 4 lane | S01 | L | 9 registry relations verify from axioms; posterior-median-as-assumption rejected; byte-identical verdicts across machines |
| 11 | **Computed rigor**: `trace_rigor(verdict, policy)` = T·M·A·E·D·P from trace metrics replaces hand-assigned weights; hard caps (phenomenological parametrization ≤0.45, external measurement ≤0.25, scored-data 0.0); kill `LeanSketch`/`Positivstellensatz` stubs | S01 | M | 50 no-op rewrites don't raise rigor; literature-only trace earns no machine rigor |
| 12 | **Dimensional type system**: `DimVec` units through every quantity/expression/term; replace asserted `Term.mass_dimension` with checked `TermAst` | S01 | M | Forged dim-4 term with dim-5 expression killed; `exp(H0)` killed |
| 13 | **Term algebra (`openqg-algebra` crate)**: Horndeski-class action IR → theorem-producing compiler → EFT functions, stability/QSA/screening COMPUTED not asserted; per-theory `qsa_epsilon` gate with Boltzmann escalation; screening compiler (thin-shell, Vainshtein) replaces declared `screening_recovery` | S03 | L | All 9 relations fall out as theorems; right-named no-action "dgp_brane" rejected; hi_class/EFTCAMB golden fixtures match |
| 14 | **Component decomposition + gap routing**: `ComponentLedger` + deterministic two-mode ablation engine (mechanism-off-keep-cost / remove-reprice) + Shapley-style attribution; `GapPriority` dashboard; `FocusedPatchSketch` frozen-digest merge-back with anti-laundering kills (JobShiftToSibling, DofLaundering…); discounted-UCB bandit routes effort to gaps | S04 | L | Planted weak component ranks top gap; patch touching frozen paths killed before physics scoring |
| 15 | **Dual-path GP track**: quarantined PySR (+pyoperon adversary) symbolic regression over covariance-whitened residuals inside a Rust physics-response harness; `openqg.midpoint.v1` shared algebra; **explanation bounties** (GP form → derivation obligations → only route to scoreboard); MAP-Elites QD archive over physical behavior descriptors replaces 3-role islands | S02 | L | Closed loop on synthetic MG: GP recovers form bottom-up AND derivation lands top-down; no quarantined form reaches champions underived |
| 16 | **Knowledge layer**: equation-aware corpus (arXiv/INSPIRE/Living Reviews; LaTeXML + Tangent-style formula index + Qdrant/Tantivy hybrid retrieval); typed `RetrievalPacket` provenance; `LiteratureEquationMatch` obligation kind (demote citation-only `LiteratureEquivalence`); deterministic cross-campaign `KnowledgeLesson` ledger; contamination firewall; **pre-registered 2-week pilot with control arm** before shipping | S09 | L | Wrong-equation citations fail; red-team posterior tables yield zero laundering survivors; pilot thresholds pre-declared |
| 17 | **H0 panel + field-state calibration**: replace single SH0ES scalar with multi-calibrator `h0-panel.jsonl` (Cepheids/TRGB/JAGB/TDCOSMO/masers/sirens/inverse-ladder); per-family residuals; external-plausibility scoring of champions vs published MG constraints | S10 | M | Panel reproduces published per-family values; V7 class external plausibility published (currently ~35/100) |
| 18 | **Budget-aware routing + stopping rules**: route_score = value/cost + UCB; marginal-token-productivity stopping rule (pause when MTP < 0.25 pts/100k tokens over 3 windows); pre-registered token-efficiency experiments (caching, repair cuts) | S12 | M | Synthetic router test prefers higher expected value per budget unit |
| 19 | **Engine KPI dashboard**: time-to-invalidate (min/median/p90), exploit discovery rate, regression-corpus growth, search volume covered, exclusion strength, cost-normalized score — published per campaign as `engine_kpis.json`; sealed exploit reserve with z-test release gate | S06 | M | KPIs reproduce from ledgers alone; LOCOCV passes held-out exploit class |
| 20 | **V8 exclusion campaign + paper program**: freeze `C_growth-suppression(V8)` class (μ0∈[−0.30,0] / A_drag∈[0,10], w0∈[−0.99,−0.80], GW170817-safe, late-time), machine-readable search-volume.yml, exclusion sentence with trials correction + rank stability; head-to-head vs human-coded scan + FunSearch-style baseline | S11 | L | Exclusion survives a blind rerun; benchmark table reports time-to-first-invalidated-champion + decoy false-positive rate |

---

## 3. The hard truths (convergent "What we got wrong")

These were each asserted by ≥2 independent reviewers; treat as accepted findings, not opinions:

- **"Total observability" is currently false** — no token/cost/provider fields exist anywhere in
  the ledgers (S12). Fix before the phrase appears in print again.
- **"Symbolic discovery" is overstated** — ProposalSketch is a small fixed schema; islands are
  role schedulers; structural entropy over generated terms is likely near zero (S02). The term
  registry is a name allowlist, not a generative mechanism gate (S03: a right-named, dim-4,
  zero-free-index candidate with NO action passes V7).
- **The V7 champion is a candidate direction, not a result** — five diagonal fσ8 points + one
  scalar S8 only support "some compressed summaries are low" (S05); external plausibility vs
  data the engine hasn't ingested ≈ 35/100 (S10); it belongs in the exclusion protocol, not the
  abstract (S11).
- **Physics-narrative bug**: the healthy nDGP normal branch ENHANCES growth (β>0 → G_eff/G>1);
  any suppressed-growth DGP story needs an additional mechanism (S03). Correct the paper.
- **Stale artifact**: `prod-1000-champion.json` still carries the debunked +36.7 number (S10).
  Purge or annotate it.
- **The registry has a consistency hole**: `dark_scattering_growth_drag` is registered and
  verified but the relation-signature matcher lacks its branch (S03, citing
  certificate.rs:184-205 vs 291-315). File as a bug now.
- **The data brief is itself a contamination channel** — telling proposers which tensions to aim
  at leaks target direction (S09).

## 4. Conflicts to resolve before V8 engineering starts

1. **Scorecard ownership**: S06 restructures the rubric; S08 wants `ForwardTier`/`InstrumentRisk`
   as first-class scorecard fields; S07 wants claim classes. One owner must merge these into a
   single ScorecardV5 schema (suggest: S06's four-way split as the frame; S07 claim classes and
   S08 tier fields as gate inputs).
2. **Who owns the midpoint algebra**: resolved by the specs themselves — S03 owns the internals
   (`openqg-algebra`), S02 only the `openqg.midpoint.v1` interface. Hold that line.
3. **Fit-set points**: S06 says zero; S07 retains data-fit within claim-class caps. Compatible if
   data fit becomes a GATE plus the forecast-points channel — but the decision must be explicit
   and replayed against V4–V7 for continuity.
4. **Effort realism**: items 4, 6, 8, 10, 13, 14, 15, 16 are each L; they will not all land in
   one phase. Phasing below.

## 5. Suggested phasing

- **Phase 0 (days–weeks, unblock honesty)**: #1 token ledger, #2 prediction registry mechanics,
  #19 engine KPIs, the claims linter (S10/S11), purge the +36.7 artifact, file the
  dark-scattering matcher bug.
- **Phase 1 (the verdict stack)**: #3 scorecard restructure, #4–5 evidence+GoF, #8–9 data pack +
  sealing, #6–7 Boltzmann + envelope.
- **Phase 2 (the rigor stack)**: #10–12 derivation sandbox, #13 term algebra.
- **Phase 3 (the discovery stack)**: #14 decomposition, #15 dual-path GP, #16 knowledge layer,
  #17–18 routing/H0 panel.
- **Phase 4 (the claim)**: #20 exclusion campaign; pre-registered DESI DR3 forecast (entries
  sealed in Phase 0!); two-paper strategy — methodology paper to JCAP/PRD now ("Adversarially
  audited autonomous model selection in late-time cosmology"), Nature-tier only after the
  12-month preregistered experiment resolves.

## 6. Paper actions (gating the rewrite — full detail in S11, S10, S05)

- Reframe: a falsifiable autonomous-science-instrument methodology paper; cosmology is the
  stress test. The headline product is the audit cascade made quantitative (time-to-invalidate,
  exploit bounty in points/nats, regression-lock latency) — S11 supplies verbatim replacement
  abstract/conclusion text.
- The profundity threshold, made explicit (S06): "profound" label only at global ≥5σ + ΔlnZ ≥ +5
  after trials correction + mechanism-off twin losing >70% of the effect.
- No external claim about the suppressed-growth class until the growth verdict pack (#8) is
  scored under sealed discipline (S05's exact dataset list).
- Position against FunSearch / AlphaEvolve / AI-Descartes / AI-Hilbert / AI Scientist /
  Coscientist / Robot Scientist with the one claim none of them can make — the engine
  adversarially discovered, corrected, and regression-locked an 8.4σ bias in its OWN evaluator
  before reporting an honest negative.

---
*Generated 2026-06-11 from spec-S01…S12 digests; full specs in this directory. Event artifacts:
`ops/jailgun/next-level-manifest.txt`, `ops/jailgun/prompts/next-level/`, `tools/jailgun-next-level.sh`.*
