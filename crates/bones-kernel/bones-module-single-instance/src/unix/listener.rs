use std::os::unix::fs::MetadataExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::Guard;
use crate::{wire, Handoff};

/// The bound socket, before anything accepts on it.
pub(crate) struct Listener {
    path: PathBuf,
    /// The socket file's inode, so dropping the claim removes this socket
    /// and never one a later instance bound at the same path.
    inode: u64,
    listener: UnixListener,
}

impl Listener {
    pub(super) fn new(path: PathBuf, listener: UnixListener) -> Result<Self, String> {
        let inode = std::fs::metadata(&path)
            .map_err(|error| format!("reading {}: {error}", path.display()))?
            .ino();
        Ok(Self {
            path,
            inode,
            listener,
        })
    }

    /// Accepts connections on a thread of its own, serving each on a thread
    /// of its own, and sends every handoff to `sender`.
    pub(crate) fn start(self, sender: Sender<Handoff>) -> Result<Guard, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let listener = self.listener;
        let thread = std::thread::Builder::new()
            .name("bones-single-instance-listener".into())
            .spawn(move || accept(&listener, &sender, &flag))
            .map_err(|error| error.to_string())?;
        Ok(Guard::new(self.path, self.inode, stop, thread))
    }
}

fn accept(listener: &UnixListener, sender: &Sender<Handoff>, stop: &AtomicBool) {
    for stream in listener.incoming() {
        if stop.load(Ordering::Acquire) {
            return;
        }
        let Ok(stream) = stream else { continue };
        let sender = sender.clone();
        let _ = std::thread::Builder::new()
            .name("bones-single-instance-connection".into())
            .spawn(move || serve(stream, sender));
    }
}

fn serve(mut stream: UnixStream, sender: Sender<Handoff>) {
    let _ = wire::serve(&mut stream, |handoff| sender.send(handoff).is_ok());
}
