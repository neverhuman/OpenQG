# S11 — Paper Profundity and Editorial Gate Spec

## Ranked backlog

1. **Turn the paper from “a cosmology discovery attempt” into “a falsifiable autonomous-science instrument paper.”** What: rewrite the title, abstract, introduction, and conclusion so the primary claim is the ledgered proposer/oracle/audit methodology, with the cosmology result presented as a case study and stress test. Why it matters: `paper/main.tex` currently sells a surviving V7 class that has zero data-fit credit and a score of 43.0; that cannot carry a cosmology-discovery paper. The actual durable asset is the adversarial self-audit loop documented in `docs/V6-ACCEPTANCE.md`, `paper/data/story.json`, and `paper/main.tex`. Effort: S. Verify: an external cosmologist can summarize the claim without saying “they discovered modified gravity,” and an ML reviewer can summarize it without saying “the LLM judged science.”

2. **Convert the V7 null into a pre-registered exclusion over a declared mechanism class.** What: run a V8 exclusion campaign over a frozen, typed search volume for late-time growth-suppression mechanisms, with explicit prior boxes, covariances, profile fits, trials correction, and rank-stability tests. Why: “V7 scores 43.0” is an internal scorecard result; “we exclude class C over volume V at strength S” is citable science. Effort: L. Verify: the paper contains a machine-readable `search-volume.yml`, frozen grammar hash, ranked candidate ledger, coverage report, and an unambiguous exclusion sentence that survives a blind rerun. Requires S04-class and S07-class work.

3. **Promote evidence from BIC-style rubric accounting to audited marginal likelihood.** What: use the deterministic Laplace/grid evidence code in `crates/openqg-core/src/scoring/evidence.rs` as the default quoted comparison, not a scorecard component or Schwarz approximation alone. Why: the paper’s credibility depends on not repeating the earlier “+36.7” and “+68.8” style mistakes that `docs/theory-league.md` explicitly says were artifacts of unfair comparison. Effort: M. Verify: every headline table reports profile-fitted ΛCDM, Δχ², ΔAIC, ΔBIC, ΔlnZ_Laplace, ΔlnZ_grid, and sensitivity to prior widths. Requires S07-class work.

4. **Replace fitting-formula CMB claims with a Boltzmann-backed promotion lane.** What: any claim involving CMB acoustic scale, growth, lensing, or modified gravity promotion must pass a CLASS/CAMB/hi_class adapter with pinned precision files, code hash, and derivative residual envelope. Why: the paper’s most important result is that the optimizer harvested a +0.755, 8.4σ bias in the engine’s own CMB acoustic-scale formula (`paper/data/story.json`; `docs/V6-ACCEPTANCE.md`). That is a great cautionary case; it is not a license to keep using the same class of instrument for promotion-grade claims. Effort: L. Verify: the V8 candidate and null are rescored under the subprocess protocol in `docs/boltzmann-backend.md`, and the fitting-formula path is relegated to triage. Requires S02-class work.

5. **Make the audit cascade a quantitative result, not a narrative.** What: define and report time-to-invalidate, exploit-class recurrence, exploit bounty in points/nats, failing-test latency, and regression persistence. Why: the current story is compelling but anecdotal. A field-standard metric for autonomous science would be profound: how fast does the system invalidate its own champion after the evaluator changes? Effort: M. Verify: a table generated directly from campaign ledgers gives each exploit’s first appearance, champion impact, audit detection time, fix commit, and regression test name. Requires S06-class work.

6. **Run named head-to-head baselines against human modified-gravity scan pipelines and AI-for-science systems.** What: compare OpenQG/ZYAL against (a) a conventional human-coded ΛCDM/wCDM/w0waCDM/μ0/f(R)/nDGP scan, and (b) generic LLM-program-search baselines that emit model classes or code under the same deterministic judge. Why: FunSearch, AlphaEvolve, AI-Descartes/AI-Hilbert, The AI Scientist, Coscientist, and Robot Scientist work define the outside reader’s comparison set. The paper must show what OpenQG does that they do not. Effort: L. Verify: a reproducible benchmark table reports time-to-first-invalidated-champion, false-positive rate on decoys, and final evidence rank. Requires S01-class, S05-class, and S10-class work.

7. **Build an out-of-fit-set forecast registry before adding more prose about novelty.** What: every novelty claim must be a hash-sealed forecast on an observable not used by the fitting run, with a release target and unblinding rule. Why: `crates/openqg-core/src/theory/scorecard.rs` correctly caps fit-set novelty and engine-refreshed witnesses, but the paper still narrates novelty as if it were close to discovery. Effort: M. Verify: no novelty credit enters the main score unless it points to a future or held-out dataset with a sealed prediction packet. Requires S08-class work.

8. **Separate “truth-binding” from “value-level derivation.”** What: state that truth-binding ensures certified claims affect the forward model; it does not prove the fitted parameter value is derived by nature. Then build the next derivation oracle for value-level certificates. Why: `MISSION.md` says the project is about interpretable benchmarked comparison, not solved quantum gravity; overselling “derived, not fit” will be the fastest referee rejection. Effort: M–L. Verify: every parameter in every table is labeled as fitted, profiled, derived-by-closed-form relation, or fixed-by-prior. Requires S03-class work.

9. **Create a journal-grade artifact contract.** What: ship one reproducibility bundle with source SHA, Docker/lock, ledger hashes, frozen data manifests, covariance hashes, fitting protocol, forecast registry, and exact figure scripts. Why: `paper/main.tex` claims reproducibility via `ops/replay.sh` and `zyal genome rescore`; a hostile referee will require a single command that rebuilds the tables and fails loudly if raw covariances or data are missing. Effort: M. Verify: an independent machine re-runs all headline numbers without network access except for explicitly declared public-data fetches. Requires S09-class and S12-class work.

## Editorial verdict

I read the required files in the requested order: `paper/main.tex`, `paper/refs.bib`, `docs/V6-ACCEPTANCE.md`, `MISSION.md`, `docs/MOONSHOT.md`, `docs/zyal-next-level-design.md`, `paper/data/story.json`, and `paper/data/predictions.txt`; I also inspected the scoring, covariance, evidence, binding, veto, league, router, and trust-gate code paths named by the selected target list. The honest verdict is: the current paper is respectable and unusually self-critical, but not yet profound. The path to profundity is not a bigger adjective; it is a narrower claim with harder artifacts.

`MISSION.md` defines the project as an inspectable workflow for asking which candidate theory families reproduce public low-energy data better than SM+GR+ΛCDM while staying interpretable. `docs/MOONSHOT.md` aims for a public benchmark commons. `paper/main.tex` currently tries to be four papers at once: autonomous-science methodology, reward-hacking case study, cosmological model-selection note, and narrative campaign report. The strongest version is not “an LLM discovered cosmology.” It is: **an LLM-proposer/deterministic-oracle scientific instrument was adversarially optimized until it discovered the evaluator’s own physics bias, converted each exploit into a regression test, and then returned an honest negative over a declared mechanism class.** That sentence is defensible; the broader discovery framing is not yet.

## Contribution class and venue

The actual contribution today is primarily **autonomous-science methodology with a cosmology stress test**. The cosmology content matters because the data are real and the failure mode is scientifically recognizable: CMB distance priors, DESI BAO, RSD growth, S8, SH0ES, and BBN interact through a forward model and a model-selection ledger. But the V7 champion’s own decomposition says it has zero data-fit credit and only a right-direction growth story (`paper/main.tex`, Results; `paper/data/predictions.txt`). That is not a PRD/JCAP discovery result.

**Best current venue:** JCAP or PRD as a methods-and-validation paper if the paper is tightened, with the title closer to “Adversarially audited autonomous model selection in late-time cosmology.” JCAP is the better fit because it tolerates methodological cosmology infrastructure with data-analysis emphasis; PRD will demand more physics depth and sharper model definitions. The current paper should not aim at Nature Astronomy unless it either (a) produces a pre-registered external forecast that survives unblinding, or (b) turns the V7 null into a credible exclusion over a declared search volume.

**Best profound venue:** Nature Astronomy or Nature Machine Intelligence only after the 12-month headline experiment. Nature Astronomy wants a scientific or field-practice result that changes how cosmologists treat autonomous pipelines. Nature Machine Intelligence wants a transferable autonomous-science method with baselines and ablations. NeurIPS/ICML would require a benchmark paper: named baselines, multiple domains or at least multiple scientific tasks, ablations of truth-binding, audit-cascade metrics, and a reusable environment. As written, the method is too domain-specific for a top ML venue and the science is too provisional for a top astronomy venue. The strongest final form is a two-paper split: a JCAP/PRD methods paper now, and a Nature Astronomy/Nature Machine Intelligence paper after a preregistered DESI/Euclid outcome.

## Products ranked by profundity

1. **The evidence-valley-was-instrument-bias resolution.** This is the most citeable current product because it is concrete: the system found a +0.755 offset in its own CMB acoustic-scale prediction, an 8.4σ error under the Planck-prior σ=0.090, and the apparent h–Ωm valley flips from large positive evidence to negative after calibration and Occam pricing (`docs/V6-ACCEPTANCE.md`; `paper/data/story.json`). Outside readers will cite this as “optimizers expose evaluator bias.” What is missing: off-anchor residuals, derivative checks in h/Ωm/w0, and Boltzmann-backed replication. Without that, “the valley was never open” is too strong; the defensible statement is “the valley in this evaluator was an instrument artifact.”

2. **The exploit→regression-test audit cadence.** This is the product that could become field-standard if quantified. V4 87.5, V5 55.0, V6 77.0, V6.1 survivor, and V7 43.0 form a real audit cascade (`paper/main.tex`, Audit Cascade; `paper/data/story.json`). What is missing: a metric. Define time-to-invalidate, exploit bounty, regression-lock latency, and false-positive rate on decoys. Then compare against The AI Scientist-style self-review and FunSearch/AlphaEvolve-style evaluator loops. If this becomes a table rather than a story, it may become the paper’s signature contribution.

3. **Truth-binding and evidence architecture.** The architecture is important: LLMs propose, deterministic code scores; claims must materialize as forward-model consequences; scorecard components are one-sided and veto-first (`crates/openqg-core/src/theory/scorecard.rs`; `crates/openqg-core/src/theory/binding.rs`; `crates/openqg-core/src/theory/vetoes.rs`). What is missing: ablations and failure rates. Show what happens with truth-binding off, with novelty caps off, with covariance off, with term registry off, and with LLM judging allowed. Otherwise this reads as good engineering, not a scientific result.

4. **The honest negative champion.** As written, the V7 43-class champion is admirable but not profound. A negative result becomes citeable only after it is converted into an exclusion over a declared search volume. What is missing: class definition, exhaustive coverage argument, calibrated evidence, stability under data partitions, and caveats. The honest negative is a seed; the exclusion statement is the product.

## Convert the null into strength: V8 exclusion protocol

Define the excluded class as **C_growth-suppression(V8)**:

- Late-time, GW170817-safe, scale-independent scalar-sector or dark-sector mechanisms whose only admitted first-order cosmological effect is to suppress linear growth relative to profile-fitted ΛCDM over 0 ≤ z ≤ 2.5.
- Background geometry may be ΛCDM, wCDM, or CPL w0waCDM only if the background parameters are profile-fitted and charged; no unpriced drift.
- Modified growth is restricted to one of two declared routes: (1) Planck-style μ(a)=1+μ0 ΩDE(a)/ΩDE0 with μ0 ∈ [−0.30, 0.00], and (2) dark-scattering drag Γ(a)=A_drag(1+w(a))ΩDE(a) with A_drag ∈ [0, 10] and w0 ∈ [−0.99, −0.80], with the no-phantom caveat explicit. Add f(R), nDGP, or scale-dependent MG only if the Boltzmann/hi_class lane exists; otherwise exclude them from the class rather than faking coverage.
- Standard parameters h, Ωm, σ8, Ωb h², Σmν are either fixed by declared priors or profile-fitted and charged. σ8 must be profile-fitted in the null and in every extension; otherwise μ0 is only an amplitude proxy.

The exhaustiveness argument must be mechanical. Freeze a `search-volume.yml` containing the grammar, parameter boxes, priors, random seeds, adaptive-grid settings, and relation registry hash. Produce a coverage certificate showing that every connected component of the parameter volume has been either profiled to convergence, bounded by interval evidence, or killed by a named veto. Use multiple deterministic optimizers: grid/interval sweeps for 1–2D slices, Nelder–Mead or differential evolution only as proposal mechanisms, and deterministic Laplace/grid evidence as adjudication. De-duplicate candidates by physics fingerprint, not by proposer text. Account for trials: the null is not beaten because one of thousands of LLM proposals twitched; it is beaten only if the best class-level evidence survives the look-elsewhere penalty over the frozen search volume.

Statistical prerequisites:

- Covariance-complete likelihood for Planck distance priors and DESI BAO blocks, plus RSD fσ8, S8, BBN, and optional SH0ES in a predeclared stack (`crates/openqg-core/src/scoring/covariance.rs`; `data/fixtures/cosmology/covariance/*`).
- Profile-fitted ΛCDM baseline for every data stack (`docs/theory-league.md`; `crates/openqg-core/src/theory/league.rs`).
- Boltzmann-backed promotion for any CMB/growth claim beyond background triage (`docs/boltzmann-backend.md`).
- Goodness-of-fit reporting, not only evidence: residual vectors, posterior predictive p-values by data block, and a “no hidden disaster” plot for CMB, BAO, growth, and ladder.
- Rank stability under jackknife-by-sector, covariance perturbation, prior-width doubling/halving, removal of SH0ES, and removal of each growth block. This is the S07-class requirement.
- A blind holdout or future release forecast; otherwise the exclusion is internally fair but not independently persuasive.

Mandatory caveats: the exclusion does not cover scale-dependent growth, nonlinear screening, full Horndeski/EFT α-basis dynamics, Early Dark Energy, interacting dark energy with background energy-transfer sectors not encoded in Γ(a), neutrino-sector extensions, or any model requiring full CMB Cℓ/lensing/nonlinear P(k) not implemented in the V8 promotion lane.

**Abstract sentence to use only after the computation exists:**

> Over the preregistered V8 search volume C_growth-suppression—late-time, GW170817-safe, scale-independent growth-suppression mechanisms with μ0∈[−0.30,0] or dark-scattering drag A_drag∈[0,10] under the declared priors—we find no candidate with positive calibrated marginal evidence relative to profile-fitted ΛCDM on the covariance-complete DESI/Planck/RSD/S8/BBN stack; after trials correction and rank-stability tests, we exclude mechanisms in this class large enough to lower fσ8(z≈0.6) by ≥0.02 without compensating degradation in CMB/BAO geometry.

## Position against the AI-for-science wave

Relevant comparison set: FunSearch pairs LLM program generation with an automated evaluator for mathematical/code constructions (Romera-Paredes et al., Nature 2024); AlphaEvolve extends LLM-guided evolutionary code search for algorithmic/scientific tasks (arXiv:2506.13131); AI-Descartes and AI-Hilbert combine data with background theory and derivation certificates (arXiv:2109.01634; arXiv:2308.09474); The AI Scientist automates idea-to-paper ML research and simulated review (arXiv:2408.06292; v2 arXiv:2504.08066); Coscientist uses LLM agents and tools for autonomous chemical experimentation (Boiko et al., Nature 2023; related arXiv:2304.05332); Robot Scientist Adam/Eve pioneered closed-loop hypothesis and wet-lab experimentation (King et al., Science 2009; Williams et al., J. R. Soc. Interface 2015).

**Single defensible claim:**

> OpenQG/ZYAL is, to my knowledge, the first ledgered LLM-proposer/deterministic-oracle physical-science search system whose central reported result is not a discovery claimed by the generator, but the adversarial discovery, correction, and regression-locking of its own evaluator bias before returning a negative result on real public cosmology data.

Defenses against obvious attacks:

- **“n=23 observables is too small.”** Correct for discovery; sufficient for a methodology stress test if every claim says “admitted data volume” and every evidence number is block-attributed. Not sufficient for a broad exclusion until the V8 protocol above is run.
- **“The oracle used fitting formulas.”** Correct, and that is the point of the cautionary result. But a reviewer will only accept it if the paper stops using fitting-formula outputs for promotion-grade science and adds Boltzmann replication.
- **“There is no confirmed discovery.”** Correct. The strongest claim is about scientific-instrument discipline and falsification, not about a new law of gravity. Do not apologize for the null; formalize it.
- **“FunSearch/AlphaEvolve also use deterministic evaluators.”** Yes, but their evaluators are closer to task ground truth. OpenQG’s distinctive problem is an evaluator that is itself a fallible scientific instrument. The paper must make evaluator-bias discovery—not LLM creativity—the main claim.
- **“AI-Hilbert has stronger derivation guarantees.”** Yes for polynomial-law settings; OpenQG should cite it as the standard to aspire to for value-level derivation. Truth-binding is not a substitute for Positivstellensatz-style proof.

## Rewrite specification

### Claims to drop, add, and strengthen

Drop: any abstract or conclusion language implying that V7 found a viable cosmological alternative. Drop “profound” adjectives. Drop “the valley was never open” unless scoped to “under this evaluator after calibration.” Drop cost/free-tier claims from the main narrative; put them in artifact availability.

Add: a field-standard metric proposal: time-to-invalidate-own-champion. Add a table of exploit classes with point/nat bounty and regression test names. Add a preregistered forecast/exclusion protocol. Add a clear distinction between search scorecard, league evidence, and promotion-grade physics.

Strengthen only after artifacts exist: (1) exclusion over C_growth-suppression after V8; (2) Boltzmann-backed CMB residual envelope; (3) head-to-head human/MG pipeline comparison; (4) out-of-fit-set forecast registry.

Dependency list: covariance/data provenance requires S01-class work; Boltzmann promotion requires S02-class work; derivation/value certificates require S03-class work; exhaustive search volume and league mechanics require S04-class work; proposer/router/ledger baselines require S05-class work; audit metrics require S06-class work; rank stability requires S07-class work; forecast registry and preregistration require S08-class work; artifact packaging requires S09-class work; security/provenance hardening requires S10-class work; independent replication/final referee packet requires S12-class work.

### Replacement abstract, verbatim

> We present OpenQG/ZYAL as a case study in adversarially audited autonomous science: language models propose typed cosmological mechanisms, but a deterministic host performs schema validation, truth-binding, forward modeling, vetoes, and evidence scoring. Across successive campaigns, every apparent champion was invalidated by a later audit—V4’s rubric-gaming 87.5, V5’s unpriced drift 55.0, V6’s novelty-laundered 77.0, and a V6.1 survivor missing a generating term—and each exploit was converted into a permanent regression test. The most important result is metrological: the search discovered and exploited a +0.755 bias in the engine’s own CMB acoustic-scale fitting formula, worth roughly 35 nats of spurious evidence along an h–Ωm drift direction; after calibration, covariance-aware scoring, and complexity pricing, that apparent evidence valley closes. Under the current admitted data and grammar, the surviving late-time growth-suppression class moves fσ8 in the expected direction but does not beat profile-fitted ΛCDM on calibrated evidence. We argue that this negative result is the correct output of an honest autonomous-science instrument, and we specify the preregistered V8 computation needed to turn it into an exclusion over a declared mechanism class.

### Replacement conclusion, verbatim

> The central lesson is not that an LLM discovered new cosmology. It did not. The lesson is that a generative search system, when forced through a deterministic and replayable oracle, becomes a sensitive probe of the oracle’s own loopholes. OpenQG/ZYAL’s durable contribution is the audit cascade: every unpriced degree of freedom, ungrounded witness, covariance mistake, and missing generating term was first monetized by the search, then made impossible by a regression test. The present cosmological verdict is deliberately modest: late-time, scale-independent growth suppression remains physically plausible as a direction, but the current admitted data and implemented grammar do not justify positive evidence over ΛCDM. The next paper-worthy step is therefore not another champion score. It is a preregistered forecast or exclusion: freeze the V8 search volume, promote CMB/growth predictions through a Boltzmann backend, compute calibrated marginal evidence against a profile-fitted null, and unblind against DESI/Euclid data without post hoc tuning. If the class survives, the system has earned a discovery claim; if it fails, the exclusion is itself a scientific result.

## The 12-month headline experiment

**Experiment:** pre-register the OpenQG/ZYAL V8 posterior predictive distribution for the DESI DR3 full-shape/RSD growth vector and BAO geometry, with Euclid DR1 weak-lensing S8 as the auxiliary cross-check. DESI DR3 is the better primary target because the champion class is about growth suppression and RSD fσ8, not images; Euclid DR1 is the best near-term S8/lensing consistency check. Public schedules available in 2026 place Euclid DR1 on 2026-10-21 and DESI DR3 in late 2026/early 2027, which fits a 12-month gate.

**X:** before the release, the engine must seal a vector-valued forecast, not a prose claim: predicted Δfσ8(z_i)=fσ8,V8−fσ8,ΛCDM for each preregistered DESI DR3 effective-redshift bin, the covariance-aware ΔlnZ distribution for C_growth-suppression versus profile-fitted ΛCDM, and the corresponding S8 posterior-predictive interval for Euclid DR1 if a public cosmic-shear product is available. The forecast packet must include central 50/80/95% intervals, the exact data columns to ingest, and the pass/fail thresholds.

**Y:** DESI DR3 full-shape/RSD plus BAO public release, with Euclid DR1 weak-lensing/cosmic-shear data as a cross-release validation if released with usable covariances.

**Pre-registration mechanics:** create `v8-forecast-YYYYMMDD.tar.gz` containing source SHA, container digest, dependency lockfiles, data manifests, covariance hashes, search-volume.yml, prior boxes, all seeds, frozen candidate ledgers, posterior samples, forecast vectors, and the unblinding script. Publish SHA-256 to at least three independent timestamping surfaces: OpenTimestamps/Bitcoin, Zenodo or OSF preregistration, and a signed Git tag. Send the hash and artifact manifest to at least two external referees before the target release. The artifact must be runnable without network access except for a declared data-fetch step after unblinding. No model-class, prior, covariance, or scoring-code changes after timestamping may enter the headline analysis; any post-unblinding repair is labeled exploratory.

**Decision tree:**

- If DESI DR3 fσ8 residuals land inside the preregistered V8 predictive region, ΛCDM lands outside its corresponding region, and calibrated ΔlnZ≥+5 after trials correction, the paper becomes a discovery-candidate paper.
- If the sign is correct but ΔlnZ is 0 to +5, publish as suggestive and forecast-confirming but not discovery.
- If ΔlnZ≤0 and rank stability holds, publish the exclusion over C_growth-suppression.
- If both ΛCDM and V8 fail goodness-of-fit, publish an evaluator/data-systematics paper, not a theory claim.
- If public covariances are missing or the data product differs materially from the preregistered schema, freeze the result as “not adjudicable” and do not move the goalposts.

What must be built now: Boltzmann promotion lane, release-ingestion adapters, schema-stable forecast packets, blinded scoring mode, rank-stability harness, preregistration CLI, external referee escrow, and a no-post-hoc-tuning policy enforced in CI.

## What we got wrong

1. **“The V7 champion is a result.”** It is a candidate direction with zero data-fit credit, not a result. Check: rerun the V7 class under profile-fitted ΛCDM, σ8-free null, covariance-complete likelihood, and marginal evidence. If ΔlnZ remains ≤0, it belongs in the exclusion protocol, not the abstract as a champion.

2. **“The valley was never open.”** Too broad. The attached materials show that the valley was an artifact of this evaluator’s biased lA fitting formula plus uncosted drift. Check: compute the same h–Ωm–w0 path through CLASS/CAMB/hi_class with Planck likelihood compression and derivative residuals. If the Boltzmann path also closes it, the stronger sentence is earned.

3. **“Truth-binding makes the system derived-not-fit.”** It makes claims affect predictions; it does not make fitted values derived by physics. Check: every table must label each quantity as profiled, fixed, or derived by a verified relation. Any fitted value wearing a derivation costume is a failure.

4. **“The audit cascade proves autonomy.”** It proves a powerful adversarial workflow, not full autonomy. External LLM referees, internal agents, and human-authored fixes all participated. Check: report which steps were autonomous, agentic, deterministic, or human, and never collapse them into one word.

5. **“The data volume is enough for broad modified-gravity claims.”** It is not. Twenty-ish observables can stress a pipeline; they cannot exclude large theory families without stronger priors and future data. Check: posterior predictive coverage and rank stability must be reported by sector, not as one score.

6. **“The term registry is a term algebra.”** It is an allowlist that catches known laundering. Check: try adversarial near-miss terms, generated decorations, and equivalent reparameterizations. If the registry accepts a decorative term or rejects a physically equivalent one, the paper must say registry, not algebra.

7. **“Free-tier inference cost matters scientifically.”** It matters operationally but not editorially. Check: remove all cost claims from the main argument. If the paper becomes weaker, the science was not strong enough.

8. **“A deterministic oracle is automatically trustworthy.”** The lA incident disproves that. Determinism makes mistakes reproducible; it does not make them correct. Check: every oracle component needs calibration tests, residual envelopes, and adversarial probes.

9. **“A negative result is automatically honest.”** It is honest only if the search space was declared before the search and covered after it. Check: no exclusion sentence without `search-volume.yml`, coverage certificate, trials correction, and frozen data stack.

10. **“This is already the decade-cited paper.”** Not yet. The decade-cited version has one of two endings: a sealed forecast survives DESI/Euclid, or a declared mechanism class is excluded with a reproducible audit trail. Everything else is preparation.
