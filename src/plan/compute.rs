use std::cmp::Ordering;

use crate::debts::{Debt, DebtKind, DebtStore};
use crate::error::Error;
use crate::plan::validate::{loan_due_on, previous_month, validate_payment_month};
use crate::plan::{Plan, PlanLine};
use crate::snowball_size::SnowballSizeStore;
use crate::statements::{Statement, StatementStore};

struct DraftLine {
    debt: Debt,
    raw_floor_cents: i64,
    required_cents: i64,
    extra_cents: i64,
    due_on: Option<String>,
    missing_statement: bool,
}

pub fn compute_plan(
    payment_month: &str,
    debts: &impl DebtStore,
    sizes: &impl SnowballSizeStore,
    statements: &impl StatementStore,
) -> Result<Plan, Error> {
    validate_payment_month(payment_month)?;
    let statement_month = previous_month(payment_month)?;
    let snowball_amount_cents = sizes.current()?.map(|size| size.amount_cents);

    let statements_for_cycle = statements.list_for_month(&statement_month)?;

    let mut drafts: Vec<DraftLine> = debts
        .list()?
        .into_iter()
        .filter(|debt| debt.balance_cents > 0)
        .map(|debt| build_draft_line(&debt, payment_month, &statements_for_cycle))
        .collect::<Result<Vec<_>, Error>>()?;

    let required_total: i64 = drafts.iter().map(|line| line.required_cents).sum();
    let freed_total: i64 = drafts
        .iter()
        .map(|line| line.raw_floor_cents - line.required_cents)
        .sum();
    let (shortfall_cents, unallocated_cents) = match snowball_amount_cents {
        None => {
            for draft in &mut drafts {
                draft.extra_cents = 0;
            }
            (None, 0)
        }
        Some(snowball) if snowball < required_total => {
            for draft in &mut drafts {
                draft.extra_cents = 0;
            }
            (Some(required_total - snowball), 0)
        }
        Some(snowball) => {
            let mut pool = snowball - required_total + freed_total;
            let mut allocation_order: Vec<usize> = (0..drafts.len()).collect();
            allocation_order.sort_by(|&left, &right| {
                let left_line = &drafts[left];
                let right_line = &drafts[right];
                left_line
                    .debt
                    .balance_cents
                    .cmp(&right_line.debt.balance_cents)
                    .then_with(|| left_line.debt.name.cmp(&right_line.debt.name))
            });

            for index in allocation_order {
                let draft = &mut drafts[index];
                let room = draft.debt.balance_cents - draft.required_cents;
                let extra = pool.min(room);
                draft.extra_cents = extra;
                pool -= extra;
            }

            (Some(0), pool)
        }
    };

    let mut lines: Vec<PlanLine> = drafts
        .into_iter()
        .map(|draft| PlanLine {
            debt_id: draft.debt.id,
            name: draft.debt.name,
            remaining_cents: draft.debt.balance_cents,
            required_cents: draft.required_cents,
            extra_cents: draft.extra_cents,
            send_cents: draft.required_cents + draft.extra_cents,
            due_on: draft.due_on,
            missing_statement: draft.missing_statement,
        })
        .collect();

    lines.sort_by(compare_lines);

    Ok(Plan {
        payment_month: payment_month.into(),
        snowball_amount_cents,
        shortfall_cents,
        unallocated_cents,
        lines,
    })
}

fn build_draft_line(
    debt: &Debt,
    payment_month: &str,
    statements_for_cycle: &[Statement],
) -> Result<DraftLine, Error> {
    match &debt.kind {
        DebtKind::Loan {
            payment_cents,
            due_day,
        } => {
            let required_cents = (*payment_cents).min(debt.balance_cents);
            Ok(DraftLine {
                debt: debt.clone(),
                raw_floor_cents: *payment_cents,
                required_cents,
                extra_cents: 0,
                due_on: Some(loan_due_on(payment_month, *due_day)?),
                missing_statement: false,
            })
        }
        DebtKind::CreditCard => {
            match statements_for_cycle
                .iter()
                .find(|statement| statement.debt_id == debt.id)
            {
                Some(statement) => {
                    let required_cents = statement.minimum_cents.min(debt.balance_cents);
                    Ok(DraftLine {
                        debt: debt.clone(),
                        raw_floor_cents: statement.minimum_cents,
                        required_cents,
                        extra_cents: 0,
                        due_on: Some(statement.due_on.clone()),
                        missing_statement: false,
                    })
                }
                None => Ok(DraftLine {
                    debt: debt.clone(),
                    raw_floor_cents: 0,
                    required_cents: 0,
                    extra_cents: 0,
                    due_on: None,
                    missing_statement: true,
                }),
            }
        }
    }
}

fn compare_lines(left: &PlanLine, right: &PlanLine) -> Ordering {
    match (&left.due_on, &right.due_on) {
        (Some(left_due), Some(right_due)) => left_due
            .cmp(right_due)
            .then_with(|| left.name.cmp(&right.name)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.name.cmp(&right.name),
    }
}
