# Implementation Plan: plan

## Overview

Amend statements so each card bill has a typed cycle month (`statement_month` `YYYY-MM`, unique per card). Then add `compute_plan`: a pure calculation (no new table) that turns a payment month into amounts to send, due dates, missing-statement flags, extra to the smallest remaining debt (rolling), and shortfall. When this plan is done, `tui` can call `compute_plan`; this module still does not read payments.

Prior module archive: `tasks/plan-payments.md`, `tasks/todo-payments.md`.

## Architecture Decisions

- **Statements first.** Plan cannot load August bills without `statement_month`. Change `CREATE TABLE statements` for new in-memory DBs: `statement_month TEXT NOT NULL`, `UNIQUE(debt_id, statement_month)`, drop `UNIQUE(debt_id, due_on)`. Tests always open a fresh `:memory:` DB; no on-disk backfill in this plan.
- **Do not infer the cycle.** Callers pass `statement_month`. `due_on` may fall in another calendar month. Upsert on `(debt_id, statement_month)` updates `minimum_cents` and `due_on`, keeps the same id.
- **`list_for_month` on `StatementStore`.** Plan joins owed cards to that list. `current_for_debt` becomes latest `statement_month` (TUI). `upcoming` stays by `due_on`.
- **`compute_plan` is a free function** on three store traits. No `PlanStore`. No SQLite table. Tests may use `SqliteDb` for debts/statements and a separate `SqliteSnowballSizeStore` (same as payments tests). Do not wire snowball-size onto `SqliteDb` in this plan.
- **Calendar helpers live in `src/plan/validate.rs`.** Payment month `YYYY-MM`; previous month (January → previous December); loan due date = payment month + `due_day` clamped to last day of that month. Copy month-length rules next to this code; do not import `statements::validate`. Statements get their own `YYYY-MM` check for `statement_month`.
- **Do not import `payments`.** Side-effect tests may open `SqlitePaymentStore` only in `tests/plan.rs` to assert row counts stay `0`.
- **YYYY-MM must be zero-padded.** Reject `2026-9` and `2026-09-01`.

## Dependency graph (this module)

```
statements: statement_month column + unique key
    │
    ├── record / get / list_for_debt / current_for_debt
    │
    └── list_for_month
            │
            └── plan validate (payment month, previous month, clamp due day)
                    │
                    └── compute_plan: required lines (no recorded snowball)
                            │
                            ├── extra rolls / cap remaining / name tie-break
                            │
                            └── shortfall when minima exceed recorded amount;
                                no writes to payments, balances, or snowball history
```

## What can be parallel vs sequential

All sequential. `list_for_month` must exist before `compute_plan` can load card bills.

## Task List

Index only. Full acceptance criteria live in `tasks/todo.md`.

### Foundation: statements cycle month

- [x] Task 1: `statement_month` on schema, types, validation, record/get; unique `(debt_id, statement_month)`
- [x] Task 2: `list_for_month`; `current_for_debt` latest cycle month; `list_for_debt` ordered by cycle month

### Checkpoint: statements amendment

- [x] `cargo test --test statements` passes
- [x] Payments / debts / snowball-size tests still pass

### Plan slices

- [x] Task 3: Scaffold `plan`; reject bad payment month; empty register → empty lines
- [x] Task 4: Required amounts, due dates, missing statement, omit paid-off; no recorded snowball
- [x] Task 5: Extra rolls, remaining cap, name tie-break, shortfall, no side effects

### Checkpoint: plan complete

- [x] All `SPEC-plan.md` success criteria met
- [x] `SPEC-statements.md` amendment criteria met
- [x] `cargo test --test plan`, statements, payments, debts, snowball_size, clippy, fmt pass
- [x] Ready for `tui` spec

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Old `UNIQUE(debt_id, due_on)` left on the table | High | Replace the `CREATE TABLE` in `SqliteDb::migrate`. Fresh in-memory DBs only. Assert unique index in a db or statements test. |
| `RecordStatement` call sites fail to compile | Med | Task 1 updates `tests/statements.rs` and `tests/payments.rs` in the same slice. |
| Extra allocated before required lines are stable | Med | Task 4 is the no-snowball path only. Task 5 adds extra and shortfall. |
| Loan due-day clamp wrong in short months | Med | Unit tests: due day 31 in September → `2026-09-30`; February non-leap. |
| Accidental payments coupling | Med | `src/plan` does not `use` payments. Integration test: compute leaves payment rows and snowball `current` unchanged. |

## Open Questions

None. Spec decisions confirmed 2026-09-08.

## Task list target

`tasks/todo.md` (markdown checklist). No external tracker.
