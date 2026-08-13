/// Closes one panel owned by the command sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosePanel<'a> {
    pub panel: &'a str,
}
