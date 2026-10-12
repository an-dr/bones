use std::path::PathBuf;

use crate::ChangeKind;

/// Everything that changed under one watch during one burst.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Change {
    /// Every path that changed, each once, in the order first reported.
    pub paths: Vec<PathBuf>,
    /// Every kind of change seen during the burst, each once. Not parallel
    /// to `paths`: a burst that created one file and removed another reports
    /// both kinds and both paths.
    pub kinds: Vec<ChangeKind>,
}

impl Change {
    /// Adds one observation, keeping both lists free of repeats.
    pub(crate) fn record(&mut self, path: PathBuf, kind: ChangeKind) {
        if !self.paths.contains(&path) {
            self.paths.push(path);
        }
        if !self.kinds.contains(&kind) {
            self.kinds.push(kind);
        }
    }
}
