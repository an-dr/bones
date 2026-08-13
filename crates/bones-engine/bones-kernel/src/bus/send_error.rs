#[derive(Debug, PartialEq, Eq)]
pub enum SendError {
    UnknownEndpoint,
    /// TODO: dispatch is single-threaded today, so a target is never
    /// actually busy long enough to need waiting out — this variant exists
    /// for the contract (messaging.md) but nothing currently returns it.
    /// Meaningful once dispatch can run targets concurrently.
    Timeout,
    Cycle,
}
