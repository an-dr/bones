//! Watching folders and reporting what changed, after it settles.
//!
//! - **Debounced.** A burst — a `git checkout` touching thousands of files —
//!   becomes one [`Change`] per watch once the folder has been quiet for a
//!   while, not thousands of calls.
//! - **Only mutations.** Reading a file is not a change. Some platforms
//!   report reads made by a recursive watch's own scan, and passing those on
//!   made a consumer refresh, scan, and refresh again without end.
//! - **No meaning.** The library reports paths; what a changed path means —
//!   "the index changed, so status is stale" — is each application's call.
//! - **Off the caller's thread.** Watching and debouncing run on threads the
//!   [`Watcher`] owns, and the sink is called there.

mod change;
mod change_kind;
mod debounce;
mod watch_id;
mod watcher;

pub use change::Change;
pub use change_kind::ChangeKind;
pub use watch_id::WatchId;
pub use watcher::{Watcher, DEFAULT_QUIET};

#[cfg(test)]
mod tests;
