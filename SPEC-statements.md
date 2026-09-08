# Spec: statements

## Objective

The `statements` module is the register of credit-card bills: this cycle’s **minimum due** and **due date**, labeled with a **cycle month** you type (`statement_month`, e.g. August → `2026-08`). Statement date and due date do not have to fall in that calendar month. That is how `plan` loads the previous month’s card bills for this payment month, and how the TUI can show what is coming due so you can leave money in a HYSA until the last comfortable moment.

**User:** you, after a statement posts, typing the cycle month, the minimum, and the due date.

**This module succeeds when** you can record one statement per card per cycle month, `plan` can read that month’s minimum and due date without guessing, a card with no row for a month is simply absent (`Ok` empty — `plan` shows **no statement yet**), and upcoming due dates can be listed without notifications.

**Out of scope for this module:** loan due days (those live on `debts`), payments, snowball size, allocation, TUI, bank import, reminders, restating card balance.

## Tech Stack

Same crate as `debts` and `snowball-size`:

- Rust (edition 2024), `snowball` library
- SQLite, same on-disk file; table `statements`
- Money as integer **cents** (`i64`)
- Calendar dates as `YYYY-MM-DD` strings (no timezone)
- Cycle month as `YYYY-MM` strings
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

Reuse `src/error.rs` and `DebtId` / `DebtKind` from `debts`. Do not put minimum due, due dates, or cycle month on `Debt`.

## Code Style

Match `debts`: `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE` error codes, `Result<T, Error>` on the public surface, opaque ids (`stmt_<ulid>`).

```rust
pub struct StatementId(String);

pub struct Statement {
    pub id: StatementId,
    pub debt_id: DebtId,
    pub statement_month: String, // YYYY-MM, the cycle you typed (e.g. "2026-08")
    pub minimum_cents: i64,
    pub due_on: String, // YYYY-MM-DD
}

pub struct RecordStatement {
    pub debt_id: DebtId,
    pub statement_month: String,
    pub minimum_cents: i64,
    pub due_on: String,
}

pub trait StatementStore {
    fn record(&self, input: RecordStatement) -> Result<Statement, Error>;
    fn get(&self, id: &StatementId) -> Result<Statement, Error>;
    fn list_for_debt(&self, debt_id: &DebtId) -> Result<Vec<Statement>, Error>;
    fn list_for_month(&self, statement_month: &str) -> Result<Vec<Statement>, Error>;
    fn current_for_debt(&self, debt_id: &DebtId) -> Result<Option<Statement>, Error>;
    fn upcoming(&self, as_of: &str) -> Result<Vec<Statement>, Error>;
}
```

`list_for_debt` returns that card’s statements, `statement_month` ascending, then `id`.

`list_for_month` returns every statement with that `statement_month`, `name` is not on the row — order by `debt_id` then `id`. Missing cards are simply not in the list. `plan` joins this to owed cards.

`current_for_debt` is the statement with the **latest `statement_month`** for that card, or `None` if none exist. A missing row is `Ok(None)` — `plan` decides what that means for a given payment month (it uses `list_for_month` for the previous month, not `current_for_debt`).

`upcoming(as_of)` returns every statement with `due_on >= as_of`, ordered by `due_on` ascending. Tie-break: `due_on`, then `id`. Overdue rows (`due_on < as_of`) are omitted; the TUI can filter `list_for_debt` when it needs overdue (no overdue store method in this module).

Recording the same `(debt_id, statement_month)` again **replaces** `minimum_cents` and `due_on` (and keeps the same id). That is a correction, not a second bill. Different cycle months are different rows. `due_on` is not unique.

This module does not infer `statement_month` from `due_on` or from a statement date. You type the cycle.

## Testing Strategy

- Framework: `cargo test`
- Location: unit tests next to validation; SQLite round-trips in `tests/statements.rs` using a shared in-memory DB that also has the `debts` schema
- Coverage: every public `StatementStore` method has a success test and a rejection test. Plus: loan rejected; unknown debt `NOT_FOUND`; duplicate `statement_month` updates min and due date; two cycle months both exist; `list_for_month` returns only that month; `current_for_debt` follows latest `statement_month`; `upcoming` excludes earlier dates; recording does not change snowball size or `Debt.balance_cents`
- **Not here:** plan allocation, payments, TUI

## Boundaries

- **Always:** Store money as cents; validate at the store boundary; only `DebtKind::CreditCard`; require the debt to exist; leave snowball size and balances untouched; one row per `(debt_id, statement_month)`; require `statement_month` from the caller
- **Ask first:** Deleting statements; statement close date / APR / fees as fields
- **Never:** Record a statement against a loan; guess a minimum, due date, or `statement_month`; auto-edit snowball size; call `set_balance` / `reduce_balance`; use `f64` for money; send notifications; add an overdue query method (TUI filters `list_for_debt`); key statements by `due_on`

## Module contract (consumers)

`plan` depends on: `list_for_month(statement_month)` for the **previous** calendar month relative to the payment month. Cards with remaining balance and no row there are **no statement yet** in `plan`.

`tui` depends on: `record` (including `statement_month`) and `list_for_debt` (overlay; overdue is `due_on <` today on those rows). Home does not call `upcoming` — due dates on `compute_plan` lines are the list. `current_for_debt` and `upcoming` stay on the store.

`payments` does **not** depend on this module.

This module depends on `debts`: `DebtId`, `get` (or equivalent lookup) to confirm the debt exists and is `CreditCard`.

### Shared SQLite

`debts` and `statements` must see the same rows. `SqliteDb` owns one `Connection`. Table `statements`: `id`, `debt_id` FK → `debts.id`, `statement_month` TEXT `YYYY-MM`, `minimum_cents`, `due_on` TEXT `YYYY-MM-DD`, `UNIQUE(debt_id, statement_month)`.

`migrate()` must result in that schema for new and existing in-memory databases used by tests. Do not keep `UNIQUE(debt_id, due_on)`.

Deleting debts is still out of scope.

### Errors (one shape)

```text
NOT_FOUND          — get on unknown statement id; record against unknown debt_id
VALIDATION_ERROR   — debt is a loan; invalid due_on; invalid statement_month; minimum_cents < 0
```

### Record rules

| Field | Rule |
|---|---|
| debt_id | required; must exist; must be `CreditCard` |
| statement_month | required, `YYYY-MM` (reject `2026-13`, `2026-9`, `2026-08-01`) |
| minimum_cents | required, `>= 0` (a $0 minimum is real) |
| due_on | required, `YYYY-MM-DD`, calendar-valid (reject `2026-02-31`) |

Callers do not mint `StatementId`. `due_on` may fall outside `statement_month` (August cycle due September 14 is normal).

## Success Criteria

- [x] Recording a statement on a credit card persists minimum and due date and reads back unchanged.
- [x] Recording also persists `statement_month` and reads it back unchanged.
- [x] Recording a statement on a loan is `VALIDATION_ERROR` and inserts nothing.
- [x] Recording against an unknown `debt_id` is `NOT_FOUND`.
- [x] Two statements on the same card with different `statement_month` values both exist; `current_for_debt` is the later `statement_month`.
- [x] Recording the same card + `statement_month` again updates `minimum_cents` and `due_on` (correction) and does not add a second row.
- [x] `minimum_cents < 0` or invalid `due_on` is `VALIDATION_ERROR`.
- [x] Invalid `statement_month` is `VALIDATION_ERROR`.
- [x] `list_for_month("2026-08")` returns August-cycle rows only, including a row whose `due_on` is in September.
- [x] `upcoming("2026-09-08")` includes due 2026-09-08 and later, excludes 2026-09-07.
- [x] Recording a statement does not change `Debt.balance_cents` or snowball size.
- [x] Public API has no snowball-size or payment fields.
- [x] `cargo test --test statements` passes with SQLite in memory after this amendment.

## Decisions

Confirmed 2026-09-08 (original).

- `current_for_debt` is not “next due from today.” `upcoming` covers what is due soon.
- No overdue store method. The TUI filters `list_for_debt` when it needs past-due rows.
- Introduce `SqliteDb`: one shared SQLite connection so `statements.debt_id` can reference `debts.id`. `snowball-size` may keep its own connection in existing tests.
- `minimum_cents >= 0`, including zero.

Amended 2026-09-08 (for `plan`).

- Unique key is `(debt_id, statement_month)`, not `(debt_id, due_on)`. You type the cycle month. Recording the same pair again replaces minimum and due date.
- `due_on` may be in a different calendar month than `statement_month`.
- `plan` uses `list_for_month`, not `current_for_debt`.
- `current_for_debt` follows latest `statement_month`.
