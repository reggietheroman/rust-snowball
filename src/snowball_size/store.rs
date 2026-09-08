use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, Error as SqliteError, params};
use ulid::Generator;

use crate::error::{Error, ErrorCode};
use crate::snowball_size::validate::validate_record;
use crate::snowball_size::{RecordSize, SizeId, SnowballSize, SnowballSizeStore};

const SCHEMA: &str = "
CREATE TABLE snowball_sizes (
    id TEXT PRIMARY KEY NOT NULL,
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    recorded_at_unix INTEGER NOT NULL
);
";

pub struct SqliteSnowballSizeStore {
    conn: Connection,
    id_gen: Mutex<Generator>,
}

impl SqliteSnowballSizeStore {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let conn = Connection::open(path).map_err(map_sqlite_error)?;
        let store = Self {
            conn,
            id_gen: Mutex::new(Generator::default()),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self, Error> {
        let conn = Connection::open_in_memory().map_err(map_sqlite_error)?;
        let store = Self {
            conn,
            id_gen: Mutex::new(Generator::default()),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), Error> {
        self.conn.execute_batch(SCHEMA).map_err(map_sqlite_error)
    }

    fn new_id(&self) -> Result<SizeId, Error> {
        let mut generator = self
            .id_gen
            .lock()
            .map_err(|_| Error::validation("id generator unavailable"))?;
        let ulid = generator
            .generate()
            .map_err(|err| Error::validation(err.to_string()))?;
        Ok(SizeId(format!("size_{ulid}")))
    }

    fn now_unix() -> Result<i64, Error> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
            .map_err(|_| Error::validation("system clock is before unix epoch"))
    }

    fn row_to_size(
        id: String,
        amount_cents: i64,
        recorded_at_unix: i64,
    ) -> Result<SnowballSize, Error> {
        if amount_cents <= 0 {
            return Err(Error::validation("stored amount must be > 0"));
        }
        Ok(SnowballSize {
            id: SizeId(id),
            amount_cents,
            recorded_at_unix,
        })
    }

    fn query_sizes(&self, order: &str) -> Result<Vec<SnowballSize>, Error> {
        let sql = format!(
            "SELECT id, amount_cents, recorded_at_unix FROM snowball_sizes ORDER BY {order}"
        );
        let mut stmt = self.conn.prepare(&sql).map_err(map_sqlite_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })
            .map_err(map_sqlite_error)?;

        rows.map(|row| {
            let (id, amount_cents, recorded_at_unix) = row.map_err(map_sqlite_error)?;
            Self::row_to_size(id, amount_cents, recorded_at_unix)
        })
        .collect()
    }
}

impl SnowballSizeStore for SqliteSnowballSizeStore {
    fn record(&self, input: RecordSize) -> Result<SnowballSize, Error> {
        validate_record(&input)?;

        let id = Self::new_id(self)?;
        let recorded_at_unix = Self::now_unix()?;

        self.conn
            .execute(
                "INSERT INTO snowball_sizes (id, amount_cents, recorded_at_unix)
                 VALUES (?1, ?2, ?3)",
                params![id.as_str(), input.amount_cents, recorded_at_unix],
            )
            .map_err(map_sqlite_error)?;

        Self::row_to_size(id.0, input.amount_cents, recorded_at_unix)
    }

    fn current(&self) -> Result<Option<SnowballSize>, Error> {
        let sizes = self.query_sizes("recorded_at_unix ASC, id ASC")?;
        Ok(sizes.into_iter().last())
    }

    fn list(&self) -> Result<Vec<SnowballSize>, Error> {
        self.query_sizes("recorded_at_unix ASC, id ASC")
    }
}

fn map_sqlite_error(err: SqliteError) -> Error {
    Error {
        code: ErrorCode::ValidationError,
        message: err.to_string(),
    }
}
