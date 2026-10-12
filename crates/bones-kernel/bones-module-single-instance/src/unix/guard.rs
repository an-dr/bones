use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;

/// Holds the claim while it lives; dropping it stops the listener and
/// removes the socket.
pub(crate) struct Guard {
    path: PathBuf,
    inode: u64,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Guard {
    pub(super) fn new(
        path: PathBuf,
        inode: u64,
        stop: Arc<AtomicBool>,
        thread: JoinHandle<()>,
    ) -> Self {
        Self {
            path,
            inode,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        // The listener blocks in accept; a connection of our own wakes it.
        let _ = UnixStream::connect(&self.path);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        // Only our own socket: a later instance may have bound the path
        // after finding this one unresponsive.
        if std::fs::metadata(&self.path).is_ok_and(|metadata| metadata.ino() == self.inode) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
