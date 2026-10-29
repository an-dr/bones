use super::PanelSource;

/// Opens one owner-scoped panel with inline HTML or a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenPanel<'a> {
    /// The caller's name for the panel, unique per owner.
    pub panel: &'a str,
    /// The page to show.
    pub source: PanelSource<'a>,
}
