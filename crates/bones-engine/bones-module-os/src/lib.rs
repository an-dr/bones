//! The `os` native module: desktop capabilities on behalf of a sandboxed guest.
//!
//! An extension has no clipboard, no browser and no file dialog, because the
//! sandbox that makes it safe to load also cuts it off from the machine. This
//! module is the trusted side of that split: it subscribes to `os/request`,
//! performs the action, and answers on `os/result`.
//!
//! Work runs on its own thread per request. A file dialog blocks until the
//! user chooses, and a fetch until the remote answers; neither may stall the
//! engine, so replies arrive when the work finishes rather than in the order
//! asked. The request id is what pairs an answer with its question.
//!
//! [`OsBackend`] is the seam. The module owns the bus protocol and nothing
//! else, so a host can swap in a backend of its own -- a test one that opens
//! no dialogs, or a platform one this crate does not cover -- without
//! reimplementing any of the plumbing.

use std::io::Read;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use base64::Engine;
use bones_kernel::bus::{Bus, Envelope, Handler, Module, ModuleContext};
use bones_messages::os::{Action, Request, Result as OsResult, ENDPOINT};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

/// Largest body accepted from [`OsBackend::fetch_url`]: an avatar-sized image
/// or a small manifest, never a bulk download.
pub const MAX_FETCH_BYTES: u64 = 5 * 1024 * 1024;

/// How long [`OsBackend::fetch_url`] waits before giving up, so a stalled
/// remote never holds a request thread indefinitely.
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

/// The desktop capabilities the module offers, behind one trait so a host can
/// substitute its own.
///
/// Every method reports an ordinary "nothing" outcome as `Ok(None)` rather
/// than an error: a cancelled dialog and a URL holding nothing are answers,
/// not failures. `Err` is for something actually going wrong.
pub trait OsBackend: Send + Sync {
    fn read_clipboard(&self) -> Result<String, String>;
    fn write_clipboard(&self, value: &str) -> Result<(), String>;
    /// Opens a URL in the user's browser. Implementations are expected to
    /// refuse a scheme they do not trust rather than hand an arbitrary string
    /// to the shell.
    fn open_url(&self, value: &str) -> Result<(), String>;
    /// Reveals a directory in the desktop's file manager (Explorer, Finder, or
    /// whatever the session configures on Linux).
    fn open_directory(&self, path: &str) -> Result<(), String>;
    fn pick_file(&self, title: &str) -> Result<Option<String>, String>;
    fn pick_folder(&self, title: &str) -> Result<Option<String>, String>;
    /// Fetches an HTTPS URL, answering `"{content-type};base64,{data}"` -- a
    /// self-describing shape the caller can turn into a data URI or decode
    /// directly. A 404 is `Ok(None)`, a normal "nothing at this URL"; network
    /// failures, other non-2xx statuses and oversized bodies are `Err`.
    fn fetch_url(&self, url: &str) -> Result<Option<String>, String>;
}

/// The real desktop, through `arboard`, `open`, `rfd` and `ureq`.
pub struct SystemOsBackend;

impl OsBackend for SystemOsBackend {
    fn read_clipboard(&self) -> Result<String, String> {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.get_text())
            .map_err(|error| error.to_string())
    }
    fn write_clipboard(&self, value: &str) -> Result<(), String> {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(value))
            .map_err(|error| error.to_string())
    }
    fn open_url(&self, value: &str) -> Result<(), String> {
        if !value.starts_with("https://")
            && !value.starts_with("http://")
            && !value.starts_with("mailto:")
        {
            return Err("unsupported external URL scheme".into());
        }
        open::that(value).map_err(|error| error.to_string())
    }
    fn open_directory(&self, path: &str) -> Result<(), String> {
        if !std::path::Path::new(path).is_dir() {
            return Err(format!("{path} is not a directory"));
        }
        open::that(path).map_err(|error| error.to_string())
    }
    fn pick_file(&self, title: &str) -> Result<Option<String>, String> {
        Ok(rfd::FileDialog::new()
            .set_title(title)
            .pick_file()
            .map(|path| path.to_string_lossy().into_owned()))
    }
    fn pick_folder(&self, title: &str) -> Result<Option<String>, String> {
        Ok(rfd::FileDialog::new()
            .set_title(title)
            .pick_folder()
            .map(|path| path.to_string_lossy().into_owned()))
    }
    fn fetch_url(&self, url: &str) -> Result<Option<String>, String> {
        if !url.starts_with("https://") {
            return Err("only https URLs may be fetched".into());
        }
        // A Windows target's Rust toolchain here has no working C compiler for
        // rustls's usual ring backend, so this goes through native-tls
        // (Schannel) instead, built fresh per call rather than cached: it is
        // cheap local setup, not a network round trip.
        let connector = native_tls::TlsConnector::new().map_err(|error| error.to_string())?;
        let agent = ureq::builder()
            .tls_connector(std::sync::Arc::new(connector))
            .timeout(FETCH_TIMEOUT)
            .build();
        let response = match agent.get(url).call() {
            Ok(response) => response,
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let content_type = response.content_type().to_string();
        let mut body = Vec::new();
        // One byte past the cap: reading exactly MAX_FETCH_BYTES would silently
        // truncate an oversized body into invalid, corrupt-looking data instead
        // of a clean error.
        response
            .into_reader()
            .take(MAX_FETCH_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|error| error.to_string())?;
        if body.len() as u64 > MAX_FETCH_BYTES {
            return Err(format!("response exceeds {MAX_FETCH_BYTES} bytes"));
        }
        Ok(Some(format!(
            "{content_type};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(body)
        )))
    }
}

/// The bus module: `os/request` in, `os/result` out.
pub struct OsModule {
    bus: Option<Bus>,
    backend: Arc<dyn OsBackend>,
}

impl OsModule {
    pub fn new(backend: Arc<dyn OsBackend>) -> Self {
        Self { bus: None, backend }
    }

    /// Runs one request off the bus thread and answers when it finishes.
    fn start(&self, request_id: u32, action: Action, value: String) {
        let Some(bus) = self.bus.clone() else { return };
        let backend = self.backend.clone();
        thread::spawn(move || {
            let outcome = execute(backend.as_ref(), action, &value);
            let (accepted, value, error) = match outcome {
                Ok(Some(value)) => (true, value, String::new()),
                Ok(None) => (false, String::new(), String::new()),
                Err(error) => (false, String::new(), error),
            };
            bus.publish(Envelope {
                topic: OsResult::TOPIC.into(),
                sender: ENDPOINT.into(),
                correlation: Some(u64::from(request_id)),
                payload: OsResult {
                    request_id,
                    accepted,
                    value: &value,
                    error: &error,
                }
                .encode(),
            });
        });
    }
}

impl Default for OsModule {
    fn default() -> Self {
        Self::new(Arc::new(SystemOsBackend))
    }
}

impl Handler for OsModule {
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic != Request::TOPIC {
            return;
        }
        if let Ok(request) = Request::decode(&envelope.payload) {
            self.start(
                request.request_id,
                request.action,
                request.value.to_string(),
            );
        }
    }
}

impl Module for OsModule {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        context.subscribe(Request::TOPIC);
        self.bus = context.get_service::<Bus>().cloned();
        self.bus
            .as_ref()
            .map(|_| ())
            .ok_or_else(|| "no Bus service available".into())
    }
}

/// Dispatches one action to the backend.
///
/// `Ok(None)` is the "nothing, and nothing wrong" answer: a cancelled dialog
/// or a URL holding nothing. Actions with no result of their own answer with
/// an empty string so the caller still learns they completed.
fn execute(backend: &dyn OsBackend, action: Action, value: &str) -> Result<Option<String>, String> {
    match action {
        Action::ReadClipboard => backend.read_clipboard().map(Some),
        Action::WriteClipboard => backend.write_clipboard(value).map(|_| Some(String::new())),
        Action::OpenUrl => backend.open_url(value).map(|_| Some(String::new())),
        Action::PickFile => backend.pick_file(value),
        Action::PickFolder => backend.pick_folder(value),
        Action::OpenDirectory => backend.open_directory(value).map(|_| Some(String::new())),
        Action::FetchUrl => backend.fetch_url(value),
    }
}

#[cfg(test)]
mod tests;
