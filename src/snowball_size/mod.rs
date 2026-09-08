mod store;
mod validate;

pub use store::SqliteSnowballSizeStore;

use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeId(String);

impl SizeId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SizeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnowballSize {
    pub id: SizeId,
    pub amount_cents: i64,
    pub recorded_at_unix: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSize {
    pub amount_cents: i64,
}

pub trait SnowballSizeStore {
    fn record(&self, input: RecordSize) -> Result<SnowballSize, Error>;
    fn current(&self) -> Result<Option<SnowballSize>, Error>;
    fn list(&self) -> Result<Vec<SnowballSize>, Error>;
}
