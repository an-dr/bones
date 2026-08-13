use super::PanelSource;

/// Opens one owner-scoped panel with inline HTML or a URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenPanel<'a> {
    pub panel: &'a str,
    pub source: PanelSource<'a>,
}
