# Scoring

The repo score is derived from the smoke benchmark scorecard.

Primary factors:

- coverage
- missing predictions
- invalid prediction count
- parameter count penalty

The score is intentionally simple in v0.1:

- a perfect smoke run should land at or near 100
- regression or missing outputs should reduce the score

