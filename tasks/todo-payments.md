# Tasks: payments

Verification commands (every task):

- Tests: `cargo test --test payments`
- Regression: `cargo test --test debts`, `cargo test --test statements`, and `cargo test --test snowball_size`
- Build: `cargo build`
- After checkpoints: `cargo clippy -- -D warnings` and `cargo fmt --check`

---

## Task 1: increase_balance on DebtStore

**Description:** Add `increase_balance` to `DebtStore` (and `OwnedDebtStore`). `cents > 0`; result is `balance + cents` with no cap. Non-positive is `VALIDATION_ERROR`. Unknown id is `NOT_FOUND`. Do not add payment logic.

**Acceptance criteria:**
- [x] `increase_balance` on $0 by $400 yields $400
- [x] `cents <= 0` is `VALIDATION_ERROR` and does not change the row
- [x] Unknown id is `NOT_FOUND`
- [x] Existing `reduce_balance` / `set_balance` tests still pass

**Verification:**
- [x] Tests pass: `cargo test --test debts`
- [x] Build succeeds: `cargo build`

**Dependencies:** None

**Files likely touched:**
- `src/debts/mod.rs`
- `src/debts/store.rs`
- `src/debts/validate.rs`
- `tests/debts.rs`

**Estimated scope:** Medium

---

## Task 2: payments table on SqliteDb

**Description:** Add `CREATE TABLE IF NOT EXISTS payments` to `SqliteDb::migrate` with FK to `debts.id`. No payment store yet.

**Acceptance criteria:**
- [x] `SqliteDb::open_in_memory()` creates the `payments` table
- [x] `PRAGMA foreign_keys` remains on
- [x] Existing debts and statements tests still pass

**Verification:**
- [x] Tests pass: `cargo test --test debts` and `cargo test db`
- [x] Build succeeds: `cargo build`

**Dependencies:** None (can follow Task 1; does not depend on `increase_balance`)

**Files likely touched:**
- `src/db.rs`

**Estimated scope:** Small

---

## Checkpoint: Foundation

- [x] `cargo test --test debts` passes
- [x] `SqliteDb::open_in_memory()` creates `payments`
- [x] `cargo test --test statements` still passes

---

## Task 3: Scaffold payments; record and get; apply balance; reject bad input

**Description:** Add `src/payments/` with types, `is_overpayment`, trait, validation (`amount_cents > 0`, calendar `paid_on`), and `SqlitePaymentStore::record` / `get` in one transaction. Record on a loan or card persists, sets `applied_cents`, and calls `reduce_balance` when applied `> 0`. Unknown `debt_id` → `NOT_FOUND`. Bad amount or date → `VALIDATION_ERROR` and no insert / no balance change.

**Acceptance criteria:**
- [x] Recording on a loan persists amount and date; `get` returns it; balance drops by `applied_cents`
- [x] Recording on a card does the same
- [x] Unknown `debt_id` is `NOT_FOUND`; nothing inserted
- [x] `amount_cents <= 0` or invalid `paid_on` is `VALIDATION_ERROR`; balance unchanged

**Verification:**
- [x] Tests pass: `cargo test --test payments`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 1, Task 2

**Files likely touched:**
- `src/payments/mod.rs`
- `src/payments/validate.rs`
- `src/payments/store.rs`
- `tests/payments.rs`
- `src/lib.rs`

**Estimated scope:** Medium

---

## Task 4: Overpayment, zero balance, exact payoff, two same-day rows

**Description:** Paying more than remaining balance saves; `applied_cents` is the prior balance; debt is `$0`; `is_overpayment` is true. Paying on `$0` saves with `applied_cents == 0` and does not call `reduce_balance`. Exact payoff is not an overpayment. Two payments on the same debt and date both exist.

**Acceptance criteria:**
- [x] $500 on $400: amount $500, applied $400, balance $0, `is_overpayment` true
- [x] $100 on $0: applied $0, balance $0, `is_overpayment` true
- [x] $400 on $400: `is_overpayment` false
- [x] Two payments same debt and date both exist

**Verification:**
- [x] Tests pass: `cargo test --test payments`

**Dependencies:** Task 3

**Files likely touched:**
- `src/payments/store.rs`
- `src/payments/mod.rs`
- `tests/payments.rs`

**Estimated scope:** Small

---

## Task 5: list_for_debt and update; no side effects on statements or snowball size

**Description:** `list_for_debt` returns that debt’s payments `paid_on` ascending, then `id`. `update` is partial. Changing amount or `debt_id` restores old `applied_cents` then applies the new amount. Changing only `paid_on` leaves balances and `applied_cents` unchanged. Recording/updating does not change snowball size or any statement. Public API has no snowball-size or statement fields.

**Acceptance criteria:**
- [x] `list_for_debt` is `paid_on` ascending, then `id`
- [x] Update amount $400 → $40 on a $1000 debt: net balance +$360 vs post-record
- [x] Update `debt_id` A → B restores applied on A and applies on B
- [x] Update only `paid_on` leaves balances and `applied_cents` unchanged
- [x] Record/update does not change snowball size or statements
- [x] Public API has no snowball-size or statement fields

**Verification:**
- [x] Tests pass: `cargo test --test payments`
- [x] Regression: `cargo test --test debts`, `cargo test --test statements`, `cargo test --test snowball_size`
- [x] Clippy/fmt: `cargo clippy -- -D warnings` and `cargo fmt --check`

**Dependencies:** Task 4

**Files likely touched:**
- `src/payments/store.rs`
- `src/payments/validate.rs`
- `tests/payments.rs`

**Estimated scope:** Medium

---

## Checkpoint: payments complete

- [x] All success criteria in `SPEC-payments.md` are met
- [x] `cargo test --test payments` passes
- [x] `cargo test --test debts`, `cargo test --test statements`, and `cargo test --test snowball_size` pass
- [x] `cargo clippy -- -D warnings` and `cargo fmt --check` pass
- [x] Ready for review; next map module to spec is `plan`
