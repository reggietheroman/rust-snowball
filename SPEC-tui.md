# Spec: tui

## Objective

The `tui` module is the Snowball you sit in. You open it to see **this payment month in pesos**: the committed snowball size, a shortfall if required minima do not fit, and one list of what to send with a due date on each row.

**User:** you, at a computer, before you transfer money out of savings — and again after a statement posts or a transfer goes out.

**This module succeeds when** the home screen is that month, vim-like keys move and open overlays, you can record and fix payments from a selected line, record a card statement the same way, record a new size, and add or edit debts on a secondary list — without a second dues pane, a vim emulator, or a browser.

**Out of scope for this module:** mobile, bank sync, web, mouse, real vim (`:` / counts / buffers), notifications, deleting payments or debts, interest, reimplementing `compute_plan`.

## Tech Stack

Same crate as the library, plus a binary:

- Rust (edition 2024), package `snowball`
- Binary name `snowball` (thin `main` that opens SQLite and calls `tui::run`)
- **ratatui** + **crossterm**, synchronous (no async runtime)
- One on-disk SQLite file via `SqliteDb` (debts, statements, payments, **and** `snowball_sizes`)
- Money as integer **centavos** in the library; TUI shows and accepts **Philippine pesos**
- Domain modules stay free of ratatui. TUI code lives at `src/tui/` so `tests/tui.rs` can drive it.

## Commands

```
cargo run -- --db /tmp/snowball.db
cargo test
cargo test --test tui
cargo clippy -- -D warnings
cargo fmt --check
cargo build
```

No real TTY is required for tests. Do not add a `dev` watch command.

## Project Structure

```
SPEC-tui.md
src/main.rs              → Binary: parse `--db`, open SqliteDb, `tui::run`
src/tui/mod.rs           → App state, event loop, `run`
src/tui/home.rs          → Home header + plan list
src/tui/keys.rs          → Key map (pure: key + mode → command)
src/tui/money.rs         → Peso format / parse
src/tui/overlays.rs      → Payments, statements, size, help, forms
src/tui/debts.rs         → Debts list + create/edit/set_balance forms
src/db.rs                → Amend: migrate `snowball_sizes` here
src/snowball_size/       → Amend: store on `&SqliteDb` (same pattern as payments)
tests/tui.rs             → TestBackend + in-memory SqliteDb
```

Home numbers come only from `compute_plan`. Do not copy allocation rules into the TUI.

## Code Style

Match the library: `snake_case` functions, `PascalCase` types, `Result<T, Error>` at the store boundary. UI state is an explicit mode enum — not a boolean soup.

```rust
pub enum Mode {
    Home,
    Help,
    Debts,
    Payments { debt_id: DebtId },
    Statements { debt_id: DebtId },
    Size,
    Form(Form),
}

// Home list (j/k). Overlays are modes on top of the same app, not a second process.
```

Peso display is a pure function. Tests assert the string, not a screenshot.

```rust
assert_eq!(format_pesos(123_456), "₱1,234.56");
assert_eq!(parse_pesos("1,234.56")?, 123_456);
```

### Screens

**Home (the app).** Header: payment month (`YYYY-MM`), committed size or **no size recorded**, **shortfall** only when `shortfall_cents` is `Some(n)` and `n > 0`, **unallocated** only when `unallocated_cents > 0`. Body: one list = `Plan.lines` (already due-date order). Columns: name, remaining, required, extra, send, due date. Marks (text, not color-only): `?` when `missing_statement`, `!` when `due_on` is `Some` and `<` local today. Extra `> 0` is visible in the extra column; no second mark is required.

Default payment month is the local calendar month at launch. `h` / `l` step one calendar month (December `l` → next January). Each step calls `compute_plan` again.

Empty debts: empty list, status hint to press `d`. No selection: `p` / `s` are no-ops with a status message. After a write, recompute the plan. Keep the selected `debt_id` if that line still exists; otherwise select the first line.

Do **not** call `StatementStore::upcoming` on home. Due dates on the plan lines are the upcoming list.

**Payments overlay (`p` on a selected line).** `list_for_debt` for that debt. Rows show amount, `paid_on`, and an **overpayment** flag when `is_overpayment` (do not block). `n` opens a record form (amount, `paid_on` defaulting to local today; debt is the line). Enter / `e` on a row opens an edit form (amount, `paid_on`, debt — you can move the payment to another debt). Esc closes the form to the overlay list, or the overlay to home. There is no delete.

**Statements overlay (`s` on a selected card).** `list_for_debt`. Rows with `due_on <` local today are marked overdue (`!`). `n` or Enter on a row opens the record form (`statement_month`, minimum, `due_on`). Default `statement_month` is the **previous** calendar month relative to the home payment month (September home → `2026-08`). Same `(card, statement_month)` upserts — that is the correction. `s` on a loan: do not open; status message that statements are for cards.

**Size overlay (`n` on home).** `list` newest first (reverse of store order). `n` records a new amount (raise or shrink). No edit, no delete. Esc to home.

**Debts list (`d` from home).** `DebtStore::list` (paid-off included, ₱0 remaining). `n` create (kind loan or card, then the fields that kind allows). Enter / `e` edit name and, for loans, usual payment and due day. Kind cannot change. `b` is `set_balance` (card restatement and balance corrections). Esc to home.

**Help (`?`).** Key map. Esc or `?` closes.

**Forms.** Typing only here. Tab / Shift-Tab move fields. Enter on the last field submits. Esc discards and returns to the overlay or debts list. `q` is a character, not quit. Store `VALIDATION_ERROR` / `NOT_FOUND` messages go to the status line; the form stays open.

### Key map

| Key | Home | Overlay / debts list | Form |
|---|---|---|---|
| `j` / `k` | Move plan selection | Move rows | — |
| `h` / `l` | Previous / next payment month | — | When the field is a debt picker: previous / next debt |
| `p` | Payments overlay for selection | — | — |
| `s` | Statements overlay if card | — | — |
| `n` | Size overlay | New record / new debt | — |
| `d` | Debts list | — | — |
| `e` / Enter | — | Edit selected row | Submit (Enter, last field) |
| `b` | — | `set_balance` (debts list only) | — |
| `?` | Help | Help | — |
| Esc | No-op | Close overlay / back to home | Discard form |
| `q` | Quit | Quit | Literal character |

No mouse. Unknown keys are ignored.

### Pesos

- Display: `₱` + thousands separators + two decimal places (`₱1,234.56`, `₱0.00`).
- Input: digits, optional commas, optional leading `₱`, optional two decimal places. More than two decimal places is invalid. Parsed to centavos for the store.
- The library still stores `i64` cents. The TUI never shows a raw cent count on a money field.

### Data file

```
snowball --db PATH
```

`--db` is optional. Default: `$XDG_DATA_HOME/snowball/snowball.db` if `XDG_DATA_HOME` is set, otherwise `~/.local/share/snowball/snowball.db`. Create parent directories on first open. Local calendar date/month for “today” and the default payment month (no timezone in the library).

## Testing Strategy

- Framework: `cargo test`
- Location: peso format/parse and key-map unit tests next to the code; screen and overlay behavior in `tests/tui.rs` using ratatui `TestBackend` and `SqliteDb::open_in_memory()`
- Coverage:
  - Home buffer shows payment month, peso size, plan names, send amounts, due dates
  - No size recorded: header says so; lines still render; no shortfall label
  - Shortfall `Some(n)` `n > 0` appears; `Some(0)` does not
  - Missing statement shows `?` and no invented due date; overdue `due_on` shows `!`
  - `h` / `l` change the payment month and the lines (September cards are August cycles — that is `compute_plan`, assert it is visible)
  - `p` → payments list; record; Esc back to home; edit overpayment row still saves and shows the flag
  - `s` on a card records a statement; `s` on a loan does not open
  - `n` records a new size; home header updates; previous size remains in the overlay list
  - `d` create loan and card; Esc home; paid-off debt is on the debts list, not on home
  - `q` from home ends `run`; Esc on home does not
  - `parse_pesos` / `format_pesos` round-trip; invalid input does not call the store
- **Not here:** re-testing allocation math (that is `tests/plan.rs`); a real terminal; mouse

## Boundaries

- **Always:** Draw home from `compute_plan` only; show pesos, store centavos; vim-like keys as in the table; overlays for payments / statements / size; debts list for the debt itself; mark overdue and missing statement in text; put `snowball_sizes` on the same `SqliteDb` as the other tables; run tests on `TestBackend`
- **Ask first:** A config file; colors beyond the terminal default; a second binary; deleting debts or payments from the UI
- **Never:** Call `upcoming` as a second home pane; bump snowball size except via `SnowballSizeStore::record`; hide required minima to make the header pretty; reimplement extra/roll/shortfall in the TUI; use `f64` for money; require a TTY for tests; handle mouse; emulate vim (`:`, counts, buffers, insert mode on home); talk to a bank; send notifications

## Module contract (consumers)

This module **is** the consumer. It depends on:

| Module | Calls |
|---|---|
| `plan` | `compute_plan` |
| `payments` | `record`, `update`, `get`, `list_for_debt`; `Payment::is_overpayment` (flag only) |
| `statements` | `record`, `list_for_debt` |
| `snowball-size` | `current`, `list`, `record` |
| `debts` | full `DebtStore` (create, update, `set_balance`, `list`, `get`) |

`SqliteDb::migrate` must create `snowball_sizes` as well as `debts`, `statements`, and `payments`, so one `--db` file is enough. Refactor `SqliteSnowballSizeStore` onto `&SqliteDb` without changing `SnowballSizeStore` behavior. Existing in-memory tests keep passing.

### Errors (one shape)

Library errors are unchanged (`NOT_FOUND`, `VALIDATION_ERROR`). The TUI does not invent codes. It shows `Error.message` on the status line.

Unknown keys, `p`/`s` with no selection, and `s` on a loan are not library errors: status line only.

## Success Criteria

- [x] `cargo run -- --db <path>` opens home for the local calendar month, in pesos, with size (or none) and `compute_plan` lines.
- [x] `h` / `l` move the payment month; September shows August card cycles and September loan due dates.
- [x] `j` / `k` move the selection; `p` lists that debt’s payments; a new payment and an edit both persist and refresh remaining balances on home.
- [x] Overpayment rows are flagged; saving is not blocked.
- [x] `s` on a card records (or corrects) a statement with a typed `statement_month`; `s` on a loan does not.
- [x] `n` on home records a new size; the header follows `current`; history in the overlay still has the old rows.
- [x] `d` can create a loan and a card, edit name / loan floor, and `set_balance`; Esc returns to home.
- [x] Missing statement is `?`; overdue plan due date is `!`; no second dues list.
- [x] Shortfall appears only when required minima exceed the recorded size; the recorded size does not change.
- [x] Esc closes a form, then an overlay; `q` on home or a list quits; Esc on home does not quit.
- [x] `cargo test --test tui` passes with `TestBackend` and in-memory SQLite (no TTY).
- [x] `cargo test` (library tests) still pass after `snowball_sizes` moves onto `SqliteDb`.

## Open Questions

None.

## Decisions

Confirmed 2026-09-08.

- One home screen: header (size, shortfall, unallocated) + this month’s plan lines. No upcoming-dues pane.
- Vim-like keys, not a vim emulator. Typing only inside forms. `q` quits from lists; Esc backs out.
- Amounts are Philippine pesos on screen (`₱1,234.56`); stores keep integer centavos.
- `p` is that debt’s payment ledger (record + edit). Debts list is the debt itself, not a second payments app.
- `s` is that card’s statements. Overdue that is not this plan’s `due_on` shows there.
- `n` on home is snowball-size history + record. Size still changes only by appending a row.
- Default DB path is XDG data home; `--db` overrides.
- `snowball_sizes` joins `SqliteDb` so the binary opens one file.
