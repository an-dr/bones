use std::fs::File;
use std::io::ErrorKind;
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

use super::{runtime_dir, Listener};
use crate::attempt::Attempt;

/// Binds the socket, or connects to the process that did.
///
/// The whole decision runs under an exclusive lock on a file beside the
/// socket. Without it, two launches could each find a stale socket, each
/// remove it, and each bind a fresh one — the second removing the first's —
/// leaving two instances. The lock is released when this returns, and by the
/// kernel infrastructure if the process dies holding it.
pub(crate) fn attempt(app_uid: &str, _timeout: Duration) -> Result<Attempt, String> {
    let (socket, lock_path) = runtime_dir::paths(app_uid)?;
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|error| format!("opening {}: {error}", lock_path.display()))?;
    // SAFETY: the descriptor is live for the duration of the call.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return Err(format!(
            "locking {}: {}",
            lock_path.display(),
            std::io::Error::last_os_error()
        ));
    }
    match bind(&socket)? {
        Some(listener) => Ok(Attempt::Claimed(listener)),
        None => match UnixStream::connect(&socket) {
            Ok(stream) => Ok(Attempt::Taken(stream)),
            // Nothing accepts on it: the instance that bound it is gone.
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::ConnectionRefused | ErrorKind::NotFound
                ) =>
            {
                std::fs::remove_file(&socket)
                    .or_else(|error| match error.kind() {
                        ErrorKind::NotFound => Ok(()),
                        _ => Err(error),
                    })
                    .map_err(|error| format!("removing a stale {}: {error}", socket.display()))?;
                Ok(match bind(&socket)? {
                    Some(listener) => Attempt::Claimed(listener),
                    None => Attempt::Stale,
                })
            }
            Err(_) => Ok(Attempt::Unavailable),
        },
    }
}

/// Binds `socket`, or reports `None` when a socket file is already there.
fn bind(socket: &Path) -> Result<Option<Listener>, String> {
    match UnixListener::bind(socket) {
        Ok(listener) => Listener::new(socket.to_path_buf(), listener).map(Some),
        Err(error) if error.kind() == ErrorKind::AddrInUse => Ok(None),
        Err(error) => Err(format!("binding {}: {error}", socket.display())),
    }
}

/// Nothing to grant: X11 and Wayland compositors decide focus themselves.
pub(crate) fn allow_foreground(_pid: u32) {}
