# S01 — V8 Derivation Sandbox Specification

Review batch: tab 1 of 1. Source reviewed from `source.tar.gz` extracted at archive root. This spec is scoped to source changes and durable policy owned by Rust; TypeScript surfaces may render traces and receipts but must not own verification policy.

## Ranked backlog

| Rank | Change | Why it matters | Effort | Hostile-reviewer acceptance test |
|---:|---|---|---|---|
| 1 | Replace value-only `DerivedCertificate` with a machine-checkable `DerivationTrace`, keeping the nine closed-form relations only as migrated fixtures. | `crates/openqg-core/src/theory/certificate.rs` currently recomputes one named formula from numeric inputs. That catches arithmetic lies but not whether the formula, assumptions, or inputs are derived. V8 must make derivation the primary artifact. | L | Each of the nine current registry relations verifies from axioms/assumptions to target in a trace; a trace that skips the defining equation for `beta` or inserts a posterior median as an assumption is rejected before scoring. |
| 2 | Add a Rust `DerivationSandbox` run contract and isolated checker worker with content-addressed receipts. | The deterministic host must remain the judge. CAS/proof tools are useful but too complex to live inside the trusted scoring path without a stable policy boundary. | L | A malicious trace cannot read the network, host files, prompts, browser profiles, receipts, or runtime state; the same trace, registry digest, checker image digest, and seed produce byte-identical verdict receipts on two machines. |
| 3 | Make derivation rigor computed from verified trace metrics, not assigned by relation name. | `relation_rigor_weight()` assigns 0.0/0.3/0.8/1.0 by hand; `scorecard.rs` averages the best verified obligation weight. That is an honest stopgap but invites registry politics and relation laundering. | M | Splitting one algebraic rewrite into 50 no-op rewrites does not raise rigor; changing a mechanistic axiom into a phenomenological parametrization lowers or caps rigor; a literature-only trace earns no machine rigor. |
| 4 | Install a dimensional type system for every declared quantity, expression node, relation input, and term. | `obligation.rs::dimensional_consistency` trusts each `Term.mass_dimension` integer, and `vetoes.rs` vetoes only asserted bad dimensions. A proposer can assert a plausible dimension for an impossible expression. | M | A forged term with `mass_dimension = 4` but expression dimension 5 is killed; `exp(H0)` is killed; `H0 = 100 h` is accepted only with the explicit `km s^-1 Mpc^-1` conversion convention. |
| 5 | Implement a checker stack: typed expression normalizer, CAS proof-path checker, e-graph rewrite checker, interval/ball numerics, deterministic Schwartz-Zippel tests, and optional Lean 4 checking. | No single tool is the trust root for mathematical physics. The stack should attribute failure to a specific step and use stronger tools for stronger claims. | L | Mutating one operator, unit, domain assumption, or approximation bound in every accepted trace yields `StepFails` or `UnjustifiedApproximation` at the first affected step. |
| 6 | Enforce an anti-laundering input provenance rule. | The V6/V7 history shows the central exploit: fit a number, dress it as derived, and gain parsimony/rigor. Current `post_search_choice_dof()` prices many inputs, but it does not make the derivation/input boundary first-class. | M | A theory that sets `mu0 = -0.1` because a DESI/RSD posterior median says so is either a fitted degree of freedom with zero derivation credit or a hard kill if labeled derived. |
| 7 | Migrate obligations and binding to trace digests. | `binding.rs` truth-binds verified certificates into `CosmologyParams`, but it can only see relation names and numeric inputs. V8 binding must bind the exact verified target and input provenance. | M | Changing a certificate used by `obligation.rs` but not by the bound parameter changes the trace digest and triggers the existing cert/binding incoherence kill, now at trace level. |
| 8 | Add proof-assistant lane for exact algebra, units, and selected limits; do not require Lean for all physics. | Lean 4 + mathlib is strong enough for field/ring arithmetic and unit lemmas, but not yet a turnkey formalization of Horndeski/nDGP cosmological perturbation theory. Overpromising here would recreate the current “LeanSketch” stub problem. | L/XL | The sandbox accepts a Lean-checked proof of `1 + 1/(3*beta)` algebra under `beta != 0`; full f(R) quasi-static screening can pass by CAS/interval approximation only and is capped unless its approximations are bounded. |
| 9 | Add derivation-by-construction and checker mutation testing. | Free-form traces are attack surfaces. A builder API that only composes verified blocks gives soundness by construction, and mutation testing tests the checker itself rather than only proposals. | M | At least 80% of generated traces for common relation families come from the builder, and the nightly mutant suite rejects 100% of safety mutants over the migrated registry corpus. |
| 10 | Add adversarial cross-examination as a deterministic auxiliary, not a judge. | A second model is valuable for finding missing assumptions, but the project’s principle is that LLMs propose/critique and never judge. | S/M | A skeptic-generated counterexample becomes a deterministic interval/SZ/typing challenge; final verdict is identical with the LLM disabled if the challenge is replayed from the receipt. |

## 1. Current gap to close

The present system is much better than an honor-code proposer. `certificate.rs` defines `DerivedCertificate { relation, inputs, expected, tolerance }` and verifies that a hard-coded relation recomputes `expected` within tolerance. The registry currently covers nine relations: nDGP effective coupling, nDGP beta from background and crossover scale, Planck `mu0`, dark-scattering drag, f(R) large-scale effective coupling, f(R) `alpha_M`, coupled-dark-energy effective coupling, `H0 = 100 h`, and flat-universe closure. `binding.rs` then truth-binds verified modified-gravity relations into the FLRW/growth forward model, specifically to close the V5 hole where a certified modified-gravity relation did not affect predictions. `scorecard.rs` is veto-first and one-sided: a tie with LCDM earns zero data-fit credit, and hidden knobs are charged.

The remaining defect is narrower and deeper: the oracle verifies arithmetic, not derivation. A proposer can cite a registry relation whose mathematical origin is not checked; the registry itself is trusted code; dimensions are mostly asserted; proof-assistant obligations are recorded but unsupported; and rigor is assigned by `relation_rigor_weight()` rather than computed from an inspected proof object. The internal critique in `docs/zyal-next-level-design.md` already says “derivation” must mean value from mechanism, not label from text. The paper’s limitations section admits the term grammar is an allowlist rather than term algebra and that promotions need stronger physics backends. S01’s job is therefore to create the derivation sandbox that makes a candidate relation auditable enough that a mathematician can say what was proved, what was approximated, and what was merely assumed.

## 2. Derivation trace format

V8 proposals must submit a `DerivationTrace` for any parameter or obligation that seeks derivation rigor. Legacy closed-form certificates may remain for replay and migration, but after V8 cutover they earn zero or compatibility-capped rigor unless backed by a trace.

### 2.1 Data model

Use a restricted, typed expression language, not raw LaTeX or Python. JSON is the interchange format; Rust deserializes with `serde`, validates with a schema digest, and computes a canonical DAG hash before any checker runs.

```json
{
  "schema": "openqg.derivation_trace.v1",
  "trace_id": "sha256:...",
  "target": {
    "relation": "ndgp_geff_over_g",
    "lhs": {"sym": "G_eff_over_G"},
    "rhs": {"add": [1, {"div": [1, {"mul": [3, {"sym":"beta"}]}]}]},
    "domain": [{"neq": [{"sym":"beta"}, 0]}, {"gt": [{"sym":"beta"}, 0]}]
  },
  "declarations": [
    {"symbol":"beta", "type":{"dimension":"dimensionless", "domain":"Real"}, "provenance":"previous_trace_output|model_axiom|proposal_choice"}
  ],
  "axioms": [
    {"id":"ndgp.brane_bending.linear_response.v1", "kind":"model_postulate", "statement":"...typed expression...", "source":"trace_library"}
  ],
  "assumptions": [
    {"id":"quasi_static_subhorizon", "strength":"controlled_approximation", "conditions":["k/(aH) >= 30"], "error_bound":{"rel":"1e-3"}}
  ],
  "steps": [
    {"id":"s1", "from":"axiom:ndgp.brane_bending.linear_response.v1", "to":"expr:...", "kind":"substitute", "justification":{"substitution":{"Pi":"..."}}},
    {"id":"s2", "from":"s1", "to":"target.rhs", "kind":"algebraic_rewrite", "justification":{"rule":"field_simp", "checker":"lean|cas|egg", "requires":["beta != 0"]}}
  ],
  "approximation_budget": {"absolute":0.0, "relative":0.0},
  "expected_output": {"value": 1.1666666666666667, "unit":"dimensionless", "tolerance":"1e-12"}
}
```

Expression nodes are limited to `const`, `sym`, `add`, `mul`, `neg`, `div`, `pow` with rational exponent, `sqrt`, `exp`, `log`, `diff`, `limit`, `piecewise`, `integral` only when a checker capability is declared, and named special functions only from a registry. Every decimal must be parsed into an exact rational plus an uncertainty interval; no binary `f64` is admitted into the checker core.

Step kinds:

* `declare`: introduces a typed symbol, unit, domain, and admissible provenance.
* `axiom_use`: imports a named axiom or theorem from the trace library by digest.
* `substitute`: replaces symbols with previously verified expressions.
* `algebraic_rewrite`: proves two expressions equal under domain assumptions.
* `differentiate` / `integrate`: performs calculus with an explicit variable and regularity assumptions.
* `limit`: proves a limit or recovers GR/LCDM under a declared parameter limit.
* `series_bound`: uses a truncated expansion with an explicit remainder bound.
* `dimensional_check`: checks expression/unit compatibility.
* `interval_bound`: proves an inequality or numeric enclosure using ball/interval arithmetic.
* `schwartz_zippel_identity`: probabilistic identity challenge over a finite field, recorded as a backstop, not as full proof.
* `literature_link`: cites a paper or registry source; useful for audit, not sufficient for machine rigor.
* `bind_target`: maps the final expression into a `Parameter` or `CosmologyParams` field.

Assumptions carry strength, not prose. Suggested enum: `definition`, `mathematical_theorem`, `model_postulate`, `domain_restriction`, `controlled_approximation`, `phenomenological_parametrization`, `external_measurement`, `proposal_choice`, `scored_data_estimate`. The last one is not a derivation input; if it reaches a `bind_target` that claims derived status, the verdict is a hard laundering failure.

### 2.2 Rust interface

Add a new module, `crates/openqg-core/src/theory/derivation.rs`, and re-export it from `theory/mod.rs`. Rust owns schemas, policy, receipts, scoring metrics, and run contracts.

```rust
pub trait DerivationSandbox: Send + Sync {
    fn capabilities(&self) -> SandboxCapabilities;
    fn verify_trace(&self, request: DerivationRequest) -> DerivationVerdict;
}

pub struct DerivationRequest {
    pub trace: DerivationTrace,
    pub target_relation: Option<String>,
    pub registry_digest: String,
    pub policy_digest: String,
    pub seed: [u8; 32],
    pub resource_limits: ResourceLimits,
}

pub enum DerivationStatus {
    Verified,
    StepFails { step_index: usize, step_id: String, checker: CheckerKind, reason: String },
    UnjustifiedApproximation { step_index: usize, claimed: ErrorBound, proved: Option<ErrorBound> },
    DimensionMismatch { step_index: usize, expected: QuantityType, found: QuantityType },
    UndeclaredSymbol { symbol: String },
    DomainViolation { step_index: usize, condition: String },
    LaunderedFitInput { symbol: String, evidence_ref: Option<String> },
    UnsupportedStep { step_index: usize, kind: String },
    NonDeterministicReceipt,
    ResourceExceeded,
    SandboxFault { detail: String },
}

pub struct DerivationVerdict {
    pub status: DerivationStatus,
    pub metrics: TraceMetrics,
    pub receipt_sha256: String,
    pub checker_versions: Vec<CheckerVersion>,
    pub normalized_trace_sha256: String,
}
```

`Provenance::Derived { certificate }` should become `Provenance::Derived { mechanism, proof: DerivationProofRef }`, where `DerivationProofRef` can be `LegacyClosedForm(DerivedCertificate)` during migration or `Trace { trace_sha256, verdict_sha256, target_digest }` in V8. `obligation.rs` should replace `CertificateOutcome` forwarding with `DerivationVerdict` forwarding. `binding.rs` should bind by `target_digest`, not by relation string plus floating inputs. This also removes fragile name matching: today `certificate.rs` input lookup is case-insensitive but `binding.rs::cert_input` matches exact strings; V8 declarations make symbols canonical at parse time.

Verdict taxonomy must be stable because scorecards, receipts, and regressions will depend on it. `Verified` means every non-literature step is checked by at least one approved checker and all approximations have proved error bounds. `UnsupportedStep` is not “maybe true”; it earns no rigor. A policy mode may allow an unsupported trace to be stored as a conjecture, but never to satisfy a derivation obligation.

## 3. Checker stack and sandboxing

The stack should be heterogeneous by design. CAS tools search and normalize; Rust validates proofs and policy; Lean checks small exact theorems; interval arithmetic bounds numerical and approximation claims.

**Process boundary.** Add a separate executable, `openqg-deriv-sandbox`, called by the core through a JSON-lines protocol. Run it with no network namespace, read-only mounted checker image, read-only registry, empty home, tmpfs scratch, fixed locale/timezone, seccomp denylist, CPU and memory limits, and deterministic PRNG seed derived from `(trace_sha256, registry_digest, policy_digest)`. The worker may not see real prompts, browser profiles, downloaded archives, local overrides, or existing receipts. The receipt includes stdin hash, stdout hash, stderr hash if nonempty, wall/CPU limits, checker image digest, and all tool versions.

**CAS exact/symbolic.** Use SymPy 1.14.0 for mature simplification and assumptions; its own docs warn that simplifications needing assumptions require explicitly declared assumptions, which is exactly the policy we need. Use Symbolica 2.0.0 or the latest audited 1.x if license review requires it; Symbolica gives a Rust/Python CAS for large expressions, pattern rewriting, exact polynomial arithmetic, and expression compression. CAS output is not trusted as “because SymPy said so”: it must produce a normalized equality certificate, rewrite path, or polynomial residual that the Rust checker can inspect.

**E-graphs.** Use `egg` 0.11.0 for equality saturation and `egglog` 2.0.0 for datalog/e-graph workflows when conditional rewrites matter. The accepted rewrite set is a versioned library: associativity/commutativity, distributivity, field rules under nonzero denominators, logarithm/exponential rules only under domain guards, and cosmology-specific definitions only as named axioms. The `egg` prior art is Willsey et al., “egg: Fast and Extensible Equality Saturation,” POPL 2021, arXiv:2004.03082.

**Validated numerics.** Use Arb/FLINT ball arithmetic for real/complex enclosures where possible; Arb explicitly tracks errors using midpoint-radius balls. Rust may bind through audited FFI or use MPFR/Rug intervals for simpler scalar bounds. Every numerical witness returns an interval, not a point estimate. Any interval claim must state the domain box, precision, monotonicity/partition strategy, and failure if the interval crosses the required boundary.

**Schwartz-Zippel backstop.** For rational/polynomial identities, the sandbox can evaluate the residual at deterministic random points in a large prime field. Record degree bound `d`, field size `p`, trial count `k`, and false-accept bound `(d/p)^k`. This is a smoke detector, not a proof: rigor contribution is capped and cannot satisfy a critical identity alone unless policy explicitly marks the relation as exploratory. For non-polynomial expressions, use finite-field tests only after an approved abstraction or refuse.

**Dimensional checker.** Run first. It is cheap, deterministic, and catches many laundering attempts before expensive tools run.

**Lean 4 lane.** Pin Lean 4.30.0 stable or the current audited stable plus a mathlib commit in `contracts/registry.yml`. Lean checks generated small theorems: ring/field equalities (`ring_nf`, `field_simp` with nonzero hypotheses), rational arithmetic (`norm_num`), linear and nonlinear inequalities (`linarith`, `nlinarith` where applicable), unit algebra encoded as integer-vector exponents, and selected derivative/limit lemmas already in mathlib. Lean failures should report the theorem text and tactic transcript digest. No `sorry`, `axiom`, or unsafe imported theorem is permitted in accepted proof artifacts.

Failure attribution is step-local. A failed algebraic rewrite reports the residual expression; a failed dimension check reports the mismatched operands; a failed approximation reports claimed and proved error budgets; a failed domain guard reports the first unproved guard. This is crucial for adversarial debugging and for preventing “CAS failed” from becoming a vague excuse.

## 4. Rigor computed, not assigned

Replace `certificate.rs::relation_rigor_weight()` with `trace_rigor(verdict, policy) -> f64`. `scorecard.rs::derivation_rigor_raw()` should average the best verified trace per physics claim, but the value comes from metrics, not relation names.

For a verified trace:

```text
R = T * M * A * E * D * P
```

where:

* `T` is the nontriviality factor after canonical DAG compression: `min(1, non_definition_core_nodes / 8)`. Pure definitions such as `H0 = 100 h` and flat closure cannot score high merely by being correct.
* `M` is the machine-check mix over core steps, weighted by strongest checker used: Lean exact 1.00, Rust exact dimension/type proof 0.98, CAS exact certificate 0.93, e-graph checked rewrite 0.90, interval/ball proof 0.78, Schwartz-Zippel only 0.45, literature link 0.05. Step splitting is neutral because weights are computed on canonical dependency edges, not raw step count.
* `A = product(p(assumption_i))` over assumptions that influence the target. Suggested penalties: definition/theorem 1.00, domain restriction 0.98, model postulate 0.90, controlled approximation 0.82, phenomenological parametrization 0.45, external measurement 0.25, proposal choice 0.20, scored-data estimate 0.00.
* `E = exp(- total_relative_error / policy.relative_error_budget) * exp(- total_absolute_error / policy.absolute_error_budget)` for approximate traces. Exact traces have `E = 1`. If any approximation lacks a proved bound, status is `UnjustifiedApproximation`, so `R = 0`.
* `D = 1 - exp(- max(0, semantic_depth_from_axioms - 1) / 3)`, capped at 1. This rewards real derivational depth but gives little credit to one-line parametrizations.
* `P` is the provenance admissibility factor: 1 for model/theorem/registry inputs, less than 1 for charged proposal choices, and 0 for scored-data inputs. If a scored-data input is mislabeled as derived, the verdict is `LaunderedFitInput`, not merely a low score.

Caps override multiplication. If any target-influencing step is only Schwartz-Zippel tested, `R <= 0.45`. If any target-influencing assumption is a phenomenological parametrization, `R <= 0.45`, matching the paper’s current intuition that Planck `mu0`-style relations should not earn mechanism-level rigor. If an external measurement is needed, `R <= 0.25` and the quantity remains a data input, not a derived theoretical prediction. If a proposal-choice continuous parameter appears without an upstream verified derivation, it must be counted as a degree of freedom by `scorecard.rs` and cannot reduce parsimony cost.

Gaming resistance follows directly. Step inflation fails because metrics use canonical DAG edges. Assumption laundering fails because assumption strength and input provenance flow to the target. Numerical-point spam fails because randomized tests are capped and deterministic. Loose tolerances fail because tolerance is policy-owned and exact/interval tools compute their own residual. Registry capture fails because relation functions are generated from accepted traces or replay-tested against them.

## 5. Dimensional type system

Every quantity must carry a type:

```rust
pub struct QuantityType {
    pub kind: ScalarKind,                 // RealScalar, PositiveReal, Integer, Bool, Tensor(rank)
    pub dim: DimVec,                      // exponents over {Mass, Length, Time, Temperature, Angle}
    pub natural_units: NaturalUnitPolicy, // c=1, hbar=1, kB=1 flags
    pub unit: Option<CanonicalUnit>,
    pub domain: DomainConstraint,
}
```

Use `DimVec` rather than only mass dimension. Natural-unit mass dimension is still useful for action terms, but cosmology data mixes dimensionless density parameters, `H0` in km/s/Mpc, `sigma8`, redshift, and growth rates. Rules: addition/subtraction require identical dimensions; multiplication adds exponents; division subtracts; rational powers multiply exponents and require integrality where needed; `exp`, `log`, trigonometric functions, and density parameters require dimensionless inputs; derivatives subtract the variable dimension; integrals add it; limits preserve type.

The current `Term { name, mass_dimension, free_lorentz_indices }` should become or be accompanied by `TermAst { name, expression, coefficient, expected_lagrangian_dim, free_indices }`. During migration, existing term names can map to library ASTs. V8 should not accept a candidate-provided integer dimension as evidence. `vetoes.rs` should call the typechecker and only then emit `DimensionalInhomogeneity` or `UncontractedLorentzIndex`. `obligation.rs::Dimensional` becomes an actual trace step over term ASTs and certificate inputs.

Certificate inputs become `QuantityValue { value: ExactOrInterval, qtype: QuantityType, provenance: InputProvenance }`. Examples: `Omega_m`, `Omega_k`, `Omega_de0`, `beta`, `mu0`, `f_R`, `alpha_M`, and `G_eff/G` are dimensionless; `H0` has time inverse or the explicit astronomical velocity-distance unit; `h` is dimensionless; dark-scattering drag must declare whether it is a dimensionless growth-ODE drag per `d ln a` or a physical rate scaled by `H0`. This one distinction would prevent a large class of plausible but meaningless drag certificates.

## 6. Proof assistants, honestly

Lean 4 is worth integrating, but as a scoped checker, not as a magical physics oracle. In 2026, Lean + mathlib is realistic for exact algebraic fragments, rational inequalities, domain side conditions, simple derivatives/limits, and a formally defined units algebra. LeanDojo-style tooling (Yang et al., arXiv:2306.15626) is useful for LLM-assisted proof search, but the accepted artifact must be ordinary Lean checked by the sandbox with no network and no `sorry`. The hybrid model should be CAS-found, Lean-checked: SymPy/Symbolica/egg propose a path; Rust emits a Lean theorem for the final equality or each critical step; Lean either checks it or the step falls back to a lower-rigor checker.

The frontier is full modified-gravity derivation from action to perturbation equations. There is no off-the-shelf mathlib formalization of Horndeski effective field theory, nDGP brane bending, f(R) chameleon screening, or Boltzmann hierarchies. Encoding that is a multi-year formalization project. Therefore V8 should require Lean for the algebra and unit subproblems it can check now, while treating literature-derived physics equations as named model postulates with penalties unless and until a formal library proves them. The current `DerivationObligationKind::LeanSketch` and `Positivstellensatz` stubs in `obligation.rs` should be removed or renamed to `ExternalFormalArtifact` and only score when an actual checker receipt exists.

Prior art to cite in the implementation docs: AI-Descartes (Cornelio et al., arXiv:2109.01634) and AI-Hilbert (Cory-Wright et al., arXiv:2308.09474) for axiom-grounded discovery; `egg` (Willsey et al., arXiv:2004.03082) for equality saturation; Lean 4/mathlib for proof checking; Arb for rigorous ball arithmetic. The lesson is not “copy AI-Hilbert.” It is that derivability must be from explicit axioms, and data consistency is a separate gate.

## 7. New sandbox techniques

**Derivation by construction.** Add a Rust builder DSL that exposes only verified constructors. The LLM can request compositions such as `Ndgp::beta_from_background(omega_rc, background_trace)`, `Field::effective_geff(beta_trace)`, `Units::convert_h_to_H0(h)`, or `Trace::compose(a, b)`. The builder emits the `DerivationTrace` automatically and refuses untyped or unproven inputs. Free-form traces remain for research, but production scoring should prefer builder traces because their soundness comes from typed constructors and library lemmas.

**Checker mutation testing.** For every accepted trace, generate deterministic mutants: flip `+`/`-`, replace `beta` with `1/beta`, delete a domain guard, widen an approximation bound, change a unit, substitute a scored-data provenance, or alter a target digest. The checker must reject or lower rigor according to expectation. Store mutants as regression fixtures beside the migrated relation corpus. This tests the checker, not the proposer, and should become as permanent as the historical exploit regressions.

**Adversarial cross-examination.** Let a skeptic model attack a trace by proposing missing assumptions, singular limits, counterexample domains, or unit mismatches. The host converts those attacks into deterministic challenges: extra interval boxes, finite-field samples, domain guards, or mutants. The LLM does not judge; it generates tests. Receipts contain the deterministic challenge, so replay does not need the model.

**Proof-carrying lemma bank.** Accepted sub-derivations become content-addressed lemmas keyed by `(statement_normal_form, assumptions_digest, checker_versions, policy_digest)`. Reuse is allowed only if the policy digest still trusts the checker versions. This prevents repeated proof search while avoiding stale trust.

## 8. Anti-laundering kill rule

A derivation input is admissible only if it is one of: a discrete model axiom, a typed mathematical definition/theorem, a registry constant external to the scored datasets with uncertainty carried forward, a previously verified trace output, or a proposal-choice parameter explicitly counted as a degree of freedom and not used to claim derivation credit. A value estimated from scored data, holdout data, refreshed-data peeking, or a posterior median from the same search campaign is `scored_data_estimate`. If it influences a target labeled derived, verdict is `LaunderedFitInput` and the theory is killed. If the theory honestly declares it as fitted, it may be scored as a parameter but earns zero derivation rigor and full complexity cost.

Test: submit `mu0 = -0.1` with a trace step “RSD posterior median implies mu0.” If `mu0` is `DerivedConstant`, kill. If it is `PhysicalFree`, score it as a free parameter; the trace can document the fit but not satisfy a physics obligation. Submit `beta = 2.0` as a proposal choice inside `ndgp_geff_over_g`; the relation may derive `G_eff/G` conditional on beta, but beta remains a charged degree of freedom unless another trace derives beta from `omega_rc` and typed background assumptions. This formalizes what `scorecard.rs::post_search_choice_dof()` already gestures at and makes it harder to evade by renaming inputs.

## 9. Migration and acceptance

Migrate the nine registry relations as traces:

1. `h0_from_h`: definition and unit conversion; verifies but nontriviality factor keeps rigor near zero.
2. `flat_universe_omega_lambda`: FRW closure definition under flatness; verifies, low mechanism rigor.
3. `planck_mu0_geff`: phenomenological parametrization `G_eff/G = 1 + mu0`; verifies, capped by phenomenological assumption.
4. `ndgp_geff_over_g`: import nDGP brane-bending linear response theorem/postulate and algebraically derive `1 + 1/(3 beta)` with `beta != 0`.
5. `ndgp_beta_from_omega_rc`: derive from typed `E(a)`, closure, `d ln E/d ln a`, and crossover scale; require `omega_rc > 0` and domain guards.
6. `dark_scattering_growth_drag`: derive the growth-ODE drag normalization from a DE-DM momentum-exchange term; until full perturbation derivation exists, mark controlled approximation and cap rigor.
7. `fr_largescale_geff_over_g`: derive the 4/3 or 1 regime from scalaron quasi-static conditions; require explicit regime conditions and approximation bounds.
8. `fr_alpha_m`: from `M_*^2 = 1 + f_R`, derive `alpha_M = d ln M_*^2 / d ln a = f_R'/(1+f_R)` under `1+f_R > 0`.
9. `coupled_de_geff_over_g`: derive `1 + 2 beta^2` from conformal coupling assumptions; beta is charged unless derived upstream.

Acceptance suite:

* V4 rediscovery/tie-credit: verified definition traces do not create novelty or high rigor.
* V5 free background drift: untraced background changes remain `UnexplainedModification`; traced proposal choices are charged.
* Bare screening: a screening string without trace, units, term AST, and recovery bound fails.
* V6 biased `l_A` witness: a posterior or forward-model calibration residual cannot enter derivation inputs; novelty remains a separate audited path (see S04/S08).
* V6 survivor costless beta: `beta` cannot be both proposal choice and free of cost; no `dgp_brane`/brane-bending axiom means no nDGP mechanism trace.
* Case/name mismatch: all input symbols canonicalize at declaration; `A_drag` and `a_drag` cannot silently diverge between certificate and binding.
* Phantom drag and negative domains: domain guards reject `w0 < -1`, negative drag amplitudes, and singular denominators before scoring.
* Mutation suite: every accepted migrated trace has mutants and all safety mutants fail.

## What we got wrong

* **“Value-level certificates close derived-not-fit.”** They close only value arithmetic. Check that settles it: can a posterior median or proposal-choice beta flow into a verified relation and receive derivation credit? In V8, no.
* **“Relation-level rigor weights are defensible.”** They are hand policy, not mathematical evidence. Check: replace a mechanism axiom with a phenomenological parametrization and watch computed rigor fall without editing a lookup table.
* **“Dimensional homogeneity is verified.”** Currently a term can assert `mass_dimension = 4`. Check: construct a dimension-5 expression with that asserted field; V8 must kill it from the AST.
* **“The term registry proves generating dynamics.”** It is an allowlist, and proposer expansion can auto-add decorative terms. Check: remove the brane-bending axiom from an nDGP trace while keeping the term name; V8 must fail the derivation.
* **“Literature equivalence is a low-rigor proof.”** A citation string is audit metadata, not a proof. Check: a syntactically valid citation with no machine-verifiable step earns no machine rigor.
* **“LeanSketch and Positivstellensatz are harmless placeholders.”** Placeholders become self-deception when included in a rigor enum. Check: no checker receipt, no score.
* **“Input names are already safe.”** Certificate verification is case-insensitive, but binding uses exact names in places. Check: vary input case in a dark-scattering certificate and ensure trace-level canonical declarations make the result either identical or invalid.
* **“A small formula registry can stay trusted code.”** As V8 grows, the registry becomes another oracle surface. Check: every registry function must be generated from or replayed against an accepted trace, and registry diffs must rerun the mutant corpus.
