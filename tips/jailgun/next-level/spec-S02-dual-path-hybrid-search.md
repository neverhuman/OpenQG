# S02 — Dual-path hybrid search: top-down theories meet bottom-up phenomenology

Review batch tab: 1 of 1.

## Ranked backlog

| Rank | What | Why it matters | Effort | Hostile-reviewer acceptance test |
|---:|---|---|---:|---|
| 1 | **Introduce a quarantined bottom-up symbolic-regression track over truth-bound observable residuals, emitting only midpoint forms, never scoreboard candidates.** | The attached materials correctly admit that the judge became much stronger than the generator: the new symbolic engine has honest gates, while population evolution still mostly mutates a small numeric surface. A data-upward path gives the engine a second source of hypotheses without letting curve fits masquerade as whitebox theories. | M | On the real 23-observable Tier-1 corpus, `openqg-bench gp residual-search` produces a content-hashed Pareto archive of midpoint forms, each marked `phenomenological_quarantined`; none appears in `ScorecardV4` champion output unless later promoted through derivation obligations. |
| 2 | **Define `openqg.midpoint.v1`: a shared algebraic emission format for both GP forms and LLM-derived theories.** | This is the load-bearing interface. Without one canonical algebra, the GP track becomes a parallel toy and cannot answer “is this residual form the quasi-static limit of that term?” The current `Term` is only `{name, mass_dimension, free_lorentz_indices}` and the V7 registry is an allowlist, not a full algebra (`crates/openqg-core/src/theory/mod.rs:194-202`, `paper/main.tex:666-668`). | M | A known Planck-MG form `mu(a)=1+mu0*Omega_DE(a)/Omega_DE0` emitted by PySR and the same form emitted by `ProposalSketch` canonicalize to the same structural hash modulo coefficient names; a deliberately equivalent expression with expanded products matches; a row-lookup expression does not. |
| 3 | **Replace the fixed island roster with a quality-diversity archive whose descriptors are physical, not administrative.** | Current theory-population code uses three roles (`Explore`, `Exploit`, `Novelty`) and cycles them by index (`theory_population.rs:31-65`, `583-599`). Docs still describe six named islands as diversity-preserving scaffolding (`docs/ZYAL.md:182-189`), but neither design maps to physical diversity. Search will collapse into the same suppressed-growth basin. | M | After 100 generations on the Tier-1 corpus, a QD report shows non-empty elites in at least four observable-pull bins and three mechanism bins, while the old three-island run reaches fewer descriptor cells under the same seed and compute. |
| 4 | **Make residual extraction covariance-whitened, baseline-refit, and block-held-out by survey/observable family.** | GP is the ultimate overfitter. The repo already has covariance blocks (`scoring/covariance.rs:1-18`) and an Occam-charged score path (`physics_score.rs:89-120`), but `holdout.rs` only offers alternating-index holdout (`holdout.rs:106-109`), which is too easy for smooth redshift formulas to leak across. | S-M | A noise-only synthetic dataset produces attractive train residual formulas but every one fails block holdout or bootstrap stability; a known injected MG signal survives leave-one-family-out splits. |
| 5 | **Add explanation bounties: GP forms that fit become LLM/top-down derivation tasks with generated obligations.** | The invariant “LLM proposes; deterministic host judges” is right (`docs/ZYAL.md:11-23`). The missing loop is upward-to-downward translation: GP should create targets, not theories. LLM agents then attempt derivations, certificates, and claim graphs using existing machinery. | M | A quarantined midpoint form opens a bounty ledger entry. A successful derivation is converted to `ProposalDoc`, passes evidence materialization, obligation verification, binding, and `ScorecardV4`; an unsuccessful derivation leaves the GP form quarantined with zero whitebox score. |
| 6 | **Use PySR as the primary engine, Operon as a second-engine adversary, and Rust as the durable authority.** | PySR gives fast Pareto symbolic regression with Julia performance and a Python interface; Operon gives an independent GP implementation. The project should not write a pure-Rust SR engine first. Rust should own configs, sealed splits, run contracts, tar/input validation, midpoint canonicalization, receipts, and promotion rules; Python/Julia workers are disposable search executors. | M | The same run manifest reproduces identical candidate canonical hashes from pinned inputs; promotion does not trust the Python process except for candidate text, which Rust re-parses, re-evaluates, hashes, and scores. |
| 7 | **Build a synthetic closed-loop acceptance suite before running V8 on real data.** | The only convincing demonstration of the two-way search is a recovery experiment: generate data from a known MG theory, recover the functional form bottom-up, then derive it top-down. | M | For synthetic data generated from f(R), nDGP, and Planck-MG `mu0` families, the bottom-up archive recovers an equivalent midpoint form in its top-5 Pareto entries, and the derivation/promoter path produces a certified `ProposalDoc` or a documented, correct non-derivability reason. |

## Repo-grounded diagnosis

The design target is not a blank slate. The source already has a defensible deterministic judge. `docs/ZYAL.md` makes the core invariant explicit: the LLM may propose and critique, but deterministic gates judge (`docs/ZYAL.md:11-23`, `135-136`). The new symbolic engine is the one to build on, not the retired identity-forward genome path: `docs/ZYAL.md` contrasts the legacy float-vector candidate with a `Theory` plus real FLRW and growth forward model (`docs/ZYAL.md:227-255`). `ScorecardV4` is veto-first, content-bound, and one-sided on evidence: it materializes claim evidence, runs physical vetoes, enforces obligations, binds certified MG claims, audits novelty, and gives zero data-fit credit for merely tying LCDM (`scorecard.rs:1-19`, `153-158`, `394-443`, `489-545`). That is the right authority layer.

The generator is the weak link. `theory_population.rs` says pure-parameter children carry empty claim graphs and earn no derivation/unification credit (`theory_population.rs:13-16`, `192-213`), while derivation-rich candidates enter through a proposer path (`theory_population.rs:541-543`). That is honest, but it means the search side is still shallow. `proposer_sketch.rs` exposes only a flat surface of `mg_family`, `mu0`, f(R), nDGP, `w0`, and `wa` (`proposer_sketch.rs:72-84`) and only four mechanism lanes (`proposer_sketch.rs:499-512`). The repo’s own internal critique says the rebuild fixed the judge but kept the old generator, and that “symbolic discovery” is still mostly numeric mutation (`docs/zyal-next-level-design.md:40-51`). S02 should not soften that: V8 needs a second generator that starts from data residuals and climbs toward theory.

The current admitted observable space is small but real: 23 Tier-1 records, consisting of 12 DESI BAO ratios, BBN helium, five `fσ8(z)` points, SH0ES `H0`, KiDS-like `S8`, and three CMB distance priors (`data/fixtures/cosmology/tier1-multisector.jsonl`, `paper/data/observables.jsonl`). The growth module computes linear growth with `mu(a)`, dark-scattering drag, `fσ8`, `S8`, and a scale-dependent f(R)/nDGP reference path (`growth.rs:1-19`, `42-76`, `147-158`, `166-212`, `265-278`). The honest V7 champion class is exactly where this observable space points: late-time suppressed growth via `mu(a)=1+mu0*Omega_DE(a)/Omega_DE0`, right direction but insufficient evidence (`paper/main.tex:527-573`). A bottom-up track should therefore search for smooth residual-driving functions in `mu`, `Sigma`, `w`, and drag space, not arbitrary functions from row numbers to observables.

## 1. Bottom-up symbolic-regression track

### 1.1 Engine choice and run boundary

Use **PySR** as the primary engine for V8: `pysr==1.5.10`, Python 3.12, Julia 1.11.x, with `SymbolicRegression.jl` locked in a checked-in Julia `Manifest.toml`. Use **pyoperon==0.6.1** as an adversarial confirmation engine for forms considered for bounties. PySR/Cranmer-style Pareto symbolic regression is the best first choice because it is mature, supports custom operators and complexity weights, and returns an accuracy-complexity frontier rather than one opaque fit. Operon is valuable because it is an independent C++ GP engine; when both engines rediscover the same simple form under the same sealed split, the result is much less likely to be a PySR artifact.

Do **not** build the first V8 GP engine in pure Rust. Rust should own the durable parts: manifest parsing, dataset sealing, covariance whitening, split generation, primitive whitelist, canonical re-parsing, midpoint emission, receipts, and promotion policy. The Python/Julia process is an untrusted search worker. It receives a manifest and an Arrow/JSONL design matrix, emits candidate expressions and metadata, and is never trusted for scoring or promotion. TypeScript should only render dashboards: Pareto fronts, QD cells, residual plots, and bounty status.

A minimal run contract:

```yaml
schema: openqg.gp_run.v1
run_id: v8-s02-tier1-smoke
input_hashes:
  observables: sha256:...
  covariance_blocks: [sha256:...]
  baseline_fit_receipt: sha256:...
engine:
  primary: {name: pysr, version: 1.5.10, julia: 1.11.x}
  challenger: {name: pyoperon, version: 0.6.1, required_for_bounty: true}
sealed_splits:
  public_train_blocks: [desi_bao_lowz, rsd_lowz, cmb_distance]
  public_validation_blocks: [desi_bao_highz, rsd_highz, s8]
  hidden_kill_blocks: content-addressed, not passed to worker
allowed_targets: [delta_mu, delta_sigma, delta_w, gamma_drag]
forbidden_features: [observable_id, row_index, survey_name, covariance_row, literal_redshift_table]
```

### 1.2 Residual extraction

The bottom-up track starts from a profile-refit LCDM baseline, not the Planck-pinned reference. `league.rs` already states the promotion-grade rule: every model, baseline included, must be profile-fit to the same data under covariance-aware likelihood and compared by AIC/BIC/Laplace evidence (`league.rs:1-17`). Reuse that path to get `theta_LCDM*` and predictions.

For each observable block, compute whitened residuals:

```text
r_raw = y_obs - y_lcdm(theta_LCDM*)
r_white = L^{-1} r_raw, where C = L L^T for the covariance block
```

Uncorrelated observations use `r_white_i = r_i / sigma_i`. This aligns the search with the scorer’s geometry. A one-sigma miss in `cmb_lA` and a one-sigma miss in `fsigma8@0.61` should be equally expensive after whitening; correlated BAO residuals should not be double-counted. The repo already implements `-1/2 r^T C^-1 r` scoring and marginalization of missing predictions (`covariance.rs:1-18`, `180-259`), so residual extraction should use the same `CovarianceRegistry` and refuse malformed covariance blocks (`covariance.rs:360-430`).

The GP target is **not** a direct map from features to residual values. With only 23 real observations, direct symbolic regression can memorize noise. Instead, run GP inside a physics response harness. A candidate expression produces a functional perturbation such as `delta_mu(a,k)`. Rust injects that function into a midpoint forward shim, predicts all observables, whitens the residual vector, and returns the loss. The search sees rows with physical coordinates, but the objective is the end-to-end observable likelihood.

### 1.3 Variables, primitives, and targets

Allowed variables are known physical quantities only:

```text
a, z, ln_a, E(a)=H(a)/H0, Omega_m(a), Omega_DE(a), w(a),
k_over_kstar, log_k_over_kstar, growth_D(a), growth_f(a)
```

`growth_D` and `growth_f` may be exposed only as baseline LCDM response variables, not as fitted data columns. Observable family may appear only as a target kernel chosen by Rust, never as a feature. For example, RSD rows evaluate `delta_mu(a,k_ref)` through the growth ODE; CMB distance-prior rows are insensitive unless the target is `delta_w` or a Boltzmann-backed extension.

Initial target spaces:

```text
delta_mu(a,k)      # matter clustering / G_eff/G - 1
delta_sigma(a,k)   # lensing Weyl response - 1; quarantined until lensing data exists
gamma_drag(a)      # extra growth friction, e.g. dark scattering
delta_w(a)         # background dark-energy equation-of-state deviation from -1
```

Primitive set:

```text
binary: +, -, *, protected_div(x,y)=x/(abs(y)+eps)
unary: square, cube, sqrt_pos, log_pos, exp_clamped, tanh, sigmoid, inv1p_abs
constants: dimensionless ephemeral constants in [-3,3], quantized for receipts
restricted powers: x^2, x^3, sqrt(x); no arbitrary pow(x,c) in first release
```

Mandatory physical guards:

```text
GR early-time limit: |delta_mu(a<0.05,k)| < epsilon unless target declares early-time physics
stability guard: 1 + delta_mu > 0, 1 + delta_sigma > 0 on the domain grid
smoothness guard: finite Lipschitz penalty on ln a and ln k
screening declaration: if nonzero small-scale limit survives, mark screening_required
```

This primitive set is deliberately boring. AI Feynman showed the value of physics-inspired decompositions, but this project’s data volume is tiny and adversarial; arbitrary functions are liability, not power. Add richer operators only after a kill-dataset audit.

### 1.4 Complexity and Pareto archive

Use an explicit MDL-style complexity ledger. The score stored for every expression is not one scalar but a vector:

```text
accuracy = -2 * delta_lnL_cov_whitened
complexity = sum(node_weight) + 2*num_constants + 3*num_protected_ops
             + 5*num_nonanalytic_ops + 4*num_target_functions
             + 2*num_piecewise_branches + dof_ledger_cost
stability_penalty = max_grid_violation + smoothness_penalty
holdout_gap = train_loss_per_mode - validation_loss_per_mode
```

Recommended node weights: `+,-,*` cost 1; protected division costs 3; `log`, `exp`, `tanh`, `sigmoid` cost 3; constants cost 2; references to `H(a)`, `Omega_m(a)`, `Omega_DE(a)`, and `w(a)` cost 1 because they are known physical state variables; branch/piecewise is disabled in the first release and, when enabled, costs at least 8 per branch.

Archive a Pareto frontier per target function and per QD cell, not one champion. A form qualifies for an explanation bounty only if:

1. it improves validation `delta_lnz` after the same Occam term used by the scorecard;
2. its hidden kill-block performance is not worse than validation by a configured tolerance;
3. at least two bootstrap resamples preserve its canonical top-level structure;
4. pyoperon or a second PySR seed recovers an equivalent response fingerprint; and
5. it is not algebraically equivalent to LCDM, w0waCDM drift, or a known registry form unless the value or mechanism target is novel.

Honest compute budget: V8 should reserve **32 CPU cores for 12 hours per target family** for the first real run, four families maximum, about 1,536 core-hours total plus Rust rescoring. Smoke tests must run in 30 minutes on 8 cores. No GPU is needed. Given 23 observations, more compute mostly finds better overfits; budget discipline is part of anti-cheating.

## 2. Midpoint representation: the meeting algebra

Both paths must emit the same representation. The LLM/top-down path emits an action/claim/certificate story today; the GP path emits an expression. The meeting point should be the **observable-response algebra**: canonical functions over physical state variables, with links to claim-graph obligations when available. S03 should design the internals of the term algebra; S02 requires this stable interface.

```rust
#[derive(Serialize, Deserialize)]
struct MidpointForm {
    schema: String, // "openqg.midpoint.v1"
    id: String,
    producer: Producer, // BottomUpGp | TopDownProposal | HumanAnchor | Synthetic
    status: MidpointStatus, // PhenomenologicalQuarantined | BountyOpen | DerivedCandidate | Promoted | Killed
    target_space: TargetSpace, // MuSigma | BackgroundW | GrowthDrag | BoltzmannResponse
    variables: Vec<VariableSpec>,
    expressions: Vec<FunctionExpr>,
    coefficients: Vec<CoefficientSpec>,
    domain: DomainSpec,
    complexity: ComplexityLedger,
    fit: Option<PhenomenologyFitReceipt>,
    claim_link: Option<ClaimGraphAttachment>,
    canonical: CanonicalReceipt,
    receipts: Vec<ReceiptRef>,
}

struct FunctionExpr {
    name: FunctionName,      // mu, Sigma, gamma_drag, w, H, alpha_m, alpha_b
    args: Vec<VariableName>, // [a], [a,k]
    expr: TermExprRef,       // see S03 for AST/e-graph internals
    limits: Vec<LimitSpec>,  // GR limit, early-time limit, k->0/k->infty
    response_fingerprint: Vec<f64>, // fixed grid evaluations, content hashed
}

struct ClaimGraphAttachment {
    generating_terms: Vec<String>,
    obligations: Vec<String>,
    certificate_relations: Vec<String>,
    binds_background_fields: Vec<String>,
}
```

Canonicalization must support four operations:

1. **Diff:** show what two forms change in variables, coefficients, and limits.
2. **Equivalence:** e-graph simplification and numeric fingerprint agreement on a fixed grid.
3. **Subsumption:** a top-down form may be a constrained version of a GP form, e.g. GP finds `c*Omega_DE(a)` and a proposal derives `c=mu0/Omega_DE0` from dark scattering.
4. **Quasi-static matching:** compare a derived theory’s QSA response to a GP-discovered `mu(a,k)`/`Sigma(a,k)` surface by symbolic rewrites plus response-grid tolerance.

The current `ProposalSketch` expander already adds generating terms when certified relations or background dials appear (`proposer_sketch.rs:344-375`). Extend it so every top-down `ProposalDoc` also emits a `MidpointForm` after binding. For example, `planck_mu0_geff` produces `FunctionExpr{name: mu, expr: 1 + mu0*Omega_DE(a)/Omega_DE0}`. Conversely, a GP expression does not invent `Term`s; it has `claim_link=None` until a derivation arrives.

## 3. Meet-in-the-middle protocol

### 3.1 Lifecycle

```text
1. Fit LCDM baseline and compute covariance-whitened residual receipts.
2. Run bottom-up GP over allowed target functions through the Rust forward harness.
3. Canonicalize expressions into openqg.midpoint.v1.
4. Score only as PhenomenologyFitCard; mark scoreboard_eligible=false.
5. Open explanation bounties for robust, simple, nontrivial forms.
6. LLM proposer/skeptic agents attempt to derive each form from registered or new terms.
7. Deterministic oracle verifies obligations/certificates, binds the background, audits novelty.
8. Successful derivation becomes a normal ProposalDoc and may enter ScorecardV4.
9. Failed derivation remains a quarantined data hypothesis and may seed future GP, not claims.
```

A new underived-fit report should be intentionally named **`PhenomenologyFitCard`**, not `ScorecardV4`:

```rust
struct PhenomenologyFitCard {
    midpoint_id: String,
    scoreboard_eligible: bool, // always false unless promoted elsewhere
    train_delta_lnz: f64,
    validation_delta_lnz: f64,
    hidden_kill_delta_lnz: Option<f64>,
    complexity: f64,
    stability: StabilityReport,
    bootstrap_support: f64,
    closest_known_forms: Vec<CanonicalMatch>,
    bounty_priority: BountyPriority,
    kill_reasons: Vec<String>,
}
```

This card can rank bounty priority but cannot award whitebox score. It preserves the project’s proudest asset: every exploit became a test, and a tie with LCDM scores zero. A pretty formula over residuals is not a theory.

### 3.2 Explanation bounties

A bounty contains the midpoint form, its canonical hash, allowed mechanism families, and the exact obligations a derivation must satisfy. For a GP form resembling suppressed growth:

```yaml
bounty_id: bounty-mu-omega-de-001
midpoint_hash: sha256:...
target: derive delta_mu(a,k)=c*Omega_DE(a)/Omega_DE0
required_outputs:
  - generating_term_or_registered_relation
  - limit_witness: GR at c -> 0 and a -> early
  - certificate: c equals a derived value or is explicitly costed as a fitted physical parameter
  - binding_consistency: background.mu0 equals certificate input after bind
  - novelty_witness: not in fit set, or credit capped as fit explanation
forbidden_shortcuts:
  - declare c fundamental without mechanism
  - cite GP fit as evidence for derivation
  - move H0/Omega_m/w0 to absorb the residual without paying DOF
```

Credit assignment: the GP track receives “target discovery” credit in the engineering ledger and QD archive. The derivation agent receives theory credit only for the certified mechanism. If derivation lands after the GP identified the form, novelty credit must be carefully capped unless the promoted theory makes a new out-of-fit-set prediction. This aligns with `scorecard.rs`, where fit-set witnesses cap novelty and only honest out-of-fit computed predictions can earn full novelty (`scorecard.rs:547-579`).

### 3.3 Top-down seeds back into GP

Every top-down proposal should emit its midpoint response even if it scores poorly. Those forms seed the GP primitive prior in three ways:

```text
force_include_subtree: known response factors such as Omega_DE(a)/Omega_DE0
mutation_prior: raise probability of nearby algebraic variants
negative_memory: reduce probability of refuted structures that failed hidden holdout
```

This is the true two-way street. Smart theories push downward by proposing physically sensible response bases; data pushes upward by revealing which response shapes deserve explanation.

## 4. Quantitative dual-path analysis

A brute-force 0%-intelligence GP can find only low-dimensional, smooth, high-signal residual modes in the current corpus. The search space is enormous even with a small primitive set. With 10 variables, roughly 12 operators, and expression trees of size 15, the count of syntactic trees is far above `10^15` before constants. Regularized evolution makes this searchable only because the acceptance signal rewards simple coherent improvements. With 23 observations and correlated blocks, the effective independent modes are smaller than 23; `physics_score.rs` already uses effective modes in the Occam term rather than raw record count (`physics_score.rs:93-103`). Therefore the identifiable functional dimension is likely **one to three smooth modes**, not a free function of `a` and `k`.

Expected discoverable forms:

```text
c * Omega_DE(a) / Omega_DE0
c * (1-a)
ctanh * tanh(beta*(a-a0)) with heavy complexity penalty
c * Omega_m(a)^p approximately, if p is restricted/quantized
scale-free drag gamma(a) proportional to (1+w(a))*Omega_DE(a)
```

Expected non-discoverable from current data alone: a Horndeski action, a unique screening mechanism, separate `mu` and `Sigma`, scale dependence beyond the single RSD reference scale, and full CMB-era physics. Background+growth admits degeneracies: `h` and `Omega_m` move distance ratios along a documented valley (`paper/main.tex:721-729`); `sigma8`, `mu0`, and drag can all suppress `fσ8`; `S8` is a broad amplitude constraint; and without CMB lensing, galaxy-galaxy lensing, or scale-resolved full-shape data, `Sigma(a,k)` is barely identified. A GP track must report this as an identifiability limit, not bury it.

The 100%-intelligence/no-derivation extreme contributes exactly what brute force lacks: priors over physically sensible forms, limits, and mechanisms. LLM agents can suggest that a residual proportional to `Omega_DE(a)` is plausible for late-time MG, that dark scattering introduces `(1+w)Omega_DE`, or that f(R) introduces scale dependence through a scalaron Compton scale. But without derivation and deterministic certificates, those are labels on knobs; the internal critique already notes that “derived, not fit” must be true at the value level, not merely prose (`docs/zyal-next-level-design.md:53-74`).

The crossover is around “simple response form with ambiguous mechanism.” GP can surface `delta_mu ~ -0.1 Omega_DE(a)/Omega_DE0`; LLM/top-down search can try to explain whether it is Planck-MG phenomenology, dark scattering, f(R) in a scale-suppressed regime, or an artifact of `sigma8`/RSD normalization. The combination unlocks something neither path has alone: a ranked queue of data-shaped forms, each forced through whitebox derivation or killed. That is closer to AI-Descartes and AI-Hilbert than to generic curve fitting: data proposes equations, background theory filters derivability.

Named prior art should be used precisely. **FunSearch** (Romera-Paredes et al., Nature 2024, DOI `10.1038/s41586-023-06924-6`) pairs an LLM with a systematic evaluator and evolutionary loop; in OpenQG terms it is a proposer for search programs, not a judge. **AlphaEvolve** (Novikov et al., arXiv:`2506.13131`) shows LLM-generated code improved through evaluators; its pattern is useful for generating GP mutation operators or derivation-search code, not for scoring cosmology. **AI-Descartes** (Cornelio et al., arXiv:`2109.01634`) and **AI-Hilbert** (Cory-Wright et al., arXiv:`2308.09474`) are the closest intellectual relatives because they combine data with background-theory derivability. **AI Feynman** (Udrescu and Tegmark, arXiv:`1905.11481`) motivates physics-inspired decompositions and symmetry constraints. **PySR** (Cranmer, arXiv:`2305.01582`) is the practical SR tool. **MAP-Elites** (Mouret and Clune, arXiv:`1504.04909`) motivates the QD archive: illumination of a search space is more useful here than one fragile champion.

## 5. Population design: physical quality-diversity

The current six-island story is not the right scaffold for V8. It is useful operationally, but the archive must be physical. Use MAP-Elites/QD over descriptors that a scientist would recognize:

```rust
struct BehaviorDescriptor {
    observable_pull: ObservablePullBin,
    mechanism_family: MechanismBin,
    response_shape: ResponseShapeBin,
    gr_limit: GrLimitBin,
    screening: ScreeningBin,
    degeneracy_direction: DegeneracyBin,
    complexity_bucket: ComplexityBucket,
    evidence_risk: EvidenceRiskBucket,
}
```

Descriptor values:

```text
observable_pull: BAO_lowz, BAO_highz, CMB_distance, RSD_lowz, RSD_highz, H0, S8, BBN, mixed
mechanism_family: background_w, mu_scale_free, mu_scale_dependent, sigma_lensing, drag, screening, null
response_shape: OmegaDE_like, transition, power_law_a, scale_transition_k, oscillatory, unknown
GR_limit: exact_today, early_time_only, coefficient_zero, no_GR_limit
screening: none_needed, chameleon_like, vainshtein_like, symmetron_like, required_unquantified
degeneracy_direction: h_Omega_m, sigma8_mu0, w0_wa, covariance_eigenmode, none
```

Each cell stores a small Pareto set, not one elite: best validation evidence, simplest expression, best holdout stability, and best derivability match. This makes failure informative. A cell full of high-fit/no-derivation `sigma8_mu0` formulas tells the team the data are asking for growth suppression but not revealing mechanism. A cell where top-down f(R) forms repeatedly fail hidden holdout becomes a refuted region for proposer memory.

## 6. GP anti-cheating and kill rules

GP will monetize every unpriced degree of freedom. Known failure modes and kill rules:

| Gaming mode | Kill rule |
|---|---|
| Row memorization via `observable_id`, index, survey, or tracer label | These features are never present in the worker matrix; Rust rejects any emitted symbol outside `VariableSpec`. |
| Redshift table lookup through piecewise cliffs | Disable piecewise first release; later require minimum branch support, branch complexity rent, and hidden redshift interpolation tests. |
| Covariance-eigenvector fitting | Hidden covariance rotations and leave-one-block-out validation; forms that improve only one covariance eigenmode without physical response coherence are killed. |
| Constants encoding measured values | Quantize constants, price constants heavily, and compare against forbidden fixture/value hashes. |
| Protected-division singularities near data points | Domain-grid finite checks and Lipschitz penalties; any pole inside or near the observable domain is killed. |
| Scale `k/k*` as row identifier | Only expose `k` where the observable truly has a physical scale kernel; all current RSD points share the same `k_ref`, so scale dependence cannot identify rows. |
| Fit-set novelty laundering | GP forms cannot claim novelty; promoted theories inherit `scorecard.rs` novelty caps for fit-set witnesses. |
| Noise rediscovery | Block holdout, hidden kill datasets, bootstrap support, and pyoperon confirmation are mandatory for bounties. |
| Algebraic duplicates flooding archive | Canonical AST/e-graph hash plus response fingerprint deduplication. |
| Baseline drift hiding as MG | Refit LCDM/w0wa competitors and charge the same DOF ledger used by `ScorecardV4` (`scorecard.rs:184-255`, `258-264`). |

The sealed-holdout protocol must be stricter than `alternating_holdout`. Define named blocks by survey and physics sector. Public train/validation splits can be ledgered; hidden kill blocks must be content-addressed and withheld from Python workers. A candidate that wins train and public validation but fails hidden blocks is not “promising”; it is a regression test.

## 7. Pipeline and acceptance tests

### 7.1 End-to-end pseudocode

```rust
fn run_dual_path(manifest: GpRunManifest) -> Result<DualPathRunReceipt> {
    let data = load_likelihood_data(manifest.input_hashes)?;
    let lcdm = profile_fit(ModelClass::lcdm(), &data)?;
    let residuals = whitened_residual_surface(&data, &lcdm)?;
    let splits = sealed_block_splits(&data, manifest.sealed_splits)?;

    let worker_inputs = build_physical_design_matrix(&residuals, &splits, manifest.allowed_targets)?;
    let raw_archive = run_untrusted_pysr_worker(&manifest.engine.primary, &worker_inputs)?;
    let challenger = run_untrusted_operon_worker_if_needed(&manifest.engine.challenger, &worker_inputs)?;

    let mut midpoint_archive = QdArchive::new();
    for raw in raw_archive.expressions {
        let expr = rust_parse_allowed_expr(raw.expr)?;
        let mid = emit_midpoint(expr, raw.target, &manifest)?;
        let card = score_phenomenology_card(&mid, &data, &splits, &challenger)?;
        midpoint_archive.insert(mid, card);
    }

    for (mid, card) in midpoint_archive.bounty_candidates() {
        open_explanation_bounty(mid, card)?;
    }
    Ok(write_receipt(midpoint_archive))
}
```

Promotion path:

```rust
fn try_promote_bounty(derivation: ProposalDoc, bounty: Bounty) -> ScorecardV4 {
    assert_eq!(derivation.midpoint_target_hash, bounty.midpoint_hash);
    let bound = bind_modified_background(&derivation.theory);
    assert_midpoint_matches_bound_response(&derivation.midpoint, &bound);
    score_proposal(&derivation, observables, covariance_blocks, baseline_ll)
}
```

### 7.2 Acceptance suite

**Synthetic Planck-MG recovery.** Generate 100 synthetic Tier-1-like datasets from `mu(a)=1+mu0*Omega_DE(a)/Omega_DE0` with `mu0=-0.12`, realistic covariance, and random noise seeds. Acceptance: in at least 80% of seeds, the bottom-up archive contains an expression equivalent to `c*Omega_DE(a)/Omega_DE0` or a one-multiply variant in the top-5 validation Pareto entries; no direct row expression survives hidden holdout. Then feed the bounty to the existing `planck_mu0_geff` lane. Acceptance: the derived proposal carries the generating term `planck_mu_parametrization`, passes binding consistency, and earns nonzero derivation rigor. It still earns data-fit only if evidence beats the same Occam bar.

**Synthetic dark-scattering recovery.** Generate growth data with `gamma_drag(a)=A*(1+w(a))*Omega_DE(a)`. Acceptance: GP recovers the factorization or a canonical equivalent; derivation requires `dark_scattering_growth_drag` certificate and cannot set `A` costlessly. This directly tests the V7 survivor class described in the paper (`paper/main.tex:561-573`).

**Synthetic f(R)/nDGP quasi-static matching.** Generate scale-dependent `mu(a,k)` response from `ModelClass::f_r()` and `ModelClass::ndgp()` (`league.rs:163-213`). Acceptance: the midpoint response fingerprint matches the derived family within tolerance on the grid; where current data lack scale leverage, the report must say “not identifiable” rather than falsely selecting the family.

**Cheater battery.** Fixtures include row-index formulas, redshift lookup tables, covariance eigenmode formulas, division singularities, and constants copied from observations. Acceptance: every fixture is killed before bounty opening, with a specific kill reason.

**Scoreboard quarantine.** Run a GP expression that has excellent validation fit but no derivation. Acceptance: it appears in `phenomenology-fit.jsonl` and dashboard QD cells, but not in `theory_population` champions and not in the whitebox league.

## What we got wrong

1. **“Symbolic discovery” is still overstated.** The repo now has symbolic containers and an honest judge, but the active generator surface is narrow: `ProposalSketch` exposes a small fixed MG/background schema, and population islands are role schedulers, not physical search spaces. Check: instrument structural entropy over generated `Term`s and midpoint functions across 100 generations. If term/function entropy is near zero while numeric parameters drift, the claim is false. Fix: implement midpoint GP plus top-down term emission.

2. **“Derived, not fit” is still vulnerable at the value level.** The scorecard prices post-search choices and background drift, which is excellent, but any new path that lets a coefficient enter as “fundamental” or “phenomenological” will recreate V5/V6. Check: replace real data with shuffled residuals; if the system still promotes a coefficient-bearing theory with derivation credit, the certificate layer is inadequate. Fix: every coefficient in a promoted midpoint form must be either value-certified or charged as a fitted physical parameter, and underived GP coefficients never enter `ScorecardV4`.

3. **The 23-observable space cannot identify mechanism families as strongly as the narrative wants.** It can detect coherent suppressed growth; it cannot, by itself, distinguish `mu`, `Sigma`, drag, `sigma8`, and background drift robustly. Check: compute a Fisher/Jacobian rank over `{h, Omega_m, sigma8, w0, wa, mu0, Sigma0, drag_a}` under registered covariance blocks, then report singular vectors. Fix: QD descriptors and bounty text must include degeneracy directions; promotion-grade claims for `Sigma` or scale dependence require lensing/full-shape/Boltzmann data (see S01/S04 as applicable).

4. **Six islands are not diversity.** Names like `foundations` and `wildcards` are useful orchestration labels, but diversity must be measured in physics behavior. Check: compare old island occupancy against the proposed QD descriptor occupancy under the same run budget. If champions all address RSD/S8 through one `mu0` basin, the scaffold is not preserving discovery diversity. Fix: MAP-Elites archive with observable-pull, mechanism, response-shape, screening, complexity, and degeneracy descriptors.

5. **A fitting formula is not a discovery, even when beautiful.** The temptation of S02 is to celebrate a GP expression that improves residuals. That would violate the project’s own hard-won standard. Check: audit every public report for the phrase “candidate theory” applied to a `PhenomenologicalQuarantined` form. Fix: underived forms are bounties and data products only; only certified `ProposalDoc`s can be whitebox candidates.

6. **Boltzmann-free GP is not promotion-grade for scale-dependent MG.** The current growth code is a valuable Tier-1 discriminator, but full CMB/lensing responses are exactly where `mu`/`Sigma` and early-time effects separate. Check: construct two synthetic theories with matched BAO, `fσ8`, `H0`, and `S8` but different lensing/CMB signatures; the current pipeline should be unable to distinguish them. Fix: keep bottom-up `Sigma` and early-time forms quarantined until a Boltzmann-backed response path exists (see S04/S05); do not let GP fill that gap with algebra.
