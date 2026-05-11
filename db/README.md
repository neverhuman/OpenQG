# Database Policy

OpenQG currently has no production database migrations.

This directory is policy surface only for now. It exists so future database work has a documented landing zone instead of inventing one ad hoc.

When a real database layer is introduced, the work must:

- add explicit migrations under `db/migrations/`
- define constraint policy under `db/constraints/`
- document the owning service or transaction boundary before code lands
- prove rollback, backfill, and lock-safety behavior before release

Do not add runtime database behavior here until the control-plane docs, proof lanes, and ownership maps are updated together.
