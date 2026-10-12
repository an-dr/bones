//! The Win32 calls behind the claim, each returning a value or an error
//! fit to report.

use std::fs::File;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::time::{Duration, Instant};

use windows::core::{HSTRING, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, GENERIC_READ,
    GENERIC_WRITE, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, WIN32_ERROR,
};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::Win32::Security::{
    EqualSid, GetTokenInformation, TokenUser, PSID, TOKEN_QUERY, TOKEN_USER,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_SHARE_NONE,
    OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows::Win32::System::Pipes::{
    CreateNamedPipeW, GetNamedPipeServerProcessId, WaitNamedPipeW, PIPE_READMODE_BYTE,
    PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// Why a connection to an existing pipe failed.
pub(super) enum ConnectError {
    /// No pipe by that name exists any more: its owner has gone.
    Missing,
    /// Every instance stayed busy past the timeout, or the open failed.
    Failed,
}

/// The pipe name for `app_uid`, scoped to the current user.
pub(super) fn name(app_uid: &str) -> Result<String, String> {
    Ok(format!(
        r"\\.\pipe\bones-single-instance-{}-{app_uid}",
        current_user_sid()?
    ))
}

/// Creates one instance of the pipe. With `first`, creation fails with
/// `ERROR_ACCESS_DENIED` when the pipe already exists, which is the claim.
pub(super) fn create(name: &str, first: bool) -> Result<File, WIN32_ERROR> {
    let mut open_mode = PIPE_ACCESS_DUPLEX;
    if first {
        open_mode |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    // SAFETY: every pointer argument is either a live HSTRING or None.
    let handle = unsafe {
        CreateNamedPipeW(
            &HSTRING::from(name),
            open_mode,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            4096,
            4096,
            0,
            None,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        // SAFETY: reads this thread's last error, set by the call above.
        return Err(unsafe { GetLastError() });
    }
    // SAFETY: the handle is valid and owned by nothing else.
    Ok(unsafe { File::from_raw_handle(handle.0) })
}

/// Opens a client end of the pipe, waiting up to `timeout` while every
/// instance is busy serving someone else.
pub(super) fn connect(name: &str, timeout: Duration) -> Result<File, ConnectError> {
    let deadline = Instant::now() + timeout;
    let wide = HSTRING::from(name);
    loop {
        // SAFETY: `wide` outlives the call; no security attributes or
        // template handle are passed.
        let opened = unsafe {
            CreateFileW(
                &wide,
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_NONE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES(0),
                None,
            )
        };
        match opened {
            // SAFETY: a successful open returns a handle nothing else owns.
            Ok(handle) => return Ok(unsafe { File::from_raw_handle(handle.0) }),
            Err(error) if error.code() == ERROR_FILE_NOT_FOUND.to_hresult() => {
                return Err(ConnectError::Missing)
            }
            Err(error) if error.code() == ERROR_PIPE_BUSY.to_hresult() => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                let millis = u32::try_from(remaining.as_millis()).unwrap_or(u32::MAX);
                // SAFETY: `wide` outlives the call.
                if millis == 0 || !unsafe { WaitNamedPipeW(&wide, millis) }.as_bool() {
                    return Err(ConnectError::Failed);
                }
            }
            Err(_) => return Err(ConnectError::Failed),
        }
    }
}

/// Whether the process serving `pipe` runs as the current user.
///
/// Anything that cannot be confirmed counts as "no": handing a launch to a
/// pipe another account created would give it the arguments and leave the
/// user with no window.
pub(super) fn server_is_same_user(pipe: &File) -> bool {
    let mut pid = 0;
    // SAFETY: the handle is the live client end of a pipe.
    if unsafe { GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pid) }.is_err() {
        return false;
    }
    // SAFETY: only queries the process; the handle is closed below.
    let Ok(server) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return false;
    };
    let same = with_user_sid(current_process(), |mine| {
        // SAFETY: both SIDs point into buffers alive for these closures.
        with_user_sid(server, |theirs| unsafe { EqualSid(mine, theirs) }.is_ok())
    });
    // SAFETY: closes the handle opened above, exactly once.
    let _ = unsafe { CloseHandle(server) };
    matches!(same, Ok(Ok(true)))
}

fn current_process() -> HANDLE {
    // SAFETY: a pseudo-handle that needs no closing.
    unsafe { GetCurrentProcess() }
}

fn current_user_sid() -> Result<String, String> {
    with_user_sid(current_process(), |sid| {
        let mut text = PWSTR::null();
        // SAFETY: `sid` is valid for this closure; the string is freed below.
        unsafe { ConvertSidToStringSidW(sid, &mut text) }
            .map_err(|error| format!("reading the user's SID: {error}"))?;
        // SAFETY: the call above produced a NUL-terminated string.
        let value = unsafe { text.to_string() }.map_err(|error| error.to_string());
        // SAFETY: the string was allocated by ConvertSidToStringSidW.
        unsafe { LocalFree(Some(HLOCAL(text.0.cast()))) };
        value
    })?
}

/// Runs `use_sid` with the user SID of `process`'s token. The SID points into
/// a buffer that lives only for the call, which is why this takes a closure.
fn with_user_sid<R>(process: HANDLE, use_sid: impl FnOnce(PSID) -> R) -> Result<R, String> {
    let mut token = HANDLE::default();
    // SAFETY: `token` is closed below on every path.
    unsafe { OpenProcessToken(process, TOKEN_QUERY, &mut token) }
        .map_err(|error| format!("opening a process token: {error}"))?;
    let mut len = 0;
    // SAFETY: a size query; failing with "insufficient buffer" is expected.
    let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut len) };
    // u64 rather than u8, so the buffer is aligned for TOKEN_USER.
    let mut buffer = vec![0u64; (len as usize).div_ceil(8)];
    // SAFETY: the buffer holds `len` bytes, as the size query asked.
    let filled = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            len,
            &mut len,
        )
    };
    // SAFETY: closes the token opened above, exactly once.
    let _ = unsafe { CloseHandle(token) };
    filled.map_err(|error| format!("reading a process token: {error}"))?;
    // SAFETY: GetTokenInformation wrote a TOKEN_USER at the buffer's start.
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    Ok(use_sid(sid))
}
