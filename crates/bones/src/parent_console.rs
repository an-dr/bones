//! Reattaches a windowed build to the console it was launched from, so the
//! engine's own log output and a fatal error are not lost.
//!
//! - A `windows_subsystem = "windows"` binary is given no console, which is
//!   what stops one appearing behind the window when it is launched from a
//!   shell folder or a shortcut.
//! - Launched *from* a terminal, that terminal's console is still there to be
//!   attached to, and an operator expects to see output in it.
//! - Attaching only ever adopts an existing console; it never creates one, so
//!   the windowed case stays windowed.

/// `ATTACH_PARENT_PROCESS` from the Win32 console API.
const ATTACH_PARENT_PROCESS: u32 = u32::MAX;

#[link(name = "kernel32")]
extern "system" {
    fn AttachConsole(process_id: u32) -> i32;
}

/// Adopts the launching terminal's console when there is one.
///
/// Called before anything writes to the standard streams, because Rust opens
/// its handles to them lazily on first use.
pub fn attach() {
    // SAFETY: no arguments to validate, and the only outcome is whether this
    // process now has a console; failure means it does not, which is the
    // windowed case and needs no handling.
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}
