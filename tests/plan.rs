use snowball::ErrorCode;
use snowball::db::SqliteDb;
use snowball::debts::{BorrowedDebtStore, CreateDebt, CreateDebtKind, DebtStore};
use snowball::payments::{PaymentStore, RecordPayment, SqlitePaymentStore};
use snowball::plan::compute_plan;
use snowball::snowball_size::{RecordSize, SnowballSizeStore, SqliteSnowballSizeStore};
use snowball::statements::{RecordStatement, SqliteStatementStore, StatementStore};

struct Harness {
    db: SqliteDb,
    sizes: SqliteSnowballSizeStore,
}

impl Harness {
    fn open() -> Self {
        Self {
            db: SqliteDb::open_in_memory().expect("open db"),
            sizes: SqliteSnowballSizeStore::open_in_memory().expect("snowball store"),
        }
    }

    fn debts(&self) -> BorrowedDebtStore<'_> {
        BorrowedDebtStore::new(&self.db)
    }

    fn statements(&self) -> SqliteStatementStore<'_> {
        SqliteStatementStore::new(&self.db)
    }

    fn payments(&self) -> SqlitePaymentStore<'_> {
        SqlitePaymentStore::new(&self.db)
    }

    fn sizes(&self) -> &SqliteSnowballSizeStore {
        &self.sizes
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

    fn loan(
        &self,
        name: &str,
        balance_cents: i64,
        payment_cents: i64,
        due_day: u8,
    ) -> snowball::debts::DebtId {
        self.debts()
            .create(CreateDebt {
                name: name.into(),
                balance_cents,
                kind: CreateDebtKind::Loan {
                    payment_cents,
                    due_day,
                },
            })
            .expect("create loan")
            .id
    }

    fn record_size(&self, amount_cents: i64) {
        self.sizes()
            .record(RecordSize { amount_cents })
            .expect("record size");
    }

    fn plan(&self, payment_month: &str) -> snowball::plan::Plan {
        compute_plan(
            payment_month,
            &self.debts(),
            self.sizes(),
            &self.statements(),
        )
        .expect("compute plan")
    }

    fn line<'a>(plan: &'a snowball::plan::Plan, name: &str) -> &'a snowball::plan::PlanLine {
        plan.lines
            .iter()
            .find(|line| line.name == name)
            .unwrap_or_else(|| panic!("missing line for {name}"))
    }
}

#[test]
fn invalid_payment_month_is_validation_error() {
    let h = Harness::open();
    for payment_month in ["2026-13", "2026-9", "2026-09-01"] {
        let err = compute_plan(payment_month, &h.debts(), h.sizes(), &h.statements()).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }
}

#[test]
fn empty_register_has_no_lines() {
    let h = Harness::open();
    let plan = h.plan("2026-09");
    assert!(plan.lines.is_empty());
    assert_eq!(plan.snowball_amount_cents, None);
    assert_eq!(plan.shortfall_cents, None);
    assert_eq!(plan.unallocated_cents, 0);
}

#[test]
fn no_recorded_snowball_amount_lists_debts_without_extra_or_shortfall() {
    let h = Harness::open();
    let card = h.card("BDO Visa", 147_933_81);
    h.statements()
        .record(RecordStatement {
            debt_id: card,
            statement_month: "2026-08".into(),
            minimum_cents: 4_438_01,
            due_on: "2026-09-14".into(),
        })
        .expect("statement");

    let plan = h.plan("2026-09");
    assert_eq!(plan.snowball_amount_cents, None);
    assert_eq!(plan.shortfall_cents, None);
    assert_eq!(plan.lines.len(), 1);
    assert_eq!(Harness::line(&plan, "BDO Visa").extra_cents, 0);
}

#[test]
fn september_plan_uses_august_statements_and_september_loan_due_dates() {
    let h = Harness::open();
    let card = h.card("BDO Visa", 147_933_81);
    h.loan("Car loan", 10_000_00, 300_00, 14);
    h.statements()
        .record(RecordStatement {
            debt_id: card,
            statement_month: "2026-08".into(),
            minimum_cents: 4_438_01,
            due_on: "2026-09-14".into(),
        })
        .expect("statement");
    h.record_size(500_00);

    let plan = h.plan("2026-09");
    let card_line = Harness::line(&plan, "BDO Visa");
    assert_eq!(card_line.required_cents, 4_438_01);
    assert_eq!(card_line.due_on.as_deref(), Some("2026-09-14"));
    assert!(!card_line.missing_statement);

    let loan_line = Harness::line(&plan, "Car loan");
    assert_eq!(loan_line.required_cents, 300_00);
    assert_eq!(loan_line.due_on.as_deref(), Some("2026-09-14"));
}

#[test]
fn january_plan_uses_december_statements() {
    let h = Harness::open();
    let card = h.card("BPI", 185_439_85);
    h.statements()
        .record(RecordStatement {
            debt_id: card,
            statement_month: "2025-12".into(),
            minimum_cents: 6_622_85,
            due_on: "2026-01-14".into(),
        })
        .expect("statement");

    let plan = h.plan("2026-01");
    let line = Harness::line(&plan, "BPI");
    assert_eq!(line.required_cents, 6_622_85);
    assert_eq!(line.due_on.as_deref(), Some("2026-01-14"));
}

#[test]
fn missing_statement_card_has_no_required_or_due_date() {
    let h = Harness::open();
    h.card("CB", 50_000_00);
    h.record_size(400_00);

    let plan = h.plan("2026-09");
    let line = Harness::line(&plan, "CB");
    assert!(line.missing_statement);
    assert_eq!(line.required_cents, 0);
    assert_eq!(line.due_on, None);
}

#[test]
fn paid_off_debts_are_omitted() {
    let h = Harness::open();
    h.card("Paid", 0);
    h.card("Still owe", 100_00);
    h.record_size(400_00);

    let plan = h.plan("2026-09");
    assert_eq!(plan.lines.len(), 1);
    assert_eq!(plan.lines[0].name, "Still owe");
}

#[test]
fn loan_usual_payment_above_remaining_caps_required_and_frees_extra() {
    let h = Harness::open();
    h.loan("Almost paid loan", 50_00, 200_00, 10);
    h.card("Smallest card", 100_00);
    h.record_size(400_00);

    let plan = h.plan("2026-09");
    let loan = Harness::line(&plan, "Almost paid loan");
    assert_eq!(loan.required_cents, 50_00);
    assert_eq!(loan.extra_cents, 0);

    let card = Harness::line(&plan, "Smallest card");
    assert_eq!(card.required_cents, 0);
    assert_eq!(card.extra_cents, 100_00);
    assert_eq!(card.send_cents, 100_00);
    assert_eq!(plan.unallocated_cents, 400_00);
}

#[test]
fn extra_rolls_to_next_smallest_remaining_debt() {
    let h = Harness::open();
    h.card("Alpha", 80_00);
    h.card("Beta", 500_00);
    h.record_size(500_00);

    let plan = h.plan("2026-09");
    let alpha = Harness::line(&plan, "Alpha");
    assert_eq!(alpha.required_cents, 0);
    assert_eq!(alpha.extra_cents, 80_00);
    assert_eq!(alpha.send_cents, 80_00);

    let beta = Harness::line(&plan, "Beta");
    assert_eq!(beta.extra_cents, 420_00);
    assert_eq!(beta.send_cents, 420_00);
    assert_eq!(plan.unallocated_cents, 0);
}

#[test]
fn name_breaks_tie_when_remaining_balances_match() {
    let h = Harness::open();
    h.card("Beta", 100_00);
    h.card("Alpha", 100_00);
    h.record_size(250_00);

    let plan = h.plan("2026-09");
    let alpha = Harness::line(&plan, "Alpha");
    let beta = Harness::line(&plan, "Beta");
    assert_eq!(alpha.extra_cents, 100_00);
    assert_eq!(beta.extra_cents, 100_00);
    assert_eq!(plan.unallocated_cents, 50_00);
}

#[test]
fn required_minima_above_recorded_amount_keep_full_required_and_show_shortfall() {
    let h = Harness::open();
    let bdo = h.card("BDO Visa", 147_933_81);
    let bpi = h.card("BPI", 185_439_85);
    h.statements()
        .record(RecordStatement {
            debt_id: bdo,
            statement_month: "2026-08".into(),
            minimum_cents: 4_438_01,
            due_on: "2026-09-14".into(),
        })
        .expect("bdo");
    h.statements()
        .record(RecordStatement {
            debt_id: bpi,
            statement_month: "2026-08".into(),
            minimum_cents: 6_622_85,
            due_on: "2026-09-14".into(),
        })
        .expect("bpi");
    h.record_size(5_000_00);

    let plan = h.plan("2026-09");
    assert_eq!(plan.snowball_amount_cents, Some(5_000_00));
    assert_eq!(plan.shortfall_cents, Some(6_060_86));
    assert_eq!(Harness::line(&plan, "BDO Visa").extra_cents, 0);
    assert_eq!(Harness::line(&plan, "BPI").extra_cents, 0);
    assert_eq!(
        h.sizes().current().expect("current").unwrap().amount_cents,
        5_000_00
    );
}

#[test]
fn missing_statement_card_can_receive_extra_when_smallest_remaining() {
    let h = Harness::open();
    h.card("CB", 40_00);
    let bpi = h.card("BPI", 500_00);
    h.statements()
        .record(RecordStatement {
            debt_id: bpi,
            statement_month: "2026-08".into(),
            minimum_cents: 100_00,
            due_on: "2026-09-14".into(),
        })
        .expect("bpi");
    h.record_size(300_00);

    let plan = h.plan("2026-09");
    let cb = Harness::line(&plan, "CB");
    assert!(cb.missing_statement);
    assert_eq!(cb.required_cents, 0);
    assert_eq!(cb.extra_cents, 40_00);
}

#[test]
fn compute_does_not_write_payments_or_change_balances_or_snowball() {
    let h = Harness::open();
    let card = h.card("Chase", 1_000_00);
    h.statements()
        .record(RecordStatement {
            debt_id: card.clone(),
            statement_month: "2026-08".into(),
            minimum_cents: 50_00,
            due_on: "2026-09-15".into(),
        })
        .expect("statement");
    h.record_size(400_00);
    let balance_before = h.debts().get(&card).expect("get").balance_cents;
    let size_before = h.sizes().current().expect("current").unwrap().amount_cents;

    let _plan = h.plan("2026-09");

    assert_eq!(
        h.debts().get(&card).expect("get").balance_cents,
        balance_before
    );
    assert_eq!(
        h.sizes().current().expect("current").unwrap().amount_cents,
        size_before
    );
    assert_eq!(
        h.payments().list_for_debt(&card).expect("payments").len(),
        0
    );

    h.payments()
        .record(RecordPayment {
            debt_id: card.clone(),
            amount_cents: 25_00,
            paid_on: "2026-09-08".into(),
        })
        .expect("record payment");
    let payment_count = h.payments().list_for_debt(&card).expect("payments").len();
    let _plan = h.plan("2026-09");
    assert_eq!(
        h.payments().list_for_debt(&card).expect("payments").len(),
        payment_count
    );
}
