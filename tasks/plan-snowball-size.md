# Implementation Plan: snowball-size

## Overview

Add the committed-size ledger to the existing `snowball` library: append-only `record`, `current` as the latest row (or `None`), `list` oldest first. No debts reads, no plan math, no TUI. When this plan is done, `plan` can call `current()` for the amount to split.

Prior module archive: `tasks/plan-debts.md`, `tasks/todo-debts.md`.

## Architecture Decisions

- **Match `debts`.** `SnowballSizeStore` trait + `SqliteSnowballSizeStore`. Sync `rusqlite`, ids `size_<ulid>`, money as `i64` cents, validation at the store boundary.
- **Own connection, own table.** This module opens SQLite and migrates `snowball_sizes` only. It does not share a `Database` type with `debts` (spec: ask first). Tests use `Connection::open_in_memory()`. Same on-disk path can hold both tables later; each store migrates its own table.
- **Table `snowball_sizes`:** `id` TEXT PK, `amount_cents` INTEGER NOT NULL, `recorded_at_unix` INTEGER NOT NULL. No update/delete APIs. CHECK `amount_cents > 0` is a backstop; validation rejects `<= 0` first.
- **List order:** `recorded_at_unix ASC, id ASC`. ULIDs break ties when two records share a second so “oldest first” is insertion order, not a flaky clock.
- **Clock:** store stamps `recorded_at_unix` from UTC now. Callers never pass a time.
- **Empty ledger is not an error.** `current` → `Ok(None)`. `NOT_FOUND` is unused.

## Dependency graph (this module)

```
Existing crate + Error
    │
    └── SnowballSize types + SnowballSizeStore trait
            │
            ├── Validation (amount_cents > 0)
            │
            └── SqliteSnowballSizeStore + schema
                    │
                    └── Integration tests (in-memory SQLite)
```

Vertical slices: scaffold → first `record`/`current` → history (`list`, raise/shrink/reaffirm).

## What can be parallel vs sequential

All sequential. One table, one trait.

## Task List

Index only. Full acceptance criteria live in `tasks/todo.md`.

### Foundation

- [ ] Task 1: Scaffold `snowball_size` module, types, trait, amount validation

### Checkpoint: Foundation

- [ ] Crate still builds; `debts` tests still pass
- [ ] Invalid amount is `VALIDATION_ERROR` in unit tests

### Ledger slices

- [ ] Task 2: Persist first size; empty `current` is `None`; reject `<= 0` with no insert
- [ ] Task 3: `list` oldest first; raise, shrink, and reaffirm keep prior rows

### Checkpoint: snowball-size complete

- [ ] All `SPEC-snowball-size.md` success criteria met
- [ ] `cargo test --test snowball_size`, `cargo test --test debts`, `cargo clippy -- -D warnings`, `cargo fmt --check` pass
- [ ] Ready for `statements` spec (depends on `debts`)

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Two stores, two connections on one file | Low | Spec forbids a shared `Database` until asked. Tests stay in-memory and isolated. |
| Same-second records look unordered | Med | Order by `(recorded_at_unix, id)`, not time alone. |
| Accidental `debts` coupling | High | Public types have no `DebtId`. Tests never open the debts schema. |

## Open Questions

None. Spec decisions (no notes, no backdating, amount `> 0`) were confirmed 2026-09-08.

## Task list target

`tasks/todo.md` (markdown checklist). No external tracker.
