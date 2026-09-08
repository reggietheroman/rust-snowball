# Tasks: debts

Verification commands (every task):

- Tests: `cargo test --test debts`
- Build: `cargo build`
- After checkpoints: `cargo clippy -- -D warnings` and `cargo fmt --check`

---

## Task 1: Scaffold lib, Error, empty debts module

**Description:** Create the Cargo library crate with shared `Error` (`code` + `message`) and a `debts` module that compiles. No schema yet. Proves the repo can test and build before domain logic exists.

**Acceptance criteria:**
- [x] `cargo build` succeeds for a `snowball` library
- [x] `Error` has stable codes `NOT_FOUND` and `VALIDATION_ERROR`
- [x] `src/debts/mod.rs` exists and is exported from `src/lib.rs`

**Verification:**
- [x] Tests pass: `cargo test`
- [x] Build succeeds: `cargo build`

**Dependencies:** None

**Files likely touched:**
- `Cargo.toml`
- `src/lib.rs`
- `src/error.rs`
- `src/debts/mod.rs`
- `.gitignore`

**Estimated scope:** Small

---

## Checkpoint: Foundation

- [x] `cargo test` and `cargo build` succeed
- [x] Review with human only if crate layout disagrees with `SPEC-debts.md`

---

## Task 2: Create and get a loan

**Description:** Add `Debt`, `DebtKind::Loan`, `CreateDebt`, `DebtStore`, and `SqliteDebtStore` with schema. Creating a valid loan persists it; `get` returns the same name, balance, payment, and due day. Invalid loan input (empty name, negative balance, payment `<= 0`, `due_day` outside 1–31) is `VALIDATION_ERROR`.

**Acceptance criteria:**
- [x] A loan created with name, balance, payment, and due day reads back unchanged
- [x] `get` on an unknown id returns `NOT_FOUND`
- [x] Invalid loan fields return `VALIDATION_ERROR` (no row inserted)

**Verification:**
- [x] Tests pass: `cargo test --test debts`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 1

**Files likely touched:**
- `Cargo.toml` (rusqlite bundled, ulid)
- `src/debts/mod.rs`
- `src/debts/validate.rs`
- `src/debts/store.rs`
- `tests/debts.rs`

**Estimated scope:** Medium

---

## Task 3: Create a credit card; reject loan fields on cards

**Description:** Add `DebtKind::CreditCard`. A card is name + balance only. Creating a card with payment or due day is `VALIDATION_ERROR`. Creating a loan still requires those fields.

**Acceptance criteria:**
- [x] A credit card created with name and balance only reads back as `CreditCard`
- [x] Creating a card with payment or due day is rejected
- [x] Public types have no card-minimum or snowball-size fields

**Verification:**
- [x] Tests pass: `cargo test --test debts`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 2

**Files likely touched:**
- `src/debts/mod.rs`
- `src/debts/validate.rs`
- `src/debts/store.rs`
- `tests/debts.rs`

**Estimated scope:** Small

---

## Checkpoint: Both kinds persist

- [x] Loan and card round-trip tests pass
- [x] Card-with-loan-fields test passes
- [x] `cargo clippy -- -D warnings` passes
- [ ] Review with human before proceeding if the union shape feels wrong

---

## Task 4: Unique names; list includes $0, sorted by name

**Description:** `list` returns every debt, including `balance_cents == 0`, ordered by name ascending. A second debt with the same name is `VALIDATION_ERROR`.

**Acceptance criteria:**
- [x] Two debts cannot share a name
- [x] `list` includes a debt whose balance is 0
- [x] `list` order is name ascending

**Verification:**
- [x] Tests pass: `cargo test --test debts`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 3

**Files likely touched:**
- `src/debts/store.rs`
- `src/debts/validate.rs`
- `tests/debts.rs`

**Estimated scope:** Small

---

## Task 5: Partial update; kind change rejected; missing id is NOT_FOUND

**Description:** `update` changes only provided fields (name, loan payment, loan due day). Omitting a field leaves it. Changing kind is rejected. Unknown id is `NOT_FOUND`. Duplicate name on rename is `VALIDATION_ERROR`.

**Acceptance criteria:**
- [x] Updating a loan payment leaves name and due day unchanged
- [x] Updating kind (loan ↔ card) is `VALIDATION_ERROR`
- [x] `update` on an unknown id is `NOT_FOUND`

**Verification:**
- [x] Tests pass: `cargo test --test debts`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 4

**Files likely touched:**
- `src/debts/mod.rs`
- `src/debts/validate.rs`
- `src/debts/store.rs`
- `tests/debts.rs`

**Estimated scope:** Medium

---

## Checkpoint: Registry complete

- [x] Create, get, list, and update tests pass
- [x] `cargo test --test debts` passes

---

## Task 6: set_balance (including raising a card)

**Description:** `set_balance` restates remaining balance (`>= 0`). A card balance can go up (new spending). This does not insert a payment. Unknown id is `NOT_FOUND`; negative is `VALIDATION_ERROR`.

**Acceptance criteria:**
- [x] `set_balance` can raise a card balance
- [x] Negative balance is `VALIDATION_ERROR`
- [x] Unknown id is `NOT_FOUND`

**Verification:**
- [x] Tests pass: `cargo test --test debts`
- [x] Build succeeds: `cargo build`

**Dependencies:** Task 5

**Files likely touched:**
- `src/debts/store.rs`
- `src/debts/validate.rs`
- `tests/debts.rs`

**Estimated scope:** Small

---

## Task 7: reduce_balance saturates at 0

**Description:** `reduce_balance` subtracts `cents > 0` and floors at 0 so a final payoff can exceed remaining balance. Zero or negative `cents` is `VALIDATION_ERROR`. Unknown id is `NOT_FOUND`.

**Acceptance criteria:**
- [x] `reduce_balance` on $400 by $500 yields $0, not an error
- [x] `cents <= 0` is `VALIDATION_ERROR`
- [x] Unknown id is `NOT_FOUND`

**Verification:**
- [x] Tests pass: `cargo test --test debts`
- [x] Build succeeds: `cargo build`
- [x] Clippy/fmt: `cargo clippy -- -D warnings` and `cargo fmt --check`

**Dependencies:** Task 6

**Files likely touched:**
- `src/debts/store.rs`
- `src/debts/validate.rs`
- `tests/debts.rs`

**Estimated scope:** Small

---

## Checkpoint: debts module complete

- [x] All success criteria in `SPEC-debts.md` are met
- [x] `cargo test --test debts` passes
- [x] `cargo clippy -- -D warnings` passes
- [x] `cargo fmt --check` passes
- [x] Ready for review; next map module to spec is `snowball-size` (parallel with debts, no dependency)
