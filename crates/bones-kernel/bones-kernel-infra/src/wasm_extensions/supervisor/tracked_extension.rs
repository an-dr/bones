use std::path::PathBuf;
use std::time::SystemTime;

use crate::bus::{Endpoint, EndpointBudget};

use crate::wasm_extensions::loading::SharedHost;

pub struct TrackedExtension {
    pub name: String,
    pub path: PathBuf,
    pub mtime: SystemTime,
    pub endpoint: Endpoint,
    pub shared: SharedHost,
    pub budget: EndpointBudget,
    /// Set once quarantined, so a later sweep doesn't re-log/re-publish for
    /// an already-handled fault every check forever.
    pub quarantined: bool,
}
