use crate::error::Error;

pub fn format_pesos(cents: i64) -> String {
    let negative = cents < 0;
    let abs = cents.unsigned_abs();
    let pesos = abs / 100;
    let centavos = abs % 100;
    let body = format!("₱{}.{:02}", format_integer_with_commas(pesos), centavos);
    if negative { format!("-{body}") } else { body }
}

pub fn parse_pesos(input: &str) -> Result<i64, Error> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(Error::validation("amount is required"));
    }

    let stripped = trimmed
        .strip_prefix('₱')
        .or_else(|| trimmed.strip_prefix("PHP"))
        .unwrap_or(trimmed)
        .replace(',', "");

    let (whole, fraction) = match stripped.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (stripped.as_str(), ""),
    };

    if whole.is_empty() && fraction.is_empty() {
        return Err(Error::validation("amount is required"));
    }
    if !whole.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
    {
        return Err(Error::validation("amount must be a number"));
    }
    if fraction.len() > 2 {
        return Err(Error::validation("amount has too many decimal places"));
    }

    let pesos: i64 = if whole.is_empty() {
        0
    } else {
        whole
            .parse()
            .map_err(|_| Error::validation("amount must be a number"))?
    };
    let centavos: i64 = if fraction.is_empty() {
        0
    } else {
        let padded = format!("{:0<2}", fraction);
        padded
            .parse()
            .map_err(|_| Error::validation("amount must be a number"))?
    };

    Ok(pesos * 100 + centavos)
}

fn format_integer_with_commas(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_pesos_shows_symbol_and_decimals() {
        assert_eq!(format_pesos(123_456), "₱1,234.56");
        assert_eq!(format_pesos(0), "₱0.00");
    }

    #[test]
    fn parse_pesos_accepts_common_shapes() {
        assert_eq!(parse_pesos("1,234.56").expect("parse"), 123_456);
        assert_eq!(parse_pesos("₱1,234.56").expect("parse"), 123_456);
        assert_eq!(parse_pesos("400").expect("parse"), 40_000);
    }

    #[test]
    fn parse_pesos_rejects_too_many_decimals() {
        assert!(parse_pesos("1.234").is_err());
    }
}
