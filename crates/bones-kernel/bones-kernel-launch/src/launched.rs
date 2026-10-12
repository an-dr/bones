/// How [`crate::launch`] ended, when nothing went wrong starting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launched {
    /// The program is running, detached from this process.
    Started,
    /// No program by that name was found; `searched` is the name looked for,
    /// so a caller can say which tool to configure.
    NotFound { searched: String },
}
