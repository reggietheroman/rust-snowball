use crate::error::Error;
use crate::statements::RecordStatement;

pub fn validate_record(input: &RecordStatement) -> Result<(), Error> {
    validate_minimum(input.minimum_cents)?;
    validate_statement_month(&input.statement_month)?;
    validate_due_on(&input.due_on)?;
    Ok(())
}

pub fn validate_minimum(minimum_cents: i64) -> Result<(), Error> {
    if minimum_cents < 0 {
        return Err(Error::validation("minimum must be >= 0"));
    }
    Ok(())
}

pub fn validate_statement_month(statement_month: &str) -> Result<(), Error> {
    let parts: Vec<&str> = statement_month.split('-').collect();
    if parts.len() != 2 {
        return Err(Error::validation("statement_month must be YYYY-MM"));
    }
    if parts[0].len() != 4 || parts[1].len() != 2 {
        return Err(Error::validation("statement_month must be YYYY-MM"));
    }

    let _year: i32 = parts[0]
        .parse()
        .map_err(|_| Error::validation("statement_month must be YYYY-MM"))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| Error::validation("statement_month must be YYYY-MM"))?;

    if !(1..=12).contains(&month) {
        return Err(Error::validation("statement_month must be YYYY-MM"));
    }

    Ok(())
}

pub fn validate_due_on(due_on: &str) -> Result<(), Error> {
    let parts: Vec<&str> = due_on.split('-').collect();
    if parts.len() != 3 {
        return Err(Error::validation("due_on must be YYYY-MM-DD"));
    }
    if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(Error::validation("due_on must be YYYY-MM-DD"));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| Error::validation("due_on must be YYYY-MM-DD"))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| Error::validation("due_on must be YYYY-MM-DD"))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| Error::validation("due_on must be YYYY-MM-DD"))?;

    if !(1..=12).contains(&month) {
        return Err(Error::validation("due_on must be a valid calendar date"));
    }

    let max_day = days_in_month(year, month);
    if !(1..=max_day).contains(&day) {
        return Err(Error::validation("due_on must be a valid calendar date"));
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

    fn record(
        debt_id: &str,
        statement_month: &str,
        minimum_cents: i64,
        due_on: &str,
    ) -> RecordStatement {
        RecordStatement {
            debt_id: DebtId::from_existing(debt_id),
            statement_month: statement_month.into(),
            minimum_cents,
            due_on: due_on.into(),
        }
    }

    #[test]
    fn rejects_invalid_dates() {
        let cases = [
            "2026-13-01",
            "2026-02-31",
            "26-01-01",
            "2026-1-01",
            "not-a-date",
        ];
        for due_on in cases {
            let err = validate_record(&record("debt_x", "2026-08", 100, due_on)).unwrap_err();
            assert_eq!(err.code, ErrorCode::ValidationError);
        }
    }

    #[test]
    fn rejects_invalid_statement_month() {
        for statement_month in ["2026-13", "2026-9", "2026-08-01", "26-08", "not-a-month"] {
            let err =
                validate_record(&record("debt_x", statement_month, 100, "2026-09-15")).unwrap_err();
            assert_eq!(err.code, ErrorCode::ValidationError);
        }
    }

    #[test]
    fn accepts_valid_date() {
        assert!(
            validate_due_on("2026-09-08").is_ok(),
            "valid date should pass"
        );
        assert!(
            validate_due_on("2024-02-29").is_ok(),
            "leap day should pass"
        );
    }

    #[test]
    fn accepts_valid_statement_month() {
        assert!(validate_statement_month("2026-08").is_ok());
        assert!(validate_statement_month("2026-01").is_ok());
    }

    #[test]
    fn rejects_negative_minimum() {
        let err = validate_minimum(-1).unwrap_err();
        assert_eq!(err.code, ErrorCode::ValidationError);
    }
}
