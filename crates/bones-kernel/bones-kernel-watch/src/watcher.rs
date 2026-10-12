use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use notify::event::{ModifyKind, RenameMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher as _};

use crate::debounce::{self, Message};
use crate::{Change, ChangeKind, WatchId};

/// How long a watch must be quiet before its burst is reported, unless the
/// caller chooses otherwise.
pub const DEFAULT_QUIET: Duration = Duration::from_millis(200);

/// Owns the operating system's watches and the thread that debounces them.
/// Dropping it stops every watch; nothing is delivered afterwards.
pub struct Watcher {
    watches: HashMap<WatchId, RecommendedWatcher>,
    next_id: u64,
    // `Option` so `drop` can close the channel before joining the thread
    // that reads it.
    messages: Option<Sender<Message>>,
    thread: Option<JoinHandle<()>>,
}

impl Watcher {
    /// Starts the debounce thread. `sink` is called on that thread, once per
    /// settled burst per watch, after `quiet` without a new event; a sink
    /// that hands the change to a bus should post and return.
    pub fn new(
        quiet: Duration,
        sink: impl Fn(WatchId, Change) + Send + 'static,
    ) -> Result<Self, String> {
        let (messages, receiver) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("bones-kernel-watch".into())
            .spawn(move || debounce::run(quiet, receiver, sink))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            watches: HashMap::new(),
            next_id: 0,
            messages: Some(messages),
            thread: Some(thread),
        })
    }

    /// Watches `path` — recursively when `recursive` — and returns the id its
    /// changes are reported under.
    ///
    /// A recursive watch walks the whole tree on some platforms, which takes
    /// seconds for a large one; call this off any thread that must stay
    /// responsive.
    pub fn watch(&mut self, path: &Path, recursive: bool) -> Result<WatchId, String> {
        let messages = self
            .messages
            .clone()
            .ok_or_else(|| String::from("the watcher has stopped"))?;
        let id = WatchId(self.next_id);
        self.next_id += 1;
        // Announced before the operating system's watch exists, so the first
        // event it reports already has somewhere to go.
        let _ = messages.send(Message::Started(id));
        let events = messages.clone();
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        let started = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else { return };
            let observed = classify(event);
            if !observed.is_empty() {
                let _ = events.send(Message::Observed(id, observed));
            }
        })
        .and_then(|mut watcher| watcher.watch(path, mode).map(|()| watcher));
        match started {
            Ok(watcher) => {
                self.watches.insert(id, watcher);
                Ok(id)
            }
            Err(error) => {
                // The id was announced; retire it so nothing waits on it.
                let _ = messages.send(Message::Forgotten(id));
                Err(format!("watching {}: {error}", path.display()))
            }
        }
    }

    /// Stops watch `id`. A change still settling for it is dropped, not
    /// delivered.
    pub fn unwatch(&mut self, id: WatchId) {
        if let Some(messages) = &self.messages {
            let _ = messages.send(Message::Forgotten(id));
        }
        // Dropped after the message: an event racing the drop is then
        // already marked as belonging to a forgotten watch.
        self.watches.remove(&id);
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        // The operating system's watches hold senders too; all of them must
        // go before the debounce thread sees its channel close.
        self.watches.clear();
        self.messages = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// The mutations one raw event describes.
///
/// - Access is never a mutation, whatever reported it; this is the filter
///   that keeps a recursive watch's own scan from reporting itself.
/// - A rename is a removal of the old path and a creation of the new one.
/// - A rename the platform could not pair is judged by whether the path
///   still exists.
pub(crate) fn classify(event: notify::Event) -> Vec<(PathBuf, ChangeKind)> {
    let kind = match event.kind {
        EventKind::Access(_) => return Vec::new(),
        EventKind::Create(_) => ChangeKind::Created,
        EventKind::Remove(_) => ChangeKind::Removed,
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => ChangeKind::Removed,
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => ChangeKind::Created,
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            let mut paths = event.paths.into_iter();
            return paths
                .next()
                .map(|from| (from, ChangeKind::Removed))
                .into_iter()
                .chain(paths.next().map(|to| (to, ChangeKind::Created)))
                .collect();
        }
        EventKind::Modify(ModifyKind::Name(_)) => {
            return event
                .paths
                .into_iter()
                .map(|path| {
                    let kind = if path.exists() {
                        ChangeKind::Created
                    } else {
                        ChangeKind::Removed
                    };
                    (path, kind)
                })
                .collect();
        }
        EventKind::Modify(_) | EventKind::Any | EventKind::Other => ChangeKind::Modified,
    };
    event.paths.into_iter().map(|path| (path, kind)).collect()
}
