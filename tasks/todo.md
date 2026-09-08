# Tasks: tui

Verification commands (every task):

- Tests: `cargo test --test tui` (after Task 3 exists); `cargo test --test snowball_size` for Task 1
- Regression: `cargo test --test debts`, `cargo test --test statements`, `cargo test --test snowball_size`, `cargo test --test payments`, `cargo test --test plan`
- Build: `cargo build`
- After checkpoints: `cargo clippy -- -D warnings` and `cargo fmt --check`

---

## Task 1: snowball_sizes on SqliteDb

**Description:** Add `CREATE TABLE IF NOT EXISTS snowball_sizes` to `SqliteDb::migrate` (same columns and checks as the current size-store schema). Refactor `SqliteSnowballSizeStore` to `SqliteSnowballSizeStore<'db>` on `&SqliteDb`, matching payments. Remove the store’s private connection and `migrate`. Tests open `SqliteDb::open_in_memory()` and `SqliteSnowballSizeStore::new(&db)`. `record` / `current` / `list` behavior is unchanged.

**Acceptance criteria:**
- [x] `SqliteDb::open_in_memory()` creates `snowball_sizes`
- [x] Existing snowball-size tests pass against the shared db (empty / record / current / list / reject `<= 0`)
- [x] Store no longer opens its own `Connection`

**Verification:**
- [x] Tests pass: `cargo test --test snowball_size`
- [x] Build succeeds: `cargo build`

**Dependencies:** None

**Files likely touched:**
- `src/db.rs`
- `src/snowball_size/store.rs`
- `src/snowball_size/mod.rs`
- `tests/snowball_size.rs`

**Estimated scope:** Medium

---

## Task 2: plan and payments tests share SqliteDb

**Description:** Point `tests/plan.rs` and `tests/payments.rs` size helpers at `SqliteSnowballSizeStore::new(&db)` on the same `SqliteDb` as debts/statements/payments. Recording a size then `compute_plan` must see that amount. Drop the second in-memory size database.

**Acceptance criteria:**
- [x] Plan tests that record a snowball amount still allocate extra / shortfall
- [x] Payments tests that assert snowball history is untouched still pass
- [x] No test opens `SqliteSnowballSizeStore::open_in_memory` (that constructor is gone)

**Verification:**
- [x] Tests pass: `cargo test --test plan`, `cargo test --test payments`, `cargo test --test snowball_size`
- [x] Regression: `cargo test --test debts`, `cargo test --test statements`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 1

**Files likely touched:**
- `tests/plan.rs`
- `tests/payments.rs`

**Estimated scope:** Small

---

## Checkpoint: Shared DB

- [x] `SqliteDb::open_in_memory()` creates `debts`, `statements`, `payments`, and `snowball_sizes`
- [x] `cargo test --test snowball_size`, `--test plan`, `--test payments` pass
- [x] `cargo clippy -- -D warnings` and `cargo fmt --check` pass

---

## Task 3: TUI scaffold, pesos, keys, empty home

**Description:** Add `src/tui/` (`mod.rs`, `money.rs`, `keys.rs`). `App` holds `Mode`, injected `today`, `payment_month`, status line. `format_pesos` / `parse_pesos` as specified. `handle_key` is a pure map for `Home` at least (`q` quit, Esc no-op, unknown ignored). Draw empty home from `mod.rs` on `TestBackend`: payment month from injected today, **no size recorded**, hint to press `d`. Add ratatui + crossterm. `pub mod tui` from `lib.rs`. No binary yet. No store writes. `home.rs` lands in Task 4.

**Acceptance criteria:**
- [x] `format_pesos(123_456) == "₱1,234.56"`; `parse_pesos("1,234.56")` is `123_456`; more than two decimals is invalid
- [x] Injected today `2026-09-08` → home shows `2026-09`
- [x] Empty register: no plan rows; header says no size recorded; no shortfall label
- [x] `q` sets quit; Esc on home does not

**Verification:**
- [x] Tests pass: `cargo test --test tui`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 2

**Files likely touched:**
- `Cargo.toml`
- `src/lib.rs`
- `src/tui/mod.rs`
- `src/tui/money.rs`
- `src/tui/keys.rs`

**Estimated scope:** Medium

---

## Task 4: Home draws compute_plan

**Description:** On each draw (and after month change), call `compute_plan` with the app’s payment month and the four stores on the same `SqliteDb`. Header: peso size, shortfall only when `Some(n)` and `n > 0`, unallocated only when `> 0`. Body: one list — name, remaining, required, extra, send, due date. `?` if `missing_statement`; `!` if `due_on` is `Some` and `< today`. Add public `next_month` beside `previous_month` and re-export both from `plan`. `h` / `l` step the month and refresh. Tests seed data with frozen today `2026-09-08`.

**Acceptance criteria:**
- [x] Seeded September home shows peso size, send amounts, and due dates from `compute_plan`
- [x] No size recorded: header says so; lines still render; no shortfall label
- [x] Shortfall `Some(n)` `n > 0` appears as pesos; recorded size unchanged
- [x] Missing statement shows `?` and no invented due date; overdue `due_on` shows `!`
- [x] `l` from `2026-09` goes to `2026-10`; `h` from `2026-01` goes to `2025-12`
- [x] September card lines use August `statement_month` (visible via due date / missing mark — do not re-test allocation math)

**Verification:**
- [x] Tests pass: `cargo test --test tui`
- [x] Build succeeds: `cargo build`
- [x] Regression: `cargo test --test plan`

**Dependencies:** Task 3

**Files likely touched:**
- `src/tui/home.rs`
- `src/tui/mod.rs`
- `src/plan/validate.rs`
- `src/plan/mod.rs`
- `tests/tui.rs`

**Estimated scope:** Medium

---

## Task 5: Selection, no-op p/s, help, quit

**Description:** `j` / `k` move the home selection (clamp at ends). No lines: `p` / `s` status-only, stay on home. `?` opens help (key map text); Esc or `?` closes to home. `q` from help quits. Wire `home.rs` list state. Still no overlays that write.

**Acceptance criteria:**
- [x] `j` / `k` move selection among seeded lines; does not wrap past the ends
- [x] `p` / `s` with no selection: status message; mode stays Home
- [x] `?` then Esc returns to home; buffer contained key hints
- [x] `q` from home and from help ends the session (`should_quit`)

**Verification:**
- [x] Tests pass: `cargo test --test tui`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 4

**Files likely touched:**
- `src/tui/keys.rs`
- `src/tui/home.rs`
- `src/tui/mod.rs`
- `src/tui/overlays.rs`
- `tests/tui.rs`

**Estimated scope:** Medium

---

## Checkpoint: Home

- [x] Home matches `SPEC-tui.md` (one list, pesos, marks, month step, help)
- [x] `cargo test --test tui` passes with `TestBackend` (no TTY)
- [x] `cargo clippy -- -D warnings` and `cargo fmt --check` pass

---

## Task 6: Payments overlay

**Description:** `p` on a selected line opens `Payments { debt_id }` with `list_for_debt`. Rows: peso amount, `paid_on`, overpayment flag when `is_overpayment` (do not block). `n` record form (amount, `paid_on` default today; debt is the line). Enter / `e` edit form (amount, `paid_on`, debt picker with `h`/`l` when that field is focused). Tab / Shift-Tab fields; Enter on last field submits; Esc discards to the list, then to home. Invalid parse or store error: status line, form stays open. After a successful write, recompute the plan; keep selection by `debt_id` if the line remains.

**Acceptance criteria:**
- [x] `p` lists that debt’s payments
- [x] Recording a payment persists, reduces remaining, and home send/remaining refresh
- [x] Edit amount persists; moving `debt_id` applies on the other debt
- [x] Overpayment row is flagged; save is not blocked
- [x] Esc closes form then overlay; `q` on the list quits

**Verification:**
- [x] Tests pass: `cargo test --test tui`
- [x] Build succeeds: `cargo build`
- [x] Regression: `cargo test --test payments`

**Dependencies:** Task 5

**Files likely touched:**
- `src/tui/overlays.rs`
- `src/tui/keys.rs`
- `src/tui/mod.rs`
- `tests/tui.rs`

**Estimated scope:** Medium

---

## Task 7: Statements overlay

**Description:** `s` on a selected card opens `Statements { debt_id }` with `list_for_debt`. Rows with `due_on < today` marked `!`. `n` or Enter on a row opens the record form (`statement_month` defaulting to `previous_month(payment_month)`, minimum, `due_on`). Same card + cycle upserts. `s` on a loan: status message, stay on home. Do not call `upcoming`.

**Acceptance criteria:**
- [x] `s` on a card lists statements; recording persists `statement_month` and refreshes home (`?` clears when that cycle is the plan’s previous month)
- [x] Same cycle again updates min/due; still one row
- [x] `s` on a loan does not change mode
- [x] Overdue statement rows in the overlay show `!`

**Verification:**
- [x] Tests pass: `cargo test --test tui`
- [x] Build succeeds: `cargo build`
- [x] Regression: `cargo test --test statements`

**Dependencies:** Task 6

**Files likely touched:**
- `src/tui/overlays.rs`
- `src/tui/keys.rs`
- `src/tui/mod.rs`
- `tests/tui.rs`

**Estimated scope:** Medium

---

## Task 8: Size overlay

**Description:** `n` on home opens `Size`. `list` displayed newest first. `n` records a new amount (peso parse → `RecordSize`). No edit/delete. Esc to home. Header uses `current` after record. Previous rows remain.

**Acceptance criteria:**
- [x] `n` on home shows history newest first
- [x] Recording ₱500 after ₱400 updates the header to ₱500; overlay still lists both
- [x] `amount <= 0` / bad parse does not insert; status line; form stays open
- [x] Recording a size is the only way the header amount changes

**Verification:**
- [x] Tests pass: `cargo test --test tui`
- [x] Build succeeds: `cargo build`
- [x] Regression: `cargo test --test snowball_size`

**Dependencies:** Task 7

**Files likely touched:**
- `src/tui/overlays.rs`
- `src/tui/keys.rs`
- `src/tui/mod.rs`
- `tests/tui.rs`

**Estimated scope:** Small

---

## Task 9: Debts list

**Description:** `d` from home opens the debts list (`DebtStore::list`, paid-off included). `n` create: kind toggle, then name, balance, and loan-only payment + due day. Enter / `e` edit name and loan floor/due day (kind cannot change). `b` is `set_balance`. Esc to home. Empty home after first create should show the new line once remaining `> 0`. Paid-off (₱0) appears on this list, not on home.

**Acceptance criteria:**
- [x] Create a loan and a card; Esc home; both appear on the plan if remaining `> 0`
- [x] Edit name / loan payment / due day persists
- [x] `set_balance` restates remaining; home recomputes
- [x] A ₱0 debt is on the debts list and omitted from home
- [x] Kind cannot be changed in the edit form

**Verification:**
- [x] Tests pass: `cargo test --test tui`
- [x] Build succeeds: `cargo build`
- [x] Regression: `cargo test --test debts`

**Dependencies:** Task 8

**Files likely touched:**
- `src/tui/debts.rs`
- `src/tui/keys.rs`
- `src/tui/mod.rs`
- `tests/tui.rs`

**Estimated scope:** Medium

---

## Checkpoint: Overlays

- [x] Payments, statements, size, and debts write paths persist and refresh home
- [x] `cargo test --test tui` passes
- [x] `cargo test --test debts`, `--test statements`, `--test payments`, `--test snowball_size`, `--test plan` pass
- [x] `cargo clippy -- -D warnings` and `cargo fmt --check` pass

---

## Task 10: Binary `--db` and `run`

**Description:** Add `src/main.rs`: parse optional `--db PATH`, else default XDG/local-share path, `create_dir_all` on the parent, `SqliteDb::open`, `tui::run` (crossterm backend, local today). Unit-test path resolution with env vars (no TTY). `cargo run -- --db` is enough to launch; do not seed demo data.

**Acceptance criteria:**
- [x] `--db /tmp/snowball-test.db` opens that file (parent created if needed)
- [x] No `--db`: uses `$XDG_DATA_HOME/snowball/snowball.db` when set, else `~/.local/share/snowball/snowball.db`
- [x] `tui::run` exists and is not called from `tests/tui.rs`
- [x] `cargo build` produces a `snowball` binary

**Verification:**
- [x] Tests pass: `cargo test --test tui` (path helpers; existing screen tests)
- [x] Build succeeds: `cargo build`
- [x] Manual check: `cargo run -- --db` on a temp file draws home (human at review)

**Dependencies:** Task 9

**Files likely touched:**
- `src/main.rs`
- `src/tui/mod.rs`
- `Cargo.toml`
- `tests/tui.rs`

**Estimated scope:** Small

---

## Checkpoint: tui complete

- [x] All success criteria in `SPEC-tui.md` are met
- [x] `cargo test --test tui` passes with `TestBackend` and in-memory SQLite
- [x] `cargo test` (full suite) passes
- [x] `cargo clippy -- -D warnings` and `cargo fmt --check` pass
- [x] Ready for review
