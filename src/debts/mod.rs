mod store;
mod validate;

pub use store::{OwnedDebtStore as SqliteDebtStore, SqliteDebtStore as BorrowedDebtStore};

use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebtId(String);

impl DebtId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[doc(hidden)]
    pub fn from_existing(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for DebtId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebtKind {
    Loan { payment_cents: i64, due_day: u8 },
    CreditCard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Debt {
    pub id: DebtId,
    pub name: String,
    pub balance_cents: i64,
    pub kind: DebtKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateDebtKind {
    Loan { payment_cents: i64, due_day: u8 },
    CreditCard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateDebt {
    pub name: String,
    pub balance_cents: i64,
    pub kind: CreateDebtKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateDebt {
    pub name: Option<String>,
    pub payment_cents: Option<i64>,
    pub due_day: Option<u8>,
}

pub trait DebtStore {
    fn create(&self, input: CreateDebt) -> Result<Debt, Error>;
    fn get(&self, id: &DebtId) -> Result<Debt, Error>;
    fn list(&self) -> Result<Vec<Debt>, Error>;
    fn update(&self, id: &DebtId, patch: UpdateDebt) -> Result<Debt, Error>;
    fn set_balance(&self, id: &DebtId, balance_cents: i64) -> Result<Debt, Error>;
    fn reduce_balance(&self, id: &DebtId, cents: i64) -> Result<Debt, Error>;
}
