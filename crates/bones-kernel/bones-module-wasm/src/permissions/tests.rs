use super::*;

#[test]
fn nothing_is_denied_by_default() {
    let permissions = Permissions::default();
    assert!(permissions.allows("os/request"));
    assert!(permissions.allows_subscription("*"));
}

#[test]
fn a_prefix_denies_every_topic_under_it() {
    let permissions = Permissions::deny(["os/*"]);
    assert!(!permissions.allows("os/request"));
    assert!(permissions.allows("gfx/clear"));
    assert!(permissions.allows("osx/thing"));
}

#[test]
fn an_exact_entry_denies_one_endpoint() {
    let permissions = Permissions::deny(["files"]);
    assert!(!permissions.allows("files"));
    assert!(permissions.allows("files-cache"));
}

#[test]
fn a_subscription_that_could_match_a_denied_topic_is_refused() {
    let permissions = Permissions::deny(["os/*"]);
    assert!(!permissions.allows_subscription("*"));
    assert!(!permissions.allows_subscription("os/*"));
    assert!(!permissions.allows_subscription("os/result"));
    assert!(!permissions.allows_subscription("o*"));
    assert!(permissions.allows_subscription("gfx/*"));
    assert!(permissions.allows_subscription("core/tick"));
}
