use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use snowball::db::SqliteDb;
use snowball::debts::{BorrowedDebtStore, CreateDebt, CreateDebtKind, DebtStore};
use snowball::plan::compute_plan;
use snowball::snowball_size::{RecordSize, SnowballSizeStore, SqliteSnowballSizeStore};
use snowball::statements::{RecordStatement, SqliteStatementStore, StatementStore};
use snowball::tui::{App, draw, format_pesos, parse_pesos};

const TODAY: &str = "2026-09-08";

fn render(app: &App) -> String {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| draw(app, frame)).expect("draw");
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

fn app() -> App {
    App::new(SqliteDb::open_in_memory().expect("db"), TODAY.to_string()).expect("app")
}

fn key_char(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)
}

#[test]
fn empty_home_shows_month_and_no_size() {
    let text = render(&app());
    assert!(text.contains("2026-09"));
    assert!(text.contains("no size recorded"));
    assert!(text.contains("press d"));
}

#[test]
fn quit_sets_flag_esc_on_home_does_not() {
    let mut app = app();
    app.handle_key(key_char('q'));
    assert!(app.quit);
    app.quit = false;
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(!app.quit);
}

#[test]
fn seeded_home_shows_plan_in_pesos() {
    let mut app = app();
    let debts = BorrowedDebtStore::new(app.db());
    debts
        .create(CreateDebt {
            name: "Card A".into(),
            balance_cents: 1_000_00,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("card");
    let _loan = debts
        .create(CreateDebt {
            name: "Loan B".into(),
            balance_cents: 500_00,
            kind: CreateDebtKind::Loan {
                payment_cents: 200_00,
                due_day: 15,
            },
        })
        .expect("loan");
    SqliteSnowballSizeStore::new(app.db())
        .record(RecordSize {
            amount_cents: 400_00,
        })
        .expect("size");
    SqliteStatementStore::new(app.db())
        .record(RecordStatement {
            debt_id: debts.list().expect("list")[0].id.clone(),
            statement_month: "2026-08".into(),
            minimum_cents: 50_00,
            due_on: "2026-09-10".into(),
        })
        .expect("stmt");
    app.refresh_plan().expect("refresh");

    let text = render(&app);
    assert!(text.contains("₱400.00"));
    assert!(text.contains("Card A"));
    assert!(text.contains("Loan B"));
    assert!(text.contains("2026-09-10"));
}

#[test]
fn shortfall_appears_when_minima_exceed_size() {
    let mut app = app();
    let debts = BorrowedDebtStore::new(app.db());
    debts
        .create(CreateDebt {
            name: "Loan".into(),
            balance_cents: 10_000_00,
            kind: CreateDebtKind::Loan {
                payment_cents: 500_00,
                due_day: 1,
            },
        })
        .expect("loan");
    SqliteSnowballSizeStore::new(app.db())
        .record(RecordSize {
            amount_cents: 100_00,
        })
        .expect("size");
    app.refresh_plan().expect("refresh");

    let text = render(&app);
    assert!(text.contains("Shortfall"));
    assert!(text.contains("₱400.00"));
}

#[test]
fn h_and_l_change_payment_month() {
    let mut app = app();
    app.handle_key(key_char('l'));
    assert_eq!(app.payment_month(), "2026-10");
    app.handle_key(key_char('h'));
    assert_eq!(app.payment_month(), "2026-09");
    app.handle_key(key_char('h'));
    assert_eq!(app.payment_month(), "2026-08");
}

#[test]
fn p_without_selection_sets_status() {
    let mut app = app();
    app.handle_key(key_char('p'));
    assert!(app.status.contains("Select"));
}

#[test]
fn help_opens_and_esc_closes() {
    let mut app = app();
    app.handle_key(key_char('?'));
    let help = render(&app);
    assert!(help.contains("Help"));
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(matches!(app.mode, snowball::tui::Mode::Home));
}

#[test]
fn parse_and_format_pesos_round_trip() {
    assert_eq!(format_pesos(123_456), "₱1,234.56");
    assert_eq!(parse_pesos("1,234.56").expect("parse"), 123_456);
}

#[test]
fn compute_plan_sees_size_on_shared_db() {
    let db = SqliteDb::open_in_memory().expect("db");
    let debts = BorrowedDebtStore::new(&db);
    debts
        .create(CreateDebt {
            name: "Loan".into(),
            balance_cents: 1_000_00,
            kind: CreateDebtKind::Loan {
                payment_cents: 100_00,
                due_day: 1,
            },
        })
        .expect("loan");
    SqliteSnowballSizeStore::new(&db)
        .record(RecordSize {
            amount_cents: 400_00,
        })
        .expect("size");
    let plan = compute_plan(
        "2026-09",
        &debts,
        &SqliteSnowballSizeStore::new(&db),
        &SqliteStatementStore::new(&db),
    )
    .expect("plan");
    assert_eq!(plan.snowball_amount_cents, Some(400_00));
}
