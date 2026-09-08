# Tasks: statements

Verification commands (every task):

- Tests: `cargo test --test statements`
- Regression: `cargo test --test debts` and `cargo test --test snowball_size`
- Build: `cargo build`
- After checkpoints: `cargo clippy -- -D warnings` and `cargo fmt --check`

---

## Task 1: SqliteDb + refactor SqliteDebtStore

**Description:** Add `src/db.rs` with `SqliteDb` (one connection, `foreign_keys` on, migrates `debts` and `statements` tables). Refactor `SqliteDebtStore` to use it. Keep `open` / `open_in_memory` working. No statement logic yet beyond empty `statements` table.

**Acceptance criteria:**
- [x] `SqliteDb::open_in_memory()` creates `debts` and `statements` tables
- [x] All existing `cargo test --test debts` tests pass unchanged
- [x] `PRAGMA foreign_keys = ON` is set on open

**Verification:**
- [x] Tests pass: `cargo test --test debts`
- [x] Build succeeds: `cargo build`

**Dependencies:** None

**Files likely touched:**
- `src/db.rs`
- `src/lib.rs`
- `src/debts/store.rs`

**Estimated scope:** Medium

---

## Checkpoint: Shared DB

- [x] `cargo test --test debts` passes
- [x] `cargo test --test snowball_size` still passes (unchanged module)

---

## Task 2: Scaffold statements; record and get; reject bad debt/input

**Description:** Add `src/statements/` with types, trait, validation, and `SqliteStatementStore::record` / `get`. Record on a credit card persists. Unknown `debt_id` → `NOT_FOUND`. Loan → `VALIDATION_ERROR`. Invalid `due_on` or negative minimum → `VALIDATION_ERROR`.

**Acceptance criteria:**
- [x] Recording on a credit card persists minimum and due date; `get` returns it
- [x] Unknown `debt_id` is `NOT_FOUND`
- [x] Loan debt is `VALIDATION_ERROR`
- [x] Invalid `due_on` or `minimum_cents < 0` is `VALIDATION_ERROR`

**Verification:**
- [x] Tests pass: `cargo test --test statements`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 1

**Files likely touched:**
- `src/statements/mod.rs`
- `src/statements/validate.rs`
- `src/statements/store.rs`
- `tests/statements.rs`
- `src/lib.rs`

**Estimated scope:** Medium

---

## Task 3: Upsert; list_for_debt; current_for_debt

**Description:** Same `(debt_id, due_on)` updates `minimum_cents` without a second row or new id. Two different due dates on one card both exist. `current_for_debt` returns the row with latest `due_on`.

**Acceptance criteria:**
- [x] Same card + due date twice updates minimum; id unchanged; one row
- [x] Two due dates on one card; `current_for_debt` is the later `due_on`
- [x] `list_for_debt` returns statements `due_on` ascending

**Verification:**
- [x] Tests pass: `cargo test --test statements`

**Dependencies:** Task 2

**Files likely touched:**
- `src/statements/store.rs`
- `tests/statements.rs`

**Estimated scope:** Small

---

## Task 4: upcoming; no side effects on debt balance

**Description:** `upcoming(as_of)` returns `due_on >= as_of` ordered by `due_on`, then `id`. Excludes earlier dates. Recording a statement does not change `Debt.balance_cents`.

**Acceptance criteria:**
- [x] `upcoming("2026-09-08")` includes 2026-09-08 and later, excludes 2026-09-07
- [x] Recording a statement leaves `Debt.balance_cents` unchanged
- [x] Public API has no snowball-size or payment fields

**Verification:**
- [x] Tests pass: `cargo test --test statements`
- [x] Regression: `cargo test --test debts`
- [x] Clippy/fmt: `cargo clippy -- -D warnings` and `cargo fmt --check`

**Dependencies:** Task 3

**Files likely touched:**
- `src/statements/store.rs`
- `tests/statements.rs`

**Estimated scope:** Small

---

## Checkpoint: statements complete

- [x] All success criteria in `SPEC-statements.md` are met
- [x] `cargo test --test statements` passes
- [x] `cargo test --test debts` and `cargo test --test snowball_size` pass
- [x] `cargo clippy -- -D warnings` and `cargo fmt --check` pass
- [x] Ready for review; next map module to spec is `payments`
