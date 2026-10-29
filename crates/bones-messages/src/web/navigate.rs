/// Navigates one panel owned by the command sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Navigate<'a> {
    /// The open panel to navigate.
    pub panel: &'a str,
    /// The URL to load.
    pub url: &'a str,
}
