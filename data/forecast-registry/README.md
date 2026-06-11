# Prediction Registry — Pre-Registered Forecast Entries

## Purpose

The prediction registry implements the **only test an LLM-driven engine cannot have memorized**:
forecasts that were hash-sealed and timestamped BEFORE the target dataset's public release.

Each entry in `entries/` records:
- What the engine predicts, at what precision, from what frozen code hash
- When the prediction was registered (git tag + OpenTimestamps proof)
- The dataset and expected release window

When the dataset releases, a human adjudicator checks the frozen prediction against the data
and writes an `adjudicated` entry. The entry is immutable after sealing.

## Seal Protocol

1. Fill the entry YAML completely EXCEPT `seal_digest` and `ots_proof`.
2. Run `bash ops/seal-forecast.sh data/forecast-registry/entries/<entry>.yml`
   - The script computes SHA-256 of the file body (with `seal_digest: null`) and writes it back.
   - It creates a signed git tag `forecast/<entry_id>/v1`.
3. Submit the git tag hash to OpenTimestamps: `ots stamp -d <tag-hash>.ots`
   - Write the resulting `.ots` proof path into `ots_proof` and commit.
4. **Do not change any other field after the seal.** The `seal_digest` will not match.

## Entry Status Values

| Status | Meaning |
|--------|---------|
| `pre_release` | Sealed before the target dataset released |
| `adjudicated` | Dataset released; human checked the prediction |

## Verification

Run `cargo test -p openqg-core -- forecast_registry` to load all entries and verify:
- All YAML parses correctly
- `pre_release` entries have a non-null `seal_digest`
- `adjudicated` entries also have a non-null `ots_proof`

## Reference

SYNTHESIS.md §5 Phase 0 item #2; spec-S06, spec-S10, spec-S11.
