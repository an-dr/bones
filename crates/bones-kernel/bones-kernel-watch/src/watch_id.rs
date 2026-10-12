/// Names one [`crate::Watcher::watch`] call, so the sink can tell watches
/// apart and the caller can stop one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WatchId(pub(crate) u64);
