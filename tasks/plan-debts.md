# Implementation Plan: debts

## Overview

Stand up the Snowball library crate and the `debts` registry: typed loans vs cards in SQLite, with `create` / `get` / `list` / `update` / `set_balance` / `reduce_balance`. No TUI, no snowball size, no statements, no payments ledger. When this plan is done, `payments` can call `reduce_balance` and `plan` can `list` and filter `balance_cents > 0`.

## Architecture Decisions

- **One Cargo lib now, binary later.** `src/lib.rs` is the crate. The `tui` binary is not created in this module.
- **`rusqlite` with `bundled`, not sqlx.** The store is synchronous; ratatui will be too. No async runtime in the library. Tests use `Connection::open_in_memory()`.
- **`DebtStore` trait + `SqliteDebtStore`.** Tests and later callers depend on the trait. SQLite is the only implementation in this module.
- **Table `debts`:** `id`, `name` (UNIQUE), `balance_cents`, `kind` (`loan` | `credit_card`), `payment_cents` NULL on cards, `due_day` NULL on cards. Kind change is rejected in validation, not by a CHECK that we would later regret.
- **`DebtId` is a newtype over `String`** (`debt_<ulid>`). Callers never pass a raw id they minted.
- **Validation at the store boundary** (`src/debts/validate.rs`). SQLite unique index is the duplicate-name backstop; the library maps it to `VALIDATION_ERROR`.

## Dependency graph (this module)

```
Cargo lib + Error
    │
    └── Debt types + DebtStore trait
            │
            ├── Validation
            │
            └── SqliteDebtStore + schema
                    │
                    └── Integration tests (in-memory SQLite)
```

Order is still **vertical**: each task after scaffold adds one store method (or one kind) with tests, not “all schema then all methods.”

## What can be parallel vs sequential

All sequential. Shared schema and `DebtStore` — do not split across agents.

## Task List

Index only. Full acceptance criteria live in `tasks/todo.md`.

### Foundation

- [ ] Task 1: Scaffold lib, `Error`, empty `debts` module

### Checkpoint: Foundation

- [ ] `cargo test` and `cargo build` succeed on an empty-but-wired crate

### Registry slices

- [ ] Task 2: Create and get a loan
- [ ] Task 3: Create a credit card; reject loan fields on cards

### Checkpoint: Both kinds persist

- [ ] Loan and card round-trip; card-with-payment is rejected
- [ ] Review with human before proceeding if the union shape feels wrong

- [ ] Task 4: Unique names; `list` includes $0, sorted by name
- [ ] Task 5: Partial `update`; kind change rejected; missing id is `NOT_FOUND`

### Checkpoint: Registry complete

- [ ] Spec create/update/list criteria met except balance mutations

- [ ] Task 6: `set_balance` (including raising a card)
- [ ] Task 7: `reduce_balance` saturates at 0

### Checkpoint: debts module complete

- [ ] All `SPEC-debts.md` success criteria met
- [ ] `cargo test debts`, `cargo clippy -- -D warnings`, `cargo fmt --check` pass
- [ ] Ready for `snowball-size` spec (next module in the map; `snowball-size` does not depend on debts)

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `rusqlite` vs a later async TUI | Low | TUI is sync (ratatui). Revisit only if we introduce a runtime. |
| Unique names vs SQLite UNIQUE | Med | Validate first; map `SQLITE_CONSTRAINT_UNIQUE` to `VALIDATION_ERROR` so the code is not the only check. |
| `reduce_balance` vs `set_balance` confusion | Med | Separate methods, separate tests; no payment rows in this module. |
| Schema churn once `statements` exists | High | Spec says ask first. This plan does not add card min or due-date columns. |

## Open Questions

None that block this module. Spec assumptions (Rust + SQLite, no APR, due day 1–31) were accepted with the spec.

## Task list target

`tasks/todo.md` (markdown checklist). No external tracker.
