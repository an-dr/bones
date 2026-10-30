/// The desktop capabilities the module offers, behind one trait so a host can
/// substitute its own.
///
/// Methods returning an optional value use `Ok(None)` for a cancelled
/// dialog or a URL holding nothing. `Err` reports a failed operation.
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
