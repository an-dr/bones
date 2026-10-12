//! Installing another bones application from a release archive this one
//! carries — a companion — unless the user already has it.
//!
//! The archive is the companion's own update payload, unchanged: one version
//! folder's contents at the zip's root, including the launcher payload. So a
//! companion installed this way is indistinguishable from one that installed
//! itself. Downloading the archive and registering the companion with the
//! desktop are not done here: the host's build carries the archive, and the
//! companion registers itself.

use std::path::{Path, PathBuf};

use crate::{
    current_version_dir, default_install_dir, extract_version, install::mark_executables,
    install_fresh, verify_checksum, AppIdentity,
};

mod outcome;
mod policy;

pub use outcome::Outcome;
pub use policy::Policy;

/// Installs the companion `identity` names from `archive`, as `version`,
/// when `policy` says so.
///
/// - **Where:** `default_install_dir(identity)`, which honours the identity's
///   override variable.
/// - **Present** means the companion's launcher exists there; its version is
///   the name of its current version folder.
/// - **Checksum:** verified before anything is written, when one is given.
/// - **Errors are values.** A corrupt archive, a checksum mismatch, or a
///   filesystem failure is `Err`, never a panic: the host must finish its own
///   install whatever happens to the companion's.
pub(crate) fn install_companion(
    identity: &AppIdentity,
    archive: &Path,
    version: &str,
    expected_sha256: Option<&str>,
    policy: Policy,
) -> Result<Outcome, String> {
    let candidate = bones_versioning::Version::parse(version)
        .map_err(|error| format!("invalid companion version: {error}"))?;
    let install_dir = default_install_dir(identity)
        .ok_or_else(|| String::from("could not resolve the companion's install directory"))?;
    let present = install_dir.join(identity.launcher_exe()).is_file();
    let installed = current_version_dir(&install_dir).and_then(|dir| {
        bones_versioning::parse_folder_name(dir.file_name()?.to_str()?)
            .map(|version| version.to_string())
    });

    if present {
        match policy {
            Policy::IfAbsent => {
                return Ok(Outcome::AlreadyPresent {
                    version: installed.unwrap_or_else(|| String::from("unknown")),
                })
            }
            Policy::IfNewer => {
                if let Some(installed) = &installed {
                    let installed_version = bones_versioning::Version::parse(installed)
                        .map_err(|error| error.to_string())?;
                    if candidate.cmp_precedence(&installed_version) != std::cmp::Ordering::Greater {
                        return Ok(Outcome::Skipped {
                            reason: format!("installed {installed} is not older than {version}"),
                        });
                    }
                }
            }
        }
    }

    let bytes = std::fs::read(archive)
        .map_err(|error| format!("reading {}: {error}", archive.display()))?;
    if let Some(expected) = expected_sha256 {
        if !verify_checksum(&bytes, expected) {
            return Err(format!(
                "{} does not match the expected sha256",
                archive.display()
            ));
        }
    }

    if present {
        // Exactly what a self-update would do: a new version folder beside
        // the current one, picked by the launcher on its next start.
        let version_dir = extract_version(&bytes, &install_dir, version)?;
        mark_executables(&version_dir)?;
    } else {
        install_from_archive(identity, &bytes, &install_dir, version)?;
    }
    Ok(Outcome::Installed {
        version: version.to_string(),
    })
}

/// Unpacks into a scratch folder, then lets `install_fresh` place the
/// launcher from its payload and make the first version folder. The scratch
/// folder is removed whatever the outcome.
fn install_from_archive(
    identity: &AppIdentity,
    bytes: &[u8],
    install_dir: &Path,
    version: &str,
) -> Result<(), String> {
    let scratch = scratch_dir();
    let result = (|| {
        let unpacked = extract_version(bytes, &scratch, version)?;
        mark_executables(&unpacked)?;
        install_fresh(identity, &unpacked, install_dir, version)
    })();
    let _ = std::fs::remove_dir_all(&scratch);
    result
}

fn scratch_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("bones-companion-{}-{nanos}", std::process::id()))
}

#[cfg(test)]
mod tests;
