/// What [`crate::UpgradeModule::install_companion`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The archive was installed as `version`.
    Installed { version: String },
    /// The companion was already installed and the policy left it alone.
    /// `version` is its current version folder's name, or `unknown`.
    AlreadyPresent { version: String },
    /// The policy chose not to install, for the reason given.
    Skipped { reason: String },
}
