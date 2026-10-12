/// What happened to a path. A rename arrives as [`ChangeKind::Removed`] for
/// the old path and [`ChangeKind::Created`] for the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    Created,
    Modified,
    Removed,
}
