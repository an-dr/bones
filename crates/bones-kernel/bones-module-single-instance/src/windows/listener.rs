use std::fs::File;
use std::os::windows::io::AsRawHandle;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

use windows::Win32::Foundation::{ERROR_PIPE_CONNECTED, HANDLE};
use windows::Win32::System::Pipes::ConnectNamedPipe;

use super::{pipe, Guard};
use crate::{wire, Handoff};

/// The claimed pipe, before anything accepts on it.
pub(crate) struct Listener {
    name: String,
    first: File,
}

impl Listener {
    pub(super) fn new(name: String, first: File) -> Self {
        Self { name, first }
    }

    /// Accepts connections on a thread of its own, serving each on a thread
    /// of its own, and sends every handoff to `sender`.
    pub(crate) fn start(self, sender: Sender<Handoff>) -> Result<Guard, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let name = self.name.clone();
        let flag = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("bones-single-instance-listener".into())
            .spawn(move || accept(&self.name, self.first, &sender, &flag))
            .map_err(|error| error.to_string())?;
        Ok(Guard::new(name, stop, thread))
    }
}

/// Waits for clients until `stop` is set.
///
/// One instance always waits: the next one is created before the connected
/// one is handed off. With no instance at all the pipe would vanish, and the
/// next launch would claim it and open a second window.
fn accept(name: &str, mut waiting: File, sender: &Sender<Handoff>, stop: &AtomicBool) {
    loop {
        // SAFETY: `waiting` is a live server instance this loop owns.
        let connected = unsafe { ConnectNamedPipe(HANDLE(waiting.as_raw_handle()), None) };
        let usable = match connected {
            Ok(()) => true,
            // A client that opened the pipe before this call is connected too.
            Err(error) => error.code() == ERROR_PIPE_CONNECTED.to_hresult(),
        };
        if stop.load(Ordering::Acquire) {
            return;
        }
        let Ok(next) = pipe::create(name, false) else {
            // No spare instance: serve this client, then let the claim go
            // rather than spin. The next launch will claim afresh.
            if usable {
                serve(waiting, sender.clone());
            }
            return;
        };
        let current = std::mem::replace(&mut waiting, next);
        if usable {
            let sender = sender.clone();
            let _ = std::thread::Builder::new()
                .name("bones-single-instance-connection".into())
                .spawn(move || serve(current, sender));
        }
    }
}

fn serve(mut stream: File, sender: Sender<Handoff>) {
    let _ = wire::serve(&mut stream, |handoff| sender.send(handoff).is_ok());
}
