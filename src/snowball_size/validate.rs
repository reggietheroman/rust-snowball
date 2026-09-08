use crate::error::Error;
use crate::snowball_size::RecordSize;

pub fn validate_record(input: &RecordSize) -> Result<(), Error> {
    if input.amount_cents <= 0 {
        return Err(Error::validation("amount must be > 0"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    #[test]
    fn rejects_non_positive_amount() {
        for amount in [0, -1] {
            let err = validate_record(&RecordSize {
                amount_cents: amount,
            })
            .unwrap_err();
            assert_eq!(err.code, ErrorCode::ValidationError);
        }
    }

    #[test]
    fn accepts_positive_amount() {
        assert!(validate_record(&RecordSize { amount_cents: 1 }).is_ok());
    }
}
