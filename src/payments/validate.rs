use crate::error::Error;
use crate::payments::{RecordPayment, UpdatePayment};

pub fn validate_record(input: &RecordPayment) -> Result<(), Error> {
    validate_amount(input.amount_cents)?;
    validate_paid_on(&input.paid_on)?;
    Ok(())
}

pub fn validate_update(patch: &UpdatePayment) -> Result<(), Error> {
    if let Some(amount_cents) = patch.amount_cents {
        validate_amount(amount_cents)?;
    }
    if let Some(paid_on) = &patch.paid_on {
        validate_paid_on(paid_on)?;
    }
    Ok(())
}

pub fn validate_amount(amount_cents: i64) -> Result<(), Error> {
    if amount_cents <= 0 {
        return Err(Error::validation("payment amount must be > 0"));
    }
    Ok(())
}

pub fn validate_paid_on(paid_on: &str) -> Result<(), Error> {
    let parts: Vec<&str> = paid_on.split('-').collect();
    if parts.len() != 3 {
        return Err(Error::validation("paid_on must be YYYY-MM-DD"));
    }
    if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(Error::validation("paid_on must be YYYY-MM-DD"));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| Error::validation("paid_on must be YYYY-MM-DD"))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| Error::validation("paid_on must be YYYY-MM-DD"))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| Error::validation("paid_on must be YYYY-MM-DD"))?;

    if !(1..=12).contains(&month) {
        return Err(Error::validation("paid_on must be a valid calendar date"));
    }

    let max_day = days_in_month(year, month);
    if !(1..=max_day).contains(&day) {
        return Err(Error::validation("paid_on must be a valid calendar date"));
    }

    Ok(())
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debts::DebtId;
    use crate::error::ErrorCode;

    #[test]
    fn rejects_invalid_dates() {
        let cases = [
            "2026-13-01",
            "2026-02-31",
            "26-01-01",
            "2026-1-01",
            "not-a-date",
        ];
        for paid_on in cases {
            let err = validate_record(&RecordPayment {
                debt_id: DebtId::from_existing("debt_x"),
                amount_cents: 100,
                paid_on: paid_on.into(),
            })
            .unwrap_err();
            assert_eq!(err.code, ErrorCode::ValidationError);
        }
    }

    #[test]
    fn accepts_valid_date() {
        assert!(validate_paid_on("2026-09-08").is_ok());
        assert!(validate_paid_on("2024-02-29").is_ok());
    }

    #[test]
    fn rejects_non_positive_amount() {
        let err = validate_amount(0).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }
}
