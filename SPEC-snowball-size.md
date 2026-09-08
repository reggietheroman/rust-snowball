# Spec: snowball-size

## Objective

The `snowball-size` module is the ledger of the monthly amount you have committed to send at all remaining debts. That number is the product: it is what you open the app to see, and it is what `plan` splits. It never moves because a debt was paid off, a card minimum changed, or required payments rose. It moves only when you record a new size.

**User:** you, deciding to start, raise, or shrink the snowball.

**This module succeeds when** every size change is an append-only row you can list later, `current` is exactly the latest row (or none), and `plan` can read that amount without knowing about debts, statements, or the TUI.

**Out of scope for this module:** debts, statements, payments, allocation, shortfall warnings, TUI, notes/reasons on a size change.

## Tech Stack

Same crate as `debts`:

- Rust (edition 2024), `snowball` library
- SQLite, same on-disk file as `debts` (table `snowball_sizes`; this module does not read `debts`)
- Money as integer **cents** (`i64`)
- Module at `src/snowball_size/`

## Commands

```
cargo test
cargo test --test snowball_size
cargo clippy -- -D warnings
cargo fmt --check
cargo build
```

## Project Structure

```
SPEC-snowball-size.md
src/snowball_size/mod.rs      → Public contract (types + SnowballSizeStore)
src/snowball_size/store.rs    → SQLite implementation
src/snowball_size/validate.rs → Boundary validation
tests/snowball_size.rs        → Integration tests against in-memory SQLite
```

Reuse `src/error.rs`. Do not add snowball-size fields to `Debt`.

## Code Style

Match `debts`: `snake_case` functions, `PascalCase` types, `SCREAMING_SNAKE` error codes, `Result<T, Error>` on the public surface, opaque ids (`size_<ulid>`).

```rust
pub struct SizeId(String);

pub struct SnowballSize {
    pub id: SizeId,
    pub amount_cents: i64,
    pub recorded_at_unix: i64, // UTC seconds, assigned by the store
}

pub struct RecordSize {
    pub amount_cents: i64,
}

pub trait SnowballSizeStore {
    fn record(&self, input: RecordSize) -> Result<SnowballSize, Error>;
    fn current(&self) -> Result<Option<SnowballSize>, Error>;
    fn list(&self) -> Result<Vec<SnowballSize>, Error>;
}
```

`list` returns every recorded size, oldest first. `current` is the last row in that order, or `None` if the ledger is empty. There is no `update`, `delete`, or `set_current` — those would let a size change without a new row.

Recording the same amount twice is allowed (a reaffirmation). It is still a new row.

## Testing Strategy

- Framework: `cargo test`
- Location: unit tests next to validation; SQLite round-trips in `tests/snowball_size.rs` using `:memory:`
- Coverage: every public `SnowballSizeStore` method has a success test and a rejection test for invalid input. Plus: empty ledger → `current` is `None`; two records → `current` is the second; `list` is oldest-first.
- **Not here:** plan shortfall, debts, TUI.

## Boundaries

- **Always:** Store money as cents; validate at the store boundary; append a new row on every `record`; leave debts and statements untouched; infer current size only from the latest row.
- **Ask first:** Deleting or editing a recorded size.
- **Never:** Change size because a debt was paid off or a card minimum moved; read or write the `debts` table; use `f64` for money; unwrap on expected user/input failures; backdate `recorded_at`; store a note/reason on a size change; allow `amount_cents == 0`.

## Module contract (consumers)

`plan` depends on: `current()` → `amount_cents`. If `None`, `plan` cannot allocate (that behavior belongs in `plan`, not here).

`tui` depends on: `current`, `list`, `record` (raise or shrink).

This module has **no** dependency on `debts`.

### Errors (one shape)

```text
VALIDATION_ERROR   — amount_cents <= 0
```

`NOT_FOUND` is unused here: there is no get-by-id, and an empty ledger is `Ok(None)`, not an error.

### Record rules

| Field | Rule |
|---|---|
| amount_cents | required, `> 0` |
| recorded_at_unix | assigned by the store at insert; callers do not pass it |

## Success Criteria

- [x] Recording $400 persists and `current` returns $400.
- [x] Recording $500 after $400 makes `current` $500; `list` is [$400, $500] in that order.
- [x] Recording $300 after $500 (a shrink) makes `current` $300; both earlier rows remain.
- [x] Recording the same amount twice yields two rows; `current` is the later one.
- [x] `amount_cents <= 0` is `VALIDATION_ERROR` and inserts nothing.
- [x] Empty ledger: `current` is `None`, `list` is empty.
- [x] Public API has no debt ids, card mins, or payment fields.
- [x] `cargo test --test snowball_size` passes with SQLite in memory.

## Decisions

Confirmed 2026-09-08.

- History is amount + time only. No note or reason on a size change.
- Callers cannot backdate. The store stamps `recorded_at_unix` at insert.
- `amount_cents` must be `> 0`. You can shrink, but not to zero.

Amended 2026-09-08 (for `tui`).

- Table `snowball_sizes` migrates on `SqliteDb` with the other tables. `SqliteSnowballSizeStore` takes `&SqliteDb` (same pattern as payments). Public `SnowballSizeStore` behavior is unchanged. The TUI opens one `--db` file.
