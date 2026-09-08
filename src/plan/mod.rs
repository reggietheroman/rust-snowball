mod compute;
mod validate;

pub use compute::compute_plan;
pub use validate::{next_month, previous_month};

use crate::debts::DebtId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub payment_month: String,
    pub snowball_amount_cents: Option<i64>,
    pub shortfall_cents: Option<i64>,
    pub unallocated_cents: i64,
    pub lines: Vec<PlanLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanLine {
    pub debt_id: DebtId,
    pub name: String,
    pub remaining_cents: i64,
    pub required_cents: i64,
    pub extra_cents: i64,
    pub send_cents: i64,
    pub due_on: Option<String>,
    pub missing_statement: bool,
}
