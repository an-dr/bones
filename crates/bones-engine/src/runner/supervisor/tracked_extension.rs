use std::path::PathBuf;
use std::time::SystemTime;

use bones_kernel::bus::{Endpoint, EndpointBudget};

use crate::runner::loading::SharedHost;

pub(crate) struct TrackedExtension {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) mtime: SystemTime,
    pub(crate) endpoint: Endpoint,
    pub(crate) shared: SharedHost,
    pub(crate) budget: EndpointBudget,
    /// Set once quarantined, so a later sweep doesn't re-log/re-publish for
    /// an already-handled fault every check forever.
    pub(crate) quarantined: bool,
}
