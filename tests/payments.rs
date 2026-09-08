use snowball::ErrorCode;
use snowball::db::SqliteDb;
use snowball::debts::{BorrowedDebtStore, CreateDebt, CreateDebtKind, DebtStore};
use snowball::payments::{PaymentStore, RecordPayment, SqlitePaymentStore, UpdatePayment};
use snowball::snowball_size::{RecordSize, SnowballSizeStore, SqliteSnowballSizeStore};
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

    fn payments(&self) -> SqlitePaymentStore<'_> {
        SqlitePaymentStore::new(&self.db)
    }

    fn statements(&self) -> SqliteStatementStore<'_> {
        SqliteStatementStore::new(&self.db)
    }

    fn snowball_sizes(&self) -> SqliteSnowballSizeStore<'_> {
        SqliteSnowballSizeStore::new(&self.db)
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

    fn loan(&self, balance_cents: i64) -> snowball::debts::DebtId {
        self.debts()
            .create(CreateDebt {
                name: "Car loan".into(),
                balance_cents,
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
fn record_on_loan_reduces_balance() {
    let h = Harness::open();
    let debt_id = h.loan(1_000_00);

    let payment = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 400_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record");

    assert!(payment.id.as_str().starts_with("pay_"));
    assert_eq!(payment.amount_cents, 400_00);
    assert_eq!(payment.applied_cents, 400_00);
    assert!(!payment.is_overpayment());

    let debt = h.debts().get(&debt_id).expect("get debt");
    assert_eq!(debt.balance_cents, 600_00);
    assert_eq!(h.payments().get(&payment.id).expect("get"), payment);
}

#[test]
fn record_on_card_reduces_balance() {
    let h = Harness::open();
    let debt_id = h.card("Chase", 2_500_00);

    let payment = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 100_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record");

    assert_eq!(payment.applied_cents, 100_00);
    assert_eq!(
        h.debts().get(&debt_id).expect("get").balance_cents,
        2_400_00
    );
}

#[test]
fn unknown_debt_is_not_found() {
    let h = Harness::open();
    let err = h
        .payments()
        .record(RecordPayment {
            debt_id: snowball::debts::DebtId::from_existing("debt_missing"),
            amount_cents: 100,
            paid_on: "2026-09-08".into(),
        })
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
}

#[test]
fn invalid_input_is_validation_error_and_leaves_balance_unchanged() {
    let h = Harness::open();
    let debt_id = h.card("Chase", 500_00);

    for (amount, paid_on) in [(0, "2026-09-08"), (-1, "2026-09-08"), (100, "2026-02-31")] {
        let err = h
            .payments()
            .record(RecordPayment {
                debt_id: debt_id.clone(),
                amount_cents: amount,
                paid_on: paid_on.into(),
            })
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    assert_eq!(h.debts().get(&debt_id).expect("get").balance_cents, 500_00);
    assert!(
        h.payments()
            .list_for_debt(&debt_id)
            .expect("list")
            .is_empty()
    );
}

#[test]
fn overpayment_floors_balance_and_flags() {
    let h = Harness::open();
    let debt_id = h.card("Chase", 400_00);

    let payment = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 500_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record");

    assert_eq!(payment.amount_cents, 500_00);
    assert_eq!(payment.applied_cents, 400_00);
    assert!(payment.is_overpayment());
    assert_eq!(h.debts().get(&debt_id).expect("get").balance_cents, 0);
}

#[test]
fn payment_on_zero_balance_debt() {
    let h = Harness::open();
    let debt_id = h.card("Paid", 0);

    let payment = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 100_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record");

    assert_eq!(payment.applied_cents, 0);
    assert!(payment.is_overpayment());
    assert_eq!(h.debts().get(&debt_id).expect("get").balance_cents, 0);
}

#[test]
fn exact_payoff_is_not_overpayment() {
    let h = Harness::open();
    let debt_id = h.card("Chase", 400_00);

    let payment = h
        .payments()
        .record(RecordPayment {
            debt_id,
            amount_cents: 400_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record");

    assert_eq!(payment.applied_cents, 400_00);
    assert!(!payment.is_overpayment());
}

#[test]
fn two_same_day_payments_both_exist() {
    let h = Harness::open();
    let debt_id = h.card("Chase", 1_000_00);

    let first = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 100_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("first");
    let second = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 200_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("second");

    assert_ne!(first.id, second.id);
    let list = h.payments().list_for_debt(&debt_id).expect("list");
    assert_eq!(list.len(), 2);
}

#[test]
fn list_for_debt_is_paid_on_then_id_ascending() {
    let h = Harness::open();
    let debt_id = h.card("Chase", 5_000_00);

    h.payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 100,
            paid_on: "2026-09-10".into(),
        })
        .expect("later");
    h.payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 100,
            paid_on: "2026-09-08".into(),
        })
        .expect("earlier");

    let list = h.payments().list_for_debt(&debt_id).expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].paid_on, "2026-09-08");
    assert_eq!(list[1].paid_on, "2026-09-10");
}

#[test]
fn update_amount_restores_then_reapplies() {
    let h = Harness::open();
    let debt_id = h.card("Chase", 1_000_00);

    let payment = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 400_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record");
    assert_eq!(h.debts().get(&debt_id).expect("get").balance_cents, 600_00);

    let updated = h
        .payments()
        .update(
            &payment.id,
            UpdatePayment {
                amount_cents: Some(40_00),
                ..Default::default()
            },
        )
        .expect("update");

    assert_eq!(updated.amount_cents, 40_00);
    assert_eq!(updated.applied_cents, 40_00);
    assert_eq!(h.debts().get(&debt_id).expect("get").balance_cents, 960_00);
}

#[test]
fn update_debt_id_moves_applied_cents() {
    let h = Harness::open();
    let debt_a = h.card("A", 1_000_00);
    let debt_b = h.card("B", 500_00);

    let payment = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_a.clone(),
            amount_cents: 300_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record");

    let updated = h
        .payments()
        .update(
            &payment.id,
            UpdatePayment {
                debt_id: Some(debt_b.clone()),
                ..Default::default()
            },
        )
        .expect("update");

    assert_eq!(updated.debt_id, debt_b);
    assert_eq!(updated.applied_cents, 300_00);
    assert_eq!(
        h.debts().get(&debt_a).expect("get a").balance_cents,
        1_000_00
    );
    assert_eq!(h.debts().get(&debt_b).expect("get b").balance_cents, 200_00);
}

#[test]
fn update_paid_on_only_leaves_balances_unchanged() {
    let h = Harness::open();
    let debt_id = h.card("Chase", 1_000_00);

    let payment = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 400_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record");

    let updated = h
        .payments()
        .update(
            &payment.id,
            UpdatePayment {
                paid_on: Some("2026-09-15".into()),
                ..Default::default()
            },
        )
        .expect("update");

    assert_eq!(updated.paid_on, "2026-09-15");
    assert_eq!(updated.applied_cents, 400_00);
    assert_eq!(h.debts().get(&debt_id).expect("get").balance_cents, 600_00);
}

#[test]
fn record_and_update_do_not_change_statements_or_snowball_size() {
    let h = Harness::open();
    let debt_id = h.card("Chase", 1_000_00);

    let statement = h
        .statements()
        .record(RecordStatement {
            debt_id: debt_id.clone(),
            statement_month: "2026-09".into(),
            minimum_cents: 50_00,
            due_on: "2026-10-01".into(),
        })
        .expect("statement");

    let size_store = h.snowball_sizes();
    size_store
        .record(RecordSize {
            amount_cents: 400_00,
        })
        .expect("size");
    let size_before = size_store.current().expect("current").expect("some");

    let payment = h
        .payments()
        .record(RecordPayment {
            debt_id: debt_id.clone(),
            amount_cents: 100_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record");

    h.payments()
        .update(
            &payment.id,
            UpdatePayment {
                amount_cents: Some(150_00),
                ..Default::default()
            },
        )
        .expect("update");

    let stmt_after = h.statements().get(&statement.id).expect("get statement");
    assert_eq!(stmt_after.minimum_cents, 50_00);

    let size_after = size_store.current().expect("current").expect("some");
    assert_eq!(size_after.amount_cents, size_before.amount_cents);
}

#[test]
fn get_unknown_payment_is_not_found() {
    let h = Harness::open();
    let err = h
        .payments()
        .get(&snowball::payments::PaymentId::from_existing("pay_missing"))
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
}
