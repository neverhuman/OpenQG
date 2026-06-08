# OpenQG / ZYAL Engineering Review — Source Archive Unavailable

## Status

I could not perform the requested independent physicist / scientific-software review because the required `source.tar.gz` archive was not available in the execution sandbox.

The user request explicitly required that the archive be extracted and the actual file contents be read before review, with the instruction: do not review from filenames or prior knowledge; if the archive is genuinely unavailable, say so explicitly. This document is therefore not a substantive review of OpenQG, ZYAL, or the selected Rust modules. It is an availability receipt explaining why no referee-grade engineering specification can honestly be produced from the requested source snapshot.

## Retrieval attempts performed

I checked the normal upload location and the local filesystem for the requested archive:

- `/mnt/data` contained no uploaded files.
- A filesystem search for `source.tar.gz`, `*.tar.gz`, and `*.tgz` returned no source archive.
- A File Library search for `source.tar.gz`, `OpenQG`, `ZYAL`, `docs/architecture.md`, `docs/theory-league.md`, `crates/openqg-core/src/cosmology/background.rs`, `scoring/covariance.rs`, and related terms did not surface the requested archive or the curated source snapshot.
- A Google Drive search for `source.tar.gz`, `OpenQG`, and `OpenQG ZYAL` found only an unrelated document snippet, not the source archive.
- A connected GitHub repository search did not expose an OpenQG repository or the selected files.
- A direct network `git ls-remote` attempt from the sandbox could not resolve GitHub, so the archive could not be reconstructed from a public repository.

## Why I am not producing a speculative review

A credible review of this project depends on reading the actual implementation and documentation named in the prompt, especially:

- `docs/architecture.md`
- `docs/theory-league.md`
- `docs/zyal-next-level-design.md`
- `docs/ZYAL.md`
- `crates/openqg-core/src/cosmology/background.rs`
- `crates/openqg-core/src/cosmology/growth.rs`
- `crates/openqg-core/src/scoring/covariance.rs`
- `crates/openqg-core/src/scoring/likelihood.rs`
- `crates/openqg-core/src/theory/league.rs`
- `crates/openqg-core/src/theory/vetoes.rs`

Without those contents, I cannot honestly cite exact claims, inspect equations, evaluate the actual likelihood implementation, verify the Cholesky covariance solve, check veto thresholds, audit determinism, inspect tests, or assess whether the documentation accurately describes the Rust engine.

Producing the requested ranked backlog anyway would risk exactly the failure mode the prompt is trying to prevent: plausible-sounding review prose that is not grounded in evidence.

## Required next step

Please re-upload the `source.tar.gz` archive in the conversation so it appears in the sandbox filesystem. Once available, the requested output can be produced as a single tarball containing exactly:

```text
engineering-spec.md
```

The substantive review should then be grounded in line-level inspection of the source snapshot and should include, at minimum, the requested prioritized backlog and detailed sections on data sources, forward-model fidelity, statistical rigor, theory coverage, ZYAL multi-agent design, and software / reproducibility.
