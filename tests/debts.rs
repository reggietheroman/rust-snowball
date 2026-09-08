use snowball::ErrorCode;
use snowball::debts::{
    CreateDebt, CreateDebtKind, DebtKind, DebtStore, SqliteDebtStore, UpdateDebt,
};

fn store() -> SqliteDebtStore {
    SqliteDebtStore::open_in_memory().expect("in-memory store")
}

#[test]
fn create_and_get_loan() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Car loan".into(),
            balance_cents: 12_000_00,
            kind: CreateDebtKind::Loan {
                payment_cents: 350_00,
                due_day: 15,
            },
        })
        .expect("create loan");

    assert!(debt.id.as_str().starts_with("debt_"));
    assert_eq!(debt.name, "Car loan");
    assert_eq!(debt.balance_cents, 12_000_00);
    assert_eq!(
        debt.kind,
        DebtKind::Loan {
            payment_cents: 350_00,
            due_day: 15,
        }
    );

    let fetched = store.get(&debt.id).expect("get loan");
    assert_eq!(fetched, debt);
}

#[test]
fn get_unknown_id_is_not_found() {
    let store = store();
    let err = store
        .get(&snowball::debts::DebtId::from_existing("debt_missing"))
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
}

#[test]
fn invalid_loan_create_is_validation_error() {
    let store = store();
    let cases = [
        CreateDebt {
            name: "".into(),
            balance_cents: 100,
            kind: CreateDebtKind::Loan {
                payment_cents: 10,
                due_day: 1,
            },
        },
        CreateDebt {
            name: "Loan".into(),
            balance_cents: -1,
            kind: CreateDebtKind::Loan {
                payment_cents: 10,
                due_day: 1,
            },
        },
        CreateDebt {
            name: "Loan".into(),
            balance_cents: 100,
            kind: CreateDebtKind::Loan {
                payment_cents: 0,
                due_day: 1,
            },
        },
        CreateDebt {
            name: "Loan".into(),
            balance_cents: 100,
            kind: CreateDebtKind::Loan {
                payment_cents: 10,
                due_day: 32,
            },
        },
    ];

    for input in cases {
        let err = store.create(input).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }

    assert_eq!(store.list().expect("list").len(), 0);
}

#[test]
fn create_and_get_credit_card() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Chase".into(),
            balance_cents: 2_500_00,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create card");

    assert_eq!(debt.kind, DebtKind::CreditCard);
    assert_eq!(store.get(&debt.id).expect("get card"), debt);
}

#[test]
fn card_update_rejects_loan_fields() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Chase".into(),
            balance_cents: 100,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create card");

    let err = store
        .update(
            &debt.id,
            UpdateDebt {
                payment_cents: Some(50),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);
}

#[test]
fn duplicate_names_are_rejected() {
    let store = store();
    store
        .create(CreateDebt {
            name: "Shared".into(),
            balance_cents: 100,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("first create");

    let err = store
        .create(CreateDebt {
            name: "Shared".into(),
            balance_cents: 200,
            kind: CreateDebtKind::CreditCard,
        })
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);
}

#[test]
fn list_includes_paid_off_debts_sorted_by_name() {
    let store = store();
    let zebra = store
        .create(CreateDebt {
            name: "Zebra".into(),
            balance_cents: 100,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create zebra");
    let alpha = store
        .create(CreateDebt {
            name: "Alpha".into(),
            balance_cents: 0,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create alpha");

    let debts = store.list().expect("list");
    assert_eq!(debts.len(), 2);
    assert_eq!(debts[0].id, alpha.id);
    assert_eq!(debts[1].id, zebra.id);
}

#[test]
fn partial_loan_update_leaves_other_fields() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Car loan".into(),
            balance_cents: 1_000_00,
            kind: CreateDebtKind::Loan {
                payment_cents: 100_00,
                due_day: 10,
            },
        })
        .expect("create loan");

    let updated = store
        .update(
            &debt.id,
            UpdateDebt {
                payment_cents: Some(125_00),
                ..Default::default()
            },
        )
        .expect("update payment");

    assert_eq!(updated.name, "Car loan");
    assert_eq!(
        updated.kind,
        DebtKind::Loan {
            payment_cents: 125_00,
            due_day: 10,
        }
    );
}

#[test]
fn update_unknown_id_is_not_found() {
    let store = store();
    let err = store
        .update(
            &snowball::debts::DebtId::from_existing("debt_missing"),
            UpdateDebt {
                name: Some("Nope".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
}

#[test]
fn set_balance_can_raise_card_balance() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Chase".into(),
            balance_cents: 500_00,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create card");

    let updated = store.set_balance(&debt.id, 750_00).expect("raise balance");
    assert_eq!(updated.balance_cents, 750_00);
}

#[test]
fn set_balance_rejects_negative_and_missing_id() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Chase".into(),
            balance_cents: 100,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create card");

    let err = store.set_balance(&debt.id, -1).unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);

    let err = store
        .set_balance(&snowball::debts::DebtId::from_existing("debt_missing"), 100)
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
}

#[test]
fn reduce_balance_saturates_at_zero() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Card".into(),
            balance_cents: 400_00,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create card");

    let updated = store
        .reduce_balance(&debt.id, 500_00)
        .expect("reduce balance");
    assert_eq!(updated.balance_cents, 0);
}

#[test]
fn increase_balance_adds_to_balance() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Card".into(),
            balance_cents: 0,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create card");

    let updated = store
        .increase_balance(&debt.id, 400_00)
        .expect("increase balance");
    assert_eq!(updated.balance_cents, 400_00);
}

#[test]
fn increase_balance_rejects_non_positive_and_missing_id() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Card".into(),
            balance_cents: 100,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create card");

    let err = store.increase_balance(&debt.id, 0).unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);

    let err = store
        .increase_balance(&snowball::debts::DebtId::from_existing("debt_missing"), 10)
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
}

#[test]
fn reduce_balance_rejects_non_positive_and_missing_id() {
    let store = store();
    let debt = store
        .create(CreateDebt {
            name: "Card".into(),
            balance_cents: 100,
            kind: CreateDebtKind::CreditCard,
        })
        .expect("create card");

    let err = store.reduce_balance(&debt.id, 0).unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationError);

    let err = store
        .reduce_balance(&snowball::debts::DebtId::from_existing("debt_missing"), 10)
        .unwrap_err();
    assert_eq!(err.code, ErrorCode::NotFound);
}
