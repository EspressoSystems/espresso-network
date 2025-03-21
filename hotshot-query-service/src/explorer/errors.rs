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

use std::fmt::{Debug, Display};

use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use tide_disco::StatusCode;

/// [ExplorerAPIError] is a trait that represents an error that can be returned
/// returned from the ExplorerAPI for various reasons.
///
/// It aims to be Serializable for the purposes of conveying the error to the
/// client.
pub trait ExplorerAPIError: Display + Debug {
    /// The code for this error will uniquely identify this specific error.
    ///
    /// > This value **SHOULD** match the rename field for the `serde` tag, if applicable.
    fn code(&self) -> &str;
}

impl std::error::Error for dyn ExplorerAPIError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

/// [Unimplemented] is an error that indicates that the feature in question is
/// no implemented, but in addition, that this status can be conveyed to the
/// called with serializable error code.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "code", rename = "UNIMPLEMENTED")]
pub struct Unimplemented {}

impl Unimplemented {
    pub fn status(&self) -> StatusCode {
        StatusCode::NOT_IMPLEMENTED
    }
}

impl ExplorerAPIError for Unimplemented {
    fn code(&self) -> &str {
        "UNIMPLEMENTED"
    }
}

/// Implement [Display] for an error message.
impl Display for Unimplemented {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "this feature is not yet implemented")
    }
}

impl std::error::Error for Unimplemented {}

impl Serialize for Unimplemented {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("Unimplemented", 2)?;
        st.serialize_field("code", &self.code())?;
        st.serialize_field("message", &format!("{}", self))?;
        st.end()
    }
}

/// [InvalidLimit] is an error that represents that the there was a problem
/// with the given limit parameter.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "code", rename = "INVALID_LIMIT")]
pub struct InvalidLimit {}

impl InvalidLimit {
    pub fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

impl ExplorerAPIError for InvalidLimit {
    fn code(&self) -> &str {
        "INVALID_LIMIT"
    }
}

impl Display for InvalidLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "limit must be provided, and must be a positive integer less than 100"
        )
    }
}

impl std::error::Error for InvalidLimit {}

impl Serialize for InvalidLimit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("InvalidLimit", 2)?;
        st.serialize_field("code", &self.code())?;
        st.serialize_field("message", &format!("{}", self))?;
        st.end()
    }
}

/// [NotFound] is an error that represents results could not be found for the
/// given key, or "search" parameters.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "code", rename = "NOT_FOUND")]
pub struct NotFound {
    pub key: String,
}

impl NotFound {
    pub fn status(&self) -> StatusCode {
        StatusCode::NOT_FOUND
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

impl ExplorerAPIError for NotFound {
    fn code(&self) -> &str {
        "NOT_FOUND"
    }
}

impl Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no data found for key: '{}'", self.key)
    }
}

impl std::error::Error for NotFound {}

impl Serialize for NotFound {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("NotFound", 2)?;
        st.serialize_field("code", &self.code())?;
        st.serialize_field("key", &self.key())?;
        st.serialize_field("message", &format!("{}", self))?;
        st.end()
    }
}

/// QueryError is an error that indicates that a specific error occurred while
/// evaluating a query, or decoding the results of a query.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "code", rename = "QUERY_ERROR")]
pub struct QueryError {
    pub error: crate::QueryError,
}

impl QueryError {
    pub fn status(&self) -> StatusCode {
        self.error.status()
    }

    pub fn error(&self) -> &crate::QueryError {
        &self.error
    }
}

impl ExplorerAPIError for QueryError {
    fn code(&self) -> &str {
        "QUERY_ERROR"
    }
}

impl Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "encountered error attempting to retrieve data: '{}'",
            self.error
        )
    }
}

impl std::error::Error for QueryError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        Some(&self.error)
    }
}

impl Serialize for QueryError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("QueryError", 2)?;
        st.serialize_field("code", &self.code())?;
        st.serialize_field("error", &self.error())?;
        st.serialize_field("message", &format!("{}", self))?;
        st.end()
    }
}

impl From<crate::QueryError> for QueryError {
    fn from(error: crate::QueryError) -> Self {
        Self { error }
    }
}

/// BadQuery is an error that indicates a submitted query to the service is
/// invalid, or malformed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "code", rename = "BAD_QUERY")]
pub struct BadQuery {}

impl BadQuery {
    pub fn status(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}

impl ExplorerAPIError for BadQuery {
    fn code(&self) -> &str {
        "BAD_QUERY"
    }
}

impl Display for BadQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "provided query is invalid, or malformed")
    }
}

impl std::error::Error for BadQuery {}

impl Serialize for BadQuery {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("BadQuery", 2)?;
        st.serialize_field("code", &self.code())?;
        st.serialize_field("message", &format!("{}", self))?;
        st.end()
    }
}

#[cfg(test)]
mod test {
    use super::{BadQuery, InvalidLimit, NotFound, QueryError, Unimplemented};

    #[test]
    fn test_serialize_deserialize_unimplemented() {
        let unimplemented = Unimplemented {};
        let serialize_result = serde_json::to_string(&unimplemented);
        assert!(
            serialize_result.is_ok(),
            "failed to serialize Unimplemented: {}",
            serialize_result.err().unwrap(),
        );
        let serialized = serialize_result.unwrap();

        {
            let have = &serialized;
            let want =
                r#"{"code":"UNIMPLEMENTED","message":"this feature is not yet implemented"}"#;
            assert_eq!(
                have, want,
                "serialized Unimplemented mismatch: have: {have}, want: {want}"
            );
        }

        let deserialize_result: Result<Unimplemented, _> = serde_json::from_str(&serialized);
        assert!(
            deserialize_result.is_ok(),
            "failed to deserialize Unimplemented: {}",
            deserialize_result.err().unwrap(),
        );
        let deserialized = deserialize_result.unwrap();
        {
            let have = deserialized;
            let want = unimplemented;
            assert_eq!(
                have, want,
                "deserialized Unimplemented mismatch: have: {have}, want: {want}"
            );
        }
    }

    #[test]
    fn test_serialize_deserialize_invalid_limit() {
        let invalid_limit = InvalidLimit {};
        let serialize_result = serde_json::to_string(&invalid_limit);
        assert!(
            serialize_result.is_ok(),
            "failed to serialize InvalidLimit: {}",
            serialize_result.err().unwrap(),
        );
        let serialized = serialize_result.unwrap();

        {
            let have = &serialized;
            let want = r#"{"code":"INVALID_LIMIT","message":"limit must be provided, and must be a positive integer less than 100"}"#;
            assert_eq!(
                have, want,
                "serialized InvalidLimit mismatch: have: {have}, want: {want}"
            );
        }

        let deserialize_result: Result<InvalidLimit, _> = serde_json::from_str(&serialized);
        assert!(
            deserialize_result.is_ok(),
            "failed to deserialize InvalidLimit: {}",
            deserialize_result.err().unwrap(),
        );
        let deserialized = deserialize_result.unwrap();
        {
            let have = deserialized;
            let want = invalid_limit;
            assert_eq!(
                have, want,
                "deserialized InvalidLimit mismatch: have: {have}, want: {want}"
            );
        }
    }

    #[test]
    fn test_serialize_deserialize_not_found() {
        let not_found = NotFound {
            key: "foo".to_string(),
        };
        let serialize_result = serde_json::to_string(&not_found);
        assert!(
            serialize_result.is_ok(),
            "failed to serialize NotFound: {}",
            serialize_result.err().unwrap(),
        );
        let serialized = serialize_result.unwrap();

        {
            let have = &serialized;
            let want =
                r#"{"code":"NOT_FOUND","key":"foo","message":"no data found for key: 'foo'"}"#;
            assert_eq!(
                have, want,
                "serialized NotFound mismatch: have: {have}, want: {want}"
            );
        }

        let deserialize_result: Result<NotFound, _> = serde_json::from_str(&serialized);
        assert!(
            deserialize_result.is_ok(),
            "failed to deserialize NotFound: {}",
            deserialize_result.err().unwrap(),
        );
        let deserialized = deserialize_result.unwrap();
        {
            let have = deserialized;
            let want = not_found;
            assert_eq!(
                have, want,
                "deserialized NotFound mismatch: have: {have}, want: {want}"
            );
        }
    }

    #[test]
    fn test_serialize_deserialize_query_error() {
        let query_error = QueryError {
            error: crate::QueryError::NotFound,
        };
        let serialize_result = serde_json::to_string(&query_error);
        assert!(
            serialize_result.is_ok(),
            "failed to serialize QueryError: {}",
            serialize_result.err().unwrap(),
        );
        let serialized = serialize_result.unwrap();

        {
            let have = &serialized;
            let want = r#"{"code":"QUERY_ERROR","error":"NotFound","message":"encountered error attempting to retrieve data: 'The requested resource does not exist or is not known to this query service.'"}"#;
            assert_eq!(
                have, want,
                "serialized QueryError mismatch: have: {have}, want: {want}"
            );
        }

        let deserialize_result: Result<QueryError, _> = serde_json::from_str(&serialized);
        assert!(
            deserialize_result.is_ok(),
            "failed to deserialize QueryError: {}",
            deserialize_result.err().unwrap(),
        );
        let deserialized = deserialize_result.unwrap();
        {
            let have = deserialized;
            let want = query_error;

            match &have.error {
                crate::QueryError::NotFound => {},
                _ => panic!("deserialized QueryError mismatch: have: {have}, want: {want}"),
            }
        }
    }

    #[test]
    fn test_serialize_deserialize_bad_query() {
        let bad_query = BadQuery {};
        let serialize_result = serde_json::to_string(&bad_query);
        assert!(
            serialize_result.is_ok(),
            "failed to serialize BadQuery: {}",
            serialize_result.err().unwrap(),
        );
        let serialized = serialize_result.unwrap();

        {
            let have = &serialized;
            let want =
                r#"{"code":"BAD_QUERY","message":"provided query is invalid, or malformed"}"#;
            assert_eq!(
                have, want,
                "serialized BadQuery mismatch: have: {have}, want: {want}"
            );
        }

        let deserialize_result: Result<BadQuery, _> = serde_json::from_str(&serialized);
        assert!(
            deserialize_result.is_ok(),
            "failed to deserialize BadQuery: {}",
            deserialize_result.err().unwrap(),
        );
        let deserialized = deserialize_result.unwrap();
        {
            let have = deserialized;
            let want = bad_query;

            assert_eq!(
                have, want,
                "deserialized BadQuery mismatch: have: {have}, want: {want}"
            );
        }
    }
}
