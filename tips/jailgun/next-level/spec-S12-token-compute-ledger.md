# RANKED BACKLOG — S12 token/compute ledger

This spec is scoped to the uploaded OpenQG/ZYAL snapshot. I read the required source files before writing it. The relevant current facts are: `proposer_router.rs` defines an OpenAI-compatible jnoccio transport request with `max_completion_tokens`, but `RouterResponse` only returns content, model, schema status, upstream repairs, elapsed seconds, and HTTP status (`crates/openqg-bench/src/zyal_genome/proposer_router.rs:32-58,117-180`). Attempt records include outcome, kill reasons, total score, raw SHA-256, raw length, elapsed seconds, winner flag, model, quality band, mechanism lane, and upstream repairs, but no token, cost, provider, or pricing fields (`crates/openqg-bench/src/zyal_genome/theory_population.rs:113-140`; `proposer_router.rs:309-343`). `ledger_sink.rs` streams proposal, attempt, progress, and checkpoint JSONL files only (`crates/openqg-bench/src/zyal_genome/ledger_sink.rs:20-83`). `docs/ZYAL.md` names two LLM backends, jnoccio and the jailgun browser bridge, and already claims budgets include cost/tokens at the broader ZYAL layer (`docs/ZYAL.md:70-90,115-136`). `paper/main.tex` claims “total observability,” but defines it as hashes, lengths, model identity, lane, band, and repair count rather than usage/cost telemetry (`paper/main.tex:629-636`).

| Rank | Change | Why it matters | Effort | Hostile-review acceptance test |
|---:|---|---|---:|---|
| 1 | Add a mandatory `LlmCallReceipt` with exact provider usage or explicit local-estimate fields on every jnoccio and jailgun call. | The present ledgers can explain yield, not economics. An API call without `prompt_tokens`, `completion_tokens`, `cached_tokens`, `reasoning_tokens`, provider, model, latency, and cost is a spend leak. | M | A fixture campaign with exact OpenAI-compatible `usage`, a jailgun response without usage, a timeout-after-send, and a transport-not-sent error emits no LLM receipt missing either exact usage or `usage_estimated=true` plus tokenizer/method. |
| 2 | Extend native JSONL ledgers as the source of truth: new `token-ledger.jsonl` events plus backward-compatible fields on `proposal-attempts.jsonl`. | Replay must remain deterministic and independent of Langfuse/Phoenix/OTel collectors. Native ledgers already survive killed processes; extend that contract, do not replace it. | M | `zyal genome replay` on old ledgers and new ledgers reports identical score results; disabling every external observability endpoint does not affect campaign completion or replay. |
| 3 | Build a derived longitudinal token/time-series store and CLI query interface. | The team explicitly wants “over all time,” not per-run anecdotes. JSONL is canonical, but reports need fast aggregation by model, provider, lane, island, generation, and repair round. | M | `zyal tokens ingest runs/**/token-ledger.jsonl --db token_rollups.sqlite` then SQL/CLI queries reproduce totals in the campaign report byte-for-byte. |
| 4 | Add campaign report sections for compute integrity, token/cost totals, kill economics, score productivity, and repair overhead. | The V6/V7 reports show yield funnels but cannot answer “what did it cost to kill this idea?” or “which model bought real score?” | M | A generated report contains total tokens by provider/model/lane with `unattributed_calls = 0`, token-to-kill/token-to-survive tables, cost-per-adjudicated-proposal, and marginal-token-productivity. |
| 5 | Replace win-rate-only routing with budget-aware routing over posterior yield per dollar/token/latency. | The current router only rotates quality bands, lanes, sample count, and `max_completion_tokens`; the paper describes win-rate routing, not cost-aware routing (`paper/main.tex:175-180,377-380`). | M | A synthetic router test with one expensive high-win model and one cheap modest-win model chooses the model with higher expected value per budget unit under a fixed campaign budget, while retaining exploration. |
| 6 | Backfill V5/V6/V7 historical campaigns with explicit confidence classes. | The team needs trend lines, but old attempt records only preserve raw lengths/hashes/call counts and sometimes model IDs. Pretending exact recovery would poison the economics. | S/M | Backfill emits `usage_source="historical_backfill"`, `backfill_quality`, confidence intervals, and refuses to label any length-only estimate as exact. |
| 7 | Add a pricing catalog with effective dates and immutable pricing snapshots. | USD cost without pricing provenance is not reproducible; providers change prices. | S | Recomputing historical cost from `token-ledger.jsonl` plus the referenced pricing snapshot hash produces identical micro-USD totals. |
| 8 | Export optional OpenTelemetry GenAI spans/metrics, with Langfuse/Phoenix as dashboards only. | OTel gives standard names for input/output/cache/reasoning tokens, and Langfuse/Phoenix are useful UIs, but neither can be load-bearing for scientific replay. | S | A campaign with OTLP collector down logs a warning, writes native JSONL, finishes normally, and replays normally. |
| 9 | Run token-efficiency experiments: prompt caching, repair overhead, strict sketch contract, and retrieval budgets. | Cost telemetry is only useful if it changes behavior. The V6 materials say repairs converted kills to scored proposals; S12 must quantify whether those repairs remain worth their tokens. | M | A/B runs publish pre-registered metrics and thresholds: cache-hit savings, repair-yield per 100k tokens, strict-schema waste reduction, and retrieval marginal yield. |

## 1. Current implementation gap

The router transport is disciplined in the ways that mattered for V6/V7: schema validation, no stdout scraping, deterministic lane assignment, best-of-K samples, and per-attempt hashes. It is not an LLM-ops ledger. `RouterRequest` carries `max_completion_tokens`, temperature, schema, sample, and attempt identifiers; `RouterResponse` projects away the OpenAI-compatible response `usage` object. The default caller parses `choices[0].message.content`, jnoccio metadata, winner model, structured schema status, and repair attempts, then returns without inspecting `v["usage"]`. Therefore any provider that already returns exact usage is silently discarded.

`Proposer::drain_attempts` also defaults to an empty vector (`crates/openqg-bench/src/zyal_genome/proposer.rs:93-104`). The current `RouterProposer` overrides it, but the trait permits future proposer backends to perform live LLM calls with no attempt receipts at all. That is incompatible with “every token attributed.” For V8, live LLM access must move behind a receipt-enforcing interface: no backend should be able to call an LLM unless it returns a `LlmCallReceipt` or a typed `LlmCallNotSent` event.

The paper’s “cost profile” says proposal-side inference ran on free-tier routed models and the only billable usage was external referee cycles (`paper/main.tex:646-650`). That may be true operationally for the reported run, but it is not an accounting model. Free-tier tokens are still scarce quota, opportunity cost, latency, provider risk, and potentially browser-session credits. V8 should price every call in one of three ways: exact USD from a provider price table, estimated USD from a pinned public/internal price table, or `unpriced=true` with provider/model/token totals still present. “Free” is a price value, not a missing row.

## 2. Token ledger data model

Add a crate-local module, for example `crates/openqg-bench/src/zyal_genome/token_ledger.rs`, with serializable structs. Use integers for tokens, milliseconds, and micro-USD to avoid float drift in replayable reports. Keep old fields in `ProposalAttemptRecord`; add new fields with `#[serde(default)]` so old V6/V7 ledgers replay unchanged.

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSource {
    ProviderExact,        // provider/gateway returned a usage object
    GatewayExact,         // jnoccio normalized usage from upstream provider
    LocalTokenizerEstimate,
    HistoricalBackfill,
    NotSent,              // local validation/client error before the request left host
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cached_tokens: u64,
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
    pub usage_source: UsageSource,
    pub usage_estimated: bool,
    pub tokenizer_name: Option<String>,
    pub tokenizer_sha256: Option<String>,
    pub estimation_method: Option<String>,
    pub coverage_gaps: Vec<String>, // e.g. browser hidden prompt, timeout completion unknown
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CostUsage {
    pub usd_micro: Option<i64>,
    pub unpriced: bool,
    pub pricing_table_sha256: Option<String>,
    pub priced_at_utc: Option<String>,
    pub input_usd_per_mtok: Option<String>,
    pub cached_input_usd_per_mtok: Option<String>,
    pub output_usd_per_mtok: Option<String>,
    pub reasoning_usd_per_mtok: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmCallReceipt {
    pub event_type: String,          // llm_call.completed, llm_call.estimated, llm_call.error
    pub schema_version: u32,
    pub call_id: String,             // run_id:generation:sample:attempt:repair:nonce
    pub run_id: String,
    pub generation: u64,
    pub island: Option<String>,
    pub mechanism_lane: Option<String>,
    pub sample_index: usize,
    pub attempt_index: usize,
    pub repair_kind: Option<String>,
    pub backend: String,             // jnoccio_http, jailgun_browser, fixture, external_referee
    pub provider: String,            // no empty/unknown in campaign mode
    pub requested_model: String,
    pub response_model: String,
    pub router_alias: Option<String>, // jnoccio/jnoccio-fusion
    pub quality_band: Option<String>,
    pub prompt_sha256: String,
    pub prompt_template_sha256: Option<String>,
    pub schema_sha256: Option<String>,
    pub raw_sha256: Option<String>,
    pub raw_len: Option<u64>,
    pub http_status: Option<u16>,
    pub finish_reason: Option<String>,
    pub started_at_utc: String,
    pub latency_ms: u64,
    pub usage: TokenUsage,
    pub cost: CostUsage,
    pub error_type: Option<String>,
}
```

Extend `ProposalAttemptRecord` with denormalized fields needed for fast grep/reporting:

```rust
pub call_id: Option<String>,
pub provider: Option<String>,
pub prompt_tokens: u64,
pub completion_tokens: u64,
pub cached_tokens: u64,
pub reasoning_tokens: u64,
pub total_tokens: u64,
pub latency_ms: u64,
pub usd_micro: Option<i64>,
pub usage_estimated: bool,
pub usage_source: Option<String>,
pub tokenizer_name: Option<String>,
pub pricing_table_sha256: Option<String>,
pub unpriced: bool,
```

The high-resolution receipt is authoritative for token accounting; the attempt record carries a summary to preserve today’s funnel workflow. Every LLM-like event must have a stable `call_id`; repairs and router-internal repairs must be distinguishable. The current `upstream_repairs` count from jnoccio is not enough, because the host cannot know whether those upstream repairs consumed tokens unless the gateway reports nested usage. V8 must require either `extra.jnoccio.structured_repair_usage[]` or a gateway-level usage object that already includes those repairs and states so.

## 3. Transport capture: jnoccio and jailgun

### 3.1 jnoccio HTTP path

Change `RouterResponse` to include `provider`, `usage`, `latency_ms`, `response_id`, and `finish_reason`. The parser should accept OpenAI-style and emerging OpenTelemetry-compatible names:

```rust
fn parse_usage(v: &Value, model: &str) -> Result<TokenUsage> {
    let usage = v.get("usage").context("missing usage")?;
    let prompt = usage["prompt_tokens"].as_u64()
        .or_else(|| usage["input_tokens"].as_u64()).unwrap_or(0);
    let completion = usage["completion_tokens"].as_u64()
        .or_else(|| usage["output_tokens"].as_u64()).unwrap_or(0);
    let cached = usage["prompt_tokens_details"]["cached_tokens"].as_u64()
        .or_else(|| usage["input_token_details"]["cached_tokens"].as_u64())
        .unwrap_or(0);
    let reasoning = usage["completion_tokens_details"]["reasoning_tokens"].as_u64()
        .or_else(|| usage["output_token_details"]["reasoning_tokens"].as_u64())
        .unwrap_or(0);
    Ok(TokenUsage {
        prompt_tokens: prompt,
        completion_tokens: completion,
        cached_tokens: cached,
        reasoning_tokens: reasoning,
        total_tokens: usage["total_tokens"].as_u64().unwrap_or(prompt + completion),
        usage_source: UsageSource::GatewayExact,
        usage_estimated: false,
        tokenizer_name: None,
        tokenizer_sha256: None,
        estimation_method: None,
        coverage_gaps: vec![],
    })
}
```

Provider attribution is mandatory in campaign mode. jnoccio currently surfaces `winner_model_id`; V8 must also surface `winner_provider_id`, `upstream_request_id`, and whether `usage` is provider exact or gateway normalized. If jnoccio can only say `winner_model_id` and not provider, the call should fail preflight for token-ledger campaigns. Unit tests may use `provider="test"`; real campaigns may not use `provider="unknown"`.

For errors, distinguish three cases. If the request never left the process, emit `llm_call.error` with `usage_source=NotSent` and all token fields zero. If the request was sent and a structured provider error returns usage, record it. If the request was sent and then timed out or lost the body, estimate prompt tokens from the exact request envelope, set `completion_tokens=0`, add `coverage_gaps=["completion_unknown_after_send"]`, and price a conservative upper-bound column separately: `completion_tokens_upper_bound=max_completion_tokens`. Upper bounds must not be folded into exact totals, but the campaign report must show them.

### 3.2 jailgun browser-bridge path

`docs/ZYAL.md` describes jailgun as a browser-bridge MCP transport using `jailgun.run`, `run_status`, and `run_summary`. Browser paths often cannot expose provider billing usage. The fallback is not “no usage”; it is local counting with a named tokenizer and `usage_estimated=true`.

Add a `TokenizerRegistry` pinned by file, for example `data/registry/llm-tokenizers.yml`:

```yaml
schema_version: 1
families:
  openai_o_series:
    tokenizer: tiktoken:o200k_base
    implementation: openai/tiktoken
  openai_legacy_chat:
    tokenizer: tiktoken:cl100k_base
    implementation: openai/tiktoken
  llama3:
    tokenizer: hf-tokenizers:meta-llama/Meta-Llama-3-tokenizer
    sha256: "..."
  qwen2_5:
    tokenizer: hf-tokenizers:Qwen/Qwen2.5-tokenizer
    sha256: "..."
  deepseek_v3:
    tokenizer: hf-tokenizers:deepseek-ai/DeepSeek-V3-tokenizer
    sha256: "..."
```

Named implementations: OpenAI `tiktoken` for OpenAI-compatible BPE families; Hugging Face `tokenizers` for model-specific tokenizer JSON files. `tiktoken` is OpenAI’s fast open-source BPE tokenizer; Hugging Face `tokenizers` is Rust-native and designed for production tokenization. Pin the tokenizer artifact hash, not just the crate version. Candidate Rust dependencies: `tokenizers` 0.22.x or current locked version, a Rust tiktoken implementation that supports `o200k_base` and `cl100k_base`, `serde` 1.x, `serde_json` 1.x, `sha2` 0.10, and `time` 0.3. Exact versions must be frozen in `Cargo.lock` and printed in `tokenizer_registry_sha256`.

For jailgun, count the host-known prompt envelope before submission and count the returned visible output. If the browser product injects hidden system text, record `coverage_gaps=["browser_hidden_prompt_unknown"]`. That flag is allowed; missing tokens are not. The campaign report must separately show exact usage, estimated usage, and estimated-with-gap usage.

## 4. Native JSONL event extension

Add `token-ledger.jsonl` to `RunDirSink`, with a trait method:

```rust
fn llm_call(&mut self, rec: &LlmCallReceipt) -> Result<()>;
fn pricing_snapshot(&mut self, rec: &PricingSnapshotRecord) -> Result<()>;
fn token_backfill(&mut self, rec: &TokenBackfillRecord) -> Result<()>;
```

New event types:

1. `llm_call.started`: emitted immediately before a live request. Fields: `call_id`, backend, provider if known, requested model, generation, island, lane, sample, attempt, repair kind, prompt hash, schema hash, `max_completion_tokens`, temperature, and timestamp.
2. `llm_call.completed`: emitted after a normal response, with exact usage when provider/gateway supplied it.
3. `llm_call.estimated`: emitted after a response without usage, using local tokenizer fallback.
4. `llm_call.error`: emitted for errors, with `NotSent`, exact error usage, or estimated prompt-only usage depending on where the failure occurred.
5. `pricing_snapshot`: emitted once per run and every time pricing changes; contains provider/model rates, source URL or internal catalog path, effective date, and content hash.
6. `token_backfill`: emitted by historical backfill tools only.

Example completed event:

```json
{"event_type":"llm_call.completed","schema_version":1,"call_id":"run42:g120:s2:a0:none","run_id":"run42","generation":120,"island":"failure-repair","mechanism_lane":"dark_scattering","sample_index":2,"attempt_index":0,"repair_kind":null,"backend":"jnoccio_http","provider":"openrouter","requested_model":"jnoccio/jnoccio-fusion","response_model":"qwen/qwen3-235b-a22b","router_alias":"jnoccio/jnoccio-fusion","quality_band":"top20","prompt_sha256":"...","schema_sha256":"...","raw_sha256":"...","raw_len":8421,"http_status":200,"finish_reason":"stop","started_at_utc":"2026-06-11T18:00:00Z","latency_ms":18423,"usage":{"prompt_tokens":6120,"completion_tokens":1544,"cached_tokens":4096,"reasoning_tokens":0,"total_tokens":7664,"usage_source":"gateway_exact","usage_estimated":false,"tokenizer_name":null,"tokenizer_sha256":null,"estimation_method":null,"coverage_gaps":[]},"cost":{"usd_micro":231,"unpriced":false,"pricing_table_sha256":"...","priced_at_utc":"2026-06-11T18:00:00Z","input_usd_per_mtok":"0.10","cached_input_usd_per_mtok":"0.025","output_usd_per_mtok":"0.40","reasoning_usd_per_mtok":null},"error_type":null}
```

The JSONL ledger remains the source of truth. External stores may be regenerated. Reports must never read only Langfuse/Phoenix/OTel.

## 5. Historical backfill

Backfill can recover less than the team wants. From old attempt records, exact recovery includes call count, generation, sample/attempt, repair kind, outcome, kill reasons, score total, model when present, quality band, mechanism lane, upstream repairs, elapsed seconds, raw hash, and raw byte length. It cannot exactly recover provider usage if the original `usage` object was discarded. It cannot exactly recover completion tokens from `raw_len` without raw text. It cannot exactly recover repair prompt tokens when the repair prompt included prior raw content that was not stored.

Implement `zyal tokens backfill --run <dir> --engine-snapshot <sha>` with quality classes:

- `exact_provider_usage`: only if an old sidecar or gateway log contains the original `usage` object.
- `exact_local_text`: raw prompt and raw completion text are available and counted with pinned tokenizer.
- `template_prompt_estimated_output_text`: prompt can be re-rendered from the pinned engine, and raw completion text exists.
- `template_prompt_length_only_output`: prompt can be re-rendered; completion estimated from `raw_len` using a calibrated chars/token distribution for that model family.
- `length_only`: only raw length and model family are available.
- `call_count_only`: no defensible token estimate; counts calls only.

Backfilled rows must set `usage_estimated=true`, `usage_source=HistoricalBackfill`, `backfill_quality`, and `confidence_interval`. Reports may include these rows in “historical trend” panels only if clearly hatched/flagged. They must not be used to enforce budget stops or claim exact cost-per-point.

## 6. Aggregation views and query interface

Create a derived SQLite database, `token_rollups.sqlite`, regenerated from JSONL. This follows `docs/ZYAL.md`’s preference for durable SQLite while preserving JSONL as replay source. Tables:

```sql
CREATE TABLE llm_call_fact (
  call_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL,
  started_at_utc TEXT NOT NULL,
  generation INTEGER,
  island TEXT,
  mechanism_lane TEXT,
  sample_index INTEGER,
  attempt_index INTEGER,
  repair_kind TEXT,
  outcome TEXT,
  killed INTEGER,
  survived INTEGER,
  score_total REAL,
  backend TEXT NOT NULL,
  provider TEXT NOT NULL,
  requested_model TEXT NOT NULL,
  response_model TEXT NOT NULL,
  quality_band TEXT,
  prompt_tokens INTEGER NOT NULL,
  completion_tokens INTEGER NOT NULL,
  cached_tokens INTEGER NOT NULL,
  reasoning_tokens INTEGER NOT NULL,
  total_tokens INTEGER NOT NULL,
  usage_estimated INTEGER NOT NULL,
  usage_source TEXT NOT NULL,
  usd_micro INTEGER,
  unpriced INTEGER NOT NULL,
  latency_ms INTEGER NOT NULL,
  raw_sha256 TEXT
);

CREATE TABLE pricing_snapshot (
  pricing_table_sha256 TEXT PRIMARY KEY,
  effective_from_utc TEXT NOT NULL,
  provider TEXT NOT NULL,
  model TEXT NOT NULL,
  input_usd_per_mtok TEXT,
  cached_input_usd_per_mtok TEXT,
  output_usd_per_mtok TEXT,
  reasoning_usd_per_mtok TEXT,
  source TEXT NOT NULL
);
```

CLI:

- `zyal tokens ingest <run-dir>... --db token_rollups.sqlite`
- `zyal tokens totals --by provider,model,lane --since 2026-01-01`
- `zyal tokens report --run <run-dir> --format markdown`
- `zyal tokens integrity --run <run-dir>`
- `zyal tokens export-otel --run <run-dir> --endpoint <otlp>`

Canonical report queries:

```sql
-- Provider/model/lane totals over time
SELECT provider, response_model, mechanism_lane,
       SUM(prompt_tokens) AS prompt,
       SUM(completion_tokens) AS completion,
       SUM(cached_tokens) AS cached,
       SUM(reasoning_tokens) AS reasoning,
       SUM(total_tokens) AS total,
       SUM(usd_micro) / 1000000.0 AS usd,
       COUNT(*) AS calls,
       SUM(usage_estimated) AS estimated_calls
FROM llm_call_fact
GROUP BY provider, response_model, mechanism_lane
ORDER BY total DESC;

-- Generation productivity
SELECT generation,
       SUM(total_tokens) AS tokens,
       SUM(usd_micro) / 1000000.0 AS usd,
       MAX(score_total) AS best_score,
       SUM(CASE WHEN survived=1 THEN 1 ELSE 0 END) AS survivors
FROM llm_call_fact
GROUP BY generation
ORDER BY generation;

-- Repair overhead
SELECT COALESCE(repair_kind,'first_attempt') AS round,
       COUNT(*) AS calls,
       SUM(total_tokens) AS tokens,
       SUM(usd_micro) / 1000000.0 AS usd,
       AVG(latency_ms) AS latency_ms
FROM llm_call_fact
GROUP BY round;
```

New campaign report sections:

1. **Compute ledger integrity**: total live calls, attributed calls, unattributed calls, exact-usage calls, estimated calls, estimated-with-gap calls, unpriced calls, pricing snapshot hash, tokenizer registry hash.
2. **Token and cost summary**: prompt/completion/cached/reasoning tokens, total tokens, USD, latency percentiles.
3. **Provider/model over time**: per-provider and per-model totals by generation bucket and wall-clock day.
4. **Search topology cost**: tokens by lane, island, generation, quality band, sample index, and repair round.
5. **Kill economics**: tokens-to-kill by veto class; tokens-to-survive for proposals that reach scorecard; cost wasted on parse/transport/schema failures.
6. **Score productivity**: tokens per scorecard point earned, USD per point, best-score improvement per 100k tokens.
7. **Efficiency experiments**: prompt-cache hit rates, repair overhead/yield, strict-schema savings, retrieval budget curves.

## 7. Cost KPIs, stopping rules, and budget-aware routing

Definitions:

- `cost_per_adjudicated_proposal = total_usd / count(proposals with terminal oracle outcome ok or killed)`. Transport-not-sent errors are excluded from adjudicated count but included in waste panels.
- `tokens_to_kill = sum(tokens for all attempts ending in deterministic kill) / killed_proposals`.
- `tokens_to_survive = sum(tokens for all attempts leading to an admitted scored proposal) / survived_proposals`.
- `cost_per_point = total_usd / max(epsilon, sum(max(score_total - baseline_score, 0)))`. If no positive points are earned, show `undefined/infinite`; do not suppress the failure.
- `marginal_token_productivity(W) = (best_score[g] - best_score[g-W]) / (tokens[g-W+1..g] / 100000)`.

Stopping rule: use deterministic rolling windows of `W=50` generations or at least `N=20` adjudicated live proposals, whichever is later. Pause live proposer spend if all are true for three consecutive windows: (1) marginal token productivity is below `0.25 scorecard points / 100k tokens`; (2) survivor rate upper confidence bound is below `1 survivor / 100k tokens`; (3) no new mechanism-attributable fingerprint exceeds the current champion’s gate. Also stop immediately at hard budget caps: total tokens, USD, wall-clock, or unpriced-call count. The confidence bound can be Wilson/Clopper-Pearson with fixed deterministic implementation; no random bootstrap is needed.

Routing answer from code: today’s router does not implement cost-awareness except indirectly through `router_every`, `samples`, `repairs`, `max_completion_tokens`, and the assumption that routed proposal calls are free-tier. The source and paper describe win-rate routing, quality-band selection, lane rotation, and winner-model telemetry; there is no provider price table, token estimate, or expected-value-per-cost term in `RouterConfig` or `RouterResponse`.

V8 routing objective:

```text
value(model,lane) = E[survive | model,lane] * E[max(score-baseline,0) | survive,model,lane]
cost(model,lane)  = E[usd] + lambda_tok * E[tokens/1000] + lambda_lat * E[latency_seconds]
ucb(model,lane)   = exploration_c * sqrt(ln(total_calls + 1) / (calls_model_lane + 1))
route_score       = value / max(cost, floor_cost) + ucb
```

Maintain per-model/per-lane Beta priors for survival and Gamma/log-normal estimates for tokens and latency. If jnoccio remains alias-only, the host cannot pin model directly; then require jnoccio to expose a cost-aware routing mode that returns a quoted provider/model before dispatch or accepts constraints such as `max_usd_micro`, `max_total_tokens_estimate`, and `quality_band`. Without that, cost-aware routing can only choose bands/lanes, not actual models, and the limitation must be shown in the report.

## 8. Standards decision

Recommendation: extend native JSONL + replay/rescore tooling as the authoritative standard, and emit OpenTelemetry GenAI as an optional mirror. Do not make Langfuse or Arize Phoenix load-bearing.

Rationale: OpenTelemetry GenAI conventions provide the right vocabulary: `gen_ai.usage.input_tokens`, `gen_ai.usage.output_tokens`, cache-read/cache-creation token attributes, reasoning-token attributes, provider name, operation name, request model, response model, and client spans. OpenAI’s API exposes `cached_tokens` in `usage.prompt_tokens_details` for prompt caching, and current conventions explicitly allow provider usage or offline token counting when provider usage is unavailable. Langfuse and Phoenix both provide useful LLM observability dashboards and token/cost views; Langfuse also consumes OTLP. But the OpenQG scientific contract is replay from local content and ledgers with no network. External stores are lossy operational mirrors, not evidence.

Citations: OpenTelemetry GenAI semantic conventions for events and spans (`https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-events/`, `https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/`); OpenAI prompt caching usage fields (`https://developers.openai.com/api/docs/guides/prompt-caching`); Langfuse token/cost tracking and OpenTelemetry integration (`https://langfuse.com/docs/observability/features/token-and-cost-tracking`, `https://langfuse.com/integrations/native/opentelemetry`); Phoenix LLM tracing (`https://arize.com/docs/phoenix/tracing/llm-traces`); OpenAI tiktoken (`https://github.com/openai/tiktoken`); Hugging Face tokenizers (`https://huggingface.co/docs/tokenizers/index`).

## 9. Token-efficiency engineering experiments

### Prompt caching across proposal fan-out

Experiment: split router prompts into a stable prefix and a small per-lane/per-sample suffix. The stable prefix includes oracle rules, strict schema, registry relation list, and data brief; the suffix includes lane instruction and sample nonce. OpenAI-compatible providers expose `cached_tokens` for prompts at or above the provider’s threshold; OpenAI documents caching for prompts of at least 1024 tokens and reports cache hits in `usage.prompt_tokens_details.cached_tokens`. Metric: `cache_read_tokens / prompt_tokens`, prompt-token USD per adjudicated proposal, OK rate, and best-score productivity. Decision threshold: keep if cached-token share exceeds 40% on fan-out calls and prompt-token USD falls at least 20% with no more than 5% drop in OK rate or score productivity. Note privacy risk: prompt caching can introduce timing side channels; never place secrets or private paths in stable cached prefixes.

### Repair-round overhead

Experiment: report repair tokens separately for parse repair, oracle repair, and router-internal structured repair. Metrics: `repair_token_share`, `repair_yield_per_100k_tokens`, `repair_to_survivor_conversion`, and score delta after repair. Decision threshold: if oracle repairs consume more than 35% of live tokens while producing fewer than one scored survivor per 100k repair tokens for two windows, lower repair count or restrict repairs to lanes with positive marginal productivity. Keep parse repair only where strict-schema invalid rates exceed 2% and repaired calls outperform first-attempt retries.

### Strict-schema V7 sketch savings

The paper states V5 lost 77% of calls to transport failure and about 90% of survivors to parse errors, while schema’d router parse failures fell below 4% (`paper/main.tex:192-195`). V8 must quantify that in tokens, not just calls. Experiment: run matched prompts through old full `ProposalDoc` prompting and V7 `ProposalSketch` strict schema for the same model bands and lanes. Metrics: output tokens per adjudicated proposal, parse-waste tokens, repair tokens, and score productivity. Decision threshold: strict sketch remains mandatory if it reduces waste tokens by at least 30% and does not reduce survivor/score productivity by more than 5%.

### Retrieval token budgets — see S09

If S09-class retrieval lands, add `retrieval_context_tokens`, `retrieval_policy_id`, `doc_count`, and `evidence_tokens` to the receipt. Experiment with caps of 2k, 4k, 8k, and 16k retrieved tokens. Metric: novel surviving fingerprints per 100k retrieval tokens and score productivity. Decision threshold: each added 1k retrieval tokens must improve admissible-proposal rate by at least 2% or produce a statistically visible increase in novel survivors; otherwise lower the cap. This S12 ledger must work without retrieval.

## 10. Acceptance test matrix

1. `router_parses_provider_usage`: fake OpenAI-compatible response with `usage.prompt_tokens`, `usage.completion_tokens`, `prompt_tokens_details.cached_tokens`, and `completion_tokens_details.reasoning_tokens` populates the new fields exactly.
2. `router_missing_usage_fails_or_estimates`: in campaign strict mode, jnoccio response without usage fails unless `allow_usage_estimation_for_backend=jnoccio_http` is set; in estimate mode it records tokenizer/method and `usage_estimated=true`.
3. `jailgun_missing_usage_uses_tokenizer`: fake `run_summary` without usage emits `llm_call.estimated`, not a blank ledger row.
4. `all_live_calls_have_token_receipts`: property test scans all run ledgers; any `source` matching live proposer/critic/referee with missing exact usage and missing estimate flag fails.
5. `pricing_snapshot_recomputes_cost`: recomputing micro-USD from tokens and referenced pricing snapshot matches ledger totals exactly.
6. `old_ledgers_replay`: V6/V7 ledgers without token fields deserialize with defaults and rescore with zero mismatches.
7. `new_ledgers_replay`: token fields and `token-ledger.jsonl` are ignored by physics replay except for integrity checks; score remains content-derived.
8. `external_observability_not_load_bearing`: OTLP endpoint down causes warnings only; JSONL report and replay still pass.
9. `zero_unattributed_campaign`: end-to-end smoke campaign report has `unattributed_calls=0`; test fails on `provider=null`, `provider="unknown"`, missing tokenizer for estimates, missing pricing status, or unclassified errors after send.

## What we got wrong

1. **“Total observability” is overstated.** The attached paper’s observability claim excludes the economic variables that matter most for autonomous search: prompt tokens, completion tokens, cached tokens, reasoning tokens, cost, provider, and tokenizer estimation status. Check: inspect `RouterResponse`, `ProposalAttemptRecord`, and `ledger_sink.rs`; none contain those fields. Fix: the receipt-enforced token ledger above. Settlement test: a campaign report with zero unattributed calls and exact/estimated usage percentages.

2. **“Free-tier routed models” is not a cost model.** A free API call still consumes quota, latency, provider reputation, browser credits, and scarce exploration budget. Check: run a V7-style campaign with token receipts and compute tokens per survivor; if spend is nonzero in tokens, the old “free” category was hiding budget pressure. Fix: price every call as exact USD, estimated USD, zero-price with explicit provider, or unpriced-but-tokened.

3. **Winner-model telemetry is not enough attribution.** `winner_model_id` helps explain yield concentration, but a CFO-grade ledger also needs provider, request ID, usage source, pricing snapshot, and whether router-internal repairs are included. Check: jnoccio responses must expose `winner_provider_id` and usage. Fix: preflight fails if gateway cannot provide or normalize these fields.

4. **The Proposer trait allows uninstrumented live backends.** `drain_attempts` defaults to empty, so a new jailgun or critic proposer could accidentally bypass receipts. Check: implement a dummy live proposer without override; today it compiles. Fix: move live calls behind a `ReceiptedLlmClient` and remove empty defaults for live modes.

5. **Historical exact token economics are unrecoverable from hashes and lengths alone.** Raw hashes prove identity but not token counts; byte length is only a rough proxy, especially across tokenizers. Check: take an old attempt with only `raw_len` and compare token counts across `cl100k_base`, `o200k_base`, Llama, and Qwen tokenizers. Fix: backfill confidence classes and never mark length-only estimates exact.

6. **External observability tools are tempting but dangerous as evidence stores.** Langfuse/Phoenix can help humans explore traces, but their databases are not the replay contract. Check: disable the external collector and re-run; any change in physics results or report generation is a design failure. Fix: native JSONL first, OTel export second.

7. **Repair loops may be buying comfort instead of discovery.** The V6 narrative says repairs converted many killed first attempts into scored proposals, but without token attribution the marginal cost is invisible. Check: compute repair-token share and repair-yield per 100k tokens by lane/model. Fix: stopping/routing rules that turn off repair modes with negative marginal productivity.
