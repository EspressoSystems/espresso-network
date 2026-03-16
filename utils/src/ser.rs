use serde::{
    Deserialize, Serialize,
    de::{DeserializeOwned, Deserializer, Error as _},
    ser::{Error as _, Serializer},
};

/// Types which can be deserialized from either integers or strings.
///
/// Some types can be represented as an integer or a string in human-readable formats like JSON or
/// TOML. For example, 1 GWEI might be represented by the integer `1000000000` or the string `"1
/// gwei"`. Such types can implement `FromStringOrInteger` and then use [`impl_string_or_integer`]
/// to derive this user-friendly serialization.
///
/// These types are assumed to have an efficient representation as an integral type in Rust --
/// [`Self::Binary`] -- and will be serialized to and from this type when using a non-human-readable
/// encoding. With human readable encodings, serialization is always to a string.
pub trait FromStringOrInteger: Sized {
    type Binary: Serialize + DeserializeOwned;
    type Integer: Serialize + DeserializeOwned;

    fn from_binary(b: Self::Binary) -> anyhow::Result<Self>;
    fn from_string(s: String) -> anyhow::Result<Self>;
    fn from_integer(i: Self::Integer) -> anyhow::Result<Self>;

    fn to_binary(&self) -> anyhow::Result<Self::Binary>;
    fn to_string(&self) -> anyhow::Result<String>;
}

/// Deserialize a type from either a string or integer in human-readable encodings.
///
/// This macro implements serde `Serialize` and `DeserializeOwned` traits with a friendly
/// deserialization mechanism that can handle strings and integers when using human-readable
/// formats. It works with any [`FromStringOrInteger`] type.
#[macro_export]
macro_rules! impl_serde_from_string_or_integer {
    ($t:ty) => {
        impl serde::Serialize for $t {
            fn serialize<S: serde::ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                $crate::ser::string_or_integer::serialize(self, s)
            }
        }

        impl<'de> serde::Deserialize<'de> for $t {
            fn deserialize<D: serde::de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                $crate::ser::string_or_integer::deserialize(d)
            }
        }
    };
}
pub use crate::impl_serde_from_string_or_integer;

/// Deserialize a type from either a string or integer in human-readable encodings.
///
/// This serialization module can be used with any [`FromStringOrInteger`] type. It is usually used
/// only indirectly by the expansion of the [`impl_string_or_integer`] macro.
pub mod string_or_integer {
    use super::*;

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum StringOrInteger<I> {
        String(String),
        Integer(I),
    }

    pub fn serialize<T: FromStringOrInteger, S: Serializer>(
        t: &T,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        if s.is_human_readable() {
            t.to_string().map_err(S::Error::custom)?.serialize(s)
        } else {
            t.to_binary().map_err(S::Error::custom)?.serialize(s)
        }
    }

    pub fn deserialize<'a, T: FromStringOrInteger, D: Deserializer<'a>>(
        d: D,
    ) -> Result<T, D::Error> {
        if d.is_human_readable() {
            match StringOrInteger::deserialize(d)? {
                StringOrInteger::String(s) => T::from_string(s).map_err(D::Error::custom),
                StringOrInteger::Integer(i) => T::from_integer(i).map_err(D::Error::custom),
            }
        } else {
            T::from_binary(T::Binary::deserialize(d)?).map_err(D::Error::custom)
        }
    }
}
