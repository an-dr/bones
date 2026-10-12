/// The one capability this crate needs from its host: retrieving a URL.
///
/// Narrower than the OS surface a desktop application usually has, and
/// deliberately so -- self-update is otherwise pure filesystem and version
/// arithmetic, and a crate that asked for clipboards and file pickers to do it
/// could not be reused by a host that has neither. A test satisfies this with a
/// map of canned responses.
pub trait Fetch {
    /// Retrieves `url`, returning the body as `"{content-type};base64,{data}"`.
    ///
    /// `Ok(None)` means the URL held nothing -- a manifest that has not been
    /// published yet is a normal answer, not a failure. Network and protocol
    /// errors are `Err`.
    fn fetch_url(&self, url: &str) -> Result<Option<String>, String>;
}
