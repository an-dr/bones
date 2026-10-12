use std::sync::Arc;

use crate::{AppIdentity, Fetch, ReleaseSource};

/// What the upgrade module updates from, fixed when the engine is
/// composed.
pub struct UpgradeConfig {
    /// Installation identity supplied by the application.
    pub identity: AppIdentity,
    /// Where releases are published, including any override the user set —
    /// resolved with [`ReleaseSource::with_override`] when the application
    /// starts.
    pub source: ReleaseSource,
    /// The running application's version, compared against the manifest and
    /// recorded for "updated to X".
    pub current_version: String,
    /// What downloads manifests and archives. `None` uses [`crate::HttpsFetch`];
    /// a test or an embedder with its own transport passes one.
    pub fetch: Option<Arc<dyn Fetch + Send + Sync>>,
}

impl UpgradeConfig {
    /// Updates from `source`, as `current_version`, over HTTPS.
    pub fn new(
        identity: AppIdentity,
        source: ReleaseSource,
        current_version: impl Into<String>,
    ) -> Self {
        Self {
            identity,
            source,
            current_version: current_version.into(),
            fetch: None,
        }
    }

    /// Downloads through `fetch` instead of HTTPS.
    pub fn fetch(mut self, fetch: Arc<dyn Fetch + Send + Sync>) -> Self {
        self.fetch = Some(fetch);
        self
    }
}
