use std::io::Read;
use std::time::Duration;

use crate::Fetch;
use base64::Engine;

/// A release archive is tens of megabytes; this leaves generous room while
/// still refusing a response that is plainly not one.
pub const MAX_DOWNLOAD_BYTES: u64 = 256 * 1024 * 1024;

/// How long establishing a connection may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// How long one read may stall. Per read rather than for the whole transfer,
/// so a slow link still finishes a large archive.
const READ_TIMEOUT: Duration = Duration::from_secs(60);

/// Fetches manifests and release archives over HTTPS.
///
/// - Separate from the `os` module's fetch, which caps a response at 5 MB
///   and ten seconds in total: right for what a guest fetches, wrong for a
///   release archive (ADR-037).
/// - `https://` only; anything else is refused before a connection is made.
/// - A 404 is `Ok(None)`, the "nothing published here" answer [`Fetch`]
///   defines.
pub struct HttpsFetch;

impl Fetch for HttpsFetch {
    fn fetch_url(&self, url: &str) -> Result<Option<String>, String> {
        if !url.starts_with("https://") {
            return Err("only https URLs may be fetched".into());
        }
        // native-tls (Schannel on Windows) rather than rustls: the Windows
        // toolchains this builds on cannot compile rustls's usual ring backend.
        let connector = native_tls::TlsConnector::new().map_err(|error| error.to_string())?;
        let agent = ureq::builder()
            .tls_connector(std::sync::Arc::new(connector))
            .timeout_connect(CONNECT_TIMEOUT)
            .timeout_read(READ_TIMEOUT)
            .build();
        let response = match agent.get(url).call() {
            Ok(response) => response,
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let content_type = response.content_type().to_string();
        let mut body = Vec::new();
        // One byte past the cap, so an oversized body is an error rather than
        // a silently truncated archive.
        response
            .into_reader()
            .take(MAX_DOWNLOAD_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|error| error.to_string())?;
        if body.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err(format!("download exceeds {MAX_DOWNLOAD_BYTES} bytes"));
        }
        Ok(Some(format!(
            "{content_type};base64,{}",
            base64::engine::general_purpose::STANDARD.encode(body)
        )))
    }
}
