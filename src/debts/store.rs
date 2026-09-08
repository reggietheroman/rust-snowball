use rusqlite::{Error as SqliteError, params};

use crate::db::SqliteDb;
use crate::debts::validate::{
    validate_balance, validate_create, validate_reduce_amount, validate_update,
};
use crate::debts::{CreateDebt, CreateDebtKind, Debt, DebtId, DebtKind, DebtStore, UpdateDebt};
use crate::error::{Error, ErrorCode};

pub struct SqliteDebtStore<'db> {
    db: &'db SqliteDb,
}

impl<'db> SqliteDebtStore<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self { db }
    }

    fn conn(&self) -> &rusqlite::Connection {
        self.db.conn()
    }

    fn new_id() -> DebtId {
        DebtId(format!("debt_{}", ulid::Ulid::new()))
    }

    fn row_to_debt(
        id: String,
        name: String,
        balance_cents: i64,
        kind: String,
        payment_cents: Option<i64>,
        due_day: Option<i64>,
    ) -> Result<Debt, Error> {
        let kind = match kind.as_str() {
            "loan" => {
                let payment_cents = payment_cents
                    .ok_or_else(|| Error::validation("loan row missing payment_cents"))?;
                let due_day =
                    due_day.ok_or_else(|| Error::validation("loan row missing due_day"))?;
                if !(1..=31).contains(&due_day) {
                    return Err(Error::validation("stored due day out of range"));
                }
                DebtKind::Loan {
                    payment_cents,
                    due_day: due_day as u8,
                }
            }
            "credit_card" => {
                if payment_cents.is_some() || due_day.is_some() {
                    return Err(Error::validation(
                        "credit card row must not have loan fields",
                    ));
                }
                DebtKind::CreditCard
            }
            _ => return Err(Error::validation("unknown debt kind in storage")),
        };

        Ok(Debt {
            id: DebtId(id),
            name,
            balance_cents,
            kind,
        })
    }

    fn fetch_one(&self, id: &DebtId) -> Result<Debt, Error> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, name, balance_cents, kind, payment_cents, due_day
                 FROM debts WHERE id = ?1",
            )
            .map_err(map_sqlite_error)?;

        let debt = stmt
            .query_row(params![id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            })
            .map_err(|err| match err {
                SqliteError::QueryReturnedNoRows => {
                    Error::not_found(format!("debt not found: {id}"))
                }
                other => map_sqlite_error(other),
            })?;

        Self::row_to_debt(debt.0, debt.1, debt.2, debt.3, debt.4, debt.5)
    }
}

impl<'db> DebtStore for SqliteDebtStore<'db> {
    fn create(&self, input: CreateDebt) -> Result<Debt, Error> {
        validate_create(&input)?;

        let id = Self::new_id();
        let (kind, payment_cents, due_day) = match input.kind {
            CreateDebtKind::Loan {
                payment_cents,
                due_day,
            } => ("loan", Some(payment_cents), Some(i64::from(due_day))),
            CreateDebtKind::CreditCard => ("credit_card", None, None),
        };

        self.conn()
            .execute(
                "INSERT INTO debts (id, name, balance_cents, kind, payment_cents, due_day)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    id.as_str(),
                    input.name,
                    input.balance_cents,
                    kind,
                    payment_cents,
                    due_day,
                ],
            )
            .map_err(map_unique_violation)?;

        self.fetch_one(&id)
    }

    fn get(&self, id: &DebtId) -> Result<Debt, Error> {
        self.fetch_one(id)
    }

    fn list(&self) -> Result<Vec<Debt>, Error> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, name, balance_cents, kind, payment_cents, due_day
                 FROM debts
                 ORDER BY name ASC",
            )
            .map_err(map_sqlite_error)?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            })
            .map_err(map_sqlite_error)?;

        rows.map(|row| {
            let (id, name, balance_cents, kind, payment_cents, due_day) =
                row.map_err(map_sqlite_error)?;
            Self::row_to_debt(id, name, balance_cents, kind, payment_cents, due_day)
        })
        .collect()
    }

    fn update(&self, id: &DebtId, patch: UpdateDebt) -> Result<Debt, Error> {
        let existing = self.fetch_one(id)?;
        validate_update(&existing.kind, &patch)?;

        let name = patch.name.unwrap_or_else(|| existing.name.clone());

        match &existing.kind {
            DebtKind::Loan {
                payment_cents,
                due_day,
            } => {
                let payment_cents = patch.payment_cents.unwrap_or(*payment_cents);
                let due_day = patch.due_day.unwrap_or(*due_day);

                validate_update(
                    &DebtKind::Loan {
                        payment_cents,
                        due_day,
                    },
                    &UpdateDebt {
                        name: Some(name.clone()),
                        payment_cents: Some(payment_cents),
                        due_day: Some(due_day),
                    },
                )?;

                self.conn()
                    .execute(
                        "UPDATE debts
                         SET name = ?1, payment_cents = ?2, due_day = ?3
                         WHERE id = ?4",
                        params![name, payment_cents, i64::from(due_day), id.as_str(),],
                    )
                    .map_err(map_unique_violation)?;
            }
            DebtKind::CreditCard => {
                self.conn()
                    .execute(
                        "UPDATE debts SET name = ?1 WHERE id = ?2",
                        params![name, id.as_str()],
                    )
                    .map_err(map_unique_violation)?;
            }
        }

        self.fetch_one(id)
    }

    fn set_balance(&self, id: &DebtId, balance_cents: i64) -> Result<Debt, Error> {
        validate_balance(balance_cents)?;

        let updated = self
            .conn()
            .execute(
                "UPDATE debts SET balance_cents = ?1 WHERE id = ?2",
                params![balance_cents, id.as_str()],
            )
            .map_err(map_sqlite_error)?;

        if updated == 0 {
            return Err(Error::not_found(format!("debt not found: {id}")));
        }

        self.fetch_one(id)
    }

    fn reduce_balance(&self, id: &DebtId, cents: i64) -> Result<Debt, Error> {
        validate_reduce_amount(cents)?;
        let existing = self.fetch_one(id)?;
        let new_balance = (existing.balance_cents - cents).max(0);
        self.set_balance(id, new_balance)
    }
}

pub struct OwnedDebtStore {
    db: SqliteDb,
}

impl OwnedDebtStore {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        Ok(Self {
            db: SqliteDb::open(path)?,
        })
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        Ok(Self {
            db: SqliteDb::open_in_memory()?,
        })
    }

    pub fn db(&self) -> &SqliteDb {
        &self.db
    }

    pub fn borrow(&self) -> SqliteDebtStore<'_> {
        SqliteDebtStore::new(&self.db)
    }
}

impl DebtStore for OwnedDebtStore {
    fn create(&self, input: CreateDebt) -> Result<Debt, Error> {
        self.borrow().create(input)
    }

    fn get(&self, id: &DebtId) -> Result<Debt, Error> {
        self.borrow().get(id)
    }

    fn list(&self) -> Result<Vec<Debt>, Error> {
        self.borrow().list()
    }

    fn update(&self, id: &DebtId, patch: UpdateDebt) -> Result<Debt, Error> {
        self.borrow().update(id, patch)
    }

    fn set_balance(&self, id: &DebtId, balance_cents: i64) -> Result<Debt, Error> {
        self.borrow().set_balance(id, balance_cents)
    }

    fn reduce_balance(&self, id: &DebtId, cents: i64) -> Result<Debt, Error> {
        self.borrow().reduce_balance(id, cents)
    }
}

fn map_sqlite_error(err: SqliteError) -> Error {
    Error {
        code: ErrorCode::ValidationError,
        message: err.to_string(),
    }
}

fn map_unique_violation(err: SqliteError) -> Error {
    if err.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
        Error::validation("debt name must be unique")
    } else {
        map_sqlite_error(err)
    }
}
