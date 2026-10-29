/// Closes one panel owned by the command sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosePanel<'a> {
    /// The panel to close.
    pub panel: &'a str,
}
