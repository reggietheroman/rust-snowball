# Spec: debts

## Objective

The `debts` module is the registry of money you owe. Everything else in Snowball hangs off a debt: statements (cards), payments, and the plan (which loan floors to pay, which remaining balance is smallest).

**User:** you, sitting down to add or correct a loan or card.

**This module succeeds when** you can record each debt once, keep its remaining balance truthful, and other modules can read a stable, typed record without knowing how it is stored.

**Out of scope for this module:** snowball size, card minimums, due dates on cards (those live on statements), payment history, allocation math, TUI.

## Tech Stack

Shared by this repo (first module establishes it):

- Rust (edition 2024), one Cargo package with a library (`snowball`) and a later binary for `tui`
- SQLite for on-disk data (single user, local file)
- Money as integer **cents** (`i64`), never floating point
- Domain modules as Rust modules under `src/` (`src/debts`, later `src/snowball_size`, …) so `plan` can be tested without a terminal

## Commands

```
cargo test              # all library tests
cargo test debts        # this module's tests
cargo clippy -- -D warnings
cargo fmt --check
cargo build
```

(The `tui` binary is not in this module. Do not add a `dev` TUI command here.)

## Project Structure

```
CAPABILITY-MAP.md           → Module index (this initiative)
SPEC-debts.md               → This spec
src/lib.rs                  → Library root, re-exports module contracts
src/debts/mod.rs            → Public contract (types + DebtStore)
src/debts/store.rs          → SQLite implementation
src/debts/validate.rs       → Boundary validation
src/error.rs                → Shared error type for the library
tests/debts.rs              → Integration tests against an in-memory SQLite DB
```

Later modules add sibling directories (`src/snowball_size`, …), not nested folders inside `debts`.

## Code Style

- Names: `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE` error codes.
- Debts are a **discriminated union**. Loan-only fields do not exist on cards.
- Public functions return `Result<T, Error>`. No `unwrap` on the public surface.
- IDs are opaque strings (`debt_<ulid>`). Callers do not mint IDs.

```rust
pub enum DebtKind {
    Loan {
        payment_cents: i64,
        due_day: u8, // 1–31; clamp to last day of the month at use
    },
    CreditCard,
}

pub struct Debt {
    pub id: DebtId,
    pub name: String,
    pub balance_cents: i64,
    pub kind: DebtKind,
}

pub trait DebtStore {
    fn create(&self, input: CreateDebt) -> Result<Debt, Error>;
    fn get(&self, id: &DebtId) -> Result<Debt, Error>;
    fn list(&self) -> Result<Vec<Debt>, Error>;
    fn update(&self, id: &DebtId, patch: UpdateDebt) -> Result<Debt, Error>;
    fn set_balance(&self, id: &DebtId, balance_cents: i64) -> Result<Debt, Error>;
    fn reduce_balance(&self, id: &DebtId, cents: i64) -> Result<Debt, Error>;
    fn increase_balance(&self, id: &DebtId, cents: i64) -> Result<Debt, Error>;
}
```

`list` returns **all** debts, paid-off included, in a stable order (name ascending). Consumers that want “still owed” filter `balance_cents > 0`. No pagination: this is a personal register, tens of rows.

## Testing Strategy

- Framework: `cargo test` (Rust unit + integration tests).
- Location: unit tests next to the code; SQLite round-trips in `tests/debts.rs` using SQLite `:memory:`.
- Coverage bar for this module: every public `DebtStore` method has a success test and a rejection test for invalid input. Discriminated union tests prove a card cannot be created with a loan payment.
- Levels:
  - **Unit:** validation (`due_day` range, empty name, negative cents, `reduce_balance` saturates at 0).
  - **Integration:** create / get / list / update / set_balance / reduce_balance against SQLite.
  - **Not here:** TUI, plan allocation, statements.

## Boundaries

- **Always:** Store money as cents; validate at the `DebtStore` boundary; keep loan payment and due day off credit cards; infer “paid off” as `balance_cents == 0` (no separate status flag); leave snowball size untouched.
- **Ask first:** New debt kinds; interest/APR; deleting debts; changing the SQLite schema after `payments` or `statements` exist; storing card minimums on the debt.
- **Never:** Use `f64` for money; auto-create or auto-edit snowball size; guess a card’s minimum or due date; talk to a bank; unwrap in library code on expected user/input failures.

## Module contract (consumers)

`statements` depends on: `DebtId`, `DebtKind::CreditCard`, `get` / `list`.

`payments` depends on: `DebtId`, `get`, `reduce_balance` (a logged payment applies here; overshoot saturates at 0 so the last payoff can exceed remaining balance), `increase_balance` (an edited payment puts applied cents back; no cap).

`plan` depends on: `list`, `balance_cents > 0`, `DebtKind::Loan { payment_cents, .. }` as the floor for loans. Card floors come from `statements`, not from this module.

`tui` depends on: full `DebtStore` (create, edit name/payment/due day, set_balance for corrections and new card balances).

### Errors (one shape)

```text
NOT_FOUND          — get/update on an unknown id
VALIDATION_ERROR   — empty/duplicate name, negative cents, due_day not in 1–31,
                     loan payment_cents <= 0, card created with loan fields
```

`message` is human-readable; `code` is stable.

### Create / update rules

| Field | Loan | Credit card |
|---|---|---|
| name | required, unique, non-empty | same |
| balance_cents | required, `>= 0` | same |
| payment_cents | required, `> 0` | **must not be set** |
| due_day | required, `1–31` | **must not be set** |

`update` is partial: omitted fields stay. Changing kind (loan ↔ card) is **not** allowed; archive-by-payoff and create a new debt instead if that ever happens.

`set_balance` is how a card’s remaining balance is restated (spending, statement balance). It does not record a payment.

`reduce_balance` subtracts `cents` (`> 0`); result is `max(0, balance - cents)`.

`increase_balance` adds `cents` (`> 0`); result is `balance + cents`. No cap. Skip the call when the cents would be `0` (same rejection as `reduce_balance` for non-positive). Added for `payments` so an edit can restore applied cents without using `set_balance`.

## Success Criteria

- [x] A loan can be created with name, balance, monthly payment, and due day, and read back unchanged.
- [x] A credit card can be created with name and balance only; creating it with a payment or due day is rejected.
- [x] Two debts cannot share a name.
- [x] `list` includes a debt whose balance is 0 (paid off); `plan` can ignore it by filtering.
- [x] `reduce_balance` on $400 by $500 yields $0, not an error.
- [x] `increase_balance` on $0 by $400 yields $400; `cents <= 0` is `VALIDATION_ERROR`. Implement with `payments`.
- [x] `set_balance` can raise a card balance (new spending) without going through `reduce_balance`.
- [x] No public API exposes card minimum due or snowball size.
- [x] `cargo test debts` passes with SQLite in memory.

## Open Questions

- Confirm Rust + SQLite (assumed; say if you want Go/Python or a file store instead).
- Confirm due day `1–31` with clamp-at-use for short months (assumed).
- Confirm we do **not** model APR/interest (snowball is smallest remaining balance; assumed).
