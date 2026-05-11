# Boundaries

OpenQG boundaries are controlled by agent-readable maps:

- `agent/owner-map.json` assigns ownership
- `agent/test-map.json` assigns proof routes
- `agent/generated-zones.toml` lists generated or regenerated zones
- `agent/boundaries.toml` lists editable and forbidden paths

Current contract boundary rules:

- edit contract sources in `contracts/specs/`
- edit the registry in `contracts/registry.yml`
- regenerate outputs with `just schema-sync`
- verify generated outputs with `just schema-check`
- do not edit `contracts/generated/schemas/` by hand

Generated and review-only zones:

- do not edit `target/` by hand
- do not edit `reports/releases/draft/` by hand
- do not edit `contracts/generated/` by hand
- keep human review receipts under `target/jankurai/review/`

The repo is DB-free for this pass:

- no `db/` surface is routed through the control plane
- if a future architecture change needs a real database layer, treat that as a separate decision instead of inventing a fake `db/` package
