use crate::debts::{CreateDebt, CreateDebtKind, UpdateDebt};
use crate::error::Error;

pub fn validate_create(input: &CreateDebt) -> Result<(), Error> {
    validate_name(&input.name)?;
    validate_balance(input.balance_cents)?;

    match &input.kind {
        CreateDebtKind::Loan {
            payment_cents,
            due_day,
        } => {
            validate_loan_payment(*payment_cents)?;
            validate_due_day(*due_day)?;
        }
        CreateDebtKind::CreditCard => {}
    }

    Ok(())
}

pub fn validate_update(
    existing_kind: &crate::debts::DebtKind,
    patch: &UpdateDebt,
) -> Result<(), Error> {
    if let Some(name) = &patch.name {
        validate_name(name)?;
    }

    match existing_kind {
        crate::debts::DebtKind::Loan { .. } => {
            if let Some(payment_cents) = patch.payment_cents {
                validate_loan_payment(payment_cents)?;
            }
            if let Some(due_day) = patch.due_day {
                validate_due_day(due_day)?;
            }
        }
        crate::debts::DebtKind::CreditCard => {
            if patch.payment_cents.is_some() || patch.due_day.is_some() {
                return Err(Error::validation(
                    "credit cards cannot have payment or due day",
                ));
            }
        }
    }

    Ok(())
}

pub fn validate_balance(balance_cents: i64) -> Result<(), Error> {
    if balance_cents < 0 {
        return Err(Error::validation("balance must be >= 0"));
    }
    Ok(())
}

pub fn validate_reduce_amount(cents: i64) -> Result<(), Error> {
    if cents <= 0 {
        return Err(Error::validation("reduction amount must be > 0"));
    }
    Ok(())
}

pub fn validate_increase_amount(cents: i64) -> Result<(), Error> {
    if cents <= 0 {
        return Err(Error::validation("increase amount must be > 0"));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), Error> {
    if name.trim().is_empty() {
        return Err(Error::validation("name must not be empty"));
    }
    Ok(())
}

fn validate_loan_payment(payment_cents: i64) -> Result<(), Error> {
    if payment_cents <= 0 {
        return Err(Error::validation("loan payment must be > 0"));
    }
    Ok(())
}

fn validate_due_day(due_day: u8) -> Result<(), Error> {
    if !(1..=31).contains(&due_day) {
        return Err(Error::validation("due day must be between 1 and 31"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debts::{CreateDebt, CreateDebtKind, DebtKind, UpdateDebt};

    #[test]
    fn rejects_empty_name() {
        let input = CreateDebt {
            name: "   ".into(),
            balance_cents: 0,
            kind: CreateDebtKind::CreditCard,
        };
        assert_eq!(
            validate_create(&input).unwrap_err().code,
            crate::error::ErrorCode::ValidationError
        );
    }

    #[test]
    fn rejects_negative_balance() {
        let input = CreateDebt {
            name: "Card".into(),
            balance_cents: -1,
            kind: CreateDebtKind::CreditCard,
        };
        assert!(validate_create(&input).is_err());
    }

    #[test]
    fn rejects_loan_payment_on_card_update() {
        let patch = UpdateDebt {
            payment_cents: Some(100),
            ..Default::default()
        };
        assert!(validate_update(&DebtKind::CreditCard, &patch).is_err());
    }

    #[test]
    fn rejects_non_positive_reduce_amount() {
        assert!(validate_reduce_amount(0).is_err());
        assert!(validate_reduce_amount(-1).is_err());
    }

    #[test]
    fn rejects_non_positive_increase_amount() {
        assert!(validate_increase_amount(0).is_err());
        assert!(validate_increase_amount(-1).is_err());
    }
}
