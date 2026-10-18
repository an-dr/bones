use super::*;
use crate::bus::ServiceRegistry;

/// One scratch tree per test, so nothing depends on another's leftovers.
struct Scratch {
    root: PathBuf,
    outside: PathBuf,
}

impl Scratch {
    fn new(name: &str) -> Self {
        let base = std::env::temp_dir().join(format!("bones-files-test-{name}"));
        std::fs::remove_dir_all(&base).ok();
        let root = base.join("granted");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::write(root.join("nested/page.txt"), b"one\ntwo\n").unwrap();
        let outside = base.join("secret.txt");
        std::fs::write(&outside, b"not yours").unwrap();
        Self { root, outside }
    }

    fn files(&self, max_bytes: u64) -> Files {
        Files::new(&self.root, max_bytes)
    }

    fn cleanup(&self) {
        std::fs::remove_dir_all(self.root.parent().unwrap()).ok();
    }
}

fn read(files: &mut Files, request: &str) -> Option<Vec<u8>> {
    files.respond("extension", request.as_bytes())
}

#[test]
fn reads_a_file_inside_the_granted_root() {
    let scratch = Scratch::new("inside");
    let mut files = scratch.files(DEFAULT_MAX_BYTES);

    assert_eq!(
        read(&mut files, "nested/page.txt"),
        Some(b"one\ntwo\n".to_vec())
    );

    scratch.cleanup();
}

#[test]
fn refuses_a_path_that_leaves_the_root() {
    let scratch = Scratch::new("escape");
    let mut files = scratch.files(DEFAULT_MAX_BYTES);

    assert_eq!(read(&mut files, "../secret.txt"), None);
    assert_eq!(
        read(&mut files, scratch.outside.to_str().unwrap()),
        None,
        "an absolute path outside the root is refused too"
    );

    scratch.cleanup();
}

#[test]
fn has_nothing_to_return_for_a_missing_entry_or_a_directory() {
    let scratch = Scratch::new("absent");
    let mut files = scratch.files(DEFAULT_MAX_BYTES);

    assert_eq!(read(&mut files, "nested/missing.txt"), None);
    assert_eq!(
        read(&mut files, "nested"),
        None,
        "a directory is not a file"
    );
    assert_eq!(read(&mut files, ""), None);

    scratch.cleanup();
}

#[test]
fn refuses_a_file_over_the_size_limit() {
    let scratch = Scratch::new("oversized");
    let mut files = scratch.files(4);

    assert_eq!(read(&mut files, "nested/page.txt"), None);

    scratch.cleanup();
}

#[test]
fn ignores_a_request_that_is_not_a_path() {
    let scratch = Scratch::new("malformed");
    let mut files = scratch.files(DEFAULT_MAX_BYTES);

    assert_eq!(files.respond("extension", &[0xff, 0xfe]), None);

    scratch.cleanup();
}

#[test]
fn registers_its_endpoint_without_subscribing_to_topics() {
    let scratch = Scratch::new("identity");
    let mut registry = ServiceRegistry::new();
    let mut ctx = ModuleContext::new(&mut registry);
    let mut files = scratch.files(DEFAULT_MAX_BYTES);

    files.init(&mut ctx).expect("no resource to prepare");

    assert_eq!(files.name(), "files");
    assert!(ctx.into_subscriptions().is_empty());

    scratch.cleanup();
}
