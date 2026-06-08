# Automated theory discovery — design briefing

> Research basis for the ZYAL symbolic-derivation rebuild (Milestones 1–4). Captures the
> state of the art in automated/AI physics-theory discovery and the *computable* consistency
> checks we will implement. Companion: [forward-model-and-unification.md](forward-model-and-unification.md).

## 1. Methods (and what makes results credible vs. degenerate)

- **BACON** (Langley, Simon, Bradshaw, Zytkow 1981–87). Heuristic search that notices
  constancies/trends and *invents theoretical terms and intrinsic properties*. Re-derived ideal
  gas, Kepler III, Coulomb, Ohm, Snell. Lesson: discovery = compression + concept formation, not
  fitting. (IJCAI-81 BACON.5.)
- **Schmidt & Lipson, "Distilling Free-Form Natural Laws"** (*Science* 324:81, 2009). GP symbolic
  regression recovering Hamiltonians/Lagrangians/conservation laws. **Non-triviality criterion:**
  reward expressions whose *partial-derivative relationships* match the data's, not ones that
  merely fit values — filters degenerate correlations.
- **AI-Feynman** (Udrescu & Tegmark, *Sci. Adv.* 6:eaay2631, 2020; arXiv:1905.11481; v2
  NeurIPS 2020 arXiv:2006.10782). Recursive divide-and-conquer: exploit **dimensional analysis,
  symmetry/invariance, separability, compositionality** to *constrain before fitting*. v2 adds
  graph-modularity + explicit **Pareto (accuracy vs description length)** selection. The
  constrain-before-fit move is the core "derived not fit" mechanism.
- **PySR / SymbolicRegression.jl** (Cranmer 2023, arXiv:2305.01582). Multi-population GP with an
  explicit **Pareto front** (error vs complexity); complexity must be *earned* by accuracy gain.
  2025 refinement: **structural-stability** criterion (arXiv:2509.21780) — penalize expressions
  whose form is unstable under data perturbation/refit (a sharper overfit veto than node count).
- **AI-Descartes** (Cornelio et al., *Nat. Commun.* 14:1777, 2023; arXiv:2109.01634). **Closest
  existing system to our goal.** Symbolic regression + a **logical-reasoning module backed by a
  theorem prover (KeYmaera X)** testing whether a formula is *derivable from background axioms*.
  Two metrics: ε(f) = distance from data; **β(f) = "derivation distance"** from what the axioms
  entail. Picks the *right* law from few points when candidates have near-identical ε but
  different β. Enforces dimensional consistency, monotonicity, limiting behavior. *Limits:*
  needs complete+consistent machine-readable axioms; proving is expensive/undecidable for some
  logics.
- **AI Hilbert** (Cory-Wright et al., *Nat. Commun.* 15:5922, 2024; arXiv:2308.09474). Derives
  equations as logical consequences of axioms via **polynomial optimization with
  Positivstellensatz certificates** (SDP/MIP). The certificate is a **machine-checkable proof**
  that the law follows from the axioms — the strongest automatable provenance object today.
  Works in the noisy/scarce-data regime where pure SR fails.
- **LLM-SR** (Shojaee et al., ICLR 2025 Oral, arXiv:2404.18400). LLM proposes equations as
  **program skeletons with placeholder params**; decouples discrete structure (LLM) from
  continuous fitting (BFGS/Adam); island-based evolutionary search. Beats classical SR
  **out-of-domain**. Authors document **LLM recitation/memorization** and built modified-physics
  benchmarks to defeat it. See **LLM-SRBench** (ICML 2025, arXiv:2504.10415) — tests discovery
  *beyond memorization*.
- **Scientific Generative Agent** (Ma et al., ICML 2024, arXiv:2405.09783). LLM + differentiable
  simulation as bilevel optimizers.

**Pitfall taxonomy + mitigations** (consistent across the field): spurious/degenerate
expressions → Pareto/MDL + structural stability; fit-not-derived → β-distance / certificates;
LLM recitation → modified-physics held-out benchmarks; dimensional nonsense → dimensional
pre-reduction; relationship mismatch → Schmidt–Lipson partial-derivative test.

## 2. Derivation / provenance verification (ordered by strength)

1. **Dimensional homogeneity** (Buckingham-π) — reduce to dimensionless groups before search;
   reject inhomogeneous expressions. Cheap, deterministic, eliminates most of the space.
2. **Symmetry/invariance** — require invariance under the relevant group (translation, rotation,
   Lorentz, gauge).
3. **Pareto front / MDL** — accuracy vs description length; strengthen with structural stability.
4. **Relationship (derivative) matching** — Schmidt–Lipson.
5. **Derivation distance β** vs axioms via a theorem prover (AI-Descartes/KeYmaera X).
6. **Formal certificates** — AI-Hilbert Positivstellensatz/SOS; machine-checkable.

## 3. What makes a theory credible to stern experts

- **Falsifiability** (Popper) as demarcation — must forbid something observable; prefer *riskier*
  corroborated predictions. (Note the live debate: Dawid's non-empirical confirmation vs. naive
  falsificationism.)
- **Out-of-sample / novel falsifiable prediction** — the single most respected signal.
- **Correspondence principle** — must reproduce the established theory in the appropriate limit
  (GR→Newton, QM→classical). Computable as a symbolic limit check.
- **Naturalness** ('t Hooft; Giudice arXiv:0801.2562) — O(1) dimensionless params unless
  protected by symmetry. Use as a *soft prior* (it's unquantified without a parameter prior).
- **Predictivity vs descriptivity** — credibility ∝ (predictions) ÷ (free parameters). A theory
  with as many parameters as data points is descriptive, not predictive.
- **Internal consistency** — unitarity, causality, stability (§4); non-negotiable.

## 4. Computable structural-consistency vetoes (the gold) — implement as hard gates

Necessary conditions; failing any one kills a candidate Lagrangian regardless of fit. Ordered by
cheapness. **Cheap tier (implement first):**

| Check | Computability on a symbolic Lagrangian |
|---|---|
| **Dimensional consistency** | Track mass dimension per symbol; every term equal, action dimensionless. Trivial symbolic check. |
| **Lorentz invariance** | Verify free-index count = 0; only invariant contractions (η, ε). Fully computable. |
| **Gauge invariance** | Restrict grammar to gauge-covariant building blocks (covariant derivatives / field strengths), or apply the transform symbolically and check δS=0. |
| **Kinetic-sign** | Wrong-sign kinetic term ⇒ ghost. Symbolic sign check. Cheap. |
| **Ostrogradsky / ghost** | Non-degenerate higher-than-first-derivative Lagrangian ⇒ Hamiltonian unbounded below. Detect derivative order >1; test degeneracy (Hessian wrt highest velocities singular). Non-degenerate ⇒ reject. (Woodard arXiv:1506.02210; arXiv:2007.01063.) |

**High-value second tier (soft scores / near-vetoes):**

| Check | Notes |
|---|---|
| **Tree-level unitarity** | Partial-wave \|a_ℓ\|≤1 bounds couplings; computable for simple operators. |
| **Positivity bounds** | Forward-scattering dispersion relations ⇒ leading s²-coefficient > 0 (Adams, Arkani-Hamed, Dubovsky, Nicolis, Rattazzi, *JHEP* 2006, hep-th/0602178). For a polynomial EFT these are explicit sign/linear inequalities. |
| **Swampland / WGC** | EFT-coupled-to-gravity inequalities (Vafa hep-th/0509212; AMNV 2007; Palti 2019). Conjectural → treat as soft prior. |
| **Energy conditions** | Compute T_μν, test null/weak/strong/dominant inequalities on representative backgrounds. |
| **Causality** | Principal symbol / characteristic cones from EOM; check hyperbolicity. Heavier but computable. |

## 5. LLM-as-physicist (2024–2026)

**Works:** LLM as **structure proposer** seeded with domain priors (LLM-SR) — shrinks the search,
generalizes OOD *when paired with a numeric/symbolic fitter and an external verifier*. The winning
architecture decouples proposal (LLM) / fitting (optimizer) / verification (prover or simulator).
Proof-assistant certification makes output trustworthy even under hallucination (arXiv:2510.12829).

**Fails / honesty risks:** recitation/memorization of textbook laws (defeat with structurally-novel
benchmarks, LLM-SRBench); **sycophantic false proofs** (BrokenMath; AI-Scientist evaluations
arXiv:2502.14297, 2504.08066); tool/output hallucination (arXiv:2510.22977); and a theoretical
result that hallucination is inevitable over computable functions (Xu et al. 2024) — **a text-only
supervisor cannot fully verify honesty.** ⇒ **Never let an LLM be the final judge.** Route every
LLM claim through a non-LLM oracle (symbolic gate, numeric simulation/fit, or certificate). Use
adversarial proposer/skeptic pairs but resolve disputes with the deterministic gates, not by vote.

## 6. Recommendations for the ZYAL engine

1. **Genome = symbolic Lagrangian/action** over a grammar of gauge/Lorentz-covariant building
   blocks, so many consistency properties are correct by construction. (Directly enforces the
   repo's whitebox-only hard gate.)
2. **Deterministic veto cascade as a pre-fitness gate**, cheapest-first (dimensional → Lorentz →
   gauge → kinetic-sign + Ostrogradsky → energy conditions). Prunes ≳90% of random genomes for
   near-zero cost; only survivors reach expensive stages.
3. **AI-Descartes dual fitness:** ε (fit/judge) *and* β (derivation/consistency). Reward β heavily
   — chase derivable theories, not better fits. Escalate β to a Positivstellensatz/SOS certificate
   where axioms allow.
4. **Multi-objective Pareto selection** over (judge-robustness, description length,
   consistency-violations) + a **structural-stability** penalty (refit under perturbed data).
5. **Second-tier physics gates as soft scores:** unitarity coupling bounds, positivity (leading
   coeff > 0), Swampland/WGC heuristic.
6. **LLM as proposer + critic, never judge.** Every LLM output clears a non-LLM oracle. Adversarial
   proposer/skeptic resolved by the symbolic gates.
7. **Anti-recitation:** version a modified-physics held-out benchmark; gate "discovery" on OOD
   predictive performance + novelty, plus perplexity/known-law fingerprinting.
8. **Correspondence-principle acceptance test:** every accepted theory must reproduce an
   established law in the appropriate symbolic limit.

**Build order:** (a) symbolic grammar + veto cascade; (b) ε/β dual fitness + Pareto+stability;
(c) LLM proposer with oracle verification + recitation guard; (d) certificates + positivity/
unitarity gates.

## Key citations

Udrescu & Tegmark, AI-Feynman, *Sci. Adv.* 6:eaay2631 (2020), arXiv:1905.11481; v2 arXiv:2006.10782 ·
Schmidt & Lipson, *Science* 324:81 (2009) · Langley et al., *Scientific Discovery* (MIT Press 1987) ·
Cranmer, PySR, arXiv:2305.01582; Cranmer et al. NeurIPS 2020 arXiv:2006.11287 ·
Cornelio et al., AI-Descartes, *Nat. Commun.* 14:1777 (2023), arXiv:2109.01634 ·
Cory-Wright et al., AI Hilbert, *Nat. Commun.* 15:5922 (2024), arXiv:2308.09474 ·
Shojaee et al., LLM-SR, ICLR 2025, arXiv:2404.18400; LLM-SRBench arXiv:2504.10415 ·
Ma et al., SGA, ICML 2024, arXiv:2405.09783 · Lu et al., The AI Scientist (2024), arXiv:2504.08066;
eval arXiv:2502.14297 · Adams et al., *JHEP* 2006, hep-th/0602178 · Vafa hep-th/0509212; AMNV (WGC)
2007; Snowmass EFT arXiv:2210.03199 · Woodard, Ostrogradsky, arXiv:1506.02210; arXiv:2007.01063 ·
Giudice, "Naturally Speaking," arXiv:0801.2562 · structural stability arXiv:2509.21780 ·
proof-assistant-certified LLM math arXiv:2510.12829.
