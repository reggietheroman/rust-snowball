use std::sync::Mutex;

use rusqlite::{Error as SqliteError, params};
use ulid::Generator;

use crate::db::SqliteDb;
use crate::debts::{BorrowedDebtStore, DebtId, DebtStore};
use crate::error::{Error, ErrorCode};
use crate::payments::validate::{validate_record, validate_update};
use crate::payments::{Payment, PaymentId, PaymentStore, RecordPayment, UpdatePayment};

pub struct SqlitePaymentStore<'db> {
    db: &'db SqliteDb,
    id_gen: Mutex<Generator>,
}

impl<'db> SqlitePaymentStore<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self {
            db,
            id_gen: Mutex::new(Generator::default()),
        }
    }

    fn conn(&self) -> &rusqlite::Connection {
        self.db.conn()
    }

    fn debts(&self) -> BorrowedDebtStore<'db> {
        BorrowedDebtStore::new(self.db)
    }

    fn new_id(&self) -> Result<PaymentId, Error> {
        let mut generator = self
            .id_gen
            .lock()
            .map_err(|_| Error::validation("id generator unavailable"))?;
        let ulid = generator
            .generate()
            .map_err(|err| Error::validation(err.to_string()))?;
        Ok(PaymentId(format!("pay_{ulid}")))
    }

    fn row_to_payment(
        id: String,
        debt_id: String,
        amount_cents: i64,
        applied_cents: i64,
        paid_on: String,
    ) -> Result<Payment, Error> {
        if amount_cents <= 0 {
            return Err(Error::validation("stored payment amount must be > 0"));
        }
        if applied_cents < 0 {
            return Err(Error::validation("stored applied amount must be >= 0"));
        }
        Ok(Payment {
            id: PaymentId(id),
            debt_id: DebtId::from_existing(debt_id),
            amount_cents,
            applied_cents,
            paid_on,
        })
    }

    fn fetch_one(&self, id: &PaymentId) -> Result<Payment, Error> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, debt_id, amount_cents, applied_cents, paid_on
                 FROM payments WHERE id = ?1",
            )
            .map_err(map_sqlite_error)?;

        let row = stmt
            .query_row(params![id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|err| match err {
                SqliteError::QueryReturnedNoRows => {
                    Error::not_found(format!("payment not found: {id}"))
                }
                other => map_sqlite_error(other),
            })?;

        Self::row_to_payment(row.0, row.1, row.2, row.3, row.4)
    }

    fn apply_to_debt(&self, debt_id: &DebtId, amount_cents: i64) -> Result<i64, Error> {
        let debt = self.debts().get(debt_id)?;
        let applied_cents = amount_cents.min(debt.balance_cents);
        if applied_cents > 0 {
            self.debts().reduce_balance(debt_id, applied_cents)?;
        }
        Ok(applied_cents)
    }

    fn restore_to_debt(&self, debt_id: &DebtId, applied_cents: i64) -> Result<(), Error> {
        if applied_cents > 0 {
            self.debts().increase_balance(debt_id, applied_cents)?;
        }
        Ok(())
    }
}

impl<'db> PaymentStore for SqlitePaymentStore<'db> {
    fn record(&self, input: RecordPayment) -> Result<Payment, Error> {
        validate_record(&input)?;
        self.debts().get(&input.debt_id)?;

        let txn = self
            .conn()
            .unchecked_transaction()
            .map_err(map_sqlite_error)?;

        let applied_cents = self.apply_to_debt(&input.debt_id, input.amount_cents)?;
        let id = self.new_id()?;

        txn.execute(
            "INSERT INTO payments (id, debt_id, amount_cents, applied_cents, paid_on)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id.as_str(),
                input.debt_id.as_str(),
                input.amount_cents,
                applied_cents,
                input.paid_on,
            ],
        )
        .map_err(map_sqlite_error)?;

        txn.commit().map_err(map_sqlite_error)?;

        self.fetch_one(&id)
    }

    fn get(&self, id: &PaymentId) -> Result<Payment, Error> {
        self.fetch_one(id)
    }

    fn list_for_debt(&self, debt_id: &DebtId) -> Result<Vec<Payment>, Error> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, debt_id, amount_cents, applied_cents, paid_on
                 FROM payments
                 WHERE debt_id = ?1
                 ORDER BY paid_on ASC, id ASC",
            )
            .map_err(map_sqlite_error)?;

        let rows = stmt
            .query_map(params![debt_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(map_sqlite_error)?;

        rows.map(|row| {
            let (id, debt_id, amount_cents, applied_cents, paid_on) =
                row.map_err(map_sqlite_error)?;
            Self::row_to_payment(id, debt_id, amount_cents, applied_cents, paid_on)
        })
        .collect()
    }

    fn update(&self, id: &PaymentId, patch: UpdatePayment) -> Result<Payment, Error> {
        validate_update(&patch)?;

        let existing = self.fetch_one(id)?;
        let new_debt_id = patch
            .debt_id
            .clone()
            .unwrap_or_else(|| existing.debt_id.clone());
        let new_amount = patch.amount_cents.unwrap_or(existing.amount_cents);
        let new_paid_on = patch
            .paid_on
            .clone()
            .unwrap_or_else(|| existing.paid_on.clone());

        let balance_fields_changed =
            new_debt_id != existing.debt_id || new_amount != existing.amount_cents;

        if balance_fields_changed {
            validate_record(&RecordPayment {
                debt_id: new_debt_id.clone(),
                amount_cents: new_amount,
                paid_on: new_paid_on.clone(),
            })?;
            self.debts().get(&new_debt_id)?;

            let txn = self
                .conn()
                .unchecked_transaction()
                .map_err(map_sqlite_error)?;

            self.restore_to_debt(&existing.debt_id, existing.applied_cents)?;
            let applied_cents = self.apply_to_debt(&new_debt_id, new_amount)?;

            txn.execute(
                "UPDATE payments
                 SET debt_id = ?1, amount_cents = ?2, applied_cents = ?3, paid_on = ?4
                 WHERE id = ?5",
                params![
                    new_debt_id.as_str(),
                    new_amount,
                    applied_cents,
                    new_paid_on,
                    id.as_str(),
                ],
            )
            .map_err(map_sqlite_error)?;

            txn.commit().map_err(map_sqlite_error)?;
        } else if patch.paid_on.is_some() {
            let updated = self
                .conn()
                .execute(
                    "UPDATE payments SET paid_on = ?1 WHERE id = ?2",
                    params![new_paid_on, id.as_str()],
                )
                .map_err(map_sqlite_error)?;

            if updated == 0 {
                return Err(Error::not_found(format!("payment not found: {id}")));
            }
        }

        self.fetch_one(id)
    }
}

fn map_sqlite_error(err: SqliteError) -> Error {
    Error {
        code: ErrorCode::ValidationError,
        message: err.to_string(),
    }
}
