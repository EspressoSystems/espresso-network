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

use std::fmt::Display;

use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};

use super::errors::ExplorerAPIError;

/// CurrencyMismatchError is an error that occurs when two different currencies
/// are attempted to be combined in any way that would result in an invalid
/// state.
///
/// For example, attempting to add two different currencies together:
/// USD 1.00 + EUR 1.00
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "code", rename = "CURRENCY_MISMATCH")]
pub struct CurrencyMismatchError {
    pub currency1: CurrencyCode,
    pub currency2: CurrencyCode,
}

impl ExplorerAPIError for CurrencyMismatchError {
    /// code returns a string identifier to uniquely identify the error
    fn code(&self) -> &str {
        "CURRENCY_MISMATCH"
    }
}

impl Display for CurrencyMismatchError {
    /// fmt formats the error into a human readable string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "attempt to add two different currencies: {:?} and {:?}",
            self.currency1, self.currency2
        )
    }
}

impl Serialize for CurrencyMismatchError {
    /// serialize converts the error into a struct representation
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("CurrencyMismatchError", 4)?;
        st.serialize_field("code", &self.code())?;
        st.serialize_field("currency1", &self.currency1)?;
        st.serialize_field("currency2", &self.currency2)?;
        st.serialize_field("message", &format!("{}", self))?;
        st.end()
    }
}

/// InvalidCurrencyCodeError is an error that occurs when an invalid currency
/// code representation is encountered.  This should only occur when the
/// currency is being decoded from a string representation.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "code", rename = "INVALID_CURRENCY_CODE")]
pub struct InvalidCurrencyCodeError {
    pub currency: String,
}

impl ExplorerAPIError for InvalidCurrencyCodeError {
    /// code returns a string identifier to uniquely identify the
    /// [InvalidCurrencyCodeError]
    fn code(&self) -> &str {
        "INVALID_CURRENCY_CODE"
    }
}

impl Display for InvalidCurrencyCodeError {
    /// fmt formats the error into a human readable string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid currency code: {}", self.currency)
    }
}

impl Serialize for InvalidCurrencyCodeError {
    /// serialize converts the error into a struct representation
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("InvalidCurrencyCodeError", 3)?;
        st.serialize_field("code", &self.code())?;
        st.serialize_field("currency", &self.currency)?;
        st.serialize_field("message", &format!("{}", self))?;
        st.end()
    }
}

/// [CurrencyCode] represents an enumeration of all representable currency
/// codes that are supported by the API.
///
/// [CurrencyCode] is an overloaded term for representing different types of
/// currencies that could potentially be referenced / utilized within our
/// system.  This list is currently not exhaustive, and is expected to grow
/// as needed.  In fact, there may be too many entries in here as it stands.
///
/// The currency codes are annotated with a specific range that should map
/// to the type that we'd expect.
///
/// Here's the allocated range for the various currency codes:
/// - 1 - 999: Fiat Currency
/// - 1001 - 9999: Crypto Currency
/// - 10001 - 99999: Token Currency
///
/// For the Fiat Currencies, the [CurrencyCode] identifier is the Alpha3
/// representation of the ISO4217 standard.  In addition, it's numeric value
/// is mapped to the ISO4217 standard as well.
///
/// No such standard currently exists for Crypto Currencies or for the Token
/// based currencies, so we just utilize an enumeration for these that is based
/// on their ordering. This is not guaranteed to be stable, and code should
/// not depend on their order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "&str")]
pub enum CurrencyCode {
    FiatCurrencyStart = 0,
    #[serde(rename = "JPY")]
    Jpy = 392,
    #[serde(rename = "GBP")]
    Gbp = 826,
    #[serde(rename = "USD")]
    Usd = 840,
    #[serde(rename = "EUR")]
    Eur = 978,
    #[serde(rename = "XXX")]
    Xxx = 999,
    FiatCurrencyEnd = 1000,

    CryptoStart = 1001,
    #[serde(rename = "ETH")]
    Eth,
    #[serde(rename = "XBT")]
    Btc,
    CryptoEnd = 10000,

    TokenStart = 10001,
    #[serde(rename = "ESP")]
    Esp,
    TokenEnd = 99999,
}

impl CurrencyCode {
    pub fn is_fiat(&self) -> bool {
        *self >= CurrencyCode::FiatCurrencyStart && *self <= CurrencyCode::FiatCurrencyEnd
    }

    pub fn is_crypto(&self) -> bool {
        *self >= CurrencyCode::CryptoStart && *self <= CurrencyCode::CryptoEnd
    }

    pub fn is_token(&self) -> bool {
        *self >= CurrencyCode::TokenStart && *self <= CurrencyCode::TokenEnd
    }

    /// significant_digits represents the total number of significant digits
    /// that the currency in question utilizes.
    ///
    /// This is used to determine the precision of the currency when formatting
    /// its representation as a string.
    pub fn significant_digits(&self) -> usize {
        match self {
            Self::Jpy => 0,
            Self::Gbp => 2,
            Self::Usd => 2,
            Self::Eur => 2,
            Self::Btc => 8,
            Self::Eth => 18,
            _ => 0,
        }
    }
}

impl Display for CurrencyCode {
    /// fmt formats the currency code into a human readable string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurrencyCode::Jpy => write!(f, "JPY"),
            CurrencyCode::Gbp => write!(f, "GBP"),
            CurrencyCode::Usd => write!(f, "USD"),
            CurrencyCode::Eur => write!(f, "EUR"),
            CurrencyCode::Eth => write!(f, "ETH"),
            CurrencyCode::Btc => write!(f, "XBT"),
            CurrencyCode::Xxx => write!(f, "XXX"),
            CurrencyCode::Esp => write!(f, "ESP"),
            _ => write!(f, "UnknownCurrency({})", *self as u16),
        }
    }
}

impl From<CurrencyCode> for u16 {
    /// from converts the [CurrencyCode] into it's mapped numeric
    /// representation.
    fn from(currency: CurrencyCode) -> u16 {
        currency as u16
    }
}

impl TryFrom<&str> for CurrencyCode {
    type Error = InvalidCurrencyCodeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "JPY" => Ok(Self::Jpy),
            "GBP" => Ok(Self::Gbp),
            "USD" => Ok(Self::Usd),
            "EUR" => Ok(Self::Eur),
            "ETH" => Ok(Self::Eth),
            "BTC" => Ok(Self::Btc),
            "XBT" => Ok(Self::Btc),
            "XXX" => Ok(Self::Xxx),
            "ESP" => Ok(Self::Esp),
            _ => Err(InvalidCurrencyCodeError {
                currency: value.to_string(),
            }),
        }
    }
}

impl From<CurrencyCode> for String {
    /// from converts the [CurrencyCode] into a string representation utilizing
    /// the fmt method for the [CurrencyCode].
    fn from(currency: CurrencyCode) -> String {
        currency.to_string()
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_serialize_deserialize() {
        #[derive(Debug, PartialEq)]
        pub enum EncodeMatch {
            Yes,
            No,
        }

        let cases = [
            (r#""JPY""#, super::CurrencyCode::Jpy, EncodeMatch::Yes),
            (r#""GBP""#, super::CurrencyCode::Gbp, EncodeMatch::Yes),
            (r#""USD""#, super::CurrencyCode::Usd, EncodeMatch::Yes),
            (r#""EUR""#, super::CurrencyCode::Eur, EncodeMatch::Yes),
            (r#""ETH""#, super::CurrencyCode::Eth, EncodeMatch::Yes),
            (r#""BTC""#, super::CurrencyCode::Btc, EncodeMatch::No),
            (r#""XBT""#, super::CurrencyCode::Btc, EncodeMatch::Yes),
            (r#""XXX""#, super::CurrencyCode::Xxx, EncodeMatch::Yes),
            (r#""ESP""#, super::CurrencyCode::Esp, EncodeMatch::Yes),
        ];

        for (json, expected, should_match) in cases.iter() {
            {
                let actual: super::CurrencyCode = serde_json::from_str(json).unwrap();
                let have = actual;
                let want = *expected;
                assert_eq!(
                    have, want,
                    "decode json: {}: have: {have}, want: {want}",
                    *json
                );
            }

            if *should_match != EncodeMatch::Yes {
                continue;
            }

            {
                let encoded = serde_json::to_string(expected);
                assert!(encoded.is_ok(), "encode json: {}", *json);

                let encoded_json = encoded.unwrap();

                let have = &encoded_json;
                let want = *json;
                assert_eq!(
                    have, want,
                    "encoded json for {} does not match expectation: have: {have}, want: {want}",
                    expected
                );
            }
        }
    }
}
