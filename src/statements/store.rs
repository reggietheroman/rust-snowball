use std::sync::Mutex;

use rusqlite::{Error as SqliteError, params};
use ulid::Generator;

use crate::db::SqliteDb;
use crate::debts::{BorrowedDebtStore, DebtId, DebtKind, DebtStore};
use crate::error::{Error, ErrorCode};
use crate::statements::validate::{validate_due_on, validate_record};
use crate::statements::{RecordStatement, Statement, StatementId, StatementStore};

pub struct SqliteStatementStore<'db> {
    db: &'db SqliteDb,
    id_gen: Mutex<Generator>,
}

impl<'db> SqliteStatementStore<'db> {
    pub fn new(db: &'db SqliteDb) -> Self {
        Self {
            db,
            id_gen: Mutex::new(Generator::default()),
        }
    }

    fn conn(&self) -> &rusqlite::Connection {
        self.db.conn()
    }

    fn new_id(&self) -> Result<StatementId, Error> {
        let mut generator = self
            .id_gen
            .lock()
            .map_err(|_| Error::validation("id generator unavailable"))?;
        let ulid = generator
            .generate()
            .map_err(|err| Error::validation(err.to_string()))?;
        Ok(StatementId(format!("stmt_{ulid}")))
    }

    fn row_to_statement(
        id: String,
        debt_id: String,
        minimum_cents: i64,
        due_on: String,
    ) -> Result<Statement, Error> {
        if minimum_cents < 0 {
            return Err(Error::validation("stored minimum must be >= 0"));
        }
        Ok(Statement {
            id: StatementId(id),
            debt_id: DebtId::from_existing(debt_id),
            minimum_cents,
            due_on,
        })
    }

    fn fetch_one(&self, id: &StatementId) -> Result<Statement, Error> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, debt_id, minimum_cents, due_on
                 FROM statements WHERE id = ?1",
            )
            .map_err(map_sqlite_error)?;

        let row = stmt
            .query_row(params![id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|err| match err {
                SqliteError::QueryReturnedNoRows => {
                    Error::not_found(format!("statement not found: {id}"))
                }
                other => map_sqlite_error(other),
            })?;

        Self::row_to_statement(row.0, row.1, row.2, row.3)
    }

    fn validate_debt_is_card(&self, debt_id: &DebtId) -> Result<(), Error> {
        let debt = BorrowedDebtStore::new(self.db).get(debt_id)?;
        match debt.kind {
            DebtKind::CreditCard => Ok(()),
            DebtKind::Loan { .. } => Err(Error::validation(
                "statements can only be recorded for credit cards",
            )),
        }
    }
}

impl<'db> StatementStore for SqliteStatementStore<'db> {
    fn record(&self, input: RecordStatement) -> Result<Statement, Error> {
        validate_record(&input)?;
        self.validate_debt_is_card(&input.debt_id)?;

        let id = self.new_id()?;
        self.conn()
            .execute(
                "INSERT INTO statements (id, debt_id, minimum_cents, due_on)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(debt_id, due_on) DO UPDATE SET
                     minimum_cents = excluded.minimum_cents",
                params![
                    id.as_str(),
                    input.debt_id.as_str(),
                    input.minimum_cents,
                    input.due_on,
                ],
            )
            .map_err(map_sqlite_error)?;

        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, debt_id, minimum_cents, due_on
                 FROM statements
                 WHERE debt_id = ?1 AND due_on = ?2",
            )
            .map_err(map_sqlite_error)?;

        let row = stmt
            .query_row(params![input.debt_id.as_str(), input.due_on], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(map_sqlite_error)?;

        Self::row_to_statement(row.0, row.1, row.2, row.3)
    }

    fn get(&self, id: &StatementId) -> Result<Statement, Error> {
        self.fetch_one(id)
    }

    fn list_for_debt(&self, debt_id: &DebtId) -> Result<Vec<Statement>, Error> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, debt_id, minimum_cents, due_on
                 FROM statements
                 WHERE debt_id = ?1
                 ORDER BY due_on ASC, id ASC",
            )
            .map_err(map_sqlite_error)?;

        let rows = stmt
            .query_map(params![debt_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(map_sqlite_error)?;

        rows.map(|row| {
            let (id, debt_id, minimum_cents, due_on) = row.map_err(map_sqlite_error)?;
            Self::row_to_statement(id, debt_id, minimum_cents, due_on)
        })
        .collect()
    }

    fn current_for_debt(&self, debt_id: &DebtId) -> Result<Option<Statement>, Error> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, debt_id, minimum_cents, due_on
                 FROM statements
                 WHERE debt_id = ?1
                 ORDER BY due_on DESC, id DESC
                 LIMIT 1",
            )
            .map_err(map_sqlite_error)?;

        let row = stmt.query_row(params![debt_id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        });

        match row {
            Ok((id, debt_id, minimum_cents, due_on)) => Ok(Some(Self::row_to_statement(
                id,
                debt_id,
                minimum_cents,
                due_on,
            )?)),
            Err(SqliteError::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(map_sqlite_error(err)),
        }
    }

    fn upcoming(&self, as_of: &str) -> Result<Vec<Statement>, Error> {
        validate_due_on(as_of)?;

        let mut stmt = self
            .conn()
            .prepare(
                "SELECT id, debt_id, minimum_cents, due_on
                 FROM statements
                 WHERE due_on >= ?1
                 ORDER BY due_on ASC, id ASC",
            )
            .map_err(map_sqlite_error)?;

        let rows = stmt
            .query_map(params![as_of], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(map_sqlite_error)?;

        rows.map(|row| {
            let (id, debt_id, minimum_cents, due_on) = row.map_err(map_sqlite_error)?;
            Self::row_to_statement(id, debt_id, minimum_cents, due_on)
        })
        .collect()
    }
}

fn map_sqlite_error(err: SqliteError) -> Error {
    Error {
        code: ErrorCode::ValidationError,
        message: err.to_string(),
    }
}
