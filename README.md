# Snowball

Personal debt-snowball tracker. Open it to see this payment month's committed size, the split of what to send, and due dates — so money can stay in savings until you transfer. Size changes only when you record a new one; when required minima do not fit, the header shows a shortfall instead of bumping the size automatically.

Local SQLite. Philippine pesos. Vim-like keys. No accounts, bank sync, or notifications.

## How the plan works

The home screen shows **this payment month** (`YYYY-MM`). Each row is a debt you still owe, with how much to send and when it is due.

- **Loans** — usual monthly payment due in the payment month.
- **Cards** — previous cycle's statement (September home uses August statements).
- **Extra** — after required minima are covered, leftover goes to the smallest remaining debt and rolls to the next when that debt is filled.
- **Marks** — `?` means no card statement for that cycle yet; `!` means the due date is before today.

## Quick start

**Requirements:** [Rust](https://rustup.rs/) (edition 2024).

```bash
cargo run -- --db /tmp/snowball.db
```

Use any path for the database file. Parent directories are created on first open. The app starts empty — no demo data is seeded.

**Default database path** (when `--db` is omitted):

- `$XDG_DATA_HOME/snowball/snowball.db` if `XDG_DATA_HOME` is set
- otherwise `~/.local/share/snowball/snowball.db`

## Typical first session

1. Press `d` to open the debts list, then `n` to add a debt.
   - **Loan:** kind, name, balance, monthly payment, due day (1–31).
   - **Card:** kind, name, balance.
2. Press `Esc` to return home, then `n` to record your snowball size.
3. For a card: select its row on home → `s` → `n` to record cycle month (`YYYY-MM`), minimum, and due date.
4. After a transfer: select the row → `p` → `n` to record amount and paid-on date.
5. When a statement posts and the balance changes: `d` → select the card → `b` to restate balance.

## Screens

### Home

The main screen. Header shows payment month, committed snowball size (or **no size recorded**), **shortfall** when required minima exceed the size, and **unallocated** when extra could not land on any debt.

The body lists what to send this month: name, remaining balance, required, extra, send amount, and due date.

If you have no debts yet, the header hints: press `d` to add one.

### Payments (`p`)

Opens for the selected debt. Record new payments (`n`) or edit existing ones (`e` or Enter). Overpayments save and are flagged — they are not blocked. There is no delete.

### Statements (`s`)

Cards only. Record cycle month, minimum due, and due date (`n` or Enter on a row). Recording the same card and cycle month again corrects the row. On a loan, `s` shows a status message instead of opening.

### Snowball size (`n` on home)

History of recorded sizes, newest first. `n` appends a new amount (raise or shrink). No edit or delete.

### Debts (`d`)

All debts, including paid-off ones at ₱0 (they do not appear on home). `n` creates; `e` edits name and loan fields; `b` sets balance (for card restatements and corrections). Debt kind cannot change after create.

### Help (`?`)

Key map. Press `?` or `Esc` to close.

## Keys

| Key | Home | Overlay / debts list | Form |
|-----|------|--------------------|------|
| `j` / `k` | Move selection | Move rows | — |
| `h` / `l` | Previous / next month | — | Debt picker: previous / next debt |
| `p` | Payments for selection | — | — |
| `s` | Statements (card only) | — | — |
| `n` | Size overlay | New record / new debt | — |
| `d` | Debts list | — | — |
| `e` / Enter | — | Edit selected row | Submit (Enter on last field) |
| `b` | — | Set balance (debts list) | — |
| `?` | Help | Help | — |
| `Esc` | No-op | Close overlay / back to home | Discard form |
| `q` | Quit | Quit | Literal character |

No mouse.

**Forms:** Tab / Shift-Tab move between fields. Enter on the last field submits. Esc discards and returns to the overlay or list. Amounts display as `₱1,234.56`; you can type commas and an optional leading `₱`. Validation errors appear on the status line; the form stays open.

## Development

| Command | Description |
|---------|-------------|
| `cargo run -- --db <path>` | Run the TUI |
| `cargo test` | Run all tests |
| `cargo test --test tui` | TUI tests only |
| `cargo clippy -- -D warnings` | Lint |
| `cargo fmt --check` | Format check |
| `cargo build` | Build without running |

Module specs and architecture: [CAPABILITY-MAP.md](CAPABILITY-MAP.md), [SPEC-tui.md](SPEC-tui.md), and sibling `SPEC-*.md` files.

## Out of scope

Mouse input, deleting debts or payments, bank sync, notifications, mobile, web, and auto-raising the snowball when minima rise.
