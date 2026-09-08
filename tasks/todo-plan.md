# Tasks: plan

Verification commands (every task):

- Tests: `cargo test --test plan` (after Task 3 exists); `cargo test --test statements` for Tasks 1–2
- Regression: `cargo test --test debts`, `cargo test --test statements`, `cargo test --test snowball_size`, and `cargo test --test payments`
- Build: `cargo build`
- After checkpoints: `cargo clippy -- -D warnings` and `cargo fmt --check`

---

## Task 1: statement_month on schema, types, record/get

**Description:** Add `statement_month` (`YYYY-MM`) to `statements`. Unique key is `(debt_id, statement_month)`. `record` / `get` persist it. Recording the same card + cycle month again updates `minimum_cents` and `due_on` and keeps the id. Invalid cycle month is `VALIDATION_ERROR`. Callers pass the cycle; do not infer it from `due_on`. Update existing `RecordStatement` literals so the crate compiles.

**Acceptance criteria:**
- [x] Recording persists `statement_month` and reads it back
- [x] Same card + `statement_month` again updates minimum and due date; one row; same id
- [x] Invalid `statement_month` (`2026-13`, `2026-9`, `2026-08-01`) is `VALIDATION_ERROR`; nothing inserted
- [x] `due_on` in a different calendar month than `statement_month` is allowed
- [x] Existing loan / unknown-debt / bad `due_on` tests still pass

**Verification:**
- [x] Tests pass: `cargo test --test statements`
- [x] Build succeeds: `cargo build`
- [x] Regression: `cargo test --test payments` (updated `RecordStatement`)

**Dependencies:** None

**Files likely touched:**
- `src/db.rs`
- `src/statements/mod.rs`
- `src/statements/store.rs`
- `src/statements/validate.rs`
- `tests/statements.rs`
- `tests/payments.rs` (call-site only)

**Estimated scope:** Medium

---

## Task 2: list_for_month; current and list order by cycle month

**Description:** Add `list_for_month`. `list_for_debt` is `statement_month` ascending, then `id`. `current_for_debt` is the latest `statement_month`. `upcoming` still filters by `due_on`. Two cycle months on one card both exist.

**Acceptance criteria:**
- [x] `list_for_month("2026-08")` returns August-cycle rows only, including a row whose `due_on` is in September
- [x] Two `statement_month` values on one card both exist; `current_for_debt` is the later month
- [x] `list_for_debt` is `statement_month` ascending, then `id`
- [x] `upcoming` still includes `due_on >= as_of` and excludes earlier dates

**Verification:**
- [x] Tests pass: `cargo test --test statements`
- [x] Build succeeds: `cargo build`
- [x] Regression: `cargo test --test debts`, `cargo test --test payments`, `cargo test --test snowball_size`

**Dependencies:** Task 1

**Files likely touched:**
- `src/statements/mod.rs`
- `src/statements/store.rs`
- `tests/statements.rs`

**Estimated scope:** Small

---

## Checkpoint: statements amendment

- [x] Amendment success criteria in `SPEC-statements.md` are met
- [x] `cargo test --test statements` passes
- [x] `cargo test --test debts`, `cargo test --test payments`, and `cargo test --test snowball_size` pass
- [x] `cargo clippy -- -D warnings` and `cargo fmt --check` pass

---

## Task 3: Scaffold plan; reject bad payment month; empty register

**Description:** Add `src/plan/` with types, `compute_plan`, and validation. Invalid `payment_month` is `VALIDATION_ERROR`. Previous-month and due-day clamp helpers have unit tests (September → August; January → previous December; due day 31 in September → `2026-09-30`). No owed debts → empty `lines`, `snowball_amount_cents` from `current()` (`None` if never recorded), `shortfall_cents` `None` when no amount, `unallocated_cents` `0`.

**Acceptance criteria:**
- [x] `2026-13`, `2026-9`, and `2026-09-01` are `VALIDATION_ERROR`
- [x] No debts: `lines` is empty
- [x] Never recorded a snowball amount: `snowball_amount_cents` is `None`, `shortfall_cents` is `None`
- [x] Unit tests: previous month of `2026-09` is `2026-08`; of `2026-01` is `2025-12`; due day 31 in September is `2026-09-30`

**Verification:**
- [x] Tests pass: `cargo test --test plan`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 2

**Files likely touched:**
- `src/plan/mod.rs`
- `src/plan/validate.rs`
- `src/plan/compute.rs`
- `src/lib.rs`
- `tests/plan.rs`

**Estimated scope:** Medium

---

## Task 4: Required amounts, due dates, missing statement, omit paid-off

**Description:** With no recorded snowball amount, `compute_plan("2026-09")` lists every debt with remaining balance `> 0`. Loans: required = `min(usual payment, remaining)`, due date in September (clamped). Cards: August `list_for_month` row → required = `min(minimum, remaining)`, `due_on` from that row; no August row → `missing_statement`, required `$0`, `due_on` `None`. Extra is `$0`. Paid-off debts omitted. Lines ordered by `due_on` ascending, missing dates last, then name. January payment month uses December cycle rows.

**Acceptance criteria:**
- [x] September plan uses August card statements and September loan due dates
- [x] January plan uses `statement_month = "2025-12"`
- [x] Card with no cycle row: `missing_statement = true`, required `$0`, `due_on` `None`
- [x] Remaining `$0` debts do not appear
- [x] Every `extra_cents` is `$0`; `shortfall_cents` is `None`

**Verification:**
- [x] Tests pass: `cargo test --test plan`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 3

**Files likely touched:**
- `src/plan/compute.rs`
- `tests/plan.rs`

**Estimated scope:** Medium

---

## Task 5: Extra rolls, cap, tie-break, shortfall, no side effects

**Description:** When a snowball amount is recorded and covers required minima, extra fills smallest remaining balance then rolls (name ascending when remaining matches). Usual payment / card min above remaining: required is remaining; unused amount is in the extra pool. When required minima exceed the recorded amount: keep full required lines, extra `$0`, `shortfall_cents` is the difference; do not change `current()` snowball amount. Extra may go to a missing-statement card if it is the smallest remaining. Compute does not insert payments or change balances. Unallocated extra (every remaining balance filled) is `unallocated_cents`.

**Acceptance criteria:**
- [x] Loan $200 usual / $50 remaining: required $50; unused $150 is extra for the next-smallest remaining debt
- [x] Extra fills smallest remaining up to remaining balance, then rolls; name breaks ties
- [x] Required minima above recorded amount: full required lines, extra $0, `shortfall_cents` is the difference; recorded amount unchanged
- [x] Missing-statement card can receive extra if it is the smallest remaining
- [x] Compute does not add payment rows or change balances or snowball history

**Verification:**
- [x] Tests pass: `cargo test --test plan`
- [x] Regression: `cargo test --test debts`, `cargo test --test statements`, `cargo test --test snowball_size`, `cargo test --test payments`
- [x] Clippy/fmt: `cargo clippy -- -D warnings` and `cargo fmt --check`

**Dependencies:** Task 4

**Files likely touched:**
- `src/plan/compute.rs`
- `tests/plan.rs`

**Estimated scope:** Medium

---

## Checkpoint: plan complete

- [x] All success criteria in `SPEC-plan.md` are met
- [x] Amendment success criteria in `SPEC-statements.md` are met
- [x] `cargo test --test plan` passes
- [x] `cargo test --test statements`, `cargo test --test debts`, `cargo test --test payments`, and `cargo test --test snowball_size` pass
- [x] `cargo clippy -- -D warnings` and `cargo fmt --check` pass
- [x] Ready for review; next map module to spec is `tui`
