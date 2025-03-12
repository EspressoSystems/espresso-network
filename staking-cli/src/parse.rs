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

// #[derive(Clone, Debug, From, Error)]
// #[error("failed to parse commission. {msg}")]
// pub struct ParseCommissionError {
//     msg: String,
// }

// pub fn commission_parser(s: &str) -> Result<u16, ParseCommissionError> {
//     let commission = s.parse()?;
//     if commission > 10000 {
//         return Err(ParseCommissionError {
//             msg: "Commission must be between 0 (0.00%) and 10000 (100.00%)".to_string(),
//         });
//     }
//     Ok(commission)
// }

#[derive(Debug, Clone)]
pub struct Commission(u64);

impl Commission {
    pub fn to_evm(&self) -> u64 {
        self.0
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
            .to_u64()
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
            ("1.000000", 100),
            ("1.2", 120),
            ("12.34", 1234),
            ("100.0", 10000),
            ("100.00", 10000),
            ("100.000", 10000),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_commission(input).unwrap().to_evm(), expected);
        }

        let failure_cases = [
            "-1", "-0.001", ".001", "100.01", "100.1", "1000", "fooo", "0.0.", "0.123", "0.1234",
            "99.999",
        ];
        for input in failure_cases {
            assert!(parse_commission(input).is_err());
        }
    }
}
