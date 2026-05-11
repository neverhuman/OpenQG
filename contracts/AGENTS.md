# Contracts Agent Notes

The `contracts/` tree owns schema registry data, editable schema specs, and generated bindings.

- edit schema sources under `contracts/specs/`
- edit the registry under `contracts/registry.yml`
- do not hand-edit generated artifacts under `contracts/generated/schemas/`
- use `just schema-sync` after contract source edits, then `just schema-check`
