# Benchmark Methodology

OpenQG compares theory manifests against a baseline theory and a suite definition.

The scorecard tracks:

- log likelihood
- delta log likelihood
- AIC
- BIC
- MDL
- coverage
- parameter count penalty

The benchmark should reward:

- exact named parameters
- complete observable coverage
- reproducible fixtures
- explicit invalid-domain handling

The benchmark should penalize:

- missing uncertainties
- missing predictions
- raw-data references in manifests
- anonymous latent knobs

