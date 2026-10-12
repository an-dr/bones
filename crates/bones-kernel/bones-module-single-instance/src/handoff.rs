use std::path::PathBuf;

/// What one launch hands to the running instance: its arguments and where it
/// was started from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handoff {
    /// The arguments, without the program name, in order.
    pub args: Vec<String>,
    /// The launching process's working directory, so a relative path among
    /// `args` resolves where the user typed it rather than where the running
    /// instance happens to be.
    pub cwd: PathBuf,
}
