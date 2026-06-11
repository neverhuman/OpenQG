# S09 — Knowledge hardening for OpenQG/ZYAL

## RANKED BACKLOG

| Rank | Change | Why it matters | Effort | Hostile-reviewer acceptance test |
|---:|---|---|---:|---|
| 1 | **Build a provenance-governed retrieval layer before any proposer sees literature.** Replace `extra_sections: String` with typed `RetrievalPacket`s carrying source IDs, redaction receipts, equation IDs, and token budgets. | The current router path already has the insertion point (`build_router_prompt(lane, sample, extra_sections)`) but no provenance contract; the router also says its embedding endpoint is fake and unused (repo: `crates/openqg-bench/src/zyal_genome/proposer_router.rs:1-15`, `proposer_sketch.rs:563-608`). Without typed retrieval, “agents study physics” is prompt stuffing. | L | Run 200 proposals with retrieval enabled. Every mechanism claim and literature-derived obligation in the proposal ledger must reference at least one valid `retrieval_id`; deleting the retrieval DB after prompt construction must make replay verification fail, not silently pass. |
| 2 | **Add `LiteratureEquationMatch` as a real obligation kind and demote/free-kill text-only `LiteratureEquivalence`.** The new witness retrieves the cited source passage, extracts the equation, normalizes it, and matches the certificate relation. | Today `LiteratureEquivalence` passes with a nonempty citation and earns 0.3 rigor; it does not inspect the paper (repo: `crates/openqg-core/src/theory/obligation.rs:25-65`, `175-275`, `679-689`). That is a citation laundering channel. | M/L | Unit test: a proposal citing the correct paper title but wrong equation fails; the same proposal with a source passage whose normalized equation matches `dark_scattering_growth_drag` passes. Mutation of the source equation hash invalidates the witness. |
| 3 | **Create a lane-scoped equation-aware physics corpus, not a generic vector database.** Acquire arXiv source/PDF, INSPIRE metadata/citations, Living Reviews, selected review articles, and optional S2ORC/Semantic Scholar metadata; parse LaTeX equations into structural indexes. | The lanes need mechanism equations, not prose summaries. A dark-scattering lane needs perturbation/Euler/drag terms; Planck-μ0 needs modified-gravity parametrization papers and constraints, with posterior values quarantined. | L | Reproducible build manifest for `astro-ph.CO`, `gr-qc`, `hep-th`, and selected `hep-ph` subsets. For a fixed query set, the corpus returns the known Simpson dark-scattering equation, Planck μ parametrization equations, and Hu-Sawicki/Bellini-Sawicki relations with section, equation, license, and content hash. |
| 4 | **Install a contamination firewall at retrieval, certificate, and ledger time.** Classify and redact posterior/data values; allow mechanism equations and structural coefficients; kill any certificate that imports a fitted posterior value as a derivation input. | The corpus contains Planck/DESI/SH0ES/KiDS measured values. The current prompt already tells proposers to aim at tensions and data pulls (repo: `proposer_prompt.rs:69-88`), while data fixtures contain the actual fit values. Retrieval can make leakage auditable, but only if value-origin is checked. | M | Red-team corpus with Planck posterior tables and DESI BAO rows. Treatment proposer may cite mechanisms but any numeric copied from redacted results into `certificate.inputs`, `expected`, or derived parameter values is killed as `LiteratureValueLaundering`. Zero survivors with leaked values. |
| 5 | **Persist cross-campaign `KnowledgeLesson`s distilled by deterministic host logic.** Store killed/survived patterns, support, lift, affected observables, relation/lane fingerprints, and retrieval-safe hints. | `docs/ZYAL.md` promises failed lanes become negative memory, and `docs/zyal-next-level-design.md` asks for derivation lemmas/refuted regions/live constraints, but the live ledgers record attempts, not reusable lessons (repo: `docs/ZYAL.md:73-90`; `docs/zyal-next-level-design.md:249-262`; `theory_population.rs:113-180`). | M | After a run with ≥20 constant-`A_drag` dark-scattering kills, a deterministic lesson appears with support/lift and is retrieved into the next dark-scattering campaign. The lesson text must be reconstructible from ledger facts without an LLM. |
| 6 | **Wire phase-specific retrieval into proposal, repair, and critique prompts.** Proposal gets mechanism primers; repair gets kill-specific source snippets; critique gets contradiction and prior-art snippets. | Repair currently redacts engine-computed values from kill reasons before sending them to the model, which is good, but repair has no external physics source to learn from (repo: `proposer_router.rs:365-505`). | M | For a forced `UnexplainedModification` kill, repair prompt must contain the matching generating-term snippets and no fit-data values; the repair success rate must be measured against a no-retrieval control. |
| 7 | **Run a pre-registered 2-week retrieval pilot with a control arm.** Freeze model router, seeds, lanes, scorecard, retrieval snapshot, and decision thresholds before running. | Retrieval will increase tokens and can increase sophistication of cheats. It should ship only if it reduces real kills and raises rigor without lowering firewall integrity. | S/M | A signed pilot manifest exists before run start. Ship/iterate/kill decision follows the predeclared thresholds below; no post-hoc metric substitution. |
| 8 | **Add token/source telemetry to every prompt and scorecard row.** Record retrieval tokens, lesson tokens, redaction counts, source classes, and survivor cost. | “Gets smarter” is a measurable systems claim. Without tokens-per-survivor and source provenance, improvement can be hidden by spending more context. | S | `ProposalAttemptRecord` extension is populated for every attempt; deleting token telemetry fails CI replay. Dashboard can show tokens per surviving proposal by lane. |
| 9 | **Defer physics-outcome fine-tuning; allow only schema/repair LoRA after retrieval proves itself.** | Fine-tuning on kill/survival transcripts can bake in oracle bugs, data leakage, and stale literature. Retrieval + deterministic lessons is reversible and auditable; LoRA is not. | S | No training job may consume score outcomes or fit values. A formatting-only LoRA can be approved only if it reduces parse/JSON failures without changing physics distributions in a blinded comparison. |

## 1. Current repo diagnosis

The project has built a serious deterministic shell around proposals. `ProposalDoc` carries a typed theory, claims, obligations, unification claim, and an evidence map; `score_proposal` hashes supplied evidence and lets the deterministic scorer judge it, not the LLM (repo: `crates/openqg-bench/src/zyal_genome/proposer.rs:1-13`, `35-90`). The prompt contract correctly says every parameter must be derived or fundamental, every physics claim needs an obligation, missing evidence is laundering, novelty must be machine-checked, and consistency is a kill gate (repo: `crates/openqg-bench/src/zyal_genome/proposer_prompt.rs:37-88`). The router-native path is also close to the right shape: strict JSON schema, model quality bands, best-of-K sampling, repair attempts, and kill-reason redaction before repair (repo: `proposer_router.rs:63-100`, `309-342`, `365-505`, `525-595`).

But the knowledge layer is absent. The proposal sketch has lane prompts and accepts `extra_sections`, described as “DATA BRIEF / MEMORY” material, yet those sections are untyped strings with no retrieval IDs, source classes, redaction receipts, or replayable query manifests (repo: `crates/openqg-bench/src/zyal_genome/proposer_sketch.rs:499-608`). The relation registry cites literature in Rust comments and recomputes closed forms, but the citations are not machine-checked source evidence (repo: `crates/openqg-core/src/theory/certificate.rs:1-14`, `103-205`). Most dangerously, `LiteratureEquivalence` is only a low-rigor textual attestation: nonempty citation passes. That was acceptable as a placeholder; with retrieval, it becomes an attack surface.

The paper limitations section also admits the exact boundary conditions this knowledge system must not pretend away: background+growth only, no Boltzmann backend, compressed likelihood limits, term grammar limits, and oracle overfitting/repair-loop risks (repo: `paper/main.tex:652-678`, `719-738`).

The internal critique already knows several adjacent problems: the current generator is weak, “derived” often holds for form but not value, Boltzmann coverage is missing, and memory should include derivation lemmas, refuted regions, and live constraints (repo: `docs/zyal-next-level-design.md:40-74`, `106-114`, `249-262`). This S09 spec is narrower: build a literature and campaign-memory system that makes agents better without letting them launder measurements into derivations.

## 2. Corpus acquisition and equation-aware indexing

### 2.1 Source policy and licensing reality

Use four corpus streams, each with an explicit license/source contract in the build manifest.

1. **arXiv bulk source/PDF**. Use arXiv OAI-PMH for metadata deltas and arXiv’s AWS S3 bulk access for source/PDF packages. arXiv states that most submissions use the default arXiv license, metadata is available through APIs, and full-text rights remain with authors; tools based on full text should link back to arXiv and respect API/bulk terms. Therefore, store raw source internally, record license per e-print, and expose only short redacted snippets and source IDs to prompts.
2. **INSPIRE-HEP metadata and citations**. Use `/api/literature`, DOI/arXiv lookup, `fields=citation_count`, references, and citation graph metadata. INSPIRE should rank and connect papers; it is not the equation source of record unless it hosts a structured record.
3. **Living Reviews and review articles**. Living Reviews in Relativity is open access and often supplies LaTeX/PDF; pin review articles as high-trust primers. These are especially valuable for preventing rediscovery loops.
4. **S2ORC/Semantic Scholar Open Data**. Use for OA structured full text and citation graph enrichment where license permits. Do not treat it as authoritative for formulas unless the formula is traced back to source LaTeX/PDF.

External references to pin in `knowledge/sources.yml`: arXiv bulk/API docs; INSPIRE REST API docs; S2ORC (Lo et al., ACL 2020, arXiv:1911.02782); Semantic Scholar Open Data Platform (Kinney et al., arXiv:2301.10140); LaTeXML/NIST documentation; formula retrieval work showing dense retrieval and structural formula search are complementary (Kristianto et al., arXiv:2203.11163).

### 2.2 Lane scope

Do not ingest “all physics” first. Build by lane because the scoring lanes are narrow.

**Dark-scattering lane corpus.** Include interacting dark energy/dark matter, elastic dark scattering, momentum exchange, perturbation equations, Euler equations, stability/sound-speed constraints, growth/RSD, weak lensing, and Boltzmann-code implementations. Seed queries: `dark matter dark energy scattering`, `momentum transfer dark energy Euler equation`, `A_drag`, `interacting dark energy perturbations`, `f sigma 8 dark scattering`. Required seed papers include Simpson 2010 (`arXiv:1007.1034`), Pourtsidou/Skordis/Copeland interacting dark energy work, Planck/late-time growth constraint papers, and review sections on dark-sector interaction stability.

**Planck-μ0 lane corpus.** Include phenomenological modified-gravity parameterizations: μ(a,k), Σ(a,k), EFT/α-basis, Planck modified gravity constraints, and mapping papers. Required seed papers include Planck 2018 (`arXiv:1807.06209`), Planck 2015 modified gravity analyses, Bellini & Sawicki α-functions (`arXiv:1404.3713`), Hu-Sawicki f(R) (`arXiv:0705.1158`), Pogosian/Silvestri parameterized post-Friedmann (`arXiv:0709.0296`). This lane needs a strict source-class warning: `planck_mu0_geff` is a parametrization relation, not a generating mechanism. Its corpus should help proposers stop pretending a posterior μ0 value is a derivation.

**Cross-lane core.** Include DGP/nDGP (`astro-ph/0511634`, `arXiv:0905.0858`), f(R), coupled quintessence, screening mechanisms, BBN constraints, and review articles. These feed relation-registry hardening and “not novel” checks.

Expected initial size: low hundreds of thousands of records, tens of millions of paragraph/equation chunks after filtering, and 1-3 TB of raw/intermediate artifacts if PDFs, source tarballs, XML, embeddings, and indexes are retained. Build cost should be budgeted as a batch job: 64 vCPU for 1-3 days for LaTeX/XML extraction, plus roughly 50-150 GPU-hours for embeddings depending on model and chunk count. Refresh cadence: daily metadata deltas; weekly lane-index rebuild; monthly full reproducibility rebuild with a signed manifest.

### 2.3 Chunking and indexes

Use a LaTeX-native pipeline:

```text
arxiv source/PDF
  -> license + category filter
  -> LaTeXML 0.8.x primary conversion; GROBID/PDF fallback for no-source PDFs
  -> XML document tree with Section, Paragraph, EquationBlock, TableBlock, CitationContext
  -> math normalization: original LaTeX, presentation MathML, content-ish MathML, symbol table
  -> value classifier/redactor
  -> chunk manifest + hybrid indexes
```

Every chunk gets:

```rust
struct CorpusChunk {
    chunk_id: ChunkId,
    source_id: SourceId,
    arxiv_id: Option<String>,
    doi: Option<String>,
    inspire_recid: Option<String>,
    license: LicenseTag,
    category: Vec<String>,
    section_path: Vec<String>,
    chunk_kind: ChunkKind,          // Prose, EquationBlock, TableBlock, ClaimContext
    source_class: SourceClass,      // MechanismEquation, Definition, Parametrization, PosteriorValue, DataVector, ReviewNarrative
    text_redacted: String,
    equation_latex: Vec<String>,
    equation_mathml_sha256: Vec<String>,
    symbol_defs: Vec<SymbolDef>,
    byte_range: (u64, u64),
    raw_content_sha256: String,
    redaction_report_id: RedactionReportId,
}
```

Index with hybrid retrieval, not one embedding. Recommended stack: PostgreSQL 16 for manifests and citation graph, Qdrant 1.14+ for dense/sparse/multivector hybrid retrieval with reciprocal-rank fusion, Tantivy 0.22+ for Rust BM25 and exact symbol search, and a structural formula index using Tangent-S/Tangent-CFT-style formula tuples. Dense embedding model should be open and reproducible: start with `BAAI/bge-m3` for multilingual dense+sparse retrieval; evaluate `nomic-embed-text-v1.5` for cheaper local indexing; keep a formula-specific reranker for equations because generic prose embeddings will blur μ, Σ, β, and Γ symbols. Retrieval returns a `RetrievalPacket`, never raw corpus text.

## 3. Retrieval wired into proposer, repair, and critique

Replace free-form `extra_sections` with a host-generated packet serialized into the prompt. The LLM sees short redacted snippets; the ledger stores full query and source receipts.

```rust
struct RetrievalPacket {
    packet_id: String,
    policy_id: String,
    lane: MechanismLane,
    phase: RetrievalPhase,          // Proposal, Repair, Critique
    query_digest: String,
    token_budget: u32,
    snippets: Vec<RetrievedSnippet>,
    redaction_report_ids: Vec<String>,
    built_at_utc: String,
}

struct RetrievedSnippet {
    retrieval_id: String,
    source_id: String,
    chunk_id: String,
    arxiv_id: Option<String>,
    doi: Option<String>,
    inspire_recid: Option<String>,
    section_path: Vec<String>,
    source_class: SourceClass,
    allowed_use: AllowedUse,        // MechanismOnly, DefinitionAllowed, CertificateEquationAllowed, NarrativeOnly
    equation_ids: Vec<String>,
    text_redacted: String,
    content_sha256: String,
    value_redaction_count: u32,
}
```

Extend `ProposalSketch` and expanded `ProposalDoc`:

```rust
struct MechanismClaimSketch {
    claim_id: String,
    claim: String,
    retrieval_ids: Vec<String>,     // mandatory for physics/mechanism claims
}

struct LiteratureEquationWitness {
    retrieval_id: String,
    equation_id: String,
    relation: String,
    symbol_map: BTreeMap<String, String>,
    normalized_equation_sha256: String,
}
```

Prompt budgets should be lane/phase-specific and capped before model invocation:

| Phase | Prompt contents | Budget |
|---|---|---:|
| Proposal | 6-10 mechanism snippets, 2-4 relation snippets, 3 negative lessons, no posterior/data values | 2,500-3,500 tokens |
| Repair | 3-6 snippets tied to the kill reason; for `UnexplainedModification`, include generating-term and registry relation snippets; for `UnknownRelation`, include valid signatures | 1,200-2,000 tokens |
| Critique | Prior-art, contradiction, stability, “this is just parametrization” snippets; no scoring values | 2,000-3,000 tokens |
| Lessons | Host-distilled cross-campaign lessons relevant to lane/relation/fingerprint | 800-1,500 tokens |

Mandatory provenance rule: a proposal with any physics mechanism claim lacking a valid `retrieval_id` is not merely low-rigor; it is disqualified as `UnprovenancedMechanismClaim`. A proposal may still invent a mechanism, but it must say which retrieved ideas it is combining or explicitly mark `retrieval_ids=[]` and `novel_unanchored=true`; that path receives zero literature credit and must survive extra novelty/prior-art checks.

The ledger adds:

```rust
struct ProposalAttemptRecordV2 {
    // existing fields in theory_population.rs remain
    retrieval_packet_ids: Vec<String>,
    retrieval_token_count: u32,
    redacted_value_count: u32,
    lesson_ids: Vec<String>,
    source_class_counts: BTreeMap<SourceClass, u32>,
    provenance_failures: Vec<String>,
}
```

## 4. Literature-anchored certificates and the new kill rule

Add to `DerivationObligationKind`:

```rust
LiteratureEquationMatch
```

Its witness proves that the relation used in a certificate is actually present in a cited source passage. Verification pipeline:

```text
obligation.citation.retrieval_id
  -> load chunk, source, raw/equation hashes
  -> assert source_class in {MechanismEquation, Definition, Parametrization}
  -> extract equation by equation_id
  -> normalize variables using witness.symbol_map
  -> parse to symbolic form
  -> match against registered relation signature and AST template
  -> if numeric inputs are present, run ValueOriginAudit
  -> return pass/fail + reproducible trace
```

Use a two-tier matcher. Tier 1 is deterministic structural matching: normalized LaTeX/MathML, commutative ordering, macro expansion, α/β/Γ symbol map, and relation signature check. Tier 2 is CAS equivalence: SymPy for algebraic expressions, with an escape hatch for a proof-assistant/CAS service when equivalence is nontrivial (see S03/S04 if those specs own formal proof infrastructure). Do not require Lean for V8 launch; require hashable extraction and symbolic equivalence for registry relations.

The registry relation file should move citations out of comments and into data:

```yaml
- relation: dark_scattering_growth_drag
  signature: {A_drag: number, w0: number, omega_de: number}
  source_anchors:
    - source_id: arxiv:1007.1034
      equation_selector: "Sec. II, momentum-transfer / drag term"
      source_class: MechanismEquation
      allowed_for_certificate: true
  forbidden_inputs:
    - PosteriorValue
    - DataVector
```

**New kill rule: `LiteratureValueLaundering`.** If a proposal cites a paper’s fitted posterior, best-fit, confidence interval, measurement table, covariance entry, or likelihood contour as an input to `DerivedCertificate.inputs`, `DerivedCertificate.expected`, a “derived” parameter value, or a novel prediction value, it dies. Values are data, not derivation. This specifically closes the channel where a Planck/DESI/KiDS/SH0ES paper is used to smuggle target values into the whitebox proof layer. Structural constants in equations remain allowed: `1/(3β)`, `4/3`, scale-factor exponents, and definitions can pass if the source classifier marks them as equation coefficients or definitions.

Acceptance tests:

1. Correct Simpson dark-scattering passage + equation + symbol map passes `LiteratureEquationMatch` for `dark_scattering_growth_drag`.
2. A Planck table containing `mu0 = ...` classified as `PosteriorValue` cannot satisfy a μ0 certificate even if the number matches a proposal.
3. A proposal with `LiteratureEquivalence { citation: "Planck 2018" }` and no passage/equation loses all literature rigor or is killed when it supports a mechanism claim.
4. Editing a source passage after indexing changes `raw_content_sha256` and invalidates replay.

## 5. Cross-campaign learning: deterministic knowledge ledger

“Memory” must not be a chat transcript. It should be a structured, host-owned ledger distilled from scorecards, kill reasons, fingerprints, relations, observables, and accepted retrieval anchors.

```rust
struct KnowledgeLesson {
    lesson_id: String,
    created_from_runs: Vec<String>,
    scope: LessonScope,             // Lane, Relation, Observable, Veto, ParameterRegion
    trigger: LessonTrigger,         // KillPattern, SurvivalPattern, RepairSuccess, PriorArtCollision
    predicates: Vec<Predicate>,     // lane=DarkScattering, relation=dark_scattering_growth_drag, A_drag=constant
    conclusion: LessonConclusion,   // DiesOnGrowthHighZ, NeedsGeneratingTerm, ParametrizationNotMechanism, etc.
    host_template_text: String,
    support: u32,
    lift_vs_background: f64,
    median_score_delta: f64,
    affected_observables: Vec<String>,
    evidence_attempt_ids: Vec<String>,
    retrieval_ids: Vec<String>,
    confidence: LessonConfidence,
    valid_until_snapshot: Option<String>,
    supersedes: Vec<String>,
    contamination_state: RedactionState,
}
```

Distillation is deterministic:

```text
for each completed campaign:
  group attempts by (lane, relation set, mg_family, kill_reason set, observable residual pattern)
  compute support, recurrence across seeds/models, and lift over lane background
  if support >= 20 and seen in >= 3 seeds and contradiction_rate < 0.2:
      instantiate lesson from approved template
      attach attempt IDs, score deltas, and observable classes
      run firewall on lesson text
      index lesson by lane/relation/observable/parameter-region
```

The LLM may propose human-friendly labels for quarantined lessons, but the host writes the accepted lesson text from facts. Example host template:

> In lane `DarkScattering`, attempts using `dark_scattering_growth_drag` with constant `A_drag` and no background coupling term were killed or scored below threshold in {support} attempts. Dominant failure: growth residual at high redshift plus insufficient BAO/background compensation. Future proposals must include a generating dark-sector interaction term and justify the time dependence of Γ(a); do not claim constant drag alone as a mechanism.

Retrieval trigger: before proposal generation, retrieve lessons where `(lane, relation, observable class, parameter region)` overlaps the sketch policy. Negative lessons appear before positive survivors because they prevent repeated waste. Lessons must be value-redacted: ranges like “w0 near the viable background band” are allowed only if they are host-owned priors with an explicit cost; exact fitted values from run champions should not be fed back as derivation hints unless the pilot is explicitly testing exploitation.

Fine-tuning recommendation: **skip physics-outcome RL and LoRA for V8.** Build the knowledge ledger and retrieval first. A small schema/repair LoRA may be considered later if it only learns JSON shape, obligation wiring, and repair syntax; it must not train on score outcomes, fit values, champion parameter values, or holdout results. Reinforcement from kills is especially risky because every historic era’s “truth” later became a regression test. The reversible, inspectable representation is host-authored lessons, not weights.

## 6. Contamination firewall

Be honest: retrieval cannot solve memorization. Frontier LLMs may already know Planck, DESI, SH0ES, and KiDS values from pretraining. The firewall can prevent **new system-provided leakage** and make cheating auditable; it cannot prove the model has no latent memory. The sealed-holdout problem remains and should be mitigated with future/unreleased or internally generated holdouts where possible (see S05/S06 if they own holdout strategy).

Allowed mechanism knowledge:

- symbolic equations and definitions;
- qualitative signs and domains, e.g. “suppressed growth,” “positive drag,” “ghost-free domain”;
- structural coefficients in equations;
- model-class relationships and prior-art novelty warnings.

Banned or priced value knowledge:

- posterior means, best-fit values, confidence intervals, covariance rows, observed BAO/CMB/RSD/H0/S8 values;
- exact benchmark residuals or champion parameter values;
- table values from the same datasets used in the scorecard;
- text like “Planck finds μ0 = ...” when used as derivation input.

Implementation:

```rust
enum NumericOrigin {
    EquationCoefficient,
    DefinitionConstant,
    PhysicalConstantRegistry,
    PosteriorValue,
    ObservedDataValue,
    CovarianceEntry,
    ChampionFitValue,
    UnknownNumeric,
}

struct ValueOriginAudit {
    numeric_literal: String,
    origin: NumericOrigin,
    source_chunk_id: Option<String>,
    allowed_in_certificate: bool,
    allowed_in_prompt: bool,
    redaction_id: Option<String>,
}
```

The redactor uses both structure and language: table/figure captions, headings such as “constraints/results/posterior/best fit,” nearby ±/confidence notation, dataset names, and columns like `D_M/r_d`, `H0`, `S8`, `fσ8`, `Ω_m`, `μ0`. Equation blocks get a different path: preserve symbolic equations, strip empirical substitutions, and classify numbers as coefficients only if they are part of the displayed relation and not in a results section.

Anti-cheating gates after retrieval exists:

1. Every numeric in a derived certificate must have `NumericOrigin` in `{EquationCoefficient, DefinitionConstant, PhysicalConstantRegistry, EngineComputed}`.
2. If a redacted number reappears in proposal text, certificate values, or novel-prediction fields within tolerance, kill as `RedactedValueEcho` unless the host generated it.
3. Self-supplied evidence should no longer be enough for literature claims. The current content-bound evidence map remains useful for local artifacts, but literature evidence must resolve to corpus source IDs.
4. Retrieval packets are replay inputs. A proposal cannot be scored if its packet is missing, redaction receipts fail, or source licenses disallow prompt exposure.

## 7. Pre-registered two-week pilot

**Design.** Two arms, same lanes and models:

- Control: current router proposer, same lane prompts, no corpus retrieval, no lessons.
- Treatment: typed retrieval packets, value firewall, literature-equation witnesses, and deterministic lessons from prior non-pilot runs. No physics fine-tuning.

Freeze before launch: code commit, scorecard, data fixtures, model router configuration, seeds, lane mix, sample count, repair count, retrieval snapshot, and all metrics. Use `PlanckMu0`, `DarkScattering`, `Free`, and `NullDiagnostic` lanes with equal attempt budgets. Suggested budget: 1,200 attempted proposals total, 600 control and 600 treatment, stratified by lane and model quality band. Keep best-of-K and repair settings identical to the current router defaults unless a signed pilot manifest changes both arms symmetrically.

**Primary metrics.** Predeclare all thresholds:

1. **Non-parse kill-rate reduction.** Treatment must reduce non-parse deterministic disqualification rate by at least 15% relative in `DarkScattering` and by at least 8% overall. Parse failures are tracked separately so JSON improvements do not masquerade as physics learning.
2. **Derivation rigor lift.** Median derivation-rigor component among non-killed proposals must rise by ≥3 score points without increasing unsupported claims.
3. **Novel-mechanism rate per 100 proposals.** Host-classified mechanism attempts not already in the relation registry, after prior-art retrieval, must increase by ≥25% or treatment must show a statistically clear reduction in rediscovery/tie-credit attempts.
4. **Tokens per surviving proposal.** Treatment tokens per survivor must be ≤1.5× control. A better kill rate bought with 3× context is not learning efficiency.
5. **Firewall integrity.** Zero survived `LiteratureValueLaundering` or `RedactedValueEcho` incidents. Any survivor with leaked fit data is an automatic pilot failure.

**Decision rule.** Ship V8 retrieval if all five primary thresholds pass and no manual audit finds unledgered source use. Iterate if firewall integrity passes but one of metrics 1-4 misses by less than 25% of its target. Kill or quarantine retrieval if firewall integrity fails, if tokens per survivor exceed 2× control, or if treatment produces more high-confidence prior-art rediscoveries than control.

Secondary metrics: repair success by kill reason, source-class distribution, lessons retrieved per survivor, citations per mechanism claim, equation-match pass rate, and model-quality-band dependence. These inform iteration but cannot override the primary decision rule.

## 8. Interfaces to existing files

- `crates/openqg-bench/src/zyal_genome/proposer_sketch.rs`: replace `extra_sections` with `PromptContext { data_brief, retrieval_packets, knowledge_lessons }`; add schema fields for `mechanism_claims[].retrieval_ids` and literature witnesses.
- `crates/openqg-bench/src/zyal_genome/proposer_prompt.rs`: update rules: literature citations must be retrieval IDs; posterior/data values cannot be derivation inputs; unprovenanced mechanism claims die.
- `crates/openqg-bench/src/zyal_genome/proposer_router.rs`: log packet IDs, token counts, redaction counts, and lesson IDs into `ProposalAttemptRecord` before each call.
- `crates/openqg-core/src/theory/obligation.rs`: add `LiteratureEquationMatch`; demote `LiteratureEquivalence` to narrative-only or make it fail when supporting physics rigor.
- `crates/openqg-core/src/theory/certificate.rs`: move relation literature anchors from comments into machine-readable registry metadata.
- `crates/openqg-core/src/theory/evidence.rs`: add `EvidenceTier::CorpusSource`, `EvidenceTier::RetrievalPacket`, and `EvidenceTier::KnowledgeLesson`; retain content hashes and replay semantics.
- `docs/ZYAL.md` and `docs/zyal-next-level-design.md`: replace aspirational memory language with these durable stores and pilot thresholds.

## What we got wrong

1. **“Literature equivalence” is not evidence.** The current obligation passes on a nonempty citation. Check: mutate the citation to a real title with an unrelated equation; the test still passes. Settling check: after `LiteratureEquationMatch`, wrong-equation citations fail replay.

2. **Code comments are not a relation provenance system.** `certificate.rs` names sources in comments while the oracle recomputes formulas from Rust. Check: change a comment’s citation or remove it; scoring is unaffected. Settling check: relation anchors are YAML/registry data with source hashes, equation selectors, and CI tests.

3. **`extra_sections` is not memory.** It is an untyped prompt string. Check: there is no retrieval packet, source ID, redaction receipt, or replay dependency in the current proposer attempt record. Settling check: replay fails if packet IDs or lesson IDs cannot be resolved.

4. **The Planck-μ0 lane is currently a parametrization lane, not a mechanism lane.** The registry itself gives `planck_mu0_geff` low rigor, and the internal critique warns that derived form is not derived value. Settling check: source classifier marks μ0 posterior/constraint passages as `PosteriorValue` and parameterization equations as `Parametrization`; only a generating theory can earn mechanism credit.

5. **The data brief is already a contamination source.** Telling the proposer which tensions and pulls to aim at may be useful, but it leaks target direction and sometimes values. Settling check: run control prompts with sign-only data briefs versus numeric data briefs; if numeric prompts increase apparent evidence without raising literature-equation rigor, they are optimization hints, not scientific knowledge.

6. **Cross-campaign survival is not learning.** A surviving champion in a ledger is not a reusable lesson unless a future run retrieves a structured, host-distilled fact. Settling check: delete all prior campaign JSONL files and see whether prompt construction changes; today it mostly will not except where manually pasted. With `KnowledgeLesson`s, deletion must change prompts and replay hashes.

7. **The firewall cannot prove the model forgot public cosmology values.** It can only prove the system did not newly provide them and did not accept them as derivations. Settling check: ask the base model for Planck/DESI/SH0ES values outside OpenQG; if it knows them, retrieval redaction is still necessary but not sufficient. Strong holdouts must be post-training, private, or synthetic-to-engine.

8. **“Agents study online physics” is false until the study is measured.** A corpus by itself is not intelligence. Settling check: the 2-week pilot must show lower real kill rate, higher rigor, higher novel-mechanism yield, and bounded tokens per survivor under a pre-registered rule. If not, retrieval is another expensive way to write more convincing wrong answers.
