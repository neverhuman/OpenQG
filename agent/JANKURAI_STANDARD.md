# OpenQG Jankurai Standard

Standard version: `0.1.0`

Read these files before editing:

1. `agent/owner-map.json`
2. `agent/test-map.json`
3. `agent/proof-lanes.toml`
4. `agent/generated-zones.toml`
5. `agent/boundaries.toml`
6. `agent/tool-adoption.toml`
7. `agent/audit-policy.toml`

Rules:

- keep generated artifacts under declared output commands
- do not edit raw upstream data or lockfiles by hand
- prefer `just fast`, `just schema-sync`, `just check`, `just data-verify`, `just bench-smoke`, `just zyal-validate`, `just score`, and `just release-check`
- treat `contracts/generated/`, `target/`, and `reports/releases/draft/` as generated zones
