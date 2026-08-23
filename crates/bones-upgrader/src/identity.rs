//! Who this install belongs to.
//!
//! Nothing about picking a version folder or replacing an entry point is
//! specific to one application, so the names arrive as data rather than
//! being spelled into the mechanism. Every host that embeds this crate
//! states its own, and the two executables it ships agree on them without
//! either parsing configuration at runtime.

/// The names one application's install is built from.
///
/// Every field is a bare name rather than a path: the crate composes the
/// paths, so a host cannot accidentally supply an absolute location that
/// only works on the machine it was written on. Executable names carry no
/// extension for the same reason -- [`AppIdentity::launcher_exe`] and
/// [`AppIdentity::app_exe`] add the platform's, so a host states the name
/// once instead of `#[cfg]`-ing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppIdentity {
    /// Directory under the user's home holding the install, e.g. `.myapp`.
    /// The install itself lives in an `app` folder inside it.
    pub install_dir_name: &'static str,
    /// Environment variable that redirects the install location, used by
    /// tests and by anyone running against a scratch directory.
    pub install_env_var: &'static str,
    /// The launcher executable's name, without a platform extension.
    pub launcher_stem: &'static str,
    /// The application executable's name, without a platform extension.
    pub app_stem: &'static str,
}

impl AppIdentity {
    /// The launcher's filename on this platform -- the permanent entry point
    /// that lives at the install root and is the name users type.
    pub fn launcher_exe(&self) -> String {
        exe_name(self.launcher_stem)
    }

    /// The application's filename on this platform -- what a version folder
    /// holds and what the launcher starts.
    pub fn app_exe(&self) -> String {
        exe_name(self.app_stem)
    }

    /// The launcher's filename *inside a version folder*, which is not the
    /// name it runs under.
    ///
    /// The entry point has to keep one stable name forever, because
    /// shortcuts, PATH and muscle memory all point at it. The copy that
    /// travels with a version cannot share that name: it would sit in the
    /// same tree as the entry point it replaces, and the code that decides
    /// what belongs in a version folder tells them apart by name. So the
    /// payload is `<stem>-launcher`, and installing it renames it to
    /// [`AppIdentity::launcher_exe`].
    pub fn launcher_payload_exe(&self) -> String {
        exe_name(&format!("{}-launcher", self.launcher_stem))
    }
}

fn exe_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

/// The identity this binary was built for.
///
/// Read at compile time through `option_env!`, so a host sets the four
/// values in its own `.cargo/config.toml` `[env]` block and both its
/// executables agree without either one parsing configuration at runtime.
/// A consumer that sets nothing still compiles and gets neutral placeholder
/// names, so the crate is usable before anyone configures it. Those defaults
/// are not meant to ship: an application that reaches release without
/// setting the four values installs itself into `.bones-app`, which is a
/// visible symptom rather than a silent one.
pub fn host_identity() -> AppIdentity {
    AppIdentity {
        install_dir_name: option_env!("UPGRADER_INSTALL_DIR_NAME").unwrap_or(".bones-app"),
        install_env_var: option_env!("UPGRADER_INSTALL_ENV_VAR").unwrap_or("BONES_APP_INSTALL_DIR"),
        launcher_stem: option_env!("UPGRADER_LAUNCHER_STEM").unwrap_or("bones-app"),
        app_stem: option_env!("UPGRADER_APP_STEM").unwrap_or("bones-app-bin"),
    }
}

/// The application version this launcher was built alongside, or `unknown`
/// when the host did not stamp one.
///
/// The launcher cannot use its own crate version: it is built by the
/// upgrader, whose version says nothing about the application it starts.
/// `UPGRADER_LAUNCHER_VERSION` is read at compile time from the same
/// `[env]` block as the identity, and this repository sets it beside the
/// app's own version -- see the version-bump rule in `AGENTS.md`, which
/// moves both together.
///
/// `unknown` is deliberately a visible answer rather than a silent
/// fallback: a launcher that cannot say what it is tells you as much, which
/// is the whole reason `--version` reports it.
pub fn launcher_version() -> &'static str {
    match option_env!("UPGRADER_LAUNCHER_VERSION") {
        Some(version) if !version.is_empty() => version,
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> AppIdentity {
        AppIdentity {
            install_dir_name: ".other",
            install_env_var: "OTHER_INSTALL_DIR",
            launcher_stem: "other",
            app_stem: "other-app",
        }
    }

    #[test]
    fn executable_names_carry_the_platform_extension() {
        let identity = identity();

        if cfg!(windows) {
            assert_eq!(identity.launcher_exe(), "other.exe");
            assert_eq!(identity.app_exe(), "other-app.exe");
            assert_eq!(identity.launcher_payload_exe(), "other-launcher.exe");
        } else {
            assert_eq!(identity.launcher_exe(), "other");
            assert_eq!(identity.app_exe(), "other-app");
            assert_eq!(identity.launcher_payload_exe(), "other-launcher");
        }
    }

    #[test]
    fn the_shipped_launcher_never_shares_the_entry_points_name() {
        // The two live in one tree during an install, and what belongs in a
        // version folder is decided by name -- so a collision here would
        // silently drop the shipped launcher from every payload.
        let identity = identity();

        assert_ne!(identity.launcher_payload_exe(), identity.launcher_exe());
    }

    #[test]
    fn the_host_identity_is_whole_however_it_was_configured() {
        // The UPGRADER_* variables are read at compile time, so what this
        // sees depends on whether a host set them -- and a checkout nested
        // inside one inherits that host's `.cargo/config.toml`. Asserting
        // the placeholder values would therefore test the build environment
        // rather than the code. What must hold either way is that all four
        // names are present and usable.
        let identity = host_identity();

        assert!(identity.install_dir_name.starts_with('.'));
        assert!(!identity.install_env_var.is_empty());
        assert!(!identity.launcher_stem.is_empty());
        assert!(!identity.app_stem.is_empty());
        assert_ne!(identity.launcher_stem, identity.app_stem);
    }
}
