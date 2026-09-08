# Implementation Plan: tui

## Overview

Move `snowball_sizes` onto `SqliteDb`, then add the `snowball` binary: one home screen (this payment month in pesos, from `compute_plan`), vim-like keys, overlays for payments / statements / size, and a debts list. When this plan is done, `cargo run -- --db <path>` is the app; library modules still have no terminal code in them.

Prior module archive: `tasks/plan-plan.md`, `tasks/todo-plan.md`.

## Architecture Decisions

- **One file, one `SqliteDb`.** `migrate()` creates `snowball_sizes` (`CREATE TABLE IF NOT EXISTS`, same columns as today). `SqliteSnowballSizeStore<'db>` takes `&SqliteDb` like payments. Drop the store’s own `Connection` and its private `migrate`. `open` / `open_in_memory` on the size store go away; tests hold `SqliteDb` and call `SqliteSnowballSizeStore::new(&db)`. Public `SnowballSizeStore` methods do not change.
- **Clock is injected.** `App` takes `today` (`YYYY-MM-DD`) and starts `payment_month` as that date’s `YYYY-MM`. `tui::run` fills these from the local calendar once. Tests pass `"2026-09-08"` so home and overdue marks are not flaky.
- **`handle_key` + `draw` are the API tests call.** `run` installs crossterm, loops, and exits when a command is quit. Tests never open a TTY: `TestBackend`, `handle_key`, inspect the buffer.
- **Home is `compute_plan` only.** No `upcoming` pane. `?` missing statement, `!` when `due_on < today`. Pesos via `format_pesos` / `parse_pesos` (`src/tui/money.rs`). Raw cent counts never appear on money fields.
- **Month step uses plan calendar helpers.** Add public `next_month` next to `previous_month` in `src/plan/validate.rs` and re-export both from `plan`. TUI does not copy month arithmetic.
- **Mode enum, not booleans.** `Home`, `Help`, `Debts`, `Payments { debt_id }`, `Statements { debt_id }`, `Size`, `Form(...)`. Forms are a variant (or a nested enum) with a focused field. `q` quits in list modes; in a form it is a character. Esc discards a form, then closes an overlay.
- **No clap.** `src/main.rs` reads `--db PATH` from `std::env::args`. Default: `$XDG_DATA_HOME/snowball/snowball.db` or `~/.local/share/snowball/snowball.db`. `create_dir_all` on the parent. No extra crates for paths.
- **ratatui + crossterm** on the same package. Domain modules do not `use` them. Pin current stable at implement time; tests use `ratatui::backend::TestBackend`.
- **No mouse. No color-only meaning.** Default terminal style. Marks are characters.

## Dependency graph (this module)

```
snowball_sizes on SqliteDb
    │
    └── plan/payments tests share that db
            │
            └── pesos + key map (pure)
                    │
                    └── App + TestBackend empty home
                            │
                            └── home draws compute_plan (h/l, marks, shortfall)
                                    │
                                    ├── payments overlay (record + edit)
                                    ├── statements overlay
                                    ├── size overlay
                                    └── debts list (create / edit / set_balance)
                                            │
                                            └── binary: --db, default path, real run loop
```

## What can be parallel vs sequential

Foundation (Tasks 1–2) is sequential and blocks everything. Everything after Task 2 is sequential. Tasks 6–8 share `overlays.rs`. Debts list (`debts.rs`) comes after those overlays so home `d` and form helpers already exist. Binary last so `run` exists.

## Task List

Index only. Full acceptance criteria live in `tasks/todo.md`.

### Foundation: one SQLite file

- [x] Task 1: `snowball_sizes` on `SqliteDb`; size store on `&SqliteDb`
- [x] Task 2: plan + payments tests use the shared db

### Checkpoint: Shared DB

- [x] `SqliteDb::open_in_memory()` creates `snowball_sizes`
- [x] `cargo test --test snowball_size`, `--test plan`, `--test payments` pass

### Home

- [x] Task 3: `src/tui/` scaffold; pesos; key map; empty home on TestBackend
- [x] Task 4: Home renders a seeded plan (pesos, `?` / `!`, shortfall, `h`/`l`)
- [x] Task 5: `j`/`k` selection; `p`/`s` with no selection; `q` / Esc / `?` help

### Checkpoint: Home

- [x] Empty and seeded home buffers match `SPEC-tui.md` header + one list
- [x] `cargo test --test tui` passes; no TTY

### Write paths

- [x] Task 6: Payments overlay — list, record, edit, overpayment flag
- [x] Task 7: Statements overlay — list, record/upsert; `s` on a loan does not open
- [x] Task 8: Size overlay — history newest first, record, header follows `current`
- [x] Task 9: Debts list — create loan/card, edit, `set_balance`; paid-off visible here only

### Checkpoint: Overlays

- [x] Record/edit paths persist and home recomputes
- [x] `cargo test --test tui` and library tests pass

### Binary

- [x] Task 10: `src/main.rs` — `--db`, default XDG path, `tui::run`

### Checkpoint: tui complete

- [x] All `SPEC-tui.md` success criteria met
- [x] `cargo test`, clippy, fmt pass
- [x] Ready for review

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Plan tests keep a second in-memory size DB | High | Task 2: one `SqliteDb`; `compute_plan` must see recorded sizes. Fail a plan test that records a size then computes if still split. |
| Home tests depend on “today” | High | Inject `today = 2026-09-08` in every TUI test. |
| `₱` width on TestBackend | Med | Unit-test `format_pesos`; buffer asserts on `1,234.56` and on `₱` if the backend keeps the codepoint. |
| Overlay + form state explodes | Med | One `Mode` enum; forms are data + focused field index; no parallel boolean flags. |
| `run` pulls crossterm into tests | Med | Tests call `handle_key` / `draw` only. `run` is thin and covered by a unit test that `--db` parsing does not need a TTY. |
| File count on Task 6 | Med | Payments overlay is list + two forms in `overlays.rs` only; do not start statements in the same task. |

## Open Questions

None. Spec decisions confirmed 2026-09-08.

## Task list target

`tasks/todo.md` (markdown checklist). No external tracker.
