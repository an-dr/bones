use std::path::PathBuf;

/// The application being registered.
#[derive(Debug, Clone)]
pub struct Integration<'a> {
    /// Stable machine name, used for file and key names.
    pub id: &'a str,
    /// Name shown in the application list.
    pub name: &'a str,
    /// One-line description shown beside the name.
    pub comment: &'a str,
    /// Label of the "open this folder" action.
    pub open_label: &'a str,
    /// freedesktop.org menu categories, each followed by `;` — for example
    /// `Development;` or `Development;RevisionControl;`. Linux only.
    pub categories: &'a str,
    /// Absolute path of the program to start; a folder is its only argument.
    pub executable: PathBuf,
    /// PNG bytes of the application icon.
    pub icon_png: &'a [u8],
}
