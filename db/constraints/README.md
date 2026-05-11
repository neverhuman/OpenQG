# Constraint Policy

OpenQG currently has no production database constraints beyond policy text.

Future database work must define:

- foreign keys where referential integrity matters
- check constraints for domain invariants
- row-level security for isolation-sensitive data
- lock-safe rollout and rollback behavior
- monitoring that confirms the constraint is active and not silently bypassed

If a constraint needs a backfill or staged rollout, document the exact order of operations and the stop condition before changing the schema.
