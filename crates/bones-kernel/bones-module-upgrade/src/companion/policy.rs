/// When [`crate::UpgradeModule::install_companion`] installs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Only when the companion is not installed. An existing install is
    /// never touched, newer or older, because the companion updates itself.
    IfAbsent,
    /// When the companion is not installed, or its installed version is older
    /// than the archive's. An existing install gains a new version folder, as
    /// a self-update would give it.
    IfNewer,
}
