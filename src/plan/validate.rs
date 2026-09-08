use crate::error::Error;

pub fn validate_payment_month(payment_month: &str) -> Result<(i32, u32), Error> {
    let parts: Vec<&str> = payment_month.split('-').collect();
    if parts.len() != 2 {
        return Err(Error::validation("payment_month must be YYYY-MM"));
    }
    if parts[0].len() != 4 || parts[1].len() != 2 {
        return Err(Error::validation("payment_month must be YYYY-MM"));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| Error::validation("payment_month must be YYYY-MM"))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| Error::validation("payment_month must be YYYY-MM"))?;

    if !(1..=12).contains(&month) {
        return Err(Error::validation("payment_month must be YYYY-MM"));
    }

    Ok((year, month))
}

pub fn previous_month(payment_month: &str) -> Result<String, Error> {
    let (year, month) = validate_payment_month(payment_month)?;
    if month == 1 {
        Ok(format!("{}-12", year - 1))
    } else {
        Ok(format!("{year}-{:02}", month - 1))
    }
}

pub fn loan_due_on(payment_month: &str, due_day: u8) -> Result<String, Error> {
    let (year, month) = validate_payment_month(payment_month)?;
    let max_day = days_in_month(year, month);
    let day = u32::from(due_day).min(max_day);
    Ok(format!("{year}-{month:02}-{day:02}"))
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

    #[test]
    fn rejects_invalid_payment_month() {
        for payment_month in ["2026-13", "2026-9", "2026-09-01"] {
            assert!(validate_payment_month(payment_month).is_err());
        }
    }

    #[test]
    fn previous_month_of_september_is_august() {
        assert_eq!(previous_month("2026-09").expect("prev"), "2026-08");
    }

    #[test]
    fn previous_month_of_january_is_previous_december() {
        assert_eq!(previous_month("2026-01").expect("prev"), "2025-12");
    }

    #[test]
    fn loan_due_day_clamps_to_last_day_of_month() {
        assert_eq!(loan_due_on("2026-09", 31).expect("due"), "2026-09-30");
        assert_eq!(loan_due_on("2026-02", 31).expect("due"), "2026-02-28");
    }
}
