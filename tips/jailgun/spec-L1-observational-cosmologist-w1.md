# Engineering Specification — Source Archive Unavailable

Batch tab: 1

## Status

This review cannot be completed as requested because the required `source.tar.gz` archive was not available to the execution sandbox. The instruction required extracting `source.tar.gz` first and reading the actual contents of the curated OpenQG / ZYAL source tree before producing a source-grounded physicist review. I could not honestly satisfy that precondition.

I checked the active artifact directory and searched the container filesystem for `source.tar.gz`, `*.tar.gz`, and the requested source files such as `docs/architecture.md`. No source archive or extracted source tree was present. I also searched the available File Library for `source.tar.gz`, OpenQG / ZYAL architecture documents, and the named documentation/code paths. That search surfaced only related generated JSON review/smoke artifacts and a high-level `unified-theory-agent-brief.v2.md`, not the requested source tarball.

Because this is explicitly a source-only hardening review, I am not going to fabricate citations, infer file contents from path names, or review from prior knowledge. The project may contain exactly the mitigations or limitations that a proper review would need to credit or attack; without the archive, any detailed ranked backlog would be misleading.

## Ranked backlog once the archive is available

1. **Restore the required source input** — Provide `source.tar.gz` in the conversation attachment / sandbox mount so it can be extracted with `tar xzf source.tar.gz`. Why it matters: the review contract requires actual file contents, not filename-level inference. Effort: S. Verification: `tar tzf source.tar.gz` lists the requested root paths, especially `docs/architecture.md`, `docs/theory-league.md`, `docs/zyal-next-level-design.md`, `docs/ZYAL.md`, and `crates/openqg-core/src/...`.

2. **Confirm archive-root layout** — Ensure the tarball contains files rooted at the repository/archive root rather than wrapped in an extra directory. Why it matters: the requested output artifact is meant to be extracted over the invocation directory, and the reviewer must cite stable paths. Effort: S. Verification: `tar tzf source.tar.gz | head` shows paths like `docs/architecture.md`, not `project/docs/architecture.md`.

3. **Run a source inventory before reviewing** — Enumerate the selected target paths and fail closed if any required file is absent. Why it matters: observational-cosmology critique depends on comparing docs, fixtures, likelihood implementation, covariance handling, and league scoring behavior. Effort: S. Verification: a manifest check reports present/missing status for all selected paths.

4. **Perform the actual observational-cosmology review** — Read the requested files in order, then inspect the Rust implementation and fixture data behind the documented claims. Why it matters: the likely highest-leverage issues are data covariance handling, compressed-CMB defensibility, model-selection fairness, and whether the league’s conclusions change correctly under geometry-only, growth, SH0ES, and combined-data score lanes. Effort: M/L. Verification: the final `engineering-spec.md` cites concrete file paths and, where useful, function names or configuration keys.

5. **Package only the completed specification** — After the real review is completed, create a tarball whose sole member is `engineering-spec.md` at archive root. Why it matters: downstream extraction should overwrite only the requested review file and nothing else. Effort: S. Verification: `tar tzf <artifact>.tar.gz` prints exactly `engineering-spec.md`.

## Review work intentionally not performed

The following requested analyses require the missing source and were not performed:

- Whether `docs/architecture.md` honestly documents a background-plus-growth forward model, its equations, data previews, and limitations.
- Whether `docs/theory-league.md` implements fair profile fitting, AIC/BIC, evidence, and data-to-conclusion behavior.
- Whether `docs/zyal-next-level-design.md` already identifies the main failures or misses deeper referee-level attacks.
- Whether `docs/ZYAL.md` cleanly separates LLM proposal from deterministic host judgment.
- Whether `crates/openqg-core/src/scoring/likelihood.rs` and `crates/openqg-core/src/scoring/covariance.rs` use diagonal likelihoods, full covariances, or any approximation that would distort cosmological inference.
- Whether `crates/openqg-core/src/cosmology/background.rs`, `growth.rs`, and `forward.rs` compute the observables claimed in the documentation.
- Whether fixtures such as `data/fixtures/cosmology/tier0-combined.jsonl`, `growth-rsd.jsonl`, and `sh0es-h0.jsonl` encode realistic data, toy data, diagonalized previews, or validation-only rows.
- Whether the league behavior described in the prompt — geometry-only favoring LCDM and SH0ES favoring a dark-energy extension — is actually implemented, tested, or merely documented.

## Minimum acceptance criteria for the real review

When the source archive is provided, the completed review should be considered acceptable only if it does all of the following:

- Opens with a ranked backlog ordered by physics leverage, not by implementation convenience.
- Names each major defect and pairs it with a concrete fix and verification test.
- Separates “trustworthy engine” claims from “cosmological inference” claims.
- Prioritizes real-data ingestion in a defensible order, likely beginning with modern BAO and supernova covariance before broadening to CMB, weak lensing / 3x2pt, standard sirens, and BBN.
- Identifies which covariance omissions most distort inference, especially correlated BAO measurements, supernova systematics, CMB compressed-prior covariance, and growth / weak-lensing covariance where relevant.
- States clearly when compressed CMB distance priors are adequate for engine smoke tests but inadequate for strong cosmological claims.
- Audits H0 and S8 tension handling for double counting, prior leakage, and over-interpreting one-tension extensions.
- Reviews theory coverage as a scoring discipline: no unification credit without a derivation chain, parameter-origin accounting, units, limiting cases, and falsifiable predictions against attainable data.
- Reviews ZYAL as a safety/reliability mechanism: the LLM may propose, but deterministic Rust policy must own durable validation, run contracts, receipt schemas, archive validation, config boundaries, and reproducibility.
- Keeps the output scoped to `engineering-spec.md` only.
