# engineering-spec.md

## Status: source archive unavailable

Invocation tab: 1.

I could not perform the requested OpenQG / ZYAL source-only hardening review because the required archive, `source.tar.gz`, was not available in the code sandbox or accessible mounted file paths at execution time.

I checked the expected sandbox and shared mount locations before making this artifact:

- `/mnt/data` was empty.
- `/home/oai/share` contained no archive.
- Searches for `source.tar.gz`, `*.tar.gz`, and `*.tgz` under `/mnt/data`, `/tmp`, `/home/oai`, `/home/oai/share`, `/workspace`, `/app`, and `/` found no matching source archive.
- A File Library search for `source.tar.gz`, `OpenQG`, `ZYAL`, `docs/architecture.md`, `docs/theory-league.md`, and `crates/openqg-core/src/theory/vetoes.rs` did not surface the curated source tarball or the requested source tree.
- A Google Drive search for `source.tar.gz` and `OpenQG ZYAL` did not find the archive.

Because your instructions explicitly required extracting `source.tar.gz` and reading the actual file contents, I am not providing a physics or engineering critique from filenames, prompt summaries, prior knowledge, or inferred project shape. That would be misleading and would violate the evidence boundary you set.

## Ranked backlog

1. **P0 — Re-run with the actual `source.tar.gz` mounted in the sandbox**
   - **What:** Attach or expose `source.tar.gz` so it appears at `/mnt/data/source.tar.gz`, then extract it with `tar xzf source.tar.gz` and read the requested files in order.
   - **Why it matters:** The review must cite and respond to concrete source claims and implementation details. Without the archive, any detailed critique of `docs/architecture.md`, `docs/theory-league.md`, `docs/zyal-next-level-design.md`, `docs/ZYAL.md`, or `crates/openqg-core/src/...` would be unverifiable.
   - **Effort:** S.
   - **How to verify:** `test -f /mnt/data/source.tar.gz && tar tzf /mnt/data/source.tar.gz | grep -E '(^|/)docs/architecture.md$'` succeeds, and extraction exposes the selected target paths at the archive root.

2. **P0 — Preserve the source-only evidence boundary**
   - **What:** Reject any review that substitutes public web search, memory, prompt summaries, or File Library snippets for the attached source tree.
   - **Why it matters:** The task is a source audit. The trust value comes from checking what the repository actually implements, including equations, likelihood construction, veto thresholds, and ZYAL run contracts.
   - **Effort:** S.
   - **How to verify:** The completed `engineering-spec.md` contains path-specific citations to actual extracted files and names concrete code/doc claims that can be verified locally.

3. **P1 — Generate the requested comprehensive engineering specification once source is available**
   - **What:** Produce the full ranked backlog and detailed sections on data sources, forward-model fidelity, statistical rigor, theory coverage, ZYAL multi-agent design, and software/reproducibility.
   - **Why it matters:** The requested deliverable must be an actionable, prioritized hardening plan grounded in the implementation.
   - **Effort:** M.
   - **How to verify:** The artifact contains exactly one root file named `engineering-spec.md`, is between the requested depth range, and every deficiency includes a file-grounded fix and verification plan.

## Non-review statement

No claims are made here about the correctness of OpenQG, ZYAL, the α-basis encoding, the CPL background, late-time μ(a) growth, GW170817 α_T veto, stability checks, PPN screening, theory league scoring, covariance likelihoods, or any proposed dark-energy / modified-gravity ModelClass. Those are precisely the claims that require the missing archive.
