use std::path::PathBuf;

/// A program to start, with everything it needs already decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// A bare name searched on `PATH`, a path, or a path starting with `~`.
    pub program: String,
    /// Arguments, passed as given.
    pub args: Vec<String>,
    /// The folder the program starts in.
    pub cwd: PathBuf,
    /// Variables set for this program on top of the inherited environment.
    pub env: Vec<(String, String)>,
}
