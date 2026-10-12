#[cfg(any(target_os = "linux", windows))]
use std::path::PathBuf;

/// Where a registration is written.
///
/// Callers normally take [`Locations::for_current_user`]; tests point every
/// field at a scratch directory or registry key so nothing reaches the real
/// desktop.
#[derive(Debug, Clone)]
pub struct Locations {
    /// The freedesktop.org data directory (`$XDG_DATA_HOME`).
    #[cfg(target_os = "linux")]
    pub data_home: PathBuf,
    /// Whether to refresh the desktop's caches after a change. Off in tests,
    /// where the caches being refreshed would be the real ones.
    #[cfg(target_os = "linux")]
    pub refresh_caches: bool,
    /// Registry key under `HKEY_CURRENT_USER` that holds file classes.
    #[cfg(windows)]
    pub classes_key: String,
    /// The Start Menu's `Programs` folder.
    #[cfg(windows)]
    pub start_menu: PathBuf,
    /// The desktop folder.
    #[cfg(windows)]
    pub desktop: PathBuf,
}

impl Locations {
    /// The current user's own locations, or `None` when they cannot be
    /// resolved (no home directory, or an operating system this crate does
    /// not support).
    pub fn for_current_user() -> Option<Self> {
        #[cfg(target_os = "linux")]
        return Some(Self {
            data_home: dirs::data_dir()?,
            refresh_caches: true,
        });
        #[cfg(windows)]
        return Some(Self {
            classes_key: String::from(r"Software\Classes"),
            start_menu: dirs::data_dir()?.join(r"Microsoft\Windows\Start Menu\Programs"),
            desktop: dirs::desktop_dir()?,
        });
        #[cfg(not(any(target_os = "linux", windows)))]
        return None;
    }
}
