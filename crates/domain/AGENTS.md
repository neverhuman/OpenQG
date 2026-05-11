# Domain Guidance

`crates/domain` owns agent-friendly structured errors and repair hints.

Rules:

- keep exception types explicit and typed
- include purpose, reason, common fixes, docs_url, and repair_hint
- keep validation helpers small and local
- use `just fast` as the default proof lane for this package
