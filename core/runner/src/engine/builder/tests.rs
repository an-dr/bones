use super::*;

#[test]
fn tick_hz_defaults_to_60_and_is_overridable() {
    assert_eq!(Engine::new().tick_hz, DEFAULT_TICK_HZ);
    assert_eq!(Engine::new().tick_hz(30.0).tick_hz, 30.0);
}

#[test]
fn extension_budget_defaults_and_overrides_are_explicit() {
    assert_eq!(Engine::new().extension_budget, BudgetLimits::default());
    let limits = BudgetLimits {
        max_inbound: 2,
        max_publishes: 3,
    };
    assert_eq!(
        Engine::new().extension_budget(limits).extension_budget,
        limits
    );
}

#[test]
fn extension_timeouts_default_and_override_one_at_a_time() {
    assert_eq!(
        Engine::new().extension_timeouts,
        ExtensionTimeouts::default()
    );

    let load = Duration::from_secs(30);
    let call = Duration::from_secs(10);
    let engine = Engine::new()
        .extension_load_timeout(load)
        .extension_call_timeout(call);

    assert_eq!(engine.extension_timeouts.load, load);
    assert_eq!(engine.extension_timeouts.call, call);

    // Setting one must not quietly reset the other to its default.
    let only_call = Engine::new().extension_call_timeout(call);
    assert_eq!(only_call.extension_timeouts.load, ExtensionTimeouts::default().load);
    assert_eq!(only_call.extension_timeouts.call, call);
}

#[test]
fn explicit_catalog_entries_preserve_embedder_identity_and_path() {
    let path = PathBuf::from("validated/session-counter.wasm");
    let engine = Engine::new().catalog_extension("session-counter", path.clone());

    assert_eq!(
        engine.catalog_extensions,
        vec![("session-counter".to_string(), path)]
    );
}

#[cfg(feature = "web")]
#[test]
fn web_is_opt_in_when_the_feature_is_available() {
    assert!(!Engine::new().web_enabled);
    assert!(Engine::new().web().web_enabled);
}

#[test]
fn an_absolute_path_passes_through_resolve_relative_to_exe_unchanged() {
    let absolute = if cfg!(windows) {
        PathBuf::from(r"C:\somewhere\saves")
    } else {
        PathBuf::from("/somewhere/saves")
    };

    assert_eq!(resolve_relative_to_exe(absolute.clone()), absolute);
}

#[test]
fn a_relative_path_resolves_against_the_test_binarys_own_directory() {
    // cargo test's own executable always has a determinable path, so this
    // exercises the real `exe_dir()` lookup end to end.
    let resolved = resolve_relative_to_exe(PathBuf::from("saves"));

    assert!(resolved.is_absolute());
    assert!(resolved.ends_with("saves"));
}
