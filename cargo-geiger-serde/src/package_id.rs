use crate::Source;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Identifies a package in the dependency tree
#[derive(
    Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
pub struct PackageId {
    /// Package name
    pub name: String,
    /// Package version
    pub version: Version,
    /// Package source (e.g. repository, crate registry)
    pub source: Source,
}

impl ToString for PackageId {
    fn to_string(&self) -> String {
        return format!(
            "{} {} ({})",
            self.name,
            self.version.to_string(),
            match &self.source {
                Source::Git { url, .. } => format!("git+{}", url.to_string()),
                Source::Registry { url, .. } =>
                    format!("registry+{}", url.to_string()),
                Source::Path(path) => format!("file+{}", path.to_string()),
            }
        );
    }
}
