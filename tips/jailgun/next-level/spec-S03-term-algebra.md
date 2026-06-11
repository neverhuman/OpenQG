# S03 — Replace the MG Allowlist with a Generated Term Algebra

## Ranked backlog

| Rank | Change | Why it matters | Effort | Hostile-review acceptance test |
|---:|---|---|---:|---|
| 1 | Add a new `openqg-algebra` crate with a canonical action-level IR and a theorem-producing compiler from action terms to EFT-of-DE functions. | The present system is still an allowlist: the proposer schema enumerates relation names and MG families, and the expander attaches trusted generating-term strings after the fact (`crates/openqg-bench/src/zyal_genome/proposer_sketch.rs:170-244,344-375`). V8 needs every claimed background/growth dial to be the image of a generating action. | L | `cargo test theorem_registry_relations_from_algebra` proves all nine existing `certificate.rs` relations by compiling algebra terms; deleting the generating action while keeping the old derived parameter must fail with `StructurallyUngenerated`. |
| 2 | Replace `Term { name, mass_dimension, free_lorentz_indices }` with structured term constructors and well-formedness by construction. | V7’s structural gate checks names in arrays such as `NDGP_TERMS`, `FR_TERMS`, `MU0_TERMS`, and `DRAG_TERMS` (`vetoes.rs:385-479`). That prevents typos, not fake physics. | M | A test candidate with `name="dgp_brane"`, dimension 4, no free indices, and no DGP action data is rejected; the real DGP extension term passes and yields the same beta theorem. |
| 3 | Make stability, tensor speed, and QSA validity compiler outputs, never candidate-provided flags. | Current ghost/gradient vetoes consume `theory.stability` supplied in the proposal (`vetoes.rs:268-284`; `mod.rs:237-260`). This is exactly the kind of self-attestation the project says it does not trust. | L | For quintessence, k-essence, f(R), and kinetic braiding fixtures, `q_s(a)`, `c_s^2(a)`, and `alpha_T(a)` are regenerated from the action. A manually edited stability field is ignored and cannot change the veto result. |
| 4 | Introduce a per-theory QSA gate and Boltzmann-escalation flag. | `growth.rs` currently uses a scale-free `mu0` path and a reference `k=0.1 h/Mpc` scale-dependent path (`growth.rs:42-56,166-212`) without a theory-by-theory proof that QSA is valid on the scored data. | M | For each scored `(z,k)` point, the compiler emits `qsa_epsilon`; if any observable uses `qsa_epsilon > 0.1`, the run must either use the full Boltzmann backend interface or receive zero growth/lensing rigor. Low sound-speed DE fixtures intentionally trip the gate. |
| 5 | Add a screening compiler for chameleon and Vainshtein mechanisms. | V5 exploited declared screening. The current PPN check uses a declared `screening_recovery` and residual formula (`vetoes.rs:491-529`), while `screening` remains an optional string in `Theory` (`mod.rs:264-292`). | L | Hu-Sawicki f(R) and nDGP fixtures compute thin-shell or Vainshtein recovery for Sun/Earth/Milky-Way environments; a fixture with the same linear `mu` but no nonlinear term fails Cassini. |
| 6 | Cross-validate against hi_class/EFTCAMB and published analytic limits. | The repository admits its default engine is a fitting-formula/background model and not a full perturbation solver (`paper/main.tex:652-668`; `docs/boltzmann-backend.md:1-8`). | L | Golden fixtures for f(R), quintessence, k-essence, coupled DE, and nDGP reproduce published `mu(a,k)`, `Sigma(a,k)`, growth, and GR limits to the tolerances below. Backend hashes and precision files are stamped in receipts. |
| 7 | Treat phenomenological alpha/mu proposals as diagnostics unless an inverse-witness action is found. | `planck_mu0_geff` and `mu0` binding (`binding.rs:233-278`) are useful probes, but a free `mu0` is not a mechanism. | S | A `mu0`-only proposal can be scored as a phenomenology lane but receives no derivation-rigor credit and cannot satisfy `StructurallyGenerated` unless the compiler finds an action whose alpha functions produce it. |
| 8 | Publish a stable algebra interface for S02 midpoint search and S08 Boltzmann consumption. | The algebra should be the common representation between LLM proposals, deterministic search, and the external perturbation stack, not a private validator. | M | JSON schema round-trip: a canonical action fingerprint, EFT-function bundle, stability report, QSA report, screening report, and theorem receipt are reproducible byte-for-byte on two hosts. |
| 9 | Migrate in two phases: registry-as-theorems first, generative proposals second. | A hard cutover risks losing the mapped proof lane. The old registry is a valuable regression oracle even though it must stop being the source of truth. | M | Existing V4-V7 replay artifacts rescore deterministically; V6’s costless beta dies for a structural reason; V7’s honest negative remains reproducible with either old registry disabled or locked behind theorem receipts. |

## 1. Problem statement from the archive

The archive already knows the danger but has not yet removed it. The paper says the V7 proposal path uses a strict JSON schema with enums generated from registered relations and that the expander attaches a generating term for every certified relation or background dial (`paper/main.tex:175-228`). In code, the proposal sketch exposes only `mg_family = none | fr_hu_sawicki | ndgp` plus floats such as `mu0`, `fr_log10_fr0`, `ndgp_omega_rc`, `w0`, and `wa` (`proposer_sketch.rs:72-84,170-172`). The certificate registry lists nine closed-form relations (`certificate.rs:103-135`). The V7 structural veto then checks term names against hard-coded arrays (`vetoes.rs:385-479`).

That is a good anti-invention patch; it is not a theory generator. It makes the V6 survivor’s “costless certified beta with no generating brane term” harder to repeat by name, but a future version can still certify a relation if the host has hand-listed the relation and a compatible term label. The archive’s own internal critique calls for term-grammar mutation and a derivation oracle (`docs/zyal-next-level-design.md:217-247,290-306`), and the paper names “term algebra replacing registry allowlist” as a next step (`paper/main.tex:652-668,732-736`). This spec is the concrete V8 design.

The core rule is: **derived cosmological functions must be compiled from an action, or they are phenomenology with no derivation-rigor credit.** No proposal may directly assert `mu0`, `alpha_M`, `q_s`, `c_s^2`, screening recovery, or a growth drag as a durable physical claim.

## 2. Generative grammar: action first, alpha basis as compiled midpoint

### Scope and position

V8 should begin with a single-field scalar-tensor algebra covering Horndeski and tightly gated extensions, because Bellini & Sawicki’s alpha basis gives the most compact linear cosmology representation for that class: `H(a)`, `Omega_m0`, `M_*^2(a)`, and four property functions `alpha_K`, `alpha_B`, `alpha_M`, `alpha_T` (Bellini & Sawicki, JCAP 07 (2014) 050, arXiv:1404.3713). hi_class implements Horndeski in CLASS and uses that space for linear observables (Zumalacarregui, Bellini, Sawicki, Lesgourgues & Ferreira, JCAP 08 (2017) 019, arXiv:1605.06102). EFTCAMB is the parallel CAMB implementation and is important because it evolves full linear dynamics and has stability checks without relying on QSA (Hu, Raveri, Frusciante & Silvestri, Phys. Rev. D 89, 103530, arXiv:1312.5742).

Beyond-Horndeski/DHOST should be **phase-2 included, default-excluded**. It is scientifically important, but only if the algebra enforces the degeneracy conditions that remove the Ostrogradsky mode and the post-GW170817 tensor-speed constraints. V8 should include a `DhostExtension` feature flag only after a symbolic degeneracy certificate exists for the selected class. Until then, a beyond-Horndeski proposal is rejected with `UnimplementedModification`, not silently approximated. Non-scalar-tensor escapes, including vector-tensor, Einstein-aether, bimetric, and massive gravity, are **excluded from the first algebra** because their physical degrees of freedom, stability matrices, screening, and Boltzmann interfaces differ. Lagos, Bellini, Noller, Ferreira & Baker (arXiv:1711.09893) show how scalar-tensor, vector-tensor, and bimetric theories can be organized in a unified linear perturbation language, but using that as the first generator would mix distinct theory spaces before OpenQG can prove even the scalar-tensor lane.

DGP is the exception: the archive already scores nDGP, and removing it would break continuity. Treat it as a separate `BraneScalarExtension`, not as generic Horndeski. Its decoupling scalar/Galileon limit can share stability, QSA, and Vainshtein machinery, but its theorem receipt must say “brane extension,” not “4D Horndeski.”

### IR constructors

Replace the current name-only `Term` (`mod.rs:197-202`) with action-level constructors. Rust owns durable policy and canonicalization; a CAS lane generates formula catalogs and receipts.

```rust
pub struct AlgebraTheory {
    pub schema_version: SemVer,
    pub fields: Vec<FieldDecl>,
    pub gravity: GravitySector,
    pub scalar_terms: Vec<ScalarTensorTerm>,
    pub brane_terms: Vec<BraneTerm>,
    pub matter_couplings: Vec<MatterCoupling>,
    pub cutoff: EftCutoff,
    pub parameter_priors: Vec<ParameterPrior>,
}

pub enum ScalarTensorTerm {
    HorndeskiG2 { basis: FunctionBasis },
    HorndeskiG3 { basis: FunctionBasis },
    HorndeskiG4 { basis: FunctionBasis },
    HorndeskiG5 { basis: FunctionBasis },
    Potential { basis: FunctionBasis },
    KEssence { basis: FunctionBasis },
    ConformalMatterCoupling { species: MatterSpecies, a_of_phi: FunctionBasis },
    DisformalMatterCoupling { species: MatterSpecies, c: FunctionBasis, d: FunctionBasis },
}

pub enum FunctionBasis {
    PolynomialPhiX { terms: Vec<PhiXMonomial> },
    ExponentialPhi { amplitudes: Vec<ParamId>, slopes: Vec<ParamId> },
    DesignerBackground { w0: ParamId, wa: ParamId, inverse_witness: WitnessId },
    AlphaPhenomenology { alpha: AlphaFunctionBasis, inverse_witness: Option<WitnessId> },
}

pub struct PhiXMonomial {
    pub coefficient: ParamId,
    pub phi_power: u8,
    pub x_power: u8,
    pub mass_power: i8,
    pub symmetry_tags: BTreeSet<SymmetryTag>,
}
```

The Horndeski action template is generated, not enumerated:

`S = ∫ d^4x sqrt(-g) [G2(phi,X) - G3(phi,X) Box phi + G4(phi,X) R + G4_X((Box phi)^2 - phi_;mu nu phi^;mu nu) + G5(phi,X) G_mu nu phi^;mu nu - (1/6)G5_X(...)] + S_m[A_i^2(phi)g_mu nu, psi_i]`.

`X = -g^munu partial_mu phi partial_nu phi / 2`. The grammar proposes finite expansions for the functions `G_i(phi,X)`; the compiler maps them to `M_*^2`, `alpha_M`, `alpha_B`, `alpha_K`, and `alpha_T`. An alpha-basis proposal is allowed only as a midpoint representation. It must either carry an `inverse_witness` proving it is in the image of an action under the current basis cutoff, or be labeled `PhenomenologyOnly` and barred from mechanism novelty and derivation rigor.

### Well-formedness rules

These rules are enforced during construction, before any scoring:

1. **Mass dimension:** every term in `sqrt(-g) L` has dimension four in natural units. Dimension is computed from fields and derivatives, not entered as a free integer.
2. **Diffeomorphism and Lorentz invariance:** all spacetime indices must be contracted with `g_munu`, `epsilon_munu rho sigma`, curvature tensors, or covariant derivatives. The current `free_lorentz_indices == 0` check remains only as a derived sanity test.
3. **Locality:** no nonlocal operators such as `Box^{-1}` in V8. They need a different causal prescription and are rejected.
4. **Parity:** default parity-even. Parity-odd terms are rejected unless the run enables a parity-odd feature lane with separate observables.
5. **Equation order / degeneracy:** Horndeski constructors are second-order by construction. DHOST constructors require an algebraic degeneracy certificate before use. A term that cannot prove either is rejected as `OstrogradskyGhost` before numeric integration.
6. **Matter coupling:** baryons are minimally coupled unless a local-gravity proof lane is enabled. Dark-sector couplings may be generated but must propagate species labels into perturbation equations; they cannot be collapsed into a universal `mu`.
7. **Tensor speed:** post-GW170817 runs require `|alpha_T(z≈0)| < 1e-15` unless the theory has an explicit frequency/environment loophole with its own data gate. The current scalar alpha speed estimate in `vetoes.rs:342-350` becomes a compiler theorem, not a proposal field.

## 3. Mechanical compilation chain

### Symbolic stage

Hard requirement: do not implement tensor variation by hand in ordinary Rust. Use a pinned CAS derivation lane and export immutable formula catalogs. The production recommendation is:

- `sympy==1.13.x` for expression canonicalization, code generation, dimensional simplification, and exact rational tests.
- A pinned `xAct/xTensor/xPert` Wolfram Engine lane for golden derivations of the Horndeski-to-alpha formula catalog. If licensing prevents running this on every CI host, run it in a controlled release job and commit only formula JSON plus a derivation hash; normal CI verifies the committed formulas against independent fixtures.
- Generated Rust evaluators using `rug`/`num-rational` for exact theorem tests and `nalgebra`/`argmin` or current Rust ODE machinery for numeric solves.

The CAS stage emits:

```json
{
  "formula_catalog_version": "v8.0.0",
  "action_fingerprint": "sha256:...",
  "background_equations": ["E_phi(N)", "Friedmann_00", "Friedmann_ii"],
  "eft_functions": ["Mstar2(a)", "alpha_K(a)", "alpha_B(a)", "alpha_M(a)", "alpha_T(a)"],
  "stability": ["q_s(a)", "c_s2(a)", "q_t(a)", "c_t2(a)"],
  "linear_qs": ["mu(a,k)", "Sigma(a,k)", "eta(a,k)"],
  "proof_obligations": ["dimension=4", "contracted_indices", "horndeski_or_degenerate"]
}
```

The formula catalog is not trusted because it exists; it is trusted only when its hash appears in a release manifest and the regression suite below passes.

### Numeric stage

The numeric compiler consumes cosmological parameters plus algebra parameters and returns `CompiledCosmology`:

```rust
pub struct CompiledCosmology {
    pub background: BackgroundSpline,       // H(a), E(z), Omega_i(a), phi(a), X(a)
    pub alpha: AlphaSplineBundle,           // alpha_K/B/M/T, Mstar2
    pub stability: StabilityReport,         // min q_s, c_s2, q_t, c_t2 and locations
    pub qsa: QsaReport,                     // valid ranges in z,k; escalation flags
    pub linear: LinearResponseBundle,       // mu, Sigma, eta on requested grid
    pub screening: Option<ScreeningReport>,
    pub theorem_receipts: Vec<TheoremReceipt>,
}
```

`background.rs` currently computes FLRW quantities from CPL `w0,wa` and a small MG family enum (`background.rs:80-127,187-229`). V8 should keep this as the baseline backend but add an algebraic background solver: solve the scalar EOM and Friedmann constraint in `N = ln a`, with shooting or initial-condition priors appropriate to the model. Designer backgrounds are allowed only if the compiler also produces a scalar potential or EFT function witness that realizes the background. Otherwise they are background phenomenology and cannot claim mechanism novelty.

### Perturbations and QSA

Linear perturbations are compiled in two modes:

1. **Full linear mode:** export `H(a)`, `M_*^2(a)`, `alpha_i(a)`, initial conditions, and stability to hi_class/EFTCAMB through the S08-facing interface. This is required for CMB ISW, CMB lensing when sound speed is small, and any theory failing the QSA gate.
2. **QSA mode:** compute closed-form `mu(a,k)`, `Sigma(a,k)`, and slip `eta(a,k)` from the alpha functions using Bellini-Sawicki/Lagos formulas. QSA is never global; it is valid per observable grid point.

The QSA report must evaluate, for each `(a,k)`, a conservative expansion parameter:

```text
sound_horizon_ratio = aH / max(c_s, 1e-6) / k
metric_ratio        = aH / k
running_ratio       = max_i |d ln alpha_i / dN| * aH / max(c_s, 1e-6) / k
mass_ratio          = a m_eff / k        // when scalar mass is explicit
qsa_epsilon         = max(sound_horizon_ratio, metric_ratio, running_ratio, optional mass transition penalty)
```

Default acceptance for using QSA in scored growth/lensing is `qsa_epsilon < 0.1` on every bin contributing non-negligible likelihood weight. The threshold is intentionally strict because Sawicki & Bellini (arXiv:1503.06831) show QSA breaks outside the dark-energy sound horizon, and for CMB ISW it should not be used. The engine may still store QSA outputs for diagnostics when invalid, but the scorecard must not award data-fit or rigor credit for them.

### Stability

Ghost and gradient checks become computed properties:

- tensor: `q_t = M_*^2 / 8 > 0`, `c_t^2 = 1 + alpha_T > 0`, with the GW speed bound above;
- scalar: `q_s > 0` and `c_s^2 > 0` from the alpha functions and background equations;
- no strong-coupling acceptance if `q_s` is positive only below a configurable floor, default `q_s > 1e-6 M_pl^2`, or if `c_s^2` is so small that all scored probes fail QSA and the full backend is absent.

The existing `Theory.stability` can remain for report display during migration, but the veto code must read only `CompiledCosmology.stability`. A hostile reviewer should be able to edit the submitted JSON stability fields and observe no score change.

## 4. Closure and correctness suite

V8 should preserve the nine current relations as regression theorems, not as the generative source of physics. The registry from `certificate.rs:103-135` maps as follows:

| Current relation | Algebra theorem source |
|---|---|
| `h0_from_h` | Unit theorem in the background algebra: `H0 = 100 h km s^-1 Mpc^-1`. This is not MG and should carry zero mechanism credit. |
| `flat_universe_omega_lambda` | Friedmann constraint theorem for a spatially flat FLRW background: `Omega_Lambda = 1 - Omega_m - Omega_r - ...`. Zero mechanism credit. |
| `ndgp_beta_from_omega_rc` | `BraneScalarExtension::NormalDgp { r_c }` compiles to the normal-branch background and beta function. The theorem receipt includes the brane term fingerprint and the background branch. |
| `ndgp_geff_over_g` | Same DGP extension in the subhorizon scalar-brane-bending limit: `G_eff/G = 1 + 1/(3 beta)`. Must be connected to the beta theorem, not entered independently. |
| `fr_alpha_m` | Horndeski image of `G4 = M_pl^2 F(phi)/2` with `F=1+f_R`; theorem `alpha_M = d ln F / d ln a`. |
| `fr_largescale_geff_over_g` | Hu-Sawicki f(R) scalaron theorem: `mu(a,k)` approaches 1 outside the Compton radius and 4/3 inside it; `Sigma` remains close to 1 in the unscreened linear limit. The old relation’s “large-scale” naming should be corrected to explicit `k/m(a)` limits. |
| `coupled_de_geff_over_g` | Matter-coupling theorem for conformally coupled dark matter: dark-matter clustering sees `1+2 beta^2` in the subhorizon limit. Baryons remain separately labeled; the theorem must not feed a universal baryonic `mu` unless a local-gravity gate is passed. |
| `dark_scattering_growth_drag` | Matter-sector momentum-exchange theorem: the coupling action or stress-transfer ansatz compiles to an Euler drag `Gamma(a)` and growth friction term. It is not a gravity-sector `mu` theorem. |
| `planck_mu0_geff` | Phenomenology compatibility theorem only. It maps a compiled `mu(a,k_ref)` to the Planck-style `mu0` summary if an action already generated `mu`. A bare `mu0` does not map backward to an action and receives no derivation-rigor credit. |

A small bug-level check falls out here: every string in `registered_relations()` should have a `relation_signature()`. In the inspected snapshot, `dark_scattering_growth_drag` is registered and verified, but the signature matcher does not include a corresponding branch (`certificate.rs:184-205` versus `291-315`). Add `registered_relations_have_signatures` before migration.

Regression fixtures:

1. **Quintessence:** canonical `G2=X-V(phi)`, `G3=0`, `G4=M_pl^2/2`, `G5=0`. Required outputs: `alpha_B=alpha_M=alpha_T=0`, `c_s^2=1`, `mu=Sigma=1`, growth matches GR with the compiled background to `2e-4` in `D(a)` and `5e-4` in `f sigma8`.
2. **K-essence:** `G2=K(X)-V(phi)`. Required theorem: `c_s^2 = p_X/(p_X+2X p_XX)`, `mu=Sigma=1` for minimal coupling. A fixture crossing `c_s^2<0` must be vetoed.
3. **Hu-Sawicki f(R):** compile the scalar-tensor form and compare `alpha_M`, scalaron mass, `mu(a,k)`, and `Sigma(a,k)` against hi_class/EFTCAMB on a grid `z in {0,0.5,1,2}`, `k in {0.01,0.05,0.1,0.2} h/Mpc`, `|f_R0| in {1e-6,1e-5}`. Tolerance: `0.5%` for QSA response where QSA-valid; `1%` for `f sigma8`; GR limit within `1e-5` as `f_R0 -> 0`.
4. **nDGP:** compile `Omega_rc` and reproduce `beta(a)` and `G_eff/G = 1+1/(3 beta)` to `1e-8` algebraically and growth to `0.5%` against the current Rust implementation plus an independent notebook. The test should assert the normal branch enhances linear growth for positive beta, matching the code comment in `growth.rs:504-521`.
5. **Coupled DE:** species-labeled dark-matter coupling reproduces `G_eff,cc/G=1+2 beta^2` while baryon-baryon force remains GR. The growth solver must evolve at least a two-fluid linear system for this fixture; collapsing it into one universal `mu` fails.

Every fixture should generate a `TheoremReceipt { action_hash, formula_hash, assumptions, limit, tolerance, references }`. The receipts are durable artifacts; prompts and local notebooks are not.

## 5. What dies and what becomes searchable

The V6 survivor’s exploit class dies because there is no longer a place to attach a certified beta without a generator. `binding.rs` currently inverts certified nDGP beta into `omega_rc` and writes `mg_family=Ndgp` (`binding.rs:107-154`), and direct-symbol copies can still bind background fields like `mu0`, `ndgp_omega_rc`, `fr_log10_fr0`, and `fr_n` (`binding.rs:328-419`). In V8, a background dial can be bound only if the compiler emits it from an action fingerprint. A derived parameter may refer to `beta(a)`; it may not create the brane action. That inversion becomes a consistency check, not a generator.

The unlocked space is qualitatively larger than “more allowlist entries.” Today’s searchable MG space is two bound MG families (`fr_hu_sawicki`, `ndgp`), one phenomenological `mu0` path, one dark-scattering lane, and nine registry formulas. A conservative V8 cutoff with 40 Horndeski monomial slots across `G2-G5`, choosing up to four active monomials, already yields roughly `C(40,1)+C(40,2)+C(40,3)+C(40,4) ≈ 102,090` discrete action structures before continuous parameters, priors, and stability cuts. After dimensional, tensor-speed, stability, and local-gravity gates, I would expect `O(10^3-10^4)` physically distinct, testable scalar-tensor mechanisms under a first-release cutoff. That estimate is intentionally a search-space planning number, not a physics claim.

Mechanism families unlocked by generation include canonical quintessence, thawing/freezing potentials, k-essence, kinetic gravity braiding, Brans-Dicke/Jordan-Fierz scalar-tensor, chameleon f(R)-like subclasses beyond one Hu-Sawicki parameterization, covariant Galileon/DGP decoupling structures, coupled quintessence with species labels, and disformal dark-sector interactions. Conversely, bare `mu0`, arbitrary `alpha` splines, and “screened” strings become diagnostics until backed by a generator.

## 6. Screening as a compiler output

Screening must be derived in the same pass that derives linear response. It is not a property of a family name; it depends on nonlinear operators, matter coupling, source mass, source radius, ambient density, scalar mass, and EFT cutoff.

### Chameleon algorithm

Inputs: scalar potential `V(phi)`, matter conformal coupling `A_i(phi)`, species labels, environmental density profile, source potential `Phi_N`, and compiled background value `phi_infty(a)`. The compiler builds

`V_eff(phi; rho) = V(phi) + rho [A(phi)-1]`

for baryonic matter if baryons are coupled, or for dark matter if only the dark sector is coupled. It solves for minima `phi_min(rho)`, masses `m_eff^2 = d^2 V_eff/dphi^2`, and thin-shell parameter

`Delta R/R = (phi_infty - phi_c)/(6 beta M_pl Phi_N)`.

For Sun/Earth/Milky-Way profiles, compute the fifth-force residual and PPN gamma shift. Feed that number into the existing Cassini-style gate, replacing `screening_recovery`. Fail modes: no minimum, negative `m_eff^2`, shell not thin enough, baryons coupled without Solar-System recovery, EFT cutoff below inverse source radius, or environment dependence so strong that cosmological background and local background cannot be matched.

### Vainshtein algorithm

Inputs: scalar derivative self-interactions from G3/G4/G5 or DGP brane term, coupling beta, source mass, source radius, and cutoff scale. The CAS reduces the static spherical quasi-static scalar equation to a polynomial in `y = phi'(r)/r`. The solver identifies the linear and leading nonlinear terms, then computes `r_V` from equality of the terms. For nDGP, this recovers `r_V = (r_s r_c^2)^{1/3}` up to convention factors. The PPN residual at radius `r` is the unscreened fifth-force fraction multiplied by the Vainshtein suppression, typically `(r/r_V)^p` with `p` derived from the dominant operator, not hard-coded from a name.

Fail modes: no positive real Vainshtein radius, screened radius smaller than the Solar-System scale, nonlinear solution branch not connected to the cosmological branch, strong-coupling scale invalidates the classical calculation, or tensor/vector modes not covered by the scalar screening compiler.

The `ScreeningReport` must contain the source profiles used, numerical residuals, branch choice, and proof assumptions. It is passed to the existing PPN/Cassini veto, which should keep durable policy in Rust.

## 7. Public interfaces

The algebra is the natural midpoint for dual-path search (see S02) and the natural target for a Boltzmann backend (see S08), but this spec cannot depend on their outputs. Provide two stable interfaces.

### Search-facing interface

```rust
pub trait AlgebraCompiler {
    fn check_well_formed(&self, t: &AlgebraTheory) -> Result<WellFormedReport, AlgebraError>;
    fn canonicalize(&self, t: &AlgebraTheory) -> CanonicalTheory;
    fn compile(&self, t: &CanonicalTheory, req: CompileRequest) -> Result<CompiledCosmology, CompileError>;
    fn prove_relation(&self, t: &CanonicalTheory, relation: RelationId) -> Result<TheoremReceipt, ProofError>;
}

pub struct CanonicalTheory {
    pub action_hash: Sha256,
    pub normalized_terms: Vec<NormalizedTerm>,
    pub dof_ledger: DofLedger,
    pub mechanism_tags: BTreeSet<MechanismTag>,
}
```

S02 can mutate `CanonicalTheory` by adding/removing basis terms, changing priors, or asking for an inverse witness from an alpha bundle. It should not mutate compiled `mu` directly.

### Boltzmann-facing interface

```rust
pub struct EftFunctionBundle {
    pub background: BackgroundSpline,
    pub mstar2: Spline1D,
    pub alpha_k: Spline1D,
    pub alpha_b: Spline1D,
    pub alpha_m: Spline1D,
    pub alpha_t: Spline1D,
    pub matter_couplings: Vec<SpeciesCouplingSpline>,
    pub stability: StabilityReport,
    pub provenance_hash: Sha256,
}
```

This extends the existing subprocess seam described in `docs/boltzmann-backend.md:10-19`: Rust writes JSON to an adapter, the adapter returns `PredictionRecord`s, and failures are hard errors. For V8, the request must include `EftFunctionBundle` and requested observables; the response must include backend code hash, precision file hash, and whether full perturbations or QSA were used. Omitted observables remain omitted, never faked.

## 8. Migration, proposer economics, and scorecard gate

Phase 0: add tests around current behavior. Freeze the nine relations, add the missing-signature test, and add decoys for name-only `dgp_brane`, name-only `f_r_correction`, declared screening, and manually supplied healthy stability.

Phase 1: implement algebra as a shadow compiler. Existing proposals still use the registry, but every accepted registry relation must also have an algebra theorem receipt. Score remains unchanged, but receipts are stored. This gives replay comparability for V4-V7 and keeps the mapped proof lane runnable.

Phase 2: switch binding to receipt-first. `bind_verified_claims_into_background` must consume `TheoremReceipt`s rather than relation strings. Direct-symbol copies of MG dials are disabled except for unit conventions and explicitly fundamental cosmological parameters. V6’s costless beta should die here even if the old relation verifier still exists.

Phase 3: change proposer prompts. Instead of saying “only these registry relation IDs verify” (`proposer_sketch.rs:563-600`), the prompt should request a small action sketch:

```json
{
  "action_terms": [
    {"kind":"HorndeskiG4", "basis":[{"coef":"c4_1", "phi_power":1, "x_power":0}]},
    {"kind":"Potential", "basis":"exp(lambda, V0)"}
  ],
  "matter_couplings": [{"species":"cdm", "kind":"conformal", "beta":"b1"}],
  "intended_mechanism": "computed_by_host",
  "claimed_observables": ["fsigma8", "s8", "bao"]
}
```

Parse/repair economics will worsen at first because the schema is richer. That is acceptable if repairs are structured: invalid dimension, unsupported DHOST, missing prior, unstable background. The repair loop should ask for minimal edits to the action, not patched outputs. Cache canonicalization and theorem proofs by action hash; most population mutations will reuse subterms.

Phase 4: move the scorecard rigor dimension. A theory earns derivation-rigor credit only if all scored non-GR observables are downstream of action-generated `CompiledCosmology` receipts, with computed stability, QSA, and screening status. Phenomenological alpha/mu lanes may still compete as diagnostic baselines, but they cannot be champions unless the scorecard labels them non-mechanistic.

Cutover gate: all current registry theorem tests pass; the five golden families pass; old champions replay; old exploit decoys fail; at least one external backend run reproduces the Rust QSA lane where QSA is valid; and the score changes caused by algebra are explained by receipts rather than prompt behavior.

## What we got wrong

1. **“The term registry enforces generated mechanisms” is oversold.** It enforces a name allowlist. `vetoes.rs:385-479` checks arrays of term names, and `proposer_sketch.rs:344-375` attaches those names deterministically from relation IDs. Settlement check: create a candidate with the right name, dimension, and zero free indices but no action constructor. V7 can treat it as structurally generated; V8 must reject it because no action fingerprint proves the theorem.

2. **“Horndeski/alpha-basis theory object” is currently mostly a value container.** `mod.rs:204-215` stores alpha values, and `mod.rs:237-260` stores stability coefficients, but the archive does not derive them from `G_i(phi,X)`. Settlement check: mutate `alpha_M` and `c_s^2` fields independently. In V8, that edit must be impossible or ignored; both values come from the compiler.

3. **“Screening is gated” is true only numerically, not physically.** The current gate uses a declared recovery value and an algebraic residual (`vetoes.rs:491-529`). Settlement check: compare a real Hu-Sawicki thin-shell calculation and a real nDGP Vainshtein calculation to the submitted `screening_recovery`. If the submitted number can decide the veto without matching the derived calculation, the gate is not V8-ready.

4. **Bare `mu0` is not a theory class.** `planck_mu0_geff` and the `mu0` growth path are useful controls (`binding.rs:254-278`; `growth.rs:42-56`), but a constant amplitude is not a Lagrangian mechanism. Settlement check: require an inverse witness from action terms to `mu(a,k_ref)`; absent that, `mu0` gets no mechanism novelty or derivation-rigor credit.

5. **The nDGP narrative must not imply suppressed growth.** The code’s own test says the healthy normal branch enhances growth (`growth.rs:504-521`). Settlement check: prove `beta>0 => G_eff/G > 1` for the accepted branch and require any suppressed-growth DGP claim to identify an additional mechanism, not the standard nDGP relation.

6. **The registry itself has consistency holes.** `dark_scattering_growth_drag` is registered and verified, but the relation-signature matcher lacks a branch in the inspected file. Settlement check: `for r in registered_relations() { assert!(relation_signature(r).is_some()) }` before any V8 migration.
