# Database Agent Notes

The `db/` tree owns policy for future database work. OpenQG currently has no production database migrations.

- edit policy docs under `db/README.md`, `db/migrations/README.md`, and `db/constraints/README.md`
- do not add runtime database code, generated migration artifacts, or ad hoc schema files here
- future migrations must prove rollback, backfill, lock safety, and monitoring before release
- proof lane: `just fast` for policy and documentation updates; add dedicated migration or constraint test lanes before any runtime DB implementation lands
