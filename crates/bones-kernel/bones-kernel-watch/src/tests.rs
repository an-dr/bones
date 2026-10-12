use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use notify::event::{AccessKind, CreateKind, ModifyKind, RenameMode};
use notify::{Event, EventKind};

use super::*;
use crate::watcher::classify;

const QUIET: Duration = Duration::from_millis(100);
/// How long a test waits for a change that should come.
const EXPECT: Duration = Duration::from_secs(5);
/// How long a test waits to be sure a change does not come.
const SILENCE: Duration = Duration::from_millis(600);

fn watched(dir: &Path) -> (Watcher, WatchId, Receiver<(WatchId, Change)>) {
    let (sender, changes) = mpsc::channel();
    let mut watcher = Watcher::new(QUIET, move |id, change| {
        let _ = sender.send((id, change));
    })
    .unwrap();
    let id = watcher.watch(dir, true).unwrap();
    (watcher, id, changes)
}

/// Collects every change until the watch stays quiet for [`SILENCE`].
fn drain(changes: &Receiver<(WatchId, Change)>) -> Vec<Change> {
    let mut seen = Vec::new();
    if let Ok((_, change)) = changes.recv_timeout(EXPECT) {
        seen.push(change);
    }
    while let Ok((_, change)) = changes.recv_timeout(SILENCE) {
        seen.push(change);
    }
    seen
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap()
}

fn mentions(changes: &[Change], path: &Path, kind: ChangeKind) -> bool {
    let path = canonical(path.parent().unwrap()).join(path.file_name().unwrap());
    changes.iter().any(|change| {
        change.kinds.contains(&kind)
            && change
                .paths
                .iter()
                .any(|seen| seen == &path || canonical_parent(seen) == path)
    })
}

/// A reported path with its folder canonicalised, so `C:\Users\…` and an
/// `8.3` short form compare equal.
fn canonical_parent(path: &Path) -> PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => std::fs::canonicalize(parent)
            .map(|parent| parent.join(name))
            .unwrap_or_else(|_| path.to_path_buf()),
        _ => path.to_path_buf(),
    }
}

#[test]
fn creating_modifying_and_removing_are_each_reported() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.txt");
    let (_watcher, _id, changes) = watched(dir.path());

    std::fs::write(&file, b"one").unwrap();
    let created = drain(&changes);
    assert!(
        mentions(&created, &file, ChangeKind::Created),
        "{created:?}"
    );

    std::fs::write(&file, b"two").unwrap();
    let modified = drain(&changes);
    assert!(
        mentions(&modified, &file, ChangeKind::Modified),
        "{modified:?}"
    );

    std::fs::remove_file(&file).unwrap();
    let removed = drain(&changes);
    assert!(
        mentions(&removed, &file, ChangeKind::Removed),
        "{removed:?}"
    );
}

#[test]
fn a_burst_of_a_thousand_writes_is_a_handful_of_changes() {
    let dir = tempfile::tempdir().unwrap();
    let (_watcher, _id, changes) = watched(dir.path());

    for index in 0..1000 {
        std::fs::write(dir.path().join(format!("{index}.txt")), b"x").unwrap();
    }

    let seen = drain(&changes);
    assert!(!seen.is_empty());
    assert!(seen.len() <= 5, "{} changes for one burst", seen.len());
    let paths: usize = seen.iter().map(|change| change.paths.len()).sum();
    assert!(paths >= 1000, "only {paths} paths reported");
}

#[test]
fn unwatching_stops_the_changes() {
    let dir = tempfile::tempdir().unwrap();
    let (mut watcher, id, changes) = watched(dir.path());

    watcher.unwatch(id);
    std::fs::write(dir.path().join("after.txt"), b"x").unwrap();

    assert!(changes.recv_timeout(SILENCE).is_err());
}

#[test]
fn two_watches_report_under_their_own_ids() {
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    let (sender, changes) = mpsc::channel();
    let mut watcher = Watcher::new(QUIET, move |id, change| {
        let _ = sender.send((id, change));
    })
    .unwrap();
    let _first = watcher.watch(one.path(), true).unwrap();
    let second = watcher.watch(two.path(), true).unwrap();

    std::fs::write(two.path().join("x.txt"), b"x").unwrap();

    let (id, _) = changes.recv_timeout(EXPECT).unwrap();
    assert_eq!(id, second);
}

#[test]
fn reading_is_never_a_change() {
    let event = Event::new(EventKind::Access(AccessKind::Read)).add_path("/a".into());

    assert!(classify(event).is_empty());
}

#[test]
#[cfg(target_os = "linux")]
fn reading_files_on_linux_reports_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("file.txt");
    std::fs::write(&file, b"content").unwrap();
    let (_watcher, _id, changes) = watched(dir.path());

    for _ in 0..20 {
        let _ = std::fs::read(&file).unwrap();
        let _ = std::fs::read_dir(dir.path()).unwrap().count();
    }

    assert!(changes.recv_timeout(SILENCE).is_err());
}

#[test]
fn a_paired_rename_is_a_removal_and_a_creation() {
    let event = Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
        .add_path("/old".into())
        .add_path("/new".into());

    assert_eq!(
        classify(event),
        [
            (PathBuf::from("/old"), ChangeKind::Removed),
            (PathBuf::from("/new"), ChangeKind::Created)
        ]
    );
}

#[test]
fn rename_halves_are_a_removal_and_a_creation() {
    let from =
        Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::From))).add_path("/old".into());
    let to =
        Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::To))).add_path("/new".into());

    assert_eq!(
        classify(from),
        [(PathBuf::from("/old"), ChangeKind::Removed)]
    );
    assert_eq!(classify(to), [(PathBuf::from("/new"), ChangeKind::Created)]);
}

#[test]
fn creation_is_created() {
    let event = Event::new(EventKind::Create(CreateKind::File)).add_path("/a".into());

    assert_eq!(
        classify(event),
        [(PathBuf::from("/a"), ChangeKind::Created)]
    );
}
