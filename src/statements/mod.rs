mod store;
mod validate;

pub use store::SqliteStatementStore;

use crate::debts::DebtId;
use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementId(String);

impl StatementId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[doc(hidden)]
    pub fn from_existing(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for StatementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    pub id: StatementId,
    pub debt_id: DebtId,
    pub minimum_cents: i64,
    pub due_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordStatement {
    pub debt_id: DebtId,
    pub minimum_cents: i64,
    pub due_on: String,
}

pub trait StatementStore {
    fn record(&self, input: RecordStatement) -> Result<Statement, Error>;
    fn get(&self, id: &StatementId) -> Result<Statement, Error>;
    fn list_for_debt(&self, debt_id: &DebtId) -> Result<Vec<Statement>, Error>;
    fn current_for_debt(&self, debt_id: &DebtId) -> Result<Option<Statement>, Error>;
    fn upcoming(&self, as_of: &str) -> Result<Vec<Statement>, Error>;
}
