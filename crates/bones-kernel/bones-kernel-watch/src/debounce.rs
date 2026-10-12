//! The thread that turns raw events into settled changes.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use crate::{Change, ChangeKind, WatchId};

/// What reaches the debounce thread.
pub(crate) enum Message {
    /// A watch began; its events are delivered from now on.
    Started(WatchId),
    /// Mutations a watch observed.
    Observed(WatchId, Vec<(PathBuf, ChangeKind)>),
    /// A watch ended; anything pending or still arriving for it is dropped.
    Forgotten(WatchId),
}

/// Coalesces observations per watch and hands each to `sink` once its watch
/// has been quiet for `quiet`. Returns when every sender is gone.
pub(crate) fn run(quiet: Duration, messages: Receiver<Message>, sink: impl Fn(WatchId, Change)) {
    let mut alive = HashSet::new();
    let mut pending: HashMap<WatchId, (Change, Instant)> = HashMap::new();
    loop {
        let wait = pending
            .values()
            .map(|(_, last)| (*last + quiet).saturating_duration_since(Instant::now()))
            .min()
            .unwrap_or(Duration::from_secs(3600));
        match messages.recv_timeout(wait) {
            Ok(Message::Started(id)) => {
                alive.insert(id);
            }
            Ok(Message::Observed(id, observed)) if alive.contains(&id) => {
                let (change, last) = pending
                    .entry(id)
                    .or_insert_with(|| (Change::default(), Instant::now()));
                for (path, kind) in observed {
                    change.record(path, kind);
                }
                *last = Instant::now();
            }
            Ok(Message::Observed(..)) => {}
            Ok(Message::Forgotten(id)) => {
                alive.remove(&id);
                pending.remove(&id);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let now = Instant::now();
        let settled: Vec<WatchId> = pending
            .iter()
            .filter(|(_, (_, last))| *last + quiet <= now)
            .map(|(id, _)| *id)
            .collect();
        for id in settled {
            if let Some((change, _)) = pending.remove(&id) {
                sink(id, change);
            }
        }
    }
}
