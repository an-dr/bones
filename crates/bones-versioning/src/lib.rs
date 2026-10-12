//! Pure SemVer and version-folder semantics, without I/O or application identity.

pub use semver::{BuildMetadata, Comparator, Error, Op, Prerelease, Version, VersionReq};

use std::cmp::Ordering;
use std::path::PathBuf;
use std::time::SystemTime;

/// Compares strict SemVer versions by precedence, ignoring build metadata.
pub fn compare_versions(left: &str, right: &str) -> Result<Ordering, Error> {
    Ok(Version::parse(left)?.cmp_precedence(&Version::parse(right)?))
}

/// Whether the candidate has greater SemVer precedence; invalid input returns false.
pub fn is_newer(current: &str, candidate: &str) -> bool {
    compare_versions(candidate, current).is_ok_and(|order| order == Ordering::Greater)
}

/// Formats an installation instance without changing its SemVer version.
///
/// - Instance identifiers contain ASCII letters, digits, and hyphens.
/// - `~` separates installation identity from prerelease and build metadata.
pub fn format_folder_name(version: &Version, instance: &str) -> Option<String> {
    valid_instance(instance).then(|| format!("{version}~{instance}"))
}

/// Parses a version folder, optionally suffixed with `~instance`.
pub fn parse_folder_name(name: &str) -> Option<Version> {
    let version = match name.split_once('~') {
        Some((version, instance)) if valid_instance(instance) => version,
        Some(_) => return None,
        None => name,
    };
    Version::parse(version).ok()
}

fn valid_instance(instance: &str) -> bool {
    !instance.is_empty()
        && instance
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

/// Orders supplied folder paths newest first, rejecting non-version names.
///
/// - SemVer precedence wins; build metadata and instance suffixes do not affect it.
/// - Equal precedence uses supplied modification time, then path for a stable tie.
/// - The caller supplies metadata; this function never accesses the filesystem.
pub fn order_version_folders(
    entries: impl IntoIterator<Item = (PathBuf, SystemTime)>,
) -> Vec<PathBuf> {
    let mut folders: Vec<_> = entries
        .into_iter()
        .filter_map(|(path, modified)| {
            let version = parse_folder_name(path.file_name()?.to_str()?)?;
            Some((version, modified, path))
        })
        .collect();
    folders.sort_by(|a, b| {
        a.0.cmp_precedence(&b.0)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    folders.into_iter().rev().map(|(_, _, path)| path).collect()
}

#[cfg(test)]
mod tests;
