use crate::AppIdentity;

/// Where a permanent install lives: `COMMITS_INSTALL_DIR` if set (tests),
/// otherwise `~/.myapp/app`. Compared against the running process's own
/// directory to tell an installed run from a dev or ad-hoc one -- the Install
/// menu action only makes sense for the latter.
pub(crate) fn default_install_dir(identity: &AppIdentity) -> Option<std::path::PathBuf> {
    if let Ok(value) = std::env::var(identity.install_env_var) {
        return Some(std::path::PathBuf::from(value));
    }
    dirs::home_dir().map(|home| home.join(identity.install_dir_name).join("app"))
}

/// Whether `dir` is a version folder directly under the canonical install
/// location -- i.e. this process is the app executable running from a version
/// folder the launcher picked, rather than a dev build or an
/// ad-hoc launch. Shared by the Install menu's own installed-or-not check and
/// by callers that need to tell a shared, install-wide location (saves,
/// extensions, logs) apart from an exe-relative one. Canonicalized before
/// comparing: a raw comparison can mismatch even for the same directory
/// (e.g. drive-letter casing or short/long name form). A canonicalize
/// failure on the install side (the ordinary case: nothing is installed
/// there yet) reads as "not installed" rather than an error.
pub(crate) fn is_installed_version_dir(identity: &AppIdentity, dir: &std::path::Path) -> bool {
    let Ok(dir) = dir.canonicalize() else {
        return false;
    };
    let Some(install_dir) = default_install_dir(identity).and_then(|d| d.canonicalize().ok())
    else {
        return false;
    };
    dir.parent() == Some(install_dir.as_path())
}

/// Whether `dir` is *shaped* like a version folder -- its own name parses
/// as a version, and its parent has a launcher executable in it --
/// regardless of whether that parent happens to be the canonical
/// `default_install_dir()`. Structural rather than location-based on
/// purpose: `cargo xtask dist` assembles a plain build directory in exactly this
/// shape of an installed tree, so a dev
/// build run straight out of it needs to find its shared data one level up
/// the same way a real install does, without `COMMITS_INSTALL_DIR` having
/// to point at wherever it happens to be sitting.
pub(crate) fn is_version_folder(identity: &AppIdentity, dir: &std::path::Path) -> bool {
    let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if bones_versioning::parse_folder_name(name).is_none() {
        return false;
    }
    dir.parent()
        .is_some_and(|parent| parent.join(identity.launcher_exe()).is_file())
}

/// Resolves `name` against the shared, install-wide location (the install
/// dir itself, a level above the version folder) when the running
/// executable is inside a version folder (see `is_version_folder`), or
/// against the running executable's own directory otherwise. The latter
/// matches the `resolve_relative_to_exe` convention the bones engine
/// builder already uses for a relative `saves_dir`/`extensions_dir`, so an
/// ad-hoc build with no launcher beside it (no version-folder shape at all)
/// keeps finding everything beside itself exactly as before -- only a run
/// from inside a version folder, real install or `dist/app` alike, needs
/// the extra step up to keep state, components, and logs shared across
/// versions instead of siloed inside whichever version folder happens to
/// be running.
pub(crate) fn shared_or_exe_relative(
    identity: &AppIdentity,
    name: &str,
) -> Option<std::path::PathBuf> {
    let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    Some(shared_or_exe_relative_from(identity, &exe_dir, name))
}

pub(crate) fn shared_or_exe_relative_from(
    identity: &AppIdentity,
    exe_dir: &std::path::Path,
    name: &str,
) -> std::path::PathBuf {
    if is_version_folder(identity, exe_dir) {
        if let Some(install_dir) = exe_dir.parent() {
            return install_dir.join(name);
        }
    }
    exe_dir.join(name)
}
