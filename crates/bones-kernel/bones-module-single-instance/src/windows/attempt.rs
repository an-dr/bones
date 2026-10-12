use std::time::Duration;

use windows::Win32::Foundation::ERROR_ACCESS_DENIED;
use windows::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow;

use super::pipe::{self, ConnectError};
use super::Listener;
use crate::attempt::Attempt;

/// Creates the pipe's first instance, or connects to the process that did.
pub(crate) fn attempt(app_uid: &str, timeout: Duration) -> Result<Attempt, String> {
    let name = pipe::name(app_uid)?;
    match pipe::create(&name, true) {
        Ok(first) => return Ok(Attempt::Claimed(Listener::new(name, first))),
        // The documented answer when the pipe already exists.
        Err(error) if error == ERROR_ACCESS_DENIED => {}
        Err(error) => {
            return Err(format!(
                "creating {name}: {}",
                windows::core::Error::from(error.to_hresult()).message()
            ))
        }
    }
    Ok(match pipe::connect(&name, timeout) {
        Ok(stream) if pipe::server_is_same_user(&stream) => Attempt::Taken(stream),
        Ok(_) => Attempt::Unavailable,
        Err(ConnectError::Missing) => Attempt::Stale,
        Err(ConnectError::Failed) => Attempt::Unavailable,
    })
}

/// Lets the running instance take the foreground. Windows refuses to raise
/// a window from a process the user is not using unless the process the
/// user is using allows it, and that process is this one.
pub(crate) fn allow_foreground(pid: u32) {
    // SAFETY: no pointers; a refusal only means the window will not come
    // to the front, which the instance cannot fix either.
    let _ = unsafe { AllowSetForegroundWindow(pid) };
}
