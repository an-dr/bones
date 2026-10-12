//! Installation names supplied by the application, never used by the launcher.

/// The names one application's install is built from.
///
/// - Supply bare directory and executable names, rather than absolute paths.
/// - Executable stems omit the extension; the module adds the platform's.
/// - The application supplies this data explicitly at composition.
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

/// Neutral installation identity for module tests; production supplies its own.
#[cfg(test)]
pub(crate) fn host_identity() -> AppIdentity {
    AppIdentity {
        install_dir_name: option_env!("UPGRADER_INSTALL_DIR_NAME").unwrap_or(".bones-app"),
        install_env_var: option_env!("UPGRADER_INSTALL_ENV_VAR").unwrap_or("BONES_APP_INSTALL_DIR"),
        launcher_stem: option_env!("UPGRADER_LAUNCHER_STEM").unwrap_or("bones-app"),
        app_stem: option_env!("UPGRADER_APP_STEM").unwrap_or("bones-app-bin"),
    }
}

#[cfg(test)]
mod tests;
