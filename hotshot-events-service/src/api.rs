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

use std::{fs, path::Path};

use tide_disco::api::{Api, ApiError};
use toml::{Value, map::Entry};
use vbs::version::StaticVersionType;

pub(crate) fn load_api<State: 'static, Error: 'static, Ver: StaticVersionType + 'static>(
    path: Option<impl AsRef<Path>>,
    default: &str,
    extensions: impl IntoIterator<Item = Value>,
) -> Result<Api<State, Error, Ver>, ApiError> {
    let mut toml = match path {
        Some(path) => load_toml(path.as_ref())?,
        None => toml::from_str(default).map_err(|err| ApiError::CannotReadToml {
            reason: err.to_string(),
        })?,
    };
    for extension in extensions {
        merge_toml(&mut toml, extension);
    }
    Api::new(toml)
}

fn merge_toml(into: &mut Value, from: Value) {
    if let (Value::Table(into), Value::Table(from)) = (into, from) {
        for (key, value) in from {
            match into.entry(key) {
                Entry::Occupied(mut entry) => merge_toml(entry.get_mut(), value),
                Entry::Vacant(entry) => {
                    entry.insert(value);
                },
            }
        }
    }
}

fn load_toml(path: &Path) -> Result<Value, ApiError> {
    let bytes = fs::read(path).map_err(|err| ApiError::CannotReadToml {
        reason: err.to_string(),
    })?;
    let string = std::str::from_utf8(&bytes).map_err(|err| ApiError::CannotReadToml {
        reason: err.to_string(),
    })?;
    toml::from_str(string).map_err(|err| ApiError::CannotReadToml {
        reason: err.to_string(),
    })
}
