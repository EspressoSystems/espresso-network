use std::{collections::BTreeSet, str::FromStr, sync::OnceLock};

use strum::{EnumString, IntoStaticStr, VariantArray};

/// A bench-branch node feature, off unless enabled at startup. No variants on `main`.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    VariantArray,
    IntoStaticStr,
    EnumString,
)]
#[strum(serialize_all = "kebab-case")]
pub enum Experiment {
    /// Consensus storage on the append-only journal whatever storage module is given.
    Journal,
    /// Transaction commitments hash the payload with BLAKE3 first; every node of a network must agree.
    Blake3TxHash,
    /// Block payloads and VID shares in `ESPRESSO_NODE_BLOB_DIR`; without it the option is ignored.
    BlobDir,
}

impl Experiment {
    /// Whether this process enabled the experiment; false before `EnabledExperiments::install`.
    pub fn enabled(self) -> bool {
        ENABLED.get().is_some_and(|set| set.contains(self))
    }

    /// Names of every experiment this binary has, in declaration order.
    pub fn names() -> Vec<&'static str> {
        Self::VARIANTS.iter().copied().map(Into::into).collect()
    }
}

/// The experiments one node process runs with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnabledExperiments(BTreeSet<Experiment>);

impl EnabledExperiments {
    /// Makes this set the one `Experiment::enabled` reads; panics on a second call.
    pub fn install(self) {
        ENABLED
            .set(self)
            .expect("experiments installed more than once");
    }

    /// No experiment enabled.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Names of the enabled experiments, in declaration order.
    pub fn names(&self) -> Vec<&'static str> {
        self.0.iter().copied().map(Into::into).collect()
    }

    /// Whether `experiment` is in this set; `Experiment::enabled` reads the installed set through it.
    pub fn contains(&self, experiment: Experiment) -> bool {
        self.0.contains(&experiment)
    }
}

/// Parses a comma list of names, `all`, or the empty string; entries are trimmed, empty entries skipped, duplicates collapse.
impl FromStr for EnabledExperiments {
    type Err = ParseExperimentsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut entries: Vec<&str> = s
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .collect();
        entries.sort_unstable();
        entries.dedup();
        match entries.as_slice() {
            ["all"] => Ok(Self(Experiment::VARIANTS.iter().copied().collect())),
            _ if entries.contains(&"all") => Err(ParseExperimentsError::AllWithNames),
            entries => entries
                .iter()
                .map(|name| {
                    name.parse().map_err(|_| ParseExperimentsError::Unknown {
                        name: (*name).to_string(),
                        known: Experiment::names(),
                    })
                })
                .collect::<Result<_, _>>()
                .map(Self),
        }
    }
}

/// `ESPRESSO_NODE_EXPERIMENTS` the binary cannot honor.
#[derive(Debug, thiserror::Error)]
pub enum ParseExperimentsError {
    /// A name this binary has no variant for.
    #[error("unknown experiment {name:?}; this binary has {known:?}")]
    Unknown {
        name: String,
        known: Vec<&'static str>,
    },
    /// `all` next to other names.
    #[error("`all` must be the only entry")]
    AllWithNames,
}

/// Set once by `EnabledExperiments::install`.
static ENABLED: OnceLock<EnabledExperiments> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experiment_names_in_declaration_order_ok() {
        assert_eq!(
            Experiment::names(),
            ["journal", "blake3-tx-hash", "blob-dir"]
        );
    }

    #[test]
    fn experiment_uninstalled_disabled_ok() {
        assert!(!Experiment::Journal.enabled());
    }

    #[test]
    #[should_panic(expected = "installed more than once")]
    fn experiment_install_twice_fails() {
        "".parse::<EnabledExperiments>().unwrap().install();
        "".parse::<EnabledExperiments>().unwrap().install();
    }

    #[test]
    fn experiment_parse_empty_ok() {
        for input in ["", " ", " , ", ",,"] {
            let set: EnabledExperiments = input.parse().unwrap();
            assert!(set.is_empty(), "{input:?}");
        }
    }

    #[test]
    fn experiment_parse_all_ok() {
        for input in ["all", " all ", "all,all"] {
            let set: EnabledExperiments = input.parse().unwrap();
            assert_eq!(set.0.len(), Experiment::VARIANTS.len(), "{input:?}");
        }
    }

    #[test]
    fn experiment_unknown_name_fails() {
        let err = "nope".parse::<EnabledExperiments>().unwrap_err();
        assert!(matches!(
            &err,
            ParseExperimentsError::Unknown { name, known }
                if name == "nope" && *known == Experiment::names()
        ));
        assert_eq!(
            err.to_string(),
            format!(
                "unknown experiment \"nope\"; this binary has {:?}",
                Experiment::names()
            )
        );
    }

    #[test]
    fn experiment_all_mixed_fails() {
        for input in ["all,nope", "nope,all"] {
            assert!(
                matches!(
                    input.parse::<EnabledExperiments>(),
                    Err(ParseExperimentsError::AllWithNames)
                ),
                "{input:?}"
            );
        }
    }
}
