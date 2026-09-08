use snowball::ErrorCode;
use snowball::db::SqliteDb;
use snowball::snowball_size::{RecordSize, SnowballSizeStore, SqliteSnowballSizeStore};

struct Harness {
    db: SqliteDb,
}

impl Harness {
    fn open() -> Self {
        Self {
            db: SqliteDb::open_in_memory().expect("open db"),
        }
    }

    fn sizes(&self) -> SqliteSnowballSizeStore<'_> {
        SqliteSnowballSizeStore::new(&self.db)
    }
}

#[test]
fn empty_ledger_has_no_current() {
    let h = Harness::open();
    let store = h.sizes();
    assert_eq!(store.current().expect("current"), None);
    assert!(store.list().expect("list").is_empty());
}

#[test]
fn record_persists_and_current_returns_amount() {
    let h = Harness::open();
    let store = h.sizes();
    let recorded = store
        .record(RecordSize {
            amount_cents: 400_00,
        })
        .expect("record");

    assert!(recorded.id.as_str().starts_with("size_"));
    assert!(recorded.recorded_at_unix > 0);
    assert_eq!(recorded.amount_cents, 400_00);

    let current = store.current().expect("current").expect("some current");
    assert_eq!(current.amount_cents, 400_00);
    assert_eq!(current.id, recorded.id);
}

#[test]
fn invalid_amount_is_validation_error_and_inserts_nothing() {
    let h = Harness::open();
    let store = h.sizes();
    for amount in [0, -100] {
        let err = store
            .record(RecordSize {
                amount_cents: amount,
            })
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }
    assert_eq!(store.current().expect("current"), None);
    assert!(store.list().expect("list").is_empty());
}

#[test]
fn list_is_oldest_first_and_current_is_latest() {
    let h = Harness::open();
    let store = h.sizes();
    let first = store
        .record(RecordSize {
            amount_cents: 400_00,
        })
        .expect("first");
    let second = store
        .record(RecordSize {
            amount_cents: 500_00,
        })
        .expect("second");

    let list = store.list().expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].amount_cents, 400_00);
    assert_eq!(list[1].amount_cents, 500_00);
    assert_eq!(list[0].id, first.id);
    assert_eq!(list[1].id, second.id);

    let current = store.current().expect("current").expect("some");
    assert_eq!(current.id, second.id);
}

#[test]
fn shrink_appends_row_and_keeps_history() {
    let h = Harness::open();
    let store = h.sizes();
    store
        .record(RecordSize {
            amount_cents: 500_00,
        })
        .expect("record 500");
    let shrunk = store
        .record(RecordSize {
            amount_cents: 300_00,
        })
        .expect("record 300");

    let list = store.list().expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].amount_cents, 500_00);
    assert_eq!(list[1].amount_cents, 300_00);

    let current = store.current().expect("current").expect("some");
    assert_eq!(current.id, shrunk.id);
    assert_eq!(current.amount_cents, 300_00);
}

#[test]
fn reaffirm_same_amount_creates_two_rows() {
    let h = Harness::open();
    let store = h.sizes();
    let first = store
        .record(RecordSize {
            amount_cents: 400_00,
        })
        .expect("first");
    let second = store
        .record(RecordSize {
            amount_cents: 400_00,
        })
        .expect("second");

    assert_ne!(first.id, second.id);

    let list = store.list().expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].amount_cents, 400_00);
    assert_eq!(list[1].amount_cents, 400_00);

    let current = store.current().expect("current").expect("some");
    assert_eq!(current.id, second.id);
}
