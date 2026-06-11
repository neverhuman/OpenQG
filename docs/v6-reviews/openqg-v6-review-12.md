# Path to external credibility

An external cosmologist will not be persuaded by a 60-point or 25-point internal rubric score. They will ask three questions: what likelihood did you evaluate, what theory class did you actually compare, and what prediction was made before looking? V6.1 can answer parts of the first question for compressed data. It cannot yet answer the second and third at publishable standard.

The minimum publishable claim is modest: "OpenQG is an automated red-team discovery system that found and closed a series of scoring exploits in compressed cosmological model selection." That claim is credible with the current architecture if the archive includes full ledgers, current rescores, and regression tests. The strongest scientific claim currently supported is negative: under the admitted compressed Planck+DESI+growth data and V6.1 scoring, the known degeneracy-valley exploit no longer beats LambdaCDM, and the V6 re-clothed champions mostly die. That is worth reporting as methodology, not as evidence for modified gravity.

A publishable positive cosmology claim requires a different bar. First, every promoted model must be a model class, not a single fixed theory instance. `crates/openqg-core/src/theory/league.rs::ModelClass` is the right direction. LambdaCDM, wCDM, w0waCDM, screened MG, f(R), and nDGP should be profile-fit to the same data with the same covariance treatment. The candidate should be compared by AIC/BIC and, for low-dimensional cases, by Laplace or grid evidence from `crates/openqg-core/src/scoring/evidence.rs`. A fixed proposal with `beta = 2` is not enough.

Second, the forward model must match the observables. The pure Rust `BackgroundForwardModel` is useful for triage, BAO distances, supernova distance moduli, BBN approximations, and simple growth tests. It is not a substitute for CLASS/hi_class when claiming CMB or modified-gravity constraints. The code already anticipates this with `ForwardKind::Boltzmann` in `crates/openqg-core/src/cosmology/forward.rs`. V7 should implement a promotion backend that can compute at least CMB distance priors from a Boltzmann solver, and preferably linear matter power or growth observables for MG families. The backend must stamp code version, parameter file, and data hash in `ForwardManifest`.

Third, novelty must be out-of-fit and time-ordered. A cosmologist will not accept a "novel prediction" generated from the same data that selected the model. V7 needs a true prediction registry: the engine registers a falsifiable observable before that observable enters the fit set. Fit-set witnesses can explain why a model fits; they cannot score novelty. A credible first registry could be small: one or two DESI Y3/Y5 BAO/RSD bins, one weak-lensing cross-correlation, or one supernova subset kept blinded until the candidate is frozen.

Fourth, the action grammar must be real enough to reject costume changes. `StructurallyUngenerated` cannot rely on substring term names. A reviewer will immediately notice a candidate with nDGP effective coupling but no DGP action term. Typed `TermKind` and relation-to-term requirements are the minimum. Later, symbolic operators and limit checks should replace term labels.

Fifth, release artifacts must be referee-replayable. `docs/V6-CAMPAIGN-REPORT.md` reports a 754-call funnel, but this source-only archive includes only three white-paper JSON files under `run-data/`. That is not enough. A credible archive should include proposal ledgers, attempt summaries, exact observables, covariance fixtures, calibration references, scorecard receipts, and a one-command replay that produces the same champion table. If data files are too large, include manifests with hashes and retrieval instructions.

A concrete V7 external-credibility ladder:

Phase 1: hardening paper. Ship V6.1/V7 as an engineering audit with all closed exploits, all regression tests, and no claim of new physics. Include the degeneracy-valley history as a case study in automated self-red-teaming.

Phase 2: benchmark league. Run `model_league` over LambdaCDM, wCDM, w0waCDM, screened MG, f(R), nDGP, and dark-scattering classes on a fixed covariance dataset. Publish only the league table and code. The claim is that the engine reproduces known model-selection baselines.

Phase 3: blinded prediction. Freeze a candidate class from Phase 2, register one or more out-of-fit predictions, then unblind or admit the data. If it fails, publish the failure. If it survives, then consider a short methods note.

Phase 4: Boltzmann promotion. Only after a candidate remains competitive under a Boltzmann-backed likelihood should it receive human-theory attention. At that point the deliverable is not "the LLM discovered a theory"; it is "the deterministic oracle selected a registered model class whose fit and out-of-fit predictions are reproducible."

For the current survivor, the path is simple: do not pitch it externally. Convert it into a regression fixture. The external credibility of OpenQG will come from refusing to oversell it.
