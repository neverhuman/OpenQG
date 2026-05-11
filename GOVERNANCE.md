# Governance

OpenQG is organized around explicit ownership boundaries:

- `agent/`: repository control-plane policy
- `contracts/`: schema registry, editable specs, and generated bindings
- `data/registry/`: source manifests for public datasets
- `benchmarks/`: suite definitions and KPI policy
- `theories/`: candidate theory manifests and adapters
- `agent/zyal/`: long-running host-owned runbooks

Generated outputs must stay under declared output directories:

- `target/`
- `reports/releases/draft/`
- `contracts/generated/`
- `contracts/generated/schemas/`

If a file is not clearly owned, stop and assign ownership before expanding scope.
