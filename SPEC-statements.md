# Spec: statements

## Objective

The `statements` module is the register of credit-card bills: this cycle’s **minimum due** and **due date**. That is how `plan` knows a card’s floor (not a payment stored on the debt), and how the TUI can show what is coming due so you can leave money in a HYSA until the last comfortable moment.

**User:** you, after a statement posts, typing the minimum and the due date.

**This module succeeds when** you can record one statement per card per due date, `plan` can read each card’s current minimum without guessing, and upcoming/overdue due dates can be listed without notifications.

**Out of scope for this module:** loan due days (those live on `debts`), payments, snowball size, allocation, TUI, bank import, reminders, restating card balance.

## Tech Stack

Same crate as `debts` and `snowball-size`:

- Rust (edition 2024), `snowball` library
- SQLite, same on-disk file; table `statements`
- Money as integer **cents** (`i64`)
- Calendar dates as `YYYY-MM-DD` strings (no timezone)
- Module at `src/statements/`

## Commands

```
cargo test
cargo test --test statements
cargo clippy -- -D warnings
cargo fmt --check
cargo build
```

## Project Structure

```
SPEC-statements.md
src/db.rs                     → Shared SQLite connection + migrations (see contract)
src/statements/mod.rs         → Public contract (types + StatementStore)
src/statements/store.rs       → SQLite implementation
src/statements/validate.rs    → Boundary validation
tests/statements.rs           → Integration tests against in-memory SQLite
```

Reuse `src/error.rs` and `DebtId` / `DebtKind` from `debts`. Do not put minimum due or card due dates on `Debt`.

## Code Style

Match `debts`: `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE` error codes, `Result<T, Error>` on the public surface, opaque ids (`stmt_<ulid>`).

```rust
pub struct StatementId(String);

pub struct Statement {
    pub id: StatementId,
    pub debt_id: DebtId,
    pub minimum_cents: i64,
    pub due_on: String, // YYYY-MM-DD
}

pub struct RecordStatement {
    pub debt_id: DebtId,
    pub minimum_cents: i64,
    pub due_on: String,
}

pub trait StatementStore {
    fn record(&self, input: RecordStatement) -> Result<Statement, Error>;
    fn get(&self, id: &StatementId) -> Result<Statement, Error>;
    fn list_for_debt(&self, debt_id: &DebtId) -> Result<Vec<Statement>, Error>;
    fn current_for_debt(&self, debt_id: &DebtId) -> Result<Option<Statement>, Error>;
    fn upcoming(&self, as_of: &str) -> Result<Vec<Statement>, Error>;
}
```

`list_for_debt` returns that card’s statements, `due_on` ascending.

`current_for_debt` is the statement with the **latest `due_on`** for that card, or `None` if none exist. `plan` uses that row’s `minimum_cents` as the card floor. A missing current statement is `Ok(None)` — `plan` decides what that means, not this module.

`upcoming(as_of)` returns every statement with `due_on >= as_of`, ordered by `due_on` ascending. Tie-break: `due_on`, then `id`. Overdue rows (`due_on < as_of`) are omitted; the TUI can filter `list_for_debt` when it needs overdue (no overdue store method in this module).

Recording the same `(debt_id, due_on)` again **replaces** minimum (and keeps the same id). That is a correction, not a second bill. Different due dates are different rows.

## Testing Strategy

- Framework: `cargo test`
- Location: unit tests next to validation; SQLite round-trips in `tests/statements.rs` using a shared in-memory DB that also has the `debts` schema
- Coverage: every public `StatementStore` method has a success test and a rejection test. Plus: loan rejected; unknown debt `NOT_FOUND`; duplicate due date updates min; `current_for_debt` follows latest `due_on`; `upcoming` excludes earlier dates; recording does not change snowball size or `Debt.balance_cents`
- **Not here:** plan allocation, payments, TUI

## Boundaries

- **Always:** Store money as cents; validate at the store boundary; only `DebtKind::CreditCard`; require the debt to exist; leave snowball size and balances untouched; one row per `(debt_id, due_on)`
- **Ask first:** Deleting statements; statement close date / APR / fees as fields
- **Never:** Record a statement against a loan; guess a minimum or due date; auto-edit snowball size; call `set_balance` / `reduce_balance`; use `f64` for money; send notifications; add an overdue query method (TUI filters `list_for_debt`); key statements by calendar month instead of `(debt_id, due_on)`

## Module contract (consumers)

`plan` depends on: `current_for_debt` → `minimum_cents` for each card with `balance_cents > 0`. If `None`, that card has no floor from this module.

`tui` depends on: `record`, `list_for_debt`, `current_for_debt`, `upcoming`.

`payments` does **not** depend on this module.

This module depends on `debts`: `DebtId`, `get` (or equivalent lookup) to confirm the debt exists and is `CreditCard`.

### Shared SQLite

`debts` and `statements` must see the same rows. Introduce `SqliteDb` (one `Connection`, runs each module’s `CREATE TABLE IF NOT EXISTS`). `SqliteDebtStore` and `SqliteStatementStore` are constructed from `&SqliteDb` (or own a `Arc`-less handle in tests via a wrapper). `snowball-size` can keep its own in-memory connection in existing tests; wiring it onto `SqliteDb` is allowed if cheaper, but not required for this module.

Foreign key: `statements.debt_id` references `debts.id`. Deleting debts is still out of scope.

### Errors (one shape)

```text
NOT_FOUND          — get on unknown statement id; record against unknown debt_id
VALIDATION_ERROR   — debt is a loan; invalid due_on; minimum_cents < 0
```

### Record rules

| Field | Rule |
|---|---|
| debt_id | required; must exist; must be `CreditCard` |
| minimum_cents | required, `>= 0` (a $0 minimum is real) |
| due_on | required, `YYYY-MM-DD`, calendar-valid (reject `2026-02-31`) |

Callers do not mint `StatementId`.

## Success Criteria

- [x] Recording a statement on a credit card persists minimum and due date and reads back unchanged.
- [x] Recording a statement on a loan is `VALIDATION_ERROR` and inserts nothing.
- [x] Recording against an unknown `debt_id` is `NOT_FOUND`.
- [x] Two statements on the same card with different due dates both exist; `current_for_debt` is the later `due_on`.
- [x] Recording the same card + due date again updates `minimum_cents` (correction) and does not add a second row.
- [x] `minimum_cents < 0` or invalid `due_on` is `VALIDATION_ERROR`.
- [x] `upcoming("2026-09-08")` includes due 2026-09-08 and later, excludes 2026-09-07.
- [x] Recording a statement does not change `Debt.balance_cents` or snowball size.
- [x] Public API has no snowball-size or payment fields.
- [x] `cargo test --test statements` passes with SQLite in memory.

## Decisions

Confirmed 2026-09-08.

- Unique key is `(debt_id, due_on)`. Recording the same pair again replaces `minimum_cents` (correction), not a second row. Not one statement per calendar month.
- `current_for_debt` is the statement with the latest `due_on` for that card, not “next due from today.” `upcoming` covers what is due soon.
- No overdue store method. The TUI filters `list_for_debt` when it needs past-due rows.
- Introduce `SqliteDb`: one shared SQLite connection so `statements.debt_id` can reference `debts.id`. `snowball-size` may keep its own connection in existing tests.
- `minimum_cents >= 0`, including zero.
