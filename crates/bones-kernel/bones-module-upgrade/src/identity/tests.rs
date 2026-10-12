use super::*;

fn identity() -> AppIdentity {
    AppIdentity {
        install_dir_name: ".other",
        install_env_var: "OTHER_INSTALL_DIR",
        launcher_stem: "other",
        app_stem: "other-app",
    }
}

#[test]
fn executable_names_carry_the_platform_extension() {
    let identity = identity();

    if cfg!(windows) {
        assert_eq!(identity.launcher_exe(), "other.exe");
        assert_eq!(identity.app_exe(), "other-app.exe");
        assert_eq!(identity.launcher_payload_exe(), "other-launcher.exe");
    } else {
        assert_eq!(identity.launcher_exe(), "other");
        assert_eq!(identity.app_exe(), "other-app");
        assert_eq!(identity.launcher_payload_exe(), "other-launcher");
    }
}

#[test]
fn the_shipped_launcher_never_shares_the_entry_points_name() {
    // The two live in one tree during an install, and what belongs in a
    // version folder is decided by name -- so a collision here would
    // silently drop the shipped launcher from every payload.
    let identity = identity();

    assert_ne!(identity.launcher_payload_exe(), identity.launcher_exe());
}

#[test]
fn the_host_identity_is_whole_however_it_was_configured() {
    // The UPGRADER_* variables are read at compile time, so what this
    // sees depends on whether a host set them -- and a checkout nested
    // inside one inherits that host's `.cargo/config.toml`. Asserting
    // the placeholder values would therefore test the build environment
    // rather than the code. What must hold either way is that all four
    // names are present and usable.
    let identity = host_identity();

    assert!(identity.install_dir_name.starts_with('.'));
    assert!(!identity.install_env_var.is_empty());
    assert!(!identity.launcher_stem.is_empty());
    assert!(!identity.app_stem.is_empty());
    assert_ne!(identity.launcher_stem, identity.app_stem);
}
