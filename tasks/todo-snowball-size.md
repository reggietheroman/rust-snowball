# Tasks: snowball-size

Verification commands (every task):

- Tests: `cargo test --test snowball_size`
- Regression: `cargo test --test debts`
- Build: `cargo build`
- After checkpoints: `cargo clippy -- -D warnings` and `cargo fmt --check`

---

## Task 1: Scaffold module, types, trait, amount validation

**Description:** Add `src/snowball_size/` with `SizeId`, `SnowballSize`, `RecordSize`, `SnowballSizeStore`, and validation that `amount_cents > 0`. Export the module from `src/lib.rs`. No SQLite schema yet.

**Acceptance criteria:**
- [x] `src/snowball_size/mod.rs` is exported from `src/lib.rs`
- [x] `amount_cents <= 0` is `VALIDATION_ERROR`
- [x] Public types have no debt ids, card mins, or payment fields

**Verification:**
- [x] Tests pass: `cargo test`
- [x] Build succeeds: `cargo build`

**Dependencies:** None

**Files likely touched:**
- `src/lib.rs`
- `src/snowball_size/mod.rs`
- `src/snowball_size/validate.rs`

**Estimated scope:** Small

---

## Checkpoint: Foundation

- [x] `cargo test --test debts` still passes
- [x] `cargo build` succeeds

---

## Task 2: Record first size; empty current is None

**Description:** Add `SqliteSnowballSizeStore` and `snowball_sizes` schema. `record` of a valid amount persists and `current` returns it. Empty store: `current` is `None`, `list` is empty. Invalid amount does not insert a row. Store assigns `id` (`size_<ulid>`) and `recorded_at_unix`.

**Acceptance criteria:**
- [x] Recording $400 persists and `current` returns $400
- [x] Empty ledger: `current` is `None`, `list` is empty
- [x] `amount_cents <= 0` is `VALIDATION_ERROR` and inserts nothing

**Verification:**
- [x] Tests pass: `cargo test --test snowball_size`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 1

**Files likely touched:**
- `src/snowball_size/mod.rs`
- `src/snowball_size/store.rs`
- `src/snowball_size/validate.rs`
- `tests/snowball_size.rs`

**Estimated scope:** Medium

---

## Task 3: List oldest first; raise, shrink, reaffirm

**Description:** `list` returns every row oldest first (`recorded_at_unix`, then `id`). A later larger amount (raise), smaller amount (shrink), or the same amount (reaffirm) each append a row. `current` is always the last row. Earlier rows remain.

**Acceptance criteria:**
- [x] Recording $500 after $400 makes `current` $500; `list` is [$400, $500]
- [x] Recording $300 after $500 makes `current` $300; both earlier rows remain
- [x] Recording the same amount twice yields two rows; `current` is the later one

**Verification:**
- [x] Tests pass: `cargo test --test snowball_size`
- [x] Build succeeds: `cargo build`
- [x] Clippy/fmt: `cargo clippy -- -D warnings` and `cargo fmt --check`
- [x] Regression: `cargo test --test debts`

**Dependencies:** Task 2

**Files likely touched:**
- `src/snowball_size/store.rs`
- `tests/snowball_size.rs`

**Estimated scope:** Small

---

## Checkpoint: snowball-size complete

- [x] All success criteria in `SPEC-snowball-size.md` are met
- [x] `cargo test --test snowball_size` passes
- [x] `cargo test --test debts` passes
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo fmt --check` passes
- [x] Ready for review; next map module to spec is `statements`
