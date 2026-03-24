use std::fmt;

pub const GIT_DESCRIBE: &str = env!("VERGEN_GIT_DESCRIBE");
pub const GIT_SHA: &str = env!("VERGEN_GIT_SHA");
pub const GIT_DIRTY: &str = env!("VERGEN_GIT_DIRTY");
pub const GIT_BRANCH: &str = env!("VERGEN_GIT_BRANCH");
pub const GIT_COMMIT_TIMESTAMP: &str = env!("VERGEN_GIT_COMMIT_TIMESTAMP");

/// Build and git metadata for version output.
///
/// `Display` prints only build details (git, os, arch) without `pkg_name` or
/// `pkg_version`. Git tags are used for binary versioning, not Cargo package
/// versions, so we omit them to avoid confusion (e.g., showing "0.1.0" when the
/// actual release tag is "20260223").
///
/// Use [`BuildInfo::with_header`] to include `pkg_name` as a header line:
///
/// ```ignore
/// print!("{}", espresso_utils::build_info!().with_header());
/// ```
///
/// For clap's `long_version`, use [`BuildInfo::clap_version`] which prepends a
/// newline (clap already prints the binary name):
///
/// ```ignore
/// #[command(long_version = espresso_utils::build_info!().clap_version())]
/// ```
pub struct BuildInfo {
    pub pkg_name: &'static str,
    pub pkg_version: &'static str,
    pub git_describe: &'static str,
    pub git_sha: &'static str,
    pub git_dirty: &'static str,
    pub git_branch: &'static str,
    pub git_commit_timestamp: &'static str,
    pub is_debug: bool,
    pub os: &'static str,
    pub arch: &'static str,
}

impl BuildInfo {
    pub fn with_header(&self) -> String {
        format!("{}\n{self}", self.pkg_name)
    }

    pub fn clap_version(&self) -> String {
        format!("\n{self}")
    }
}

impl fmt::Display for BuildInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "describe: {}", self.git_describe)?;
        writeln!(f, "rev: {}", self.git_sha)?;
        writeln!(f, "modified: {}", self.git_dirty)?;
        writeln!(f, "branch: {}", self.git_branch)?;
        writeln!(f, "commit-timestamp: {}", self.git_commit_timestamp)?;
        writeln!(f, "debug: {}", self.is_debug)?;
        writeln!(f, "os: {}", self.os)?;
        write!(f, "arch: {}", self.arch)
    }
}

#[macro_export]
macro_rules! build_info {
    () => {
        $crate::build_info::BuildInfo {
            pkg_name: env!("CARGO_PKG_NAME"),
            pkg_version: env!("CARGO_PKG_VERSION"),
            git_describe: $crate::build_info::GIT_DESCRIBE,
            git_sha: $crate::build_info::GIT_SHA,
            git_dirty: $crate::build_info::GIT_DIRTY,
            git_branch: $crate::build_info::GIT_BRANCH,
            git_commit_timestamp: $crate::build_info::GIT_COMMIT_TIMESTAMP,
            is_debug: cfg!(debug_assertions),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
        }
    };
}

#[cfg(test)]
mod test {
    #[test]
    fn test_build_info_display() {
        let info = crate::build_info!();
        let output = info.to_string();
        for field in [
            "describe:",
            "rev:",
            "modified:",
            "branch:",
            "commit-timestamp:",
            "debug:",
            "os:",
            "arch:",
        ] {
            assert!(output.contains(field), "missing {field}: {output}");
        }
        assert!(output.starts_with("describe:"));
        assert!(!output.contains("sequencer-utils"));
        assert_eq!(info.pkg_name, "sequencer-utils");
        assert!(!info.git_sha.is_empty());
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
    }

    #[test]
    fn test_clap_version() {
        let info = crate::build_info!();
        let clap_ver = info.clap_version();
        assert!(clap_ver.starts_with('\n'),);
        assert!(!clap_ver.contains("sequencer-utils"),);
        assert!(!clap_ver.contains("0.1.0"),);
        assert!(clap_ver.contains("describe:"));
    }
}
