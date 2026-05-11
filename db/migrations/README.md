# Migration Policy

OpenQG currently has no production migrations.

When migrations are introduced:

- every migration must be reversible or have an explicit rollback plan
- schema changes must be reviewed for lock safety and backfill cost
- application code must own the transaction boundary that applies the migration
- migration evidence must be tied to a proof lane before release

Keep generated migration artifacts out of hand-edited paths and document the regeneration command next to any future generated receipt.
