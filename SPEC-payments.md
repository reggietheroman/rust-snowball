# Spec: payments

## Objective

The `payments` module is the ledger of money you actually paid toward a debt. That is how remaining balance falls, and how you can later see that you hit the snowball instead of only intending to.

**User:** you, after a transfer, typing what you paid (and which debt). You will mistype amounts and pick the wrong debt; those rows still save, and you fix them by editing the payment.

**This module succeeds when** every payment is a row you can read back and edit, a record/edit keeps that debt’s remaining balance in sync (overpayment floors the debt at 0 and is still visible on the row), and `plan` can ignore this module entirely.

**Out of scope for this module:** snowball size, card minimums, due dates, allocation, drawing the overpayment flag (that is `tui`), bank import, matching a payment to a statement, deleting payments, credit balances, refunds from lenders.

## Tech Stack

Same crate as `debts` and `statements`:

- Rust (edition 2024), `snowball` library
- SQLite, same on-disk file; table `payments` on `SqliteDb`
- Money as integer **cents** (`i64`)
- Calendar dates as `YYYY-MM-DD` strings (no timezone)
- Module at `src/payments/`

## Commands

```
cargo test
cargo test --test payments
cargo clippy -- -D warnings
cargo fmt --check
cargo build
```

## Project Structure

```
SPEC-payments.md
src/db.rs                  → Shared SQLite connection + migrations (add `payments`)
src/payments/mod.rs        → Public contract (types + PaymentStore)
src/payments/store.rs      → SQLite implementation
src/payments/validate.rs   → Boundary validation
tests/payments.rs          → Integration tests against in-memory SQLite
```

Reuse `src/error.rs` and `DebtId` / `DebtStore` from `debts`. Do not put payment history on `Debt`. Do not read `statements` or snowball size.

## Code Style

Match `debts` / `statements`: `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE` error codes, `Result<T, Error>` on the public surface, opaque ids (`pay_<ulid>`).

```rust
pub struct PaymentId(String);

pub struct Payment {
    pub id: PaymentId,
    pub debt_id: DebtId,
    pub amount_cents: i64,  // what you logged as paid
    pub applied_cents: i64, // what actually left the remaining balance
    pub paid_on: String,    // YYYY-MM-DD
}

impl Payment {
    /// True when you paid more than remaining balance at apply time.
    pub fn is_overpayment(&self) -> bool {
        self.amount_cents > self.applied_cents
    }
}

pub struct RecordPayment {
    pub debt_id: DebtId,
    pub amount_cents: i64,
    pub paid_on: String,
}

pub struct UpdatePayment {
    pub debt_id: Option<DebtId>,
    pub amount_cents: Option<i64>,
    pub paid_on: Option<String>,
}

pub trait PaymentStore {
    fn record(&self, input: RecordPayment) -> Result<Payment, Error>;
    fn get(&self, id: &PaymentId) -> Result<Payment, Error>;
    fn list_for_debt(&self, debt_id: &DebtId) -> Result<Vec<Payment>, Error>;
    fn update(&self, id: &PaymentId, patch: UpdatePayment) -> Result<Payment, Error>;
}
```

`list_for_debt` returns that debt’s payments, `paid_on` ascending, then `id`. Two payments on the same debt and date both exist — there is no unique `(debt_id, paid_on)`.

`applied_cents` is `min(amount_cents, balance_before)` at the moment the row is applied. It can be `0` (debt already at `$0`). Callers do not pass it.

`is_overpayment` is `amount_cents > applied_cents`. This module does not warn, reject, or create a credit. The leftover is permanent from our side (you cannot control the lender). `tui` uses this to flag the row so you can see you overpaid.

`update` is partial: omitted fields stay. Changing `amount_cents` or `debt_id` reverses the old `applied_cents` on the old debt, then applies the new amount on the (possibly new) debt. Changing only `paid_on` does not touch balances.

Record and update run in **one SQLite transaction** with the balance change. A failed validation inserts nothing and leaves balances unchanged.

There is no `delete`. A payment that should not exist is edited into the payment you meant (amount and/or debt).

### Balance rules

`record` applies the payment with `DebtStore::reduce_balance` when `applied_cents > 0`. Paying past remaining balance saturates the debt at `0` (already specified on `debts`). The extra stays on the payment (`amount_cents - applied_cents`); we do not put a credit on the debt or model a lender refund.

`update` that changes amount or debt restores the old applied cents with `DebtStore::increase_balance` when `applied_cents > 0`, then reduces the target debt for the new applied amount. `increase_balance` is the dual of `reduce_balance`: `cents > 0`, remaining balance goes up with no cap. It is added to `DebtStore` for this module (see `SPEC-debts.md`). Skip the reduce or increase call when the cents would be `0` (`reduce_balance` / `increase_balance` reject non-positive).

Do not use `set_balance` to apply or reverse a payment. That method is for restating a card (spending), not for this ledger.

Recording or editing does **not** check the plan, a loan floor, or a card minimum. A “wrong” amount or debt is still a valid row.

## Testing Strategy

- Framework: `cargo test`
- Location: unit tests next to validation; SQLite round-trips in `tests/payments.rs` using `SqliteDb::open_in_memory()` (debts schema present)
- Coverage: every public `PaymentStore` method has a success test and a rejection test. Plus: unknown debt `NOT_FOUND`; overpayment saves, `applied_cents` is the prior balance, and `is_overpayment` is true; payment against `$0` saves with `applied_cents == 0` and `is_overpayment` true; two same-day payments both exist; edit amount restores then re-applies; edit debt moves applied cents from old debt to new; edit `paid_on` only leaves balances unchanged; record/update do not change snowball size
- **Not here:** plan allocation, statements, TUI

## Boundaries

- **Always:** Store money as cents; validate at the store boundary; require the debt to exist (loan or card); persist even when amount exceeds remaining balance or the debt is already `$0`; keep remaining balance in sync via `applied_cents`; expose `is_overpayment` so `tui` can flag; one transaction per record/update; leave snowball size and statements untouched
- **Ask first:** Notes on a payment; linking a payment to a statement
- **Never:** Delete a payment (edit it instead); reject an overpayment; put a credit balance on the debt; model a lender refund; reject a payment because it does not match the plan or a minimum; auto-edit snowball size; call `set_balance` to apply or reverse a payment; use `f64` for money; talk to a bank; key payments by `(debt_id, paid_on)`; depend on `statements`; draw the overpayment flag here (that is `tui`)

## Module contract (consumers)

`tui` depends on: `record`, `update`, `get`, `list_for_debt`, and `Payment::is_overpayment` (flag the row; do not block saving).

`plan` does **not** depend on this module. This month’s split is size + loan floors + card mins, not “what’s left after you’ve logged payments.”

This module depends on `debts`: `DebtId`, `get`, `reduce_balance`, `increase_balance`.

`statements` does not depend on this module; this module does not depend on `statements`.

### Shared SQLite

Add table `payments` to `SqliteDb` migrations. Foreign key: `payments.debt_id` references `debts.id`. Deleting debts is still out of scope.

### Errors (one shape)

```text
NOT_FOUND          — get/update on unknown payment id; record/update against unknown debt_id
VALIDATION_ERROR   — amount_cents <= 0; invalid paid_on
```

### Record / update rules

| Field | Rule |
|---|---|
| debt_id | required on record; must exist (loan or card) |
| amount_cents | required on record, `> 0` |
| paid_on | required on record, `YYYY-MM-DD`, calendar-valid (reject `2026-02-31`) |
| applied_cents | assigned by the store; callers do not pass it |

Callers do not mint `PaymentId`. Future `paid_on` is allowed (you typed it).

## Success Criteria

- [x] Recording a payment on a loan or card persists amount and date, reduces remaining balance by `applied_cents`, and reads back unchanged.
- [x] Recording against an unknown `debt_id` is `NOT_FOUND` and inserts nothing.
- [x] `amount_cents <= 0` or invalid `paid_on` is `VALIDATION_ERROR` and inserts nothing.
- [x] Paying $500 on a $400 debt saves; `amount_cents` is $500, `applied_cents` is $400, balance is $0, `is_overpayment` is true.
- [x] Paying $100 on a $0 debt saves; `applied_cents` is $0; balance stays $0; `is_overpayment` is true.
- [x] Paying $400 on a $400 debt: `is_overpayment` is false.
- [x] Two payments on the same debt and date both exist.
- [x] Updating amount from $400 to $40 on a debt that had $1000 restores $400 then applies $40 (net balance +$360 vs the post-record state).
- [x] Updating `debt_id` from A to B restores applied cents on A and applies on B.
- [x] Updating only `paid_on` leaves both balances and `applied_cents` unchanged.
- [x] Recording a payment does not change snowball size or any statement.
- [x] Public API has no snowball-size or statement fields.
- [x] `cargo test --test payments` passes with SQLite in memory.

## Open Questions

None.

## Decisions

Confirmed 2026-09-08.

- A wrong amount or wrong debt still saves. Each payment is editable (`debt_id`, `amount_cents`, `paid_on`). There is no `delete`.
- Reversals use `increase_balance` on `DebtStore`, not `set_balance`.
- Overpayment is permanent from our side: debt floors at `$0`, leftover stays on the payment as `amount_cents - applied_cents`. No credit balance, no refund. `tui` flags `is_overpayment`; this module only exposes that fact.
- `applied_cents` is stored so an overpayment (or a payment on an already-zero debt) can be reversed on edit without inventing remaining balance, and so the client can flag it.
- `plan` does not read this ledger.
