use rusqlite::{Connection, Error as SqliteError};

use crate::error::{Error, ErrorCode};

const DEBTS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS debts (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    balance_cents INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('loan', 'credit_card')),
    payment_cents INTEGER,
    due_day INTEGER
);
";

const STATEMENTS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS statements (
    id TEXT PRIMARY KEY NOT NULL,
    debt_id TEXT NOT NULL REFERENCES debts(id),
    minimum_cents INTEGER NOT NULL CHECK (minimum_cents >= 0),
    due_on TEXT NOT NULL,
    UNIQUE (debt_id, due_on)
);
";

const PAYMENTS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS payments (
    id TEXT PRIMARY KEY NOT NULL,
    debt_id TEXT NOT NULL REFERENCES debts(id),
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    applied_cents INTEGER NOT NULL CHECK (applied_cents >= 0),
    paid_on TEXT NOT NULL
);
";

pub struct SqliteDb {
    conn: Connection,
}

impl SqliteDb {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let conn = Connection::open(path).map_err(map_sqlite_error)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        let conn = Connection::open_in_memory().map_err(map_sqlite_error)?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    fn migrate(&self) -> Result<(), Error> {
        self.conn
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(map_sqlite_error)?;
        self.conn
            .execute_batch(DEBTS_SCHEMA)
            .map_err(map_sqlite_error)?;
        self.conn
            .execute_batch(STATEMENTS_SCHEMA)
            .map_err(map_sqlite_error)?;
        self.conn
            .execute_batch(PAYMENTS_SCHEMA)
            .map_err(map_sqlite_error)?;
        Ok(())
    }
}

fn map_sqlite_error(err: SqliteError) -> Error {
    Error {
        code: ErrorCode::ValidationError,
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_creates_debts_statements_and_payments_tables() {
        let db = SqliteDb::open_in_memory().expect("open db");
        let conn = db.conn();

        for table in ["debts", "statements", "payments"] {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("table count");
            assert_eq!(count, 1, "missing table {table}");
        }

        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("foreign_keys");
        assert_eq!(fk, 1);
    }
}
