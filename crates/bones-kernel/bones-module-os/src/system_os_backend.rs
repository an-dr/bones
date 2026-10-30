use crate::OsBackend;
use base64::Engine;
use std::io::Read;
use std::time::Duration;

/// Largest body accepted from [`OsBackend::fetch_url`]: an avatar-sized image
/// or a small manifest, never a bulk download.
pub const MAX_FETCH_BYTES: u64 = 5 * 1024 * 1024;

/// How long [`OsBackend::fetch_url`] waits before giving up, so a stalled
/// remote never holds a request thread indefinitely.
pub const FETCH_TIMEOUT: Duration = Duration::from_secs(10);

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
