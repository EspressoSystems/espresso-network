// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use std::{
    fmt::{Debug, Display},
    ops::{Add, Sub},
};

use itertools::Itertools;
use serde::{Deserialize, Serialize, Serializer};

use super::currency::{CurrencyCode, CurrencyMismatchError};

#[derive(Debug, Clone, PartialEq, Eq)]
/// [MonetaryValue]s is a struct that paris a [CurrencyCode] with a value.
/// This structure is able to represent both positive and negative currencies.
pub struct MonetaryValue {
    pub currency: CurrencyCode,
    pub value: i128,
}

impl MonetaryValue {
    /// new creates a new MonetaryValue instance with the given [CurrencyCode]
    /// and [i128] value.
    pub fn new(currency: CurrencyCode, value: i128) -> Self {
        Self { currency, value }
    }
    /// usd is a convenience function to create a [MonetaryValue] with the
    /// USD currency code.
    pub fn usd(value: i128) -> Self {
        Self::new(CurrencyCode::Usd, value)
    }

    /// btc is a convenience function to create a [MonetaryValue] with the
    /// BTC currency code.
    pub fn btc(value: i128) -> Self {
        Self::new(CurrencyCode::Btc, value)
    }

    /// eth is a convenience function to create a [MonetaryValue] with the
    /// ETH currency code.
    pub fn eth(value: i128) -> Self {
        Self::new(CurrencyCode::Eth, value)
    }

    /// esp is a convenience function to create a [MonetaryValue] with the
    /// ESP currency code.
    ///
    /// This is used to represents Espresso Tokens, and is the default that
    /// is used for Espresso Fees and Rewards
    pub fn esp(value: i128) -> Self {
        Self::new(CurrencyCode::Esp, value)
    }
}

impl Display for MonetaryValue {
    /// fmt formats the [MonetaryValue] into a human readable string.  There's
    /// no official standard for formatting a [MonetaryValue] into a string,
    /// but there are highly utilized cases that we can use as a reference
    /// to inform our implementation.
    ///
    /// In referencing [ISO 4217](https://en.wikipedia.org/wiki/ISO_4217) we
    /// encounter a section titled
    /// [Code position in amount formatting](https://en.wikipedia.org/wiki/ISO_4217#Code_position_in_amount_formatting)
    /// which references a style guide provided by the European Union's
    /// Publication office which indicates that the value should be prefixed
    /// with the [CurrencyCode], followed by a "hard space" (non-breaking space)
    /// and then the value of the currency itself.  It also lists several
    /// countries which swaps the position of the currency code and the value.
    ///
    /// In this case, we opt to use the European Union's style guide, as it
    /// is at least backed by some form of standardization.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let currency = self.currency;
        let value = self.value;
        let significant_figures = currency.significant_digits();
        let abs_value = value.abs();
        let sign = if value < 0 { "-" } else { "" };

        let max_post_decimal_digits = 10i128.pow(significant_figures as u32);

        let whole = abs_value / max_post_decimal_digits;
        let fraction = abs_value % max_post_decimal_digits;

        let fraction_str = format!("{:0width$}", fraction, width = significant_figures);
        if fraction == 0 {
            write!(f, "{}\u{00a0}{}{}", currency, sign, whole)
        } else {
            write!(f, "{}\u{00a0}{}{}.{}", currency, sign, whole, fraction_str)
        }
    }
}

impl Add for MonetaryValue {
    type Output = Result<MonetaryValue, CurrencyMismatchError>;

    /// add attempts to add the two [MonetaryValue]s together.  This returns
    /// a [Result] because this addition **can** fail if the two
    /// [MonetaryValue]s do not have the same [CurrencyCode].
    fn add(self, rhs: Self) -> Self::Output {
        if self.currency != rhs.currency {
            return Err(CurrencyMismatchError {
                currency1: self.currency,
                currency2: rhs.currency,
            });
        }

        Ok(MonetaryValue {
            currency: self.currency,
            value: self.value + rhs.value,
        })
    }
}

impl Sub for MonetaryValue {
    type Output = Result<MonetaryValue, CurrencyMismatchError>;

    /// sub attempts to subtract the two [MonetaryValue]s together.  This returns
    /// a [Result] because this subtraction **can** fail if the two
    /// [MonetaryValue]s do not have the same [CurrencyCode].
    fn sub(self, rhs: Self) -> Self::Output {
        if self.currency != rhs.currency {
            return Err(CurrencyMismatchError {
                currency1: self.currency,
                currency2: rhs.currency,
            });
        }

        Ok(MonetaryValue {
            currency: self.currency,
            value: self.value - rhs.value,
        })
    }
}

impl Serialize for MonetaryValue {
    /// serialize converts the [MonetaryValue] into a String representation.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct MonetaryValueVisitor;

/// is_ascii_digit is a convenience function for converting char::is_ascii_digit
/// into a function that conforms to [Pattern].
fn is_ascii_digit(c: char) -> bool {
    char::is_ascii_digit(&c)
}

/// Reorder the currency code and the value strings into the expected order.
/// This is done to simplify the down-stream parsing logic as we will be able
/// to assume that the first element of the pair is the string containing the
/// currency code, and the second part of the pair is the portion of the string
/// that contains the monetary value.
///
/// This function does not attempt to do anything beyond separating these two
/// representations into a pair of strings.
fn reorder_currency_code_and_value<E>(first: String, last: String) -> Result<(String, String), E>
where
    E: serde::de::Error,
{
    // We need to sus out which portion of the split is the currency code
    // versus which is likely to be the numeric value.
    // This **should** be fairly easily, just identify which part has any
    // numerical value within it.

    if first.contains(is_ascii_digit) {
        Ok((last, first))
    } else {
        Ok((first, last))
    }
}

/// Split a string into a currency code and a value strings in order to simplify
/// parsing the string into a [MonetaryValue].
fn split_str_into_currency_code_and_value_string<E>(value: &str) -> Result<(String, String), E>
where
    E: serde::de::Error,
{
    let (index, _) = match value.chars().enumerate().find(|(_, c)| *c == '\u{00a0}') {
        Some((i, c)) => (i, c),
        None => {
            return Err(E::custom(
                "no non-breaking space found in expected MonetaryValue",
            ));
        },
    };

    let first: String = value.chars().take(index).collect();
    let last: String = value.chars().dropping(index + 1).collect();

    reorder_currency_code_and_value(first, last)
}

fn is_possibly_a_decimal_point(c: char) -> bool {
    c == '.' || c == ',' || char::is_whitespace(c)
}

/// determine_pre_and_post_decimal_strings attempts to determine if there is
/// a decimal point in the value string, and if so, it will returned the split
/// string based on the location of the decimal point.
///
/// The only supported decimal points are '.', ',', and ' '.
///
/// This implementation takes advantage of the notion that the decimal point is
/// the last symbol in a string representation of a number.  This may
/// potentially return false positives in some cases, but it's best to assume
/// that the value is formatted with a decimal rather than a grouping separator
/// instead.
///
/// For convenience the returned strings are pruned of any non-numeric value.
fn determine_pre_and_post_decimal_strings(value: &str) -> (String, Option<String>) {
    let decimal_point = value
        .chars()
        .enumerate()
        .filter(|(_, c)| is_possibly_a_decimal_point(*c))
        .last();

    match decimal_point {
        None => (value.chars().filter(char::is_ascii_digit).collect(), None),
        Some((index, _)) => {
            let pre_decimal_string: String = value
                .chars()
                .take(index)
                .filter(char::is_ascii_digit)
                .collect();

            let post_decimal_string: String = value
                .chars()
                .dropping(index + 1)
                .filter(char::is_ascii_digit)
                .collect();

            (pre_decimal_string, Some(post_decimal_string))
        },
    }
}

fn parse_pre_and_post_decimal_digits<E>(
    significant_digits: u32,
    value_raw_str: String,
) -> Result<i128, E>
where
    E: serde::de::Error,
{
    // We need to know the sign
    let sign = match value_raw_str
        .trim_start_matches(char::is_whitespace)
        .chars()
        .next()
    {
        Some('-') => -1,
        _ => 1,
    };

    // We want to see if we can determine the value before the decimal
    // separator, versus the value after the decimal separator.
    // This **should** support the ability to omit the decimal separator
    // and the trailing fractional portion of the value.
    // For now the only supported decimal separators we need to consider
    // are '.', ',', and ' '.
    let (pre_decimal_string, post_decimal_string_option) =
        determine_pre_and_post_decimal_strings(&value_raw_str);

    match post_decimal_string_option {
        None => match pre_decimal_string.parse::<i128>() {
            Ok(value) => Ok(sign * value * 10i128.pow(significant_digits)),
            Err(err) => Err(E::custom(err)),
        },
        Some(post_decimal_string) => {
            let pre_decimal_value = pre_decimal_string.parse::<i128>().map_err(E::custom)?;
            let post_decimal_value = post_decimal_string.parse::<i128>().map_err(E::custom)?;
            let num_digits = post_decimal_string.len() as u32;

            let value = sign
                * (pre_decimal_value * 10i128.pow(significant_digits)
                    + 10i128.pow(significant_digits - num_digits) * post_decimal_value);

            Ok(value)
        },
    }
}

impl serde::de::Visitor<'_> for MonetaryValueVisitor {
    type Value = MonetaryValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string in a ticker format, with the required number of significant digits.  For example: `USD 100.00` or `ETH 0.000000000000000001`")
    }

    /// We're wanting to deserialize a [MonetaryValue] from a string that can
    /// be in a Ticker Style format.  It **could** be beneficial to support
    /// multiple formats for convenience due to different localizations of
    /// the world.  This would allow for maximum flexibility in representation.
    /// This does mean that the numerical format can be wide and varied. This
    /// does not mean that this format does not come without restrictions.
    ///
    /// A list of parsing restrictions:
    ///
    /// - The value's numerical representation is expected to be formatting
    ///   using arabic numerals.
    /// - The value's numerical representation must have the expected number
    ///   of significant digits of the currency determined by the currency
    ///   code.
    /// - The currency code and the numerical value **MUST** be separated
    ///   by a non-breaking space character.
    /// - The Currency Code cannot have numeric digits within it.
    /// - The Currency Code must be a valid ISO 4217 Currency Code for all
    ///   fiat currencies.
    /// - The Currency Code may be a valid Crypto Currency Identifier
    /// - The Currency Code may be a valid Token Identifier
    /// - The Currency Code **MUST** be at least 3 characters long.
    ///
    /// This means that his function should be able to support parsing the
    /// following representations:
    ///
    /// # Supported Representations
    /// - USD 0.00
    /// - USD 0,00
    /// - USD 0 00
    /// - USD 0
    /// - 0.00 USD
    /// - 0,00 USD
    /// - 0 00 USD
    /// - 0 USD
    /// - USD 1,000.00
    /// - USD 1.000,00
    /// - USD 1 000,00
    /// - 1,000.00 USD
    /// - 1.000,00 USD
    /// - 1 000,00 USD
    /// - USD 1,00,000.00
    /// - 1,00,000.00 USD
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let (currency_code_str, value_raw_str) =
            split_str_into_currency_code_and_value_string::<E>(value)?;

        let currency = match CurrencyCode::try_from(&currency_code_str[..]) {
            Ok(currency) => Ok(currency),
            Err(err) => Err(E::custom(err)),
        }?;

        let value = parse_pre_and_post_decimal_digits::<E>(
            currency.significant_digits() as u32,
            value_raw_str,
        )?;

        Ok(MonetaryValue { currency, value })
    }
}

impl<'de> Deserialize<'de> for MonetaryValue {
    /// deserialize attempts to convert a string into a [MonetaryValue].
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(MonetaryValueVisitor)
    }
}

impl From<i128> for MonetaryValue {
    /// from converts an [i128] into a [MonetaryValue] with the USD currency
    /// code.
    fn from(value: i128) -> Self {
        Self::esp(value)
    }
}

#[cfg(test)]
mod test {
    use crate::explorer::{currency::CurrencyCode, monetary_value::MonetaryValue};

    #[test]
    fn test_monetary_value_json_deserialization() {
        {
            let values = vec![
                "\"USD 100.00\"",
                "\"USD 100,00\"",
                "\"USD 100 00\"",
                "\"USD 100\"",
                "\"100.00 USD\"",
                "\"100,00 USD\"",
                "\"100 00 USD\"",
                "\"100 USD\"",
            ];

            for value in values {
                let result: serde_json::Result<MonetaryValue> = serde_json::from_str(value);

                let result = match result {
                    Err(err) => {
                        panic!("{} failed to parse: {}", value, err);
                    },
                    Ok(result) => result,
                };

                let have = result;
                let want = MonetaryValue::usd(10000);

                assert_eq!(
                    have, want,
                    "{} parse result: have {}, want {}",
                    value, have, want,
                );
            }
        }

        {
            let values = vec![
                "\"USD 100000\"",
                "\"USD 100,000.00\"",
                "\"USD 100.000,00\"",
                "\"USD 100 000,00\"",
                "\"USD 1,00,000.00\"",
            ];

            for value in values {
                let result: serde_json::Result<MonetaryValue> = serde_json::from_str(value);

                let result = match result {
                    Err(err) => {
                        panic!("{} failed to parse: {}", value, err);
                    },
                    Ok(result) => result,
                };

                let have = result;
                let want = MonetaryValue::usd(10000000);

                assert_eq!(
                    have, want,
                    "{} parse result: have {}, want {}",
                    value, have, want,
                );
            }
        }

        assert!(serde_json::from_str::<MonetaryValue>("\"USD 0\"").is_ok());
        assert!(serde_json::from_str::<MonetaryValue>("\"USD 00\"").is_ok());
        assert!(serde_json::from_str::<MonetaryValue>("\"USD 100\"").is_err());
        assert!(serde_json::from_str::<MonetaryValue>("\"BTC 100\"").is_ok());
        assert!(serde_json::from_str::<MonetaryValue>("\"XBT 100\"").is_ok());
        assert!(serde_json::from_str::<MonetaryValue>("\"ETH 100\"").is_ok());

        {
            let cases = [
                ("\"USD 0.00\"", MonetaryValue::usd(0)),
                ("\"USD -1.00\"", MonetaryValue::usd(-100)),
                ("\"USD -1\"", MonetaryValue::usd(-100)),
                ("\"USD 1.23\"", MonetaryValue::usd(123)),
                ("\"USD 0.50\"", MonetaryValue::usd(50)),
                (
                    "\"ETH 0.000000001000000000\"",
                    MonetaryValue::eth(1000000000),
                ),
                ("\"ETH 0.000000000000000001\"", MonetaryValue::eth(1)),
                (
                    "\"ETH 1.000000000000000000\"",
                    MonetaryValue::eth(1000000000000000000),
                ),
                ("\"XBT 0.00000001\"", MonetaryValue::btc(1)),
            ];

            for case in cases {
                let value = case.0;
                let have = serde_json::from_str::<MonetaryValue>(value).unwrap();
                let want = case.1;
                assert_eq!(
                    have, want,
                    "{} parse result: have {}, want {}",
                    value, have, want
                );
            }
        }
    }

    #[test]
    fn test_monetary_value_json_serialization() {
        let cases = [
            (MonetaryValue::usd(0), "\"USD 0\""),
            (MonetaryValue::usd(-100), "\"USD -1\""),
            (MonetaryValue::usd(123), "\"USD 1.23\""),
            (MonetaryValue::usd(50), "\"USD 0.50\""),
            (
                MonetaryValue::eth(1000000000),
                "\"ETH 0.000000001000000000\"",
            ),
            (MonetaryValue::eth(1), "\"ETH 0.000000000000000001\""),
            (MonetaryValue::eth(1000000000000000000), "\"ETH 1\""),
            (MonetaryValue::btc(1), "\"XBT 0.00000001\""),
        ];

        for case in cases {
            let value = case.0;
            let have = serde_json::to_string(&value).unwrap();
            let want = case.1;
            assert_eq!(
                have, want,
                "{} encode result: have {}, want {}",
                value, have, want
            );
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        for currency in [
            CurrencyCode::Usd,
            CurrencyCode::Eth,
            CurrencyCode::Btc,
            CurrencyCode::Jpy,
        ]
        .iter()
        {
            for i in -100..=1000 {
                let value = MonetaryValue::new(*currency, i);
                let serialized = serde_json::to_string(&value).unwrap();
                let deserialized = serde_json::from_str::<MonetaryValue>(&serialized).unwrap();
                assert_eq!(
                    value, deserialized,
                    "{} {} encoded result: {}: have {}, want {}",
                    currency, i, serialized, deserialized, value
                );
            }
        }
    }

    #[test]
    fn test_arithmetic() {
        {
            let a = MonetaryValue::usd(100);
            let b = MonetaryValue::usd(100);
            let c = a + b;
            assert!(c.is_ok());
            assert_eq!(c.unwrap(), MonetaryValue::usd(200));
        }

        {
            let a = MonetaryValue::usd(100);
            let b = MonetaryValue::usd(100);
            let c = a - b;
            assert!(c.is_ok());
            assert_eq!(c.unwrap(), MonetaryValue::usd(0));
        }

        {
            let a = MonetaryValue::usd(100);
            let b = MonetaryValue::eth(100);
            let c = a + b;
            assert!(c.is_err());
        }

        {
            let a = MonetaryValue::usd(100);
            let b = MonetaryValue::eth(100);
            let c = a - b;
            assert!(c.is_err());
        }
    }
}
