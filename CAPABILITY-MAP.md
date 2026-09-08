# Capability Map: Snowball

Approved 2026-09-07.

Personal debt-snowball tracker. Open it to see the committed snowball size, this month's split, and upcoming due dates. Size never changes unless you record a new one.

| Module id | Responsibility | Depends on |
|---|---|---|
| debts | Loans and cards: name, balance, type, loan payment / due day | — |
| snowball-size | Recorded size history. Size changes only when you record them | — |
| statements | Card statements: minimum due and due date for a month | debts |
| payments | Payments you actually made, against a debt | debts |
| plan | This month's split from current size, loan floors, and card mins; shortfall if required payments exceed the size; leftover goes to the smallest remaining debt | debts, snowball-size, statements |
| tui | Terminal UI, vim keys, upcoming due dates, recording sizes / statements / payments | plan, payments, statements, snowball-size, debts |

Build order: `debts`, `snowball-size` → `statements`, `payments` → `plan` → `tui`

## Specs

| Module id | Spec |
|---|---|
| debts | [SPEC-debts.md](SPEC-debts.md) |
| snowball-size | [SPEC-snowball-size.md](SPEC-snowball-size.md) |
| statements | [SPEC-statements.md](SPEC-statements.md) |
| payments | [SPEC-payments.md](SPEC-payments.md) |
| plan | *(not written)* |
| tui | *(not written)* |

## Confirmed intent (summary)

- **Outcome:** Committed snowball size, this month's split (fixed loans, current card mins, leftover on the smallest remaining debt), upcoming due dates so money can stay in a HYSA until the last comfortable moment.
- **User:** One person. Local data. No accounts, sync, or bank import.
- **Success:** Size changes only via a recorded ledger entry (up or down). Required payments over the size show a shortfall, not an automatic bump. Payments and card statements are logged.
- **Constraint:** Computer TUI with vim key bindings. No notifications.
- **Out of scope:** Mobile, bank sync, push reminders, auto-changing snowball size, general budget/HYSA tracking, pay-early coaching.
