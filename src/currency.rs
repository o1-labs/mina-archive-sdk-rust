use std::fmt;

use crate::error::{Error, Result};

/// 1 MINA = 10^9 nanomina.
const NANOMINA_PER_MINA: u64 = 1_000_000_000;

/// A Mina currency amount stored as nanomina (the atomic unit).
///
/// # Examples
/// ```
/// use mina_archive_sdk::Currency;
///
/// let amount = Currency::from_mina("1.5").unwrap();
/// assert_eq!(amount.nanomina(), 1_500_000_000);
/// assert_eq!(amount.mina(), "1.500000000");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Currency(u64);

impl Currency {
    /// Build a Currency from a nanomina value.
    pub fn from_nanomina(nanomina: u64) -> Self {
        Self(nanomina)
    }

    /// Parse a decimal MINA string like "1.5", "100", or "0.000000001".
    /// Up to 9 decimal places; negative or malformed inputs return
    /// `Error::InvalidCurrency`.
    pub fn from_mina(s: &str) -> Result<Self> {
        parse_decimal(s).map(Self)
    }

    /// Parse a nanomina-decimal string as it appears in Archive-Node-API
    /// responses (coinbase, fee, user-command amounts).
    pub fn from_graphql(s: &str) -> Result<Self> {
        s.parse::<u64>()
            .map(Self)
            .map_err(|_| Error::InvalidCurrency(s.to_string()))
    }

    /// Value in nanomina.
    pub fn nanomina(&self) -> u64 {
        self.0
    }

    /// Value as a decimal MINA string with 9 fractional digits.
    pub fn mina(&self) -> String {
        let whole = self.0 / NANOMINA_PER_MINA;
        let frac = self.0 % NANOMINA_PER_MINA;
        format!("{whole}.{frac:09}")
    }

    /// Value as a nanomina decimal string (the format GraphQL accepts on input).
    pub fn to_nanomina_str(&self) -> String {
        self.0.to_string()
    }

    /// Overflow-safe addition. `None` on overflow.
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    /// Underflow-safe subtraction.
    pub fn checked_sub(self, rhs: Self) -> Result<Self> {
        self.0
            .checked_sub(rhs.0)
            .map(Self)
            .ok_or(Error::CurrencyUnderflow(self.0, rhs.0))
    }

    /// Overflow-safe multiplication by a scalar.
    pub fn checked_mul(self, rhs: u64) -> Option<Self> {
        self.0.checked_mul(rhs).map(Self)
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mina())
    }
}

impl std::ops::Add for Currency {
    type Output = Currency;
    fn add(self, rhs: Self) -> Self::Output {
        Currency(self.0 + rhs.0)
    }
}

impl std::ops::Sub for Currency {
    type Output = Currency;
    /// Panics on underflow. Use `checked_sub` for fallible subtraction.
    fn sub(self, rhs: Self) -> Self::Output {
        Currency(self.0.checked_sub(rhs.0).expect("currency underflow"))
    }
}

impl std::ops::Mul<u64> for Currency {
    type Output = Currency;
    fn mul(self, rhs: u64) -> Self::Output {
        Currency(self.0 * rhs)
    }
}

impl std::ops::Mul<Currency> for u64 {
    type Output = Currency;
    fn mul(self, rhs: Currency) -> Self::Output {
        Currency(self * rhs.0)
    }
}

fn parse_decimal(s: &str) -> Result<u64> {
    let s_orig = s;
    let s = s.trim();
    if s.is_empty() {
        return Err(Error::InvalidCurrency(s_orig.to_string()));
    }
    if s.starts_with('-') {
        return Err(Error::InvalidCurrency(s_orig.to_string()));
    }

    let (whole_str, frac_str) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s, ""),
    };

    let whole: u64 = if whole_str.is_empty() {
        0
    } else {
        whole_str
            .parse()
            .map_err(|_| Error::InvalidCurrency(s_orig.to_string()))?
    };

    if frac_str.len() > 9 {
        return Err(Error::InvalidCurrency(format!(
            "too many decimal places (max 9): {s_orig}"
        )));
    }

    let frac: u64 = if frac_str.is_empty() {
        0
    } else {
        let padded = format!("{frac_str:0<9}");
        padded
            .parse()
            .map_err(|_| Error::InvalidCurrency(s_orig.to_string()))?
    };

    whole
        .checked_mul(NANOMINA_PER_MINA)
        .and_then(|w| w.checked_add(frac))
        .ok_or_else(|| Error::InvalidCurrency(format!("overflow: {s_orig}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_mina_integer() {
        assert_eq!(Currency::from_mina("5").unwrap().nanomina(), 5_000_000_000);
    }

    #[test]
    fn from_mina_decimal() {
        assert_eq!(
            Currency::from_mina("1.5").unwrap().nanomina(),
            1_500_000_000
        );
    }

    #[test]
    fn from_mina_smallest_unit() {
        assert_eq!(Currency::from_mina("0.000000001").unwrap().nanomina(), 1);
    }

    #[test]
    fn from_mina_no_whole() {
        assert_eq!(Currency::from_mina(".5").unwrap().nanomina(), 500_000_000);
    }

    #[test]
    fn from_graphql() {
        let c = Currency::from_graphql("1500000000").unwrap();
        assert_eq!(c.nanomina(), 1_500_000_000);
        assert_eq!(c.mina(), "1.500000000");
    }

    #[test]
    fn arithmetic() {
        let a = Currency::from_mina("3").unwrap();
        let b = Currency::from_mina("1").unwrap();
        assert_eq!((a + b).nanomina(), 4_000_000_000);
        assert_eq!((a - b).nanomina(), 2_000_000_000);
        assert_eq!((a * 3).nanomina(), 9_000_000_000);
        assert_eq!((3_u64 * a).nanomina(), 9_000_000_000);
    }

    #[test]
    fn checked_sub_underflow() {
        let a = Currency::from_mina("1").unwrap();
        let b = Currency::from_mina("2").unwrap();
        assert!(a.checked_sub(b).is_err());
    }

    #[test]
    fn checked_add_overflow() {
        let max = Currency::from_nanomina(u64::MAX);
        assert!(max.checked_add(Currency::from_nanomina(1)).is_none());
    }

    #[test]
    fn ordering_and_equality() {
        let a = Currency::from_mina("1").unwrap();
        let b = Currency::from_mina("2").unwrap();
        let c = Currency::from_nanomina(1_000_000_000);
        assert!(a < b);
        assert_eq!(a, c);
    }

    #[test]
    fn display() {
        let c = Currency::from_nanomina(1);
        assert_eq!(format!("{c}"), "0.000000001");
    }

    #[test]
    fn rejects_too_many_decimals() {
        assert!(Currency::from_mina("1.0000000001").is_err());
    }

    #[test]
    fn rejects_invalid_format() {
        assert!(Currency::from_mina("abc").is_err());
        assert!(Currency::from_mina("").is_err());
        assert!(Currency::from_graphql("not_a_number").is_err());
    }

    #[test]
    fn rejects_negative() {
        assert!(Currency::from_mina("-1").is_err());
    }

    #[test]
    fn zero() {
        let c = Currency::from_nanomina(0);
        assert_eq!(c.mina(), "0.000000000");
        assert_eq!(c.to_nanomina_str(), "0");
    }

    #[test]
    fn nanomina_string_roundtrip() {
        let c = Currency::from_mina("3").unwrap();
        assert_eq!(c.to_nanomina_str(), "3000000000");
        let back = Currency::from_graphql(&c.to_nanomina_str()).unwrap();
        assert_eq!(c, back);
    }
}
