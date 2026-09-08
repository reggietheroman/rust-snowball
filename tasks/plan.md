# Implementation Plan: statements

## Overview

Add credit-card statement tracking: minimum due and due date per `(card, due_on)`, with upsert on correction. Introduce `SqliteDb` so `statements.debt_id` references `debts.id` on one connection. Refactor `SqliteDebtStore` onto `SqliteDb` without changing public debt behavior. When this plan is done, `plan` can call `current_for_debt` and the TUI can call `upcoming`.

Prior module archive: `tasks/plan-snowball-size.md`, `tasks/todo-snowball-size.md`.

## Architecture Decisions

- **`SqliteDb` owns one `Connection`.** `migrate()` runs `CREATE TABLE IF NOT EXISTS` for `debts` and `statements` (and can add `snowball_sizes` later). `PRAGMA foreign_keys = ON` on open.
- **Stores hold `&SqliteDb` in tests, or wrap owned `SqliteDb`.** `SqliteDebtStore::open_in_memory()` stays for regression tests: create `SqliteDb`, return store that owns the db. Add `SqliteDebtStore::new(db: &SqliteDb)` for shared-db tests. Same pattern for `SqliteStatementStore`.
- **Table `statements`:** `id`, `debt_id` FK → `debts.id`, `minimum_cents`, `due_on` TEXT `YYYY-MM-DD`, `UNIQUE(debt_id, due_on)`. Upsert on conflict updates `minimum_cents` only (id unchanged).
- **Debt lookup on record.** Statement store calls `DebtStore::get` (via `SqliteDebtStore` on same db) or reads debt row directly — prefer reusing `get` through a shared db reference to avoid duplicating loan/card rules.
- **Date validation** in `validate.rs`: strict `YYYY-MM-DD`, calendar-valid, no new date crate unless manual validation gets ugly.
- **Ids:** `stmt_<ulid>` via monotonic `Generator` (same lesson as snowball-size).
- **`current_for_debt`:** `ORDER BY due_on DESC, id DESC LIMIT 1`.
- **`upcoming(as_of)`:** `WHERE due_on >= ? ORDER BY due_on ASC, id ASC`. No overdue method.
- **`snowball-size`:** unchanged connection model in this plan; statement tests do not assert snowball size.

## Dependency graph (this module)

```
SqliteDb (debts + statements schema)
    │
    ├── Refactor SqliteDebtStore (behavior unchanged)
    │
    └── Statement types + StatementStore trait
            │
            ├── Validation (date, minimum >= 0, card-only via debt get)
            │
            └── SqliteStatementStore
                    │
                    └── Integration tests (shared in-memory db)
```

## Task List

Index only. Full acceptance criteria live in `tasks/todo.md`.

### Foundation

- [ ] Task 1: `SqliteDb` + refactor `SqliteDebtStore`; debts tests still pass

### Checkpoint: Shared DB

- [ ] `cargo test --test debts` passes
- [ ] `SqliteDb::open_in_memory()` creates both tables

### Statement slices

- [ ] Task 2: Scaffold statements module; `record` / `get`; reject loan, unknown debt, bad input
- [ ] Task 3: Upsert same due date; `list_for_debt`; `current_for_debt` (latest `due_on`)
- [ ] Task 4: `upcoming(as_of)`; recording does not change debt balance

### Checkpoint: statements complete

- [ ] All `SPEC-statements.md` success criteria met
- [ ] `cargo test --test statements`, `cargo test --test debts`, `cargo test --test snowball_size`, clippy, fmt pass
- [ ] Ready for `payments` spec

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Refactoring debts breaks tests | High | Task 1 is only refactor; run debts tests before statements code. |
| Upsert id stability | Med | Integration test: record twice same due date, same `StatementId`, new minimum. |
| Date parsing without chrono | Med | Unit tests for invalid dates including Feb 31; add `time` only if manual validation is error-prone. |
| FK in in-memory SQLite | Low | Enable foreign_keys pragma; test record against missing debt. |

## Open Questions

None. Spec decisions confirmed 2026-09-08.

## Task list target

`tasks/todo.md` (markdown checklist). No external tracker.
