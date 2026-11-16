use std::{fmt, sync::Arc};

/// The longest table name, in bytes.
const MAX_LEN: usize = 64;

/// The name of a table.
///
/// A name is 1 to 64 bytes from `[a-z0-9_-]` and does not start with `_`, which backends
/// reserve for their own tables.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Table(Arc<str>);

impl Table {
    pub fn new(name: impl AsRef<str>) -> Result<Self, InvalidTableName> {
        let name = name.as_ref();
        let valid = !name.is_empty()
            && name.len() <= MAX_LEN
            && !name.starts_with('_')
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        if valid {
            Ok(Self(name.into()))
        } else {
            Err(InvalidTableName(name.to_owned()))
        }
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid table name {0:?}: expected 1 to 64 bytes of [a-z0-9_-], not starting with _")]
pub struct InvalidTableName(pub String);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names() {
        for name in ["a", "decide-log", "epoch_17", "0", &"x".repeat(MAX_LEN)] {
            assert_eq!(Table::new(name).unwrap().name(), name);
        }
    }

    #[test]
    fn invalid_names() {
        for name in [
            "",
            "_usage",
            "Upper",
            "with space",
            "slash/name",
            "dot.name",
            "ü",
            &"x".repeat(MAX_LEN + 1),
        ] {
            assert_eq!(Table::new(name), Err(InvalidTableName(name.to_owned())));
        }
    }
}
