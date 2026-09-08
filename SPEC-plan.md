# Spec: plan

## Objective

The `plan` module answers: **what do I send this payment month, to whom, and when is it due?** You open the September plan while spending September earnings. Card bills on that plan are the **previous** cycle (August), because that is how cards work. Loan installments on that plan are due in September.

**User:** you, looking at this month before you transfer money out of savings.

**This module succeeds when** it can list every debt you still owe, with the amount to send and a due date (or **no statement yet** on a card), extra after required minima goes to the smallest remaining balance and rolls to the next when that debt is filled, and required minima that exceed the recorded snowball amount still show in full plus a shortfall — the recorded amount does not change.

**Out of scope for this module:** TUI, logging payments, recording statements or snowball amounts, upcoming-due lists beyond what appears on these lines, interest, bank import.

## Tech Stack

Same crate as `debts` / `statements` / `snowball-size`:

- Rust (edition 2024), `snowball` library
- No new SQLite table. The plan is calculated when asked for.
- Money as integer **cents** (`i64`)
- Payment month and statement cycle as `YYYY-MM` strings (no timezone)
- Calendar dates as `YYYY-MM-DD` strings
- Module at `src/plan/`

## Commands

```
cargo test
cargo test --test plan
cargo clippy -- -D warnings
cargo fmt --check
cargo build
```

## Project Structure

```
SPEC-plan.md
SPEC-statements.md         → Amended: `statement_month` (see that spec)
src/plan/mod.rs            → Public contract (types + compute)
src/plan/compute.rs        → Allocation
src/plan/validate.rs       → Payment-month / calendar helpers
tests/plan.rs              → Integration tests against in-memory SQLite
```

Reuse `DebtStore`, `SnowballSizeStore`, `StatementStore`, and `src/error.rs`. Do not read `payments`.

Implement the `statement_month` amendment on `statements` **before** `compute` (plan cannot load August bills without it).

## Code Style

Match `debts` / `statements`: `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE` error codes, `Result<T, Error>` on the public surface.

```rust
pub struct Plan {
    pub payment_month: String, // YYYY-MM, e.g. "2026-09"
    /// Latest recorded snowball amount, or `None` if you have never recorded one.
    pub snowball_amount_cents: Option<i64>,
    /// `None` when there is no recorded snowball amount.
    /// `Some(0)` when required minima fit (or there are none).
    /// `Some(n)` when required minima exceed the recorded amount by `n` cents.
    pub shortfall_cents: Option<i64>,
    /// Extra that could not land because every remaining balance was already filled.
    pub unallocated_cents: i64,
    pub lines: Vec<PlanLine>,
}

pub struct PlanLine {
    pub debt_id: DebtId,
    pub name: String,
    pub remaining_cents: i64,
    /// Required this month, never more than remaining balance.
    /// Cards with no statement for the cycle month: `0`.
    pub required_cents: i64,
    /// Extra after required minima, rolled onto this debt.
    pub extra_cents: i64,
    /// `required_cents + extra_cents`. Never more than `remaining_cents`.
    pub send_cents: i64,
    /// Loans: payment month + due day (clamped).
    /// Cards with a statement: that statement's `due_on`.
    /// Cards with no statement: `None`.
    pub due_on: Option<String>,
    /// Credit card with no row for the previous calendar month.
    pub missing_statement: bool,
}

pub fn compute_plan(
    payment_month: &str,
    debts: &impl DebtStore,
    sizes: &impl SnowballSizeStore,
    statements: &impl StatementStore,
) -> Result<Plan, Error>;
```

`lines` include every debt with `remaining_cents > 0`. Paid-off debts (`0`) are omitted.

Order of `lines`: `due_on` ascending, missing due dates last, then `name` ascending.

### How the payment month maps to data

| Input | Meaning |
|---|---|
| `payment_month` | The month whose earnings you are sending (`2026-09` = September). |
| Card cycle | Previous calendar month (`2026-08`). January → previous December (`2026-01` → `2025-12`). |
| Recorded snowball amount | `SnowballSizeStore::current()` → `amount_cents`, or `None`. |
| Loan required | `min(payment_cents, remaining_cents)`. Due date = that due day in `payment_month`, clamped to the last day of the month (same clamp as `SPEC-debts.md`). |
| Card required | If `StatementStore` has a row for that card and cycle month: `min(minimum_cents, remaining_cents)`. If not: required `0`, `missing_statement = true`, `due_on = None`. Extra may still go to this card. |

### Allocation

1. Build one line per owed debt with `required_cents` as above. `extra_cents` starts at `0`.
2. Let `required_total` be the sum of `required_cents`.
3. **No recorded snowball amount:** `snowball_amount_cents = None`, `shortfall_cents = None`, every `extra_cents = 0`, `unallocated_cents = 0`. Stop.
4. **Recorded amount `<` `required_total`:** still keep every line’s `required_cents` (you send what each lender requires). `extra_cents` stays `0`. `shortfall_cents = Some(required_total - snowball)`. `unallocated_cents = 0`. Stop.
5. **Otherwise:** `shortfall_cents = Some(0)`. Let `freed_total` be the sum of each debt’s raw floor minus its capped `required_cents` (loan usual payment or card statement minimum). Extra pool = `snowball - required_total + freed_total`.
6. Walk owed debts by **remaining balance ascending**, then **name ascending**. For each, room left = `remaining_cents - required_cents`. Give `min(pool, room)` as `extra_cents`. Subtract from the pool. A later debt only gets extra after earlier ones are filled up to remaining balance.
7. Whatever is still in the pool is `unallocated_cents`.

`send_cents = required_cents + extra_cents`.

This module does not write debts, statements, or snowball rows. It does not look at payments.

## Testing Strategy

- Framework: `cargo test`
- Location: month arithmetic / clamp unit tests next to the code; full plans in `tests/plan.rs` using `SqliteDb::open_in_memory()`
- Coverage: September plan loads August card statements and September loan due dates; January → previous December; remaining below usual payment caps required and the unused amount is extra for the next-smallest; extra rolls after payoff; name tie-break when remaining balances match; required minima above the recorded amount keep full required lines and set shortfall; no recorded amount lists debts with no extra and `shortfall_cents = None`; card without a cycle row is `missing_statement` and can still receive extra; paid-off debts omitted; invalid `payment_month` is `VALIDATION_ERROR`; compute does not insert payments or change balances / snowball history
- **Not here:** TUI, `PaymentStore`

## Boundaries

- **Always:** Calculate only; money as cents; cap required at remaining balance; extra rolls to the next-smallest remaining (name tie-break); show **no statement yet** instead of inventing a card minimum; use previous calendar month for card cycles; leave payments, balances, and snowball history untouched
- **Ask first:** Persisting a calculated plan; using a snowball amount other than `current()`; putting loan due dates in the cycle month instead of the payment month
- **Never:** Read `payments`; bump the recorded snowball amount when minima do not fit; hide required minima to make the numbers fit; treat a missing card statement as a `$0` minimum without setting `missing_statement`; guess `statement_month` from `due_on`; use `f64` for money; draw the TUI

## Module contract (consumers)

`tui` depends on: `compute_plan` (amounts, due dates, missing-statement flag, shortfall, unallocated).

This module depends on:

- `debts`: `list`, remaining balance, loan `payment_cents` / `due_day`
- `snowball-size`: `current()`
- `statements`: `list_for_month` (cycle month = previous month)

`payments` does **not** depend on this module; this module does not depend on `payments`.

### Errors (one shape)

```text
VALIDATION_ERROR   — payment_month not YYYY-MM (reject `2026-13`, `2026-9`, `2026-09-01`)
```

Unknown debts or missing statements are not errors: missing card statements appear on the line.

## Success Criteria

- [x] `compute_plan("2026-09", …)` uses card rows with `statement_month = "2026-08"` and loan due dates in September 2026.
- [x] `compute_plan("2026-01", …)` uses card rows with `statement_month = "2025-12"`.
- [x] A loan of $200 usual / $50 remaining shows required $50; the unused $150 is in the extra pool for the next-smallest remaining debt.
- [x] Extra fills the smallest remaining debt up to remaining balance, then rolls to the next-smallest (name ascending when remaining matches).
- [x] When required minima sum to more than the recorded snowball amount, every line still shows its required minimum, extra is $0, and `shortfall_cents` is the difference. The recorded snowball amount is unchanged.
- [x] Never recorded a snowball amount: lines still exist; extra is $0; `shortfall_cents` is `None`.
- [x] A card with remaining balance and no row for the cycle month has `missing_statement = true`, `due_on = None`, required $0, and can still receive extra if it is the smallest remaining.
- [x] Paid-off debts do not appear.
- [x] Invalid `payment_month` is `VALIDATION_ERROR`.
- [x] Compute does not read or write payments, and does not change balances or snowball history.
- [x] `cargo test --test plan` passes with SQLite in memory.

## Open Questions

None.

## Decisions

Confirmed 2026-09-08.

- Payment month is the month you are funding (`2026-09`). Card `statement_month` is the previous calendar month. You type that cycle when recording the statement.
- Extra after required minima rolls to the next-smallest remaining debt; names break ties.
- Required never exceeds remaining balance; unused usual payment stays in the extra pool.
- Minima that exceed the recorded snowball amount: still list those minima; show shortfall; do not change the recorded amount.
- No recorded snowball amount: still list debts; no extra; no shortfall value.
- Missing card statement is visible (`missing_statement`); extra can still go to that card.
- Plan is calculated, not stored, and does not read payments.
- `SPEC-statements.md` unique key becomes `(debt_id, statement_month)`.
