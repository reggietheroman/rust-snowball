mod store;
mod validate;

pub use store::SqlitePaymentStore;

use crate::debts::DebtId;
use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentId(String);

impl PaymentId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[doc(hidden)]
    pub fn from_existing(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for PaymentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payment {
    pub id: PaymentId,
    pub debt_id: DebtId,
    pub amount_cents: i64,
    pub applied_cents: i64,
    pub paid_on: String,
}

impl Payment {
    pub fn is_overpayment(&self) -> bool {
        self.amount_cents > self.applied_cents
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPayment {
    pub debt_id: DebtId,
    pub amount_cents: i64,
    pub paid_on: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdatePayment {
    pub debt_id: Option<DebtId>,
    pub amount_cents: Option<i64>,
    pub paid_on: Option<String>,
}

pub trait PaymentStore {
    fn record(&self, input: RecordPayment) -> Result<Payment, Error>;
    fn get(&self, id: &PaymentId) -> Result<Payment, Error>;
    fn list_for_debt(&self, debt_id: &DebtId) -> Result<Vec<Payment>, Error>;
    fn update(&self, id: &PaymentId, patch: UpdatePayment) -> Result<Payment, Error>;
}
