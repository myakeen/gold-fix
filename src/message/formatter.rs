use std::collections::HashMap;
use chrono::{DateTime, NaiveDateTime, Utc};
use crate::Result;
use crate::error::FixError;

pub trait FieldFormatter: Send + Sync {
    fn format(&self, value: &str) -> Result<String>;
    fn parse(&self, value: &str) -> Result<String>;
}

#[derive(Clone)]
pub struct DateTimeFormatter;

impl FieldFormatter for DateTimeFormatter {
    fn format(&self, value: &str) -> Result<String> {
        let dt = DateTime::parse_from_rfc3339(value)
            .map_err(|_| FixError::ParseError("Invalid datetime format".into()))?;
        Ok(dt.format("%Y%m%d-%H:%M:%S").to_string())
    }

    fn parse(&self, value: &str) -> Result<String> {
        NaiveDateTime::parse_from_str(value, "%Y%m%d-%H:%M:%S")
            .map_err(|_| FixError::ParseError("Invalid FIX datetime format".into()))?;
        Ok(value.to_string())
    }
}

#[derive(Clone)]
pub struct IntegerFormatter;

impl FieldFormatter for IntegerFormatter {
    fn format(&self, value: &str) -> Result<String> {
        value.parse::<i64>()
            .map_err(|_| FixError::ParseError("Invalid integer value".into()))?
            .to_string()
            .into()
    }

    fn parse(&self, value: &str) -> Result<String> {
        value.parse::<i64>()
            .map_err(|_| FixError::ParseError("Invalid integer format".into()))?;
        Ok(value.to_string())
    }
}

#[derive(Clone)]
pub struct DecimalFormatter {
    precision: usize,
}

impl DecimalFormatter {
    pub fn new(precision: usize) -> Self {
        DecimalFormatter { precision }
    }
}

impl FieldFormatter for DecimalFormatter {
    fn format(&self, value: &str) -> Result<String> {
        let num = value.parse::<f64>()
            .map_err(|_| FixError::ParseError("Invalid decimal value".into()))?;
        Ok(format!("{:.1$}", num, self.precision))
    }

    fn parse(&self, value: &str) -> Result<String> {
        value.parse::<f64>()
            .map_err(|_| FixError::ParseError("Invalid decimal format".into()))?;
        Ok(value.to_string())
    }
}

#[derive(Clone)]
pub struct CharFormatter;

impl FieldFormatter for CharFormatter {
    fn format(&self, value: &str) -> Result<String> {
        if value.len() != 1 {
            return Err(FixError::ParseError("Character field must be exactly one character".into()));
        }
        Ok(value.to_string())
    }

    fn parse(&self, value: &str) -> Result<String> {
        if value.len() != 1 {
            return Err(FixError::ParseError("Character field must be exactly one character".into()));
        }
        Ok(value.to_string())
    }
}

#[derive(Clone)]
pub struct StringFormatter;

impl FieldFormatter for StringFormatter {
    fn format(&self, value: &str) -> Result<String> {
        Ok(value.replace('|', "").into())
    }

    fn parse(&self, value: &str) -> Result<String> {
        Ok(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_formatter() {
        let formatter = DateTimeFormatter;
        let rfc3339 = "2025-01-24T12:34:56Z";
        let fix_format = "20250124-12:34:56";

        let formatted = formatter.format(rfc3339).unwrap();
        assert_eq!(formatted, fix_format);

        let parsed = formatter.parse(fix_format).unwrap();
        assert_eq!(parsed, fix_format);
    }

    #[test]
    fn test_integer_formatter() {
        let formatter = IntegerFormatter;

        let formatted = formatter.format("123").unwrap();
        assert_eq!(formatted, "123");

        assert!(formatter.format("12.3").is_err());
        assert!(formatter.parse("abc").is_err());
    }

    #[test]
    fn test_decimal_formatter() {
        let formatter = DecimalFormatter::new(2);

        let formatted = formatter.format("123.456").unwrap();
        assert_eq!(formatted, "123.46");

        assert!(formatter.format("abc").is_err());
        assert!(formatter.parse("abc").is_err());
    }

    #[test]
    fn test_char_formatter() {
        let formatter = CharFormatter;

        let formatted = formatter.format("A").unwrap();
        assert_eq!(formatted, "A");

        assert!(formatter.format("AB").is_err());
        assert!(formatter.parse("").is_err());
    }

    #[test]
    fn test_string_formatter() {
        let formatter = StringFormatter;

        let formatted = formatter.format("ABC|DEF").unwrap();
        assert_eq!(formatted, "ABCDEF");

        let parsed = formatter.parse("ABC").unwrap();
        assert_eq!(parsed, "ABC");
    }
}