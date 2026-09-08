# Implementation Plan: payments

## Overview

Add the payment ledger: `record` / `get` / `list_for_debt` / `update`, each row reducing (or on edit, restoring then re-applying) remaining balance. Add `increase_balance` to `DebtStore`. Overpayment floors the debt at `$0`, stores `applied_cents`, and exposes `is_overpayment` for the TUI — this module does not draw the flag. When this plan is done, `tui` can log and edit payments; `plan` still does not read this module.

Prior module archive: `tasks/plan-statements.md`, `tasks/todo-statements.md`.

## Architecture Decisions

- **Match `statements`.** `PaymentStore` trait + `SqlitePaymentStore<'db>` on `&SqliteDb`. Ids `pay_<ulid>` via monotonic `Generator`. Money as `i64` cents. Same in-memory harness as statements tests.
- **Table `payments` on `SqliteDb`.** Columns: `id`, `debt_id` FK → `debts.id`, `amount_cents` CHECK `> 0`, `applied_cents` CHECK `>= 0`, `paid_on` TEXT. No `UNIQUE(debt_id, paid_on)`.
- **`increase_balance` on `DebtStore`.** Dual of `reduce_balance`: `cents > 0`, `balance + cents`, no cap. Same `validate_*` as reduce for the amount. `OwnedDebtStore` forwards it. Implement before any payment write.
- **Apply via `DebtStore`, not `set_balance`.** `record` / `update` call `reduce_balance` / `increase_balance` when the cents are `> 0`; skip the call at `0`.
- **One transaction.** `record` and `update` wrap lookup + payment row + balance change in `unchecked_transaction()` on the shared connection, then `commit`. Failed validation rolls back. `SqliteDebtStore` already uses that same `Connection`; statements on it participate in the open txn.
- **`applied_cents`.** Computed as `min(amount_cents, balance_before)` inside the txn. Callers never pass it. `Payment::is_overpayment` is `amount_cents > applied_cents`.
- **Date validation.** Same calendar `YYYY-MM-DD` rules as statements. Copy into `src/payments/validate.rs` (field name `paid_on`). Do not import `statements` and do not extract a shared date module in this plan.
- **Loans and cards.** Both are valid `debt_id`s. Unknown id → `NOT_FOUND`.
- **No delete, no notes, no statement link, no snowball-size writes.**

## Dependency graph (this module)

```
increase_balance on DebtStore
    │
    └── payments table on SqliteDb
            │
            └── Payment types + PaymentStore + validation
                    │
                    └── SqlitePaymentStore::record / get
                            │
                            ├── overpayment / $0 / is_overpayment / two same-day
                            │
                            └── list_for_debt + update (amount, debt, paid_on)
```

## What can be parallel vs sequential

All sequential. `increase_balance` and the `payments` table must exist before `record`.

## Task List

Index only. Full acceptance criteria live in `tasks/todo.md`.

### Foundation

- [x] Task 1: `increase_balance` on `DebtStore`; debts tests still pass
- [x] Task 2: `payments` table on `SqliteDb`

### Checkpoint: Shared DB

- [x] `cargo test --test debts` passes
- [x] `SqliteDb::open_in_memory()` creates `payments`

### Payment slices

- [x] Task 3: Scaffold payments; `record` / `get`; apply `reduce_balance`; reject unknown debt and bad input
- [x] Task 4: Overpayment, `$0` debt, exact payoff, two same-day rows
- [x] Task 5: `list_for_debt`; `update` amount / debt / `paid_on`; no statement or snowball-size side effects

### Checkpoint: payments complete

- [x] All `SPEC-payments.md` success criteria met
- [x] `cargo test --test payments`, `cargo test --test debts`, `cargo test --test statements`, `cargo test --test snowball_size`, clippy, fmt pass
- [x] Ready for `plan` spec

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Transaction does not wrap debt updates | High | Integration test: invalid `paid_on` after a valid shape still leaves balance unchanged; crash-safety via one `commit`. |
| Overpayment reverse invents balance | High | Store `applied_cents`; edit restores that, not `amount_cents`. Test $500 on $400 then edit amount. |
| Date logic drifts from statements | Low | Copy the same calendar rules; shared extractor is out of scope. |
| Accidental `statements` / snowball coupling | Med | Public types have no statement or size fields. Tests assert those tables/rows are untouched. |

## Open Questions

None. Spec decisions confirmed 2026-09-08.

## Task list target

`tasks/todo.md` (markdown checklist). No external tracker.
