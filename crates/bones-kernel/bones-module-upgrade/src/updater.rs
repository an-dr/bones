//! Private check, stage, and install orchestration used by the upgrade module.

use std::path::{Path, PathBuf};

use crate::{
    copy_version_from_dir, default_install_dir, download_asset_verified, extract_version,
    fetch_manifest, install_fresh, is_installed_version_dir, AppIdentity, Fetch, Manifest,
    ReleaseSource,
};

/// Reads `source`'s manifest and returns it when it is newer than `current`.
///
/// `Ok(None)` means up to date — including a GitHub source on a platform
/// with no releases, where there is nothing to be newer than.
pub(crate) fn check(
    fetch: &dyn Fetch,
    source: &ReleaseSource,
    current: &str,
) -> Result<Option<Manifest>, String> {
    let Some(url) = source.manifest_url() else {
        return Ok(None);
    };
    let manifest = fetch_manifest(fetch, &url)?;
    let order = bones_versioning::compare_versions(&manifest.version, current)
        .map_err(|error| format!("invalid release or current version: {error}"))?;
    Ok((order == std::cmp::Ordering::Greater).then_some(manifest))
}

/// Downloads `manifest`'s archive, refuses it on a checksum mismatch, and
/// unpacks it into its own version folder under `install_dir`.
///
/// Nothing is written before the checksum passes, and the running version is
/// never touched: the launcher picks the new folder on its next start. Returns
/// the new version folder.
pub(crate) fn stage(
    fetch: &dyn Fetch,
    manifest: &Manifest,
    install_dir: &Path,
) -> Result<PathBuf, String> {
    let archive = download_asset_verified(fetch, manifest)?;
    let version_dir = extract_version(&archive, install_dir, &manifest.version)?;
    Ok(version_dir)
}

/// Copies the build in `source_dir` — normally the running executable's own
/// folder — into `identity`'s install location as `version`.
///
/// - With a launcher already installed, the build becomes a new version
///   folder beside the current one, exactly as a staged update would.
/// - With nothing installed, the launcher and the first version folder go
///   straight to the install location.
///
/// Returns whether the install was fresh. Refuses a `source_dir` that is
/// already an installed version folder: copying it into itself would destroy
/// it.
pub(crate) fn install(
    identity: &AppIdentity,
    source_dir: &Path,
    version: &str,
) -> Result<bool, String> {
    if is_installed_version_dir(identity, source_dir) {
        return Err(String::from(
            "this build is already the one installed; nothing to do",
        ));
    }
    let install_dir = default_install_dir(identity)
        .ok_or_else(|| String::from("could not resolve the install directory"))?;
    if install_dir.join(identity.launcher_exe()).is_file() {
        copy_version_from_dir(identity, source_dir, &install_dir, version)?;
        Ok(false)
    } else {
        install_fresh(identity, source_dir, &install_dir, version)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests;
