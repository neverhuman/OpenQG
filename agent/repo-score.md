# jankurai Repo Score

- Standard: `jankurai`
- Auditor: `0.8.13`
- Schema: `1.6.1`
- Paper edition: `2026.05-ed8`
- Target stack ID: `openqg-rust-bench-physics`
- Target stack: `Rust core + TypeScript/React/Vite + PostgreSQL + generated contracts + exception-only Python AI/data service`
- Repo: `.`
- Run ID: `1781213052`
- Started at: `1781213052`
- Elapsed: `4032` ms
- Scope: `full`
- Raw score: `73`
- Final score: `64`
- Decision: `advisory`
- Minimum score: `85`
- Caps applied: `fallback-soup-in-product-code, future-hostile-dead-language-in-product-code, streaming-runtime-drift, repo-rot-bad-behavior`

## Hard Rule Caps

| Rule | Max Score | Applied |
| --- | ---: | --- |
| `no-root-agent-instructions` | 75 | no |
| `no-one-command-setup-or-validation` | 70 | no |
| `no-deterministic-fast-lane` | 65 | no |
| `no-security-lane-on-high-risk-repo` | 60 | no |
| `generated-contracts-or-public-api-drift-untested` | 80 | no |
| `python-direct-product-truth-or-db-ownership` | 72 | no |
| `no-secret-or-dependency-scanning-in-ci` | 78 | no |
| `no-jankurai-audit-lane-in-ci` | 82 | no |
| `jankurai-required-tool-ci-evidence-gap` | 88 | no |
| `non-optimal-product-language-found` | 74 | no |
| `too-much-python-in-product-surface` | 72 | no |
| `boundary-reclassification-evidence-gap` | 72 | no |
| `vibe-placeholders-in-product-code` | 68 | no |
| `fallback-soup-in-product-code` | 70 | yes |
| `future-hostile-dead-language-in-product-code` | 64 | yes |
| `severe-duplication-in-product-code` | 70 | no |
| `generated-zone-mutation-risk` | 76 | no |
| `direct-db-access-from-wrong-layer` | 66 | no |
| `missing-web-e2e-lane` | 82 | no |
| `missing-rendered-ux-qa-lane` | 84 | no |
| `prompt-injection-risk` | 78 | no |
| `overbroad-agent-agency` | 65 | no |
| `secret-like-content-detected` | 60 | no |
| `false-green-test-risk` | 76 | no |
| `destructive-migration-risk` | 70 | no |
| `authz-or-data-isolation-gap` | 78 | no |
| `input-boundary-gap` | 78 | no |
| `agent-tool-supply-chain-gap` | 78 | no |
| `release-readiness-gap` | 80 | no |
| `missing-rust-property-or-integration-tests` | 82 | no |
| `no-agent-friendly-exception-pattern` | 76 | no |
| `missing-agent-readable-docs` | 80 | no |
| `streaming-runtime-drift` | 78 | yes |
| `rust-bad-behavior` | 72 | no |
| `sql-bad-behavior` | 72 | no |
| `typescript-bad-behavior` | 72 | no |
| `docker-bad-behavior` | 72 | no |
| `python-bad-behavior` | 72 | no |
| `ci-bad-behavior` | 70 | no |
| `git-bad-behavior` | 70 | no |
| `gittools-bad-behavior` | 70 | no |
| `release-bad-behavior` | 70 | no |
| `web-security-bad-behavior` | 68 | no |
| `repo-rot-bad-behavior` | 88 | yes |
| `comment-hygiene-dangerous-residue` | 72 | no |
| `ci-local-parity` | 70 | no |

## Copy-Code Redundancy

- Status: `review` hard=`0` warning=`14` files=`103`
- Policy: min-lines=`10` min-tokens=`100` max-findings=`50` include-tests=`false` strict=`false`
- Duplicate volume: lines=`16` tokens=`59` bytes=`601`

- Notes:
  - hard classes are limited to exact active-source file matches and substantial exact same-name units
  - warning classes include same-body different-name units and token/block duplication
  - tests, fixtures, stories, config, Docker, and migrations are omitted unless --include-tests is set

| Kind | Severity | Language | Lines | Tokens | Instances | Reason |
| --- | --- | --- | ---: | ---: | --- | --- |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 3 | `crates/openqg-bench/src/zyal_genome/proposer_memory.rs:629-630, crates/openqg-bench/src/zyal_genome/proposer_memory.rs:661-662, crates/openqg-bench/src/zyal_genome/proposer_memory.rs:708-709, crates/openqg-bench/src/zyal_genome/proposer_memory.rs:721-722` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 2 | 6 | `crates/openqg-bench/src/zyal_judge.rs:35-37, crates/openqg-bench/src/zyal_robustness.rs:380-382` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 3 | `crates/openqg-core/src/theory/proposal.rs:323-324, crates/openqg-core/src/theory/proposal.rs:336-337, crates/openqg-core/src/theory/proposal.rs:349-350` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 2 | 6 | `crates/openqg-bench/src/zyal_judge.rs:32-34, crates/openqg-bench/src/zyal_robustness.rs:384-386` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 6 | `crates/openqg-bench/src/zyal_genome/proposer_router.rs:652-653, crates/openqg-bench/src/zyal_genome/proposer_router.rs:664-665` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/openqg-core/src/theory/unification.rs:206-207, crates/openqg-core/src/theory/unification.rs:347-348` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/openqg-bench/src/zyal_genome/proposer_memory.rs:567-568, crates/openqg-bench/src/zyal_genome/proposer_memory.rs:749-750` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/openqg-core/src/validation/yaml.rs:107-108, crates/openqg-core/src/validation/yaml.rs:112-113` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 5 | `crates/openqg-core/src/theory/unification.rs:249-250, crates/openqg-core/src/theory/unification.rs:287-288` | `same body appears under different names across files` |
| `ExactUnitSameName` | `Warning` | `rust` | 1 | 4 | `crates/openqg-bench/src/zyal_genome/proposer.rs:283-284, crates/openqg-bench/src/zyal_genome/proposer_router.rs:608-609` | `same-name semantic unit copied across multiple files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 4 | `crates/openqg-bench/src/zyal_genome/proposer.rs:374-375, crates/openqg-bench/src/zyal_genome/proposer.rs:398-399` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 3 | `crates/openqg-bench/src/zyal_genome/theory_population.rs:946-947, crates/openqg-bench/src/zyal_genome/theory_population.rs:981-982` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 3 | `crates/openqg-bench/src/zyal_genome/proposer.rs:299-300, crates/openqg-bench/src/zyal_genome/proposer_router.rs:634-635` | `same body appears under different names across files` |
| `ExactUnitDifferentName` | `Warning` | `rust` | 1 | 1 | `crates/openqg-bench/src/zyal_genome/proposer_memory.rs:440-441, crates/openqg-bench/src/zyal_genome/proposer_memory.rs:609-610` | `same body appears under different names across files` |

## Dimensions

| Dimension | Weight | Score | Weighted | Evidence |
| --- | ---: | ---: | ---: | --- |
| Ownership and navigation surface | 13 | 87 | 11.31 | root `AGENTS.md` present; owner map present |
| Contract and boundary integrity | 13 | 86 | 11.18 | contract surface found; generated contract artifacts found |
| Proof lanes and test routing | 12 | 100 | 12.00 | one-command setup/validation lane found; deterministic fast lane found |
| Security and supply-chain posture | 12 | 64 | 7.68 | lockfile present; secret or dependency scan tooling found |
| Code shape and semantic surface | 12 | 3 | 0.36 | largest authored code file: crates/openqg-core/src/theory/league.rs (1056 LOC); code file exceeds 500 LOC |
| Data truth and workflow safety | 8 | 100 | 8.00 | database surface present; migration directory present |
| Observability and repair evidence | 8 | 88 | 7.04 | observability libraries or patterns found; ops/observability directory present |
| Context economy and agent instructions | 7 | 93 | 6.51 | root `AGENTS.md` present; root `AGENTS.md` stays short |
| Jankurai tool adoption and CI replacement | 7 | 29 | 2.03 | control-plane files present; applicable=15 |
| Python containment and polyglot hygiene | 4 | 100 | 4.00 | no Python files in scope |
| Build speed signals | 4 | 80 | 3.20 | build acceleration markers found; targeted test/build commands found |

## Reference Profile Structure

- Applicable cells: `5` canonical=`5` noncanonical=`0` guidance missing=`0`

| Cell | Status | Canonical | Detected | Aliases | Guidance | Owner | Proof lane | Agent fix |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `web` | `not_applicable` | `apps/web/` | `-` | `frontend/, ui/, packages/web/, packages/ui/` | `not_required` | `apps/web` | `rendered UX / Playwright` | `no action` |
| `api` | `not_applicable` | `apps/api/` | `-` | `api/, server/, backend/` | `not_required` | `apps/api` | `edge handler / contract tests` | `no action` |
| `domain` | `canonical` | `crates/domain/` | `crates/domain` | `domain/, core/` | `present` | `crates/domain` | `unit / property tests` | `keep `crates/domain/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `application` | `not_applicable` | `crates/application/` | `-` | `application/, usecases/, use-cases/` | `not_required` | `crates/application` | `use-case / authz tests` | `no action` |
| `adapters` | `not_applicable` | `crates/adapters/` | `-` | `adapters/, infra/, integrations/` | `not_required` | `crates/adapters` | `adapter integration tests` | `no action` |
| `workers` | `not_applicable` | `crates/workers/` | `-` | `workers/, jobs/, scheduler/, queue/` | `not_required` | `crates/workers` | `workflow / replay tests` | `no action` |
| `contracts` | `canonical` | `contracts/` | `contracts` | `openapi/, protobuf/, json-schema/, generated/` | `present` | `contracts` | `generation / drift checks` | `keep `contracts/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `db` | `canonical` | `db/` | `db` | `migrations/, constraints/, sql/` | `present` | `db` | `migration / constraint tests` | `keep `db/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `python-ai` | `canonical` | `python/ai-service/` | `python, python/ai-service` | `python/, ai-service/, evals/, embeddings/, model/` | `present` | `python/ai-service` | `eval / contract tests` | `keep `python/ai-service/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |
| `ops` | `canonical` | `ops/` | `.github, .github/workflows, ops` | `.github/, .github/workflows/, ci/, release/, observability/, security/` | `present` | `ops` | `security lane / workflow lint` | `keep `ops/AGENTS.md` aligned with owns / forbidden / proof lane guidance` |

## Rendered UX QA

- Web surface: `false`
- Layered UX lane: `true`
- Missing: `none`

## Tool Adoption

- Control plane present: `true`
- Applicable tools: `15`
- Configured: `14`
- CI evidence: `0`
- Artifact verified: `0`
- Replaced count: `0`
- Missing CI evidence: `audit-ci, proof-routing, proofbind, proofmark-rust, copy-code, security, ci-bad-behavior, git-bad-behavior, release-bad-behavior, contract-drift, rust-witness, authz-matrix, agent-tool-supply, release-readiness, cost-budget`

| Tool | Category | Mode | Status | Replaced | Artifacts |
| --- | --- | --- | --- | --- | --- |
| `audit-ci` | `audit` | `auto` | `configured` | `manual repo scoring, ad hoc score gates` | `agent/repo-score.json, agent/repo-score.md` |
| `proof-routing` | `proof` | `auto` | `configured` | `ad hoc proof lane selection, manual proof receipts` | `agent/repo-score.json, agent/repo-score.md, target/jankurai/repair-queue.jsonl` |
| `proofbind` | `proof` | `auto` | `configured` | `manual changed-surface routing, ad hoc proof obligation lists` | `target/jankurai/proofbind/surface-witness.json, target/jankurai/proofbind/obligations.json` |
| `proofmark-rust` | `proof` | `auto` | `configured` | `line-only coverage review, manual in-diff mutation review` | `target/jankurai/proofmark/proofmark-receipt.json, target/jankurai/proofmark/proof-receipt.json` |
| `copy-code` | `audit` | `auto` | `missing` | `ad hoc copy-code review, manual duplication triage` | `target/jankurai/copy-code.json, target/jankurai/copy-code.md` |
| `security` | `security` | `auto` | `configured` | `gitleaks, dependency review, SBOM/provenance` | `target/jankurai/security/evidence.json` |
| `ci-bad-behavior` | `security` | `auto` | `configured` | `mutable workflow refs, secret echo/debug workflow checks, non-blocking security scans` | `target/jankurai/language-bad-behavior.log` |
| `git-bad-behavior` | `audit` | `auto` | `configured` | `destructive git automation, force-push release scripts, hidden stash-based state` | `target/jankurai/language-bad-behavior.log` |
| `release-bad-behavior` | `release` | `auto` | `configured` | `manual release checklist, ad hoc tag and artifact review, manual provenance review` | `target/jankurai/language-bad-behavior.log` |
| `ux-qa` | `ux` | `auto` | `not_applicable` | `playwright, axe-core, visual baselines` | `target/jankurai/ux-qa.json` |
| `db-migration-analyze` | `db` | `auto` | `not_applicable` | `manual migration review` | `target/jankurai/migration-report.json` |
| `contract-drift` | `contract` | `auto` | `configured` | `handwritten contract drift checks, openapi diff` | `agent/repo-score.json, agent/repo-score.md` |
| `rust-witness` | `rust` | `auto` | `configured` | `manual witness graphing` | `target/jankurai/rust/witness-graph.json` |
| `vibe-coverage` | `audit` | `auto` | `not_applicable` | `manual vibe-coding coverage spreadsheet` | `target/jankurai/vibe-coverage.json, target/jankurai/vibe-coverage.md` |
| `coverage-evidence` | `proof` | `auto` | `not_applicable` | `manual coverage report review, ad hoc mutation survivor review` | `target/jankurai/coverage/coverage-audit.json, target/jankurai/coverage/coverage-audit.md` |
| `authz-matrix` | `security` | `auto` | `configured` | `manual authz matrix review` | `agent/repo-score.json, agent/repo-score.md` |
| `input-boundary` | `security` | `auto` | `not_applicable` | `manual unsafe sink review` | `agent/repo-score.json, agent/repo-score.md` |
| `agent-tool-supply` | `security` | `auto` | `configured` | `manual MCP/tool trust review` | `agent/repo-score.json, agent/repo-score.md` |
| `release-readiness` | `release` | `auto` | `configured` | `manual launch checklist` | `agent/repo-score.json, agent/repo-score.md` |
| `cost-budget` | `release` | `auto` | `configured` | `manual spend review` | `agent/repo-score.json, agent/repo-score.md` |

## Boundary manifest (ingested)

- Path: `agent/boundaries.toml`
- Stack: `openqg-rust-bench-physics` · version: `0.1.0`
- Queue path counts — adapter: `2`, event_contract: `1`, generated_type: `1`, client_marker: `1`, streaming_exception: `0`
- Content fingerprint: `sha256:7cd373de06ea30f137fc1d95cc63efca5962462a47021f0a6781bede0f7f5993`

## Boundary Reclassifications

No audited runtime boundary reclassifications declared.

## Findings

1. `medium` `shape` `.`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:shape` `soft` confidence `0.76`
   Route: TLR `Entropy`, lane `fast`, owner `tools`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: `Code shape and semantic surface` scored 3 below the standard floor of 85
   Fix: split large or ambiguous authored code into smaller semantic modules with focused tests
   Rerun: `just fast`
   Fingerprint: `sha256:05df968c14a80e4b92039efa256ca0cd05c1018144351d04bc8bcd2bc2adb9b9`
   Evidence: largest authored code file: crates/openqg-core/src/theory/league.rs (1056 LOC), code file exceeds 500 LOC, code file exceeds 1000 LOC, most code files stay under 300 LOC
2. `medium` `security` `.github/workflows/jankurai.yml`
   Rule: `HLT-016-SUPPLY-CHAIN-DRIFT`
   Check: `HLT-016-SUPPLY-CHAIN-DRIFT:security` `soft` confidence `0.76`
   Route: TLR `Security, secrets, agency`, lane `security`, owner `ops`
   Docs: `docs/audit-rubric.md#top-level-risk-mapping`
   Reason: `Security and supply-chain posture` scored 64 below the standard floor of 85
   Fix: wire secret, dependency, provenance, and workflow scans into an operational CI lane
   Rerun: `just security`
   Fingerprint: `sha256:30aecd17f8c38b130e9fffe5ddb77283bfc0e7a20c1ddb0acb233c5951f0c738`
   Evidence: lockfile present, secret or dependency scan tooling found, security lane present, canonical security lane wrapper present
3. `medium` `proof` `Justfile`
   Rule: `HLT-018-PERF-CONCURRENCY-DRIFT`
   Check: `HLT-018-PERF-CONCURRENCY-DRIFT:proof` `soft` confidence `0.76`
   Route: TLR `Verification`, lane `fast`, owner `workspace`
   Docs: `docs/testing.md`
   Reason: `Build speed signals` scored 80 below the standard floor of 85
   Fix: add fast deterministic build/test targets, caches, and narrow proof lanes for agent iteration
   Rerun: `just fast`
   Fingerprint: `sha256:2f2531223d7f7036c20d44b58cd52e64aa53ffd6cb85e01e541c1feff0c09cb2`
   Evidence: build acceleration markers found, targeted test/build commands found, locked dependency graph present, CI cache hint found
4. `high` `generated` `agent/jankurai-badge.json:1`
   Rule: `HLT-002-GENERATED-MUTATION`
   Check: `HLT-002-GENERATED-MUTATION:generated` `hard` confidence `0.95`
   Route: TLR `Contracts/data`, lane `contract`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#generated-zones`
   Reason: generated zone file `agent/jankurai-badge.json` missing generated header
   Fix: add a `Generated by: <tool>` / `DO NOT EDIT BY HAND` header block with source and regeneration command
   Rerun: `just fast`
   Fingerprint: `sha256:528ffc186c7616233838b4f8d25fc77f79691d13470bfbf075e9a84f490450f6`
   Evidence: generated zone integrity violation
5. `high` `generated` `agent/jankurai-badge.svg:1`
   Rule: `HLT-002-GENERATED-MUTATION`
   Check: `HLT-002-GENERATED-MUTATION:generated` `hard` confidence `0.95`
   Route: TLR `Contracts/data`, lane `contract`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#generated-zones`
   Reason: generated zone file `agent/jankurai-badge.svg` missing generated header
   Fix: add a `Generated by: <tool>` / `DO NOT EDIT BY HAND` header block with source and regeneration command
   Rerun: `just fast`
   Fingerprint: `sha256:5d16dc47c5408e5cc58f5b3307765240a53705af82dcc237a551ff5a6f1fdcb1`
   Evidence: generated zone integrity violation
6. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `managed-browsers.json` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:5242ae55ab9a1f2c29ad9620261081cdc57cbcbcb54e2c4ab8f6ddb2dce98e35`
   Evidence: managed-browsers.json
7. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `paper/README.md` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:eabdf8d61ea271dccf3ffe0c4827f356a96b35abc169e8ef2b7f57d0965fcf1b`
   Evidence: paper/README.md
8. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `paper/data/campaigns.json` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:fd1f1449071c3f500e2c72297fc9a99ddf84c3ffd8f84f93de33db5860989e38`
   Evidence: paper/data/campaigns.json
9. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `paper/data/funnels.json` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:7329910c7bede7df9f7c092409bbabc725052c9cbeeaaa197806bea6ac7d738c`
   Evidence: paper/data/funnels.json
10. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `paper/data/observables.jsonl` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:fdde0612cff3526c327f7ab5b8abbe161d5b0a038a6c14fa53972839d16ce061`
   Evidence: paper/data/observables.jsonl
11. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `paper/data/predictions.txt` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:fdbc09cb7d3c75b28f9d023d94789f99b521afb2c526cb4a1d9fb649d530c750`
   Evidence: paper/data/predictions.txt
12. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `paper/data/story.json` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:1f3f004ceb8af013b2e8395549fbf5692834079b53c875f2d30fed920c959bef`
   Evidence: paper/data/story.json
13. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `paper/figs/fig1_architecture.pdf` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:6c54852ec841bddae95c87b4aac777881e4be9adaff5154c9c0e11a745334b4b`
   Evidence: paper/figs/fig1_architecture.pdf
14. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `paper/figs/fig1_architecture.png` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:5ad5bbf86ce3ea8fcc121f9c3d51a996f9ff38f4257efba122ac841b615e3f78`
   Evidence: paper/figs/fig1_architecture.png
15. `high` `context` `agent/owner-map.json`
   Rule: `HLT-003-OWNERLESS-PATH`
   Check: `HLT-003-OWNERLESS-PATH:context` `hard` confidence `0.88`
   Route: TLR `Context/setup`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#ownership-boundaries`
   Reason: path `paper/figs/fig2_timeline.pdf` has no owner-map route
   Fix: add the narrowest stable prefix for this path to `agent/owner-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:1ebe6a4e98099dfacb8c7a65f8e20e74b04ff39d0180ab706bc3741aedeae1fe`
   Evidence: paper/figs/fig2_timeline.pdf
16. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `data/manifest.json` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:b6412c1fc3146f790c23d33e469c25bc1e89792fc71c3cf43045bca8c062b0ac`
   Evidence: data/manifest.json
17. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `managed-browsers.json` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:bbba2d176d28f3c27986a5ff490ac539b7191fa46b985d840a597c996924b784`
   Evidence: managed-browsers.json
18. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `ops/jailgun/next-level-manifest.txt` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:ce93f0f7c9cfa4a0c6c71833a6e33cd5c094a0fdc8aa93bd6da486e2c794da38`
   Evidence: ops/jailgun/next-level-manifest.txt
19. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `ops/jailgun/next-level-payload/results/prod-1000-champion.json` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:85256cb332aab8a02e0d014cd0026c8082efe0f378ede101521c2d0e6d2f982e`
   Evidence: ops/jailgun/next-level-payload/results/prod-1000-champion.json
20. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `ops/jailgun/next-level-payload/results/prod-1000-quality-gate.json` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:19931ff96491b96a3ea67446196cde544f7e80467b8dca945c9153d88746d8bc`
   Evidence: ops/jailgun/next-level-payload/results/prod-1000-quality-gate.json
21. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `ops/jailgun/next-level-payload/results/prod-1000-run-summary.json` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:48d0612418e5f8cd9e82619e101586e3d93a05b87a9081e51836588da22054f3`
   Evidence: ops/jailgun/next-level-payload/results/prod-1000-run-summary.json
22. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `ops/jailgun/next-level-payload/results/v6-replay-champion.json` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:93813f06a33311cb5a0d2cf3628e3d27abfdf00e487a39476a8e2c5aa571f0f5`
   Evidence: ops/jailgun/next-level-payload/results/v6-replay-champion.json
23. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `ops/jailgun/next-level-payload/results/v6-replay-quality-gate.json` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:97c16b6b0673ebafee9a26164f2ad1cae92b69cb7b1f76c8e8ad1df1d606199a`
   Evidence: ops/jailgun/next-level-payload/results/v6-replay-quality-gate.json
24. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `ops/jailgun/next-level-payload/results/v6-replay-run-summary.json` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:752760525afb68d672797ddf54c390d99d4877dee08b6cd95d971df763e89d9c`
   Evidence: ops/jailgun/next-level-payload/results/v6-replay-run-summary.json
25. `high` `proof` `agent/test-map.json`
   Rule: `HLT-004-UNMAPPED-PROOF`
   Check: `HLT-004-UNMAPPED-PROOF:proof` `hard` confidence `0.88`
   Route: TLR `Verification`, lane `fast`, owner `agent`
   Docs: `agent/JANKURAI_STANDARD.md#proof-lanes`
   Reason: path `ops/jailgun/next-level-payload/results/v7-smoke-progress-ledger.jsonl` has no test-map proof route
   Fix: add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Rerun: `just fast`
   Fingerprint: `sha256:42075573410b2222d55fae857abb4dfa43d79da42d018ba73c36c005d218ea94`
   Evidence: ops/jailgun/next-level-payload/results/v7-smoke-progress-ledger.jsonl
26. `high` `vibe` `crates/openqg-bench/src/zyal_genome/physics_score.rs:20`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `workspace`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `fallback` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:ce62e92e9408b6ea489b382bb61b79969c0f4836ab4db05fcd2e17660afc03d2`
   Evidence: crates/openqg-bench/src/zyal_genome/physics_score.rs:20, future-hostile/dead-language term `fallback` appears
27. `high` `vibe` `crates/openqg-bench/src/zyal_genome/physics_score.rs:24`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `workspace`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `fallback` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:0f5cea9ff92a142305a7d7a20ea772f7e1553c32b26d8bfcbef29bc3bb6ea214`
   Evidence: crates/openqg-bench/src/zyal_genome/physics_score.rs:24, future-hostile/dead-language term `fallback` appears
28. `medium` `proof` `crates/openqg-bench/src/zyal_genome/physics_score.rs:618`
   Rule: `HLT-027-HUMAN-REVIEW-EVIDENCE-GAP`
   Check: `HLT-027-HUMAN-REVIEW-EVIDENCE-GAP:proof` `soft` confidence `0.88`
   Route: TLR `Repair`, lane `audit`, owner `workspace`
   Docs: `docs/testing.md`
   Matched term: `review evidence`
   Reason: proof and review claims need receipts
   Fix: attach raw CI logs, review receipts, and replayable commands instead of accepting claims or summaries
   Rerun: `just score`
   Fingerprint: `sha256:bbe42eb61ec4de9b45e5171bf1e1b561f7b4525eb73181f9c2ada7f1300c9007`
   Evidence: fn fabricated_witness_values_are_a_kill() {
29. `high` `vibe` `crates/openqg-bench/src/zyal_genome/proposer_memory.rs:57`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `workspace`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: fallback soup detected in product code
   Fix: collapse fallback chains into explicit typed states with bounded retry policy, telemetry, and documented repair guidance
   Rerun: `just fast`
   Fingerprint: `sha256:fa35814e3f735e0d85d2a85441eb12b6adcc2e9adc73e4aa5c89d6bdef057d72`
   Evidence: crates/openqg-bench/src/zyal_genome/proposer_memory.rs:57 .unwrap_or_default();
30. `high` `vibe` `crates/openqg-bench/src/zyal_genome/proposer_sketch.rs:590`
   Rule: `HLT-001-DEAD-MARKER`
   Check: `HLT-001-DEAD-MARKER:vibe` `hard` confidence `0.88`
   Route: TLR `Entropy`, lane `fast`, owner `workspace`
   Docs: `docs/audit-rubric.md#future-hostile-language-rule`
   Reason: future-hostile/dead-language term `unused` appears in product/runtime code
   Fix: remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Rerun: `just fast`
   Fingerprint: `sha256:4a9e68b7aa7b526cddf03e8a6a91bf98fdaf54ecdbf2dcbaa314e4a5c99404e4`
   Evidence: crates/openqg-bench/src/zyal_genome/proposer_sketch.rs:590, future-hostile/dead-language term `unused` appears
31. `medium` `context` `crates/openqg-bench/tests-fixtures/v5-champion-chunk1.json:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `workspace`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:674902a706fa5dd146faf1b024b03540dbdeaa6e307bf66c3510355574184d46`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=crates/openqg-bench/tests-fixtures/v5-champion-chunk1.json, line=1, proof_window=None, snippet={
32. `medium` `context` `crates/openqg-bench/tests-fixtures/v6-survivor-chunk6.json:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `workspace`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:3ff696410b2cbf9d55b5870f60b9025c2589dd295322eebfbb0d8811a0622bf6`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=crates/openqg-bench/tests-fixtures/v6-survivor-chunk6.json, line=1, proof_window=None, snippet={
33. `high` `boundary` `crates/openqg-core/src/cosmology/forward.rs:94`
   Rule: `HLT-019-STREAMING-RUNTIME-DRIFT`
   Check: `HLT-019-STREAMING-RUNTIME-DRIFT:boundary` `hard` confidence `0.95`
   Route: TLR `Contracts/data`, lane `db`, owner `workspace`
   Docs: `docs/streaming.md`
   Reason: queue or streaming runtime client appears outside the declared adapter boundary
   Fix: move Kafka/Tansu/Iggy/Fluvio/NATS/Redis-stream clients behind `crates/adapters/queues` or document a brownfield exception with owner, expiry, and migration path
   Rerun: `just fast`
   Fingerprint: `sha256:98203e905a6cc16536bd7994e145657ee9e8ba1deb456c48718de556b6e48c49`
   Evidence: streaming client marker `nats` appears outside `crates/adapters/queues`
34. `medium` `context` `ops/jailgun/next-level-payload/results/v6-replay-champion.json:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `ops`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:548fdc312d093e22444cf6bfa1dfe7c962c48fb19a053f1e85a8a8ed41ecd0ca`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=ops/jailgun/next-level-payload/results/v6-replay-champion.json, line=1, proof_window=None, snippet={
35. `medium` `context` `ops/jailgun/next-level-payload/results/v6-replay-quality-gate.json:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `ops`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:ba3aa63d28f1ce240be22042187d42082d9637de225fe01893704702ee880e7b`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=ops/jailgun/next-level-payload/results/v6-replay-quality-gate.json, line=1, proof_window=None, snippet={
36. `medium` `context` `ops/jailgun/next-level-payload/results/v6-replay-run-summary.json:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `ops`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:bc227d4cf43c0ec0f930a7c3bdb6afc76133c904ff5eb7974b776a547931eb7e`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=ops/jailgun/next-level-payload/results/v6-replay-run-summary.json, line=1, proof_window=None, snippet={
37. `medium` `context` `ops/jailgun/next-level-payload/results/v7-smoke-progress-ledger.jsonl:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `ops`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:3c7e42e9c78df64b65f277554d327ca74455f6ffe537ea0385f131c8b89b1cde`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=ops/jailgun/next-level-payload/results/v7-smoke-progress-ledger.jsonl, line=1, proof_window=None
38. `medium` `context` `ops/jailgun/next-level-payload/results/v7-smoke-proposal-ledger.jsonl:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `ops`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:6b46498db8d2cecc88426f22bace6c49c564f8f20a4ffcda4c21ff7621abbea3`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=ops/jailgun/next-level-payload/results/v7-smoke-proposal-ledger.jsonl, line=1, proof_window=None
39. `medium` `context` `ops/jailgun/next-level-payload/results/v7-tg-trust-gate.json:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `ops`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:ca4c3fa980d54b667ce409f44a79c49983c9bf7b6f0e261f7f5e1ce22f585a83`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=ops/jailgun/next-level-payload/results/v7-tg-trust-gate.json, line=1, proof_window=None, snippet={
40. `medium` `context` `ops/replay/expected-league-v2.json:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `ops`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:3e56f6481ddf95a0cf1b3faa3cce007cacd96ee1d0ae53f60a6efec2d8bd004c`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=ops/replay/expected-league-v2.json, line=1, proof_window=None, snippet={
41. `medium` `context` `ops/v5-chunked-campaign.sh:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `ops`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:fe400becf4c1a94f19e0cf8756d2d3a985958792febfd588366752e640caf803`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=ops/v5-chunked-campaign.sh, line=1, proof_window=None, snippet=#!/usr/bin/env bash
42. `medium` `context` `ops/v6-campaign.sh:1`
   Rule: `HLT-040-REPO-ROT-BAD-BEHAVIOR`
   Check: `HLT-040-REPO-ROT-BAD-BEHAVIOR:context` `soft` confidence `0.88`
   Route: TLR `Context/setup`, lane `audit`, owner `ops`
   Docs: `docs/language-bad-behavior.md#web-security-and-repo-rot-detectors`
   Matched term: `repo-rot.path.fake-versioned-source`
   Reason: ambiguous old-looking active source makes agents and reviewers guess whether code is live
   Fix: delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Rerun: `just score`
   Fingerprint: `sha256:799d637987ac66a7b47f16f61285a3dac0bdf7e53ffbb8dd793aa40f5bf4a6f5`
   Evidence: detector=repo-rot.path.fake-versioned-source, path=ops/v6-campaign.sh, line=1, proof_window=None, snippet=#!/usr/bin/env bash

## Policy

- Policy file: `./agent/audit-policy.toml`
- Minimum score: `85`
- Fail on: ``

## Agent Fix Queue

1. `high` `HLT-002-GENERATED-MUTATION` `agent/jankurai-badge.json` - add a `Generated by: <tool>` / `DO NOT EDIT BY HAND` header block with source and regeneration command
   Route: `Contracts/data`/`contract`
2. `high` `HLT-002-GENERATED-MUTATION` `agent/jankurai-badge.svg` - add a `Generated by: <tool>` / `DO NOT EDIT BY HAND` header block with source and regeneration command
   Route: `Contracts/data`/`contract`
3. `high` `HLT-019-STREAMING-RUNTIME-DRIFT` `crates/openqg-core/src/cosmology/forward.rs` - move Kafka/Tansu/Iggy/Fluvio/NATS/Redis-stream clients behind `crates/adapters/queues` or document a brownfield exception with owner, expiry, and migration path
   Route: `Contracts/data`/`db`
4. `high` `HLT-004-UNMAPPED-PROOF` `agent/test-map.json` - add the narrowest stable prefix and runnable proof command to `agent/test-map.json`
   Route: `Verification`/`fast`
5. `medium` `HLT-018-PERF-CONCURRENCY-DRIFT` `Justfile` - add fast deterministic build/test targets, caches, and narrow proof lanes for agent iteration
   Route: `Verification`/`fast`
6. `medium` `HLT-027-HUMAN-REVIEW-EVIDENCE-GAP` `crates/openqg-bench/src/zyal_genome/physics_score.rs` - attach raw CI logs, review receipts, and replayable commands instead of accepting claims or summaries
   Route: `Repair`/`audit`
7. `high` `HLT-003-OWNERLESS-PATH` `agent/owner-map.json` - add the narrowest stable prefix for this path to `agent/owner-map.json`
   Route: `Context/setup`/`fast`
8. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `crates/openqg-bench/tests-fixtures/v5-champion-chunk1.json` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
9. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `crates/openqg-bench/tests-fixtures/v6-survivor-chunk6.json` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
10. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `ops/jailgun/next-level-payload/results/v6-replay-champion.json` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
11. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `ops/jailgun/next-level-payload/results/v6-replay-quality-gate.json` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
12. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `ops/jailgun/next-level-payload/results/v6-replay-run-summary.json` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
13. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `ops/jailgun/next-level-payload/results/v7-smoke-progress-ledger.jsonl` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
14. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `ops/jailgun/next-level-payload/results/v7-smoke-proposal-ledger.jsonl` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
15. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `ops/jailgun/next-level-payload/results/v7-tg-trust-gate.json` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
16. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `ops/replay/expected-league-v2.json` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
17. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `ops/v5-chunked-campaign.sh` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
18. `medium` `HLT-040-REPO-ROT-BAD-BEHAVIOR` `ops/v6-campaign.sh` - delete the stale copy, move history to VCS/archive tooling, or document owner, proof lane, expiry, and migration plan
   Route: `Context/setup`/`audit`
19. `high` `HLT-001-DEAD-MARKER` `crates/openqg-bench/src/zyal_genome/physics_score.rs` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
20. `high` `HLT-001-DEAD-MARKER` `crates/openqg-bench/src/zyal_genome/proposer_memory.rs` - collapse fallback chains into explicit typed states with bounded retry policy, telemetry, and documented repair guidance
   Route: `Entropy`/`fast`
21. `high` `HLT-001-DEAD-MARKER` `crates/openqg-bench/src/zyal_genome/proposer_sketch.rs` - remove or rename the marker, implement the intended behavior, model a typed unsupported state, or move docs/generated/vendor/product-copy text into an allowlisted context
   Route: `Entropy`/`fast`
22. `medium` `HLT-001-DEAD-MARKER` `.` - split large or ambiguous authored code into smaller semantic modules with focused tests
   Route: `Entropy`/`fast`
23. `medium` `HLT-016-SUPPLY-CHAIN-DRIFT` `.github/workflows/jankurai.yml` - wire secret, dependency, provenance, and workflow scans into an operational CI lane
   Route: `Security, secrets, agency`/`security`
