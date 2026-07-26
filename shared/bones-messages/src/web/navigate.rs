/// Navigates one panel owned by the command sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Navigate<'a> {
    pub panel: &'a str,
    pub url: &'a str,
}
