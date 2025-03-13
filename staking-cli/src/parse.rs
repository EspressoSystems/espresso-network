use std::str::FromStr as _;

use derive_more::From;
use hotshot_types::{light_client::StateSignKey, signature_key::BLSPrivKey};
use rust_decimal::{prelude::ToPrimitive as _, Decimal};
use tagged_base64::{TaggedBase64, Tb64Error};
use thiserror::Error;

pub fn parse_bls_priv_key(s: &str) -> Result<BLSPrivKey, Tb64Error> {
    Ok(TaggedBase64::parse(s)?.try_into()?)
}

pub fn parse_state_priv_key(s: &str) -> Result<StateSignKey, Tb64Error> {
    Ok(TaggedBase64::parse(s)?.try_into()?)
}

#[derive(Debug, Copy, Clone)]
pub struct Commission(u16);

impl Commission {
    pub fn to_evm(&self) -> u16 {
        self.0
    }
}

impl TryFrom<u64> for Commission {
    type Error = ParseCommissionError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > 10000 {
            return Err("Commission must be between 0 (0.00%) and 100 (100.00%)"
                .to_string()
                .into());
        }
        Ok(Self(value as u16))
    }
}

#[derive(Clone, Debug, From, Error)]
#[error("failed to parse ByteSize. {msg}")]
pub struct ParseCommissionError {
    msg: String,
}

/// Parse a percentage string into a `Percentage` type.
pub fn parse_commission(s: &str) -> Result<Commission, ParseCommissionError> {
    let dec = Decimal::from_str(s).map_err(|e| ParseCommissionError { msg: e.to_string() })?;
    if dec != dec.round_dp(2) {
        return Err(
            "Commission must be in percent with at most 2 decimal places"
                .to_string()
                .into(),
        );
    }
    let hundred = Decimal::new(100, 0);
    if dec < Decimal::ZERO || dec > hundred {
        return Err(
            format!("Commission must be between 0 (0.00%) and 100 (100.00%), got {dec}")
                .to_string()
                .into(),
        );
    }
    Ok(Commission(
        dec.checked_mul(hundred)
            .expect("multiplication succeeds")
            .to_u16()
            .expect("conversion to u64 succeeds"),
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_commission() {
        let cases = [
            ("0", 0),
            ("0.0", 0),
            ("0.00", 0),
            ("0.000", 0),
            ("0.01", 1),
            ("1", 100),
            ("2", 200),
            ("1.000000", 100),
            ("1.2", 120),
            ("12.34", 1234),
            ("100", 10000),
            ("100.0", 10000),
            ("100.00", 10000),
            ("100.000", 10000),
        ];
        for (input, expected) in cases {
            let parsed = parse_commission(input).unwrap().to_evm();
            assert_eq!(
                parsed, expected,
                "input: {input}, parsed: {parsed} != expected {expected}"
            );
        }

        let failure_cases = [
            /// negative
            "-1",
            "-0.001",
            /// too many decimals
            "0.123",
            "0.1234",
            "99.999",
            ".001",
            /// too large
            "100.01",
            "100.1",
            "1000",
            /// not a number
            "fooo",
            "0.0.",
        ];
        for input in failure_cases {
            assert!(
                parse_commission(input).is_err(),
                "input: {input} did not fail"
            );
        }
    }
}
