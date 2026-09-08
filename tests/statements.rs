use snowball::ErrorCode;
use snowball::db::SqliteDb;
use snowball::debts::{BorrowedDebtStore, CreateDebt, CreateDebtKind, DebtStore, SqliteDebtStore};
use snowball::statements::{RecordStatement, SqliteStatementStore, StatementStore};

struct Harness {
    db: SqliteDb,
}

impl Harness {
    fn open() -> Self {
        Self {
            db: SqliteDb::open_in_memory().expect("open db"),
        }
    }

    fn debts(&self) -> BorrowedDebtStore<'_> {
        BorrowedDebtStore::new(&self.db)
    }

    fn statements(&self) -> SqliteStatementStore<'_> {
        SqliteStatementStore::new(&self.db)
    }

    fn card(&self, name: &str, balance_cents: i64) -> snowball::debts::DebtId {
        self.debts()
            .create(CreateDebt {
                name: name.into(),
                balance_cents,
                kind: CreateDebtKind::CreditCard,
            })
            .expect("create card")
            .id
    }

    fn loan(&self) -> snowball::debts::DebtId {
        self.debts()
            .create(CreateDebt {
                name: "Car loan".into(),
                balance_cents: 10_000_00,
                kind: CreateDebtKind::Loan {
                    payment_cents: 300_00,
                    due_day: 15,
                },
            })
            .expect("create loan")
            .id
    }
}

#[test]
fn record_on_credit_card_persists() {
    let h = Harness::open();
    let card = h.card("Chase", 2_500_00);

    let stmt = h
        .statements()
        .record(RecordStatement {
            debt_id: card.clone(),
            minimum_cents: 75_00,
            due_on: "2026-09-15".into(),
        })
        .expect("record");

    assert!(stmt.id.as_str().starts_with("stmt_"));
    assert_eq!(stmt.debt_id, card);
    assert_eq!(stmt.minimum_cents, 75_00);
    assert_eq!(stmt.due_on, "2026-09-15");

    let fetched = h.statements().get(&stmt.id).expect("get");
    assert_eq!(fetched, stmt);
}

#[test]
fn record_on_loan_is_validation_error() {
    let h = Harness::open();
    let loan = h.loan();

    let err = h
        .statements()
        .record(RecordStatement {
            debt_id: loan,
            minimum_cents: 50_00,
            due_on: "2026-09-15".into(),
        })
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);
}

#[test]
fn record_on_unknown_debt_is_not_found() {
    let h = Harness::open();
    let err = h
        .statements()
        .record(RecordStatement {
            debt_id: snowball::debts::DebtId::from_existing("debt_missing"),
            minimum_cents: 50_00,
            due_on: "2026-09-15".into(),
        })
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
}

#[test]
fn invalid_input_is_validation_error() {
    let h = Harness::open();
    let card = h.card("Chase", 100);

    for (minimum_cents, due_on) in [(-1, "2026-09-15"), (50, "2026-02-31")] {
        let err = h
            .statements()
            .record(RecordStatement {
                debt_id: card.clone(),
                minimum_cents,
                due_on: due_on.into(),
            })
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }
}

#[test]
fn same_due_date_updates_minimum_and_keeps_id() {
    let h = Harness::open();
    let card = h.card("Chase", 1_000_00);

    let first = h
        .statements()
        .record(RecordStatement {
            debt_id: card.clone(),
            minimum_cents: 50_00,
            due_on: "2026-09-15".into(),
        })
        .expect("first record");

    let second = h
        .statements()
        .record(RecordStatement {
            debt_id: card.clone(),
            minimum_cents: 60_00,
            due_on: "2026-09-15".into(),
        })
        .expect("second record");

    assert_eq!(first.id, second.id);
    assert_eq!(second.minimum_cents, 60_00);
    assert_eq!(h.statements().list_for_debt(&card).expect("list").len(), 1);
}

#[test]
fn current_for_debt_is_latest_due_on() {
    let h = Harness::open();
    let card = h.card("Chase", 1_000_00);

    h.statements()
        .record(RecordStatement {
            debt_id: card.clone(),
            minimum_cents: 40_00,
            due_on: "2026-09-01".into(),
        })
        .expect("sep 1");
    let later = h
        .statements()
        .record(RecordStatement {
            debt_id: card.clone(),
            minimum_cents: 55_00,
            due_on: "2026-10-01".into(),
        })
        .expect("oct 1");

    let current = h
        .statements()
        .current_for_debt(&card)
        .expect("current")
        .expect("some");
    assert_eq!(current.id, later.id);
    assert_eq!(current.minimum_cents, 55_00);
}

#[test]
fn list_for_debt_is_due_on_ascending() {
    let h = Harness::open();
    let card = h.card("Chase", 1_000_00);

    h.statements()
        .record(RecordStatement {
            debt_id: card.clone(),
            minimum_cents: 55_00,
            due_on: "2026-10-01".into(),
        })
        .expect("oct");
    h.statements()
        .record(RecordStatement {
            debt_id: card.clone(),
            minimum_cents: 40_00,
            due_on: "2026-09-01".into(),
        })
        .expect("sep");

    let list = h.statements().list_for_debt(&card).expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].due_on, "2026-09-01");
    assert_eq!(list[1].due_on, "2026-10-01");
}

#[test]
fn upcoming_includes_today_and_later_excludes_earlier() {
    let h = Harness::open();
    let card_a = h.card("Alpha", 100);
    let card_b = h.card("Beta", 100);

    h.statements()
        .record(RecordStatement {
            debt_id: card_a.clone(),
            minimum_cents: 25_00,
            due_on: "2026-09-07".into(),
        })
        .expect("earlier");
    h.statements()
        .record(RecordStatement {
            debt_id: card_b,
            minimum_cents: 30_00,
            due_on: "2026-09-08".into(),
        })
        .expect("today");
    h.statements()
        .record(RecordStatement {
            debt_id: card_a,
            minimum_cents: 35_00,
            due_on: "2026-09-20".into(),
        })
        .expect("later");

    let upcoming = h.statements().upcoming("2026-09-08").expect("upcoming");
    assert_eq!(upcoming.len(), 2);
    assert_eq!(upcoming[0].due_on, "2026-09-08");
    assert_eq!(upcoming[1].due_on, "2026-09-20");
}

#[test]
fn recording_does_not_change_debt_balance() {
    let h = Harness::open();
    let card = h.card("Chase", 2_500_00);

    h.statements()
        .record(RecordStatement {
            debt_id: card.clone(),
            minimum_cents: 75_00,
            due_on: "2026-09-15".into(),
        })
        .expect("record");

    let debt = h.debts().get(&card).expect("get debt");
    assert_eq!(debt.balance_cents, 2_500_00);
}

#[test]
fn zero_minimum_is_allowed() {
    let h = Harness::open();
    let card = h.card("Chase", 0);

    let stmt = h
        .statements()
        .record(RecordStatement {
            debt_id: card,
            minimum_cents: 0,
            due_on: "2026-09-15".into(),
        })
        .expect("record zero min");
    assert_eq!(stmt.minimum_cents, 0);
}

#[test]
fn owned_debt_store_still_works() {
    let store = SqliteDebtStore::open_in_memory().expect("open");
    let card = store
        .create(CreateDebt {
            name: "Chase".into(),
            balance_cents: 100,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create");
    assert!(card.id.as_str().starts_with("debt_"));
}
