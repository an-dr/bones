use super::*;

#[test]
fn tick_hz_defaults_to_60_and_is_overridable() {
    assert_eq!(Engine::new().tick_hz, DEFAULT_TICK_HZ);
    assert_eq!(Engine::new().tick_hz(30.0).tick_hz, 30.0);
}

#[test]
fn the_default_and_ordinary_rates_produce_a_period() {
    assert_eq!(
        tick_period(DEFAULT_TICK_HZ).unwrap(),
        Duration::from_secs_f64(1.0 / 60.0)
    );
    assert_eq!(tick_period(1.0).unwrap(), Duration::from_secs(1));
    // Absurd but representable, which is the boundary that matters: the
    // rejection below is about unrepresentable values, not implausible ones.
    assert!(tick_period(1.0e9).is_ok());
}

#[test]
fn a_rate_that_is_not_a_positive_finite_number_is_an_error_not_a_panic() {
    // Each of these reaches `Duration::from_secs_f64` as a negative, NaN, or
    // infinite number of seconds, which panics — through a `Result`-returning
    // public API that has a perfectly good way to report it.
    for rate in [0.0, -1.0, -0.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = tick_period(rate)
            .expect_err(&format!("tick_hz {rate} must be rejected"))
            .to_string();
        assert!(
            error.contains("finite and greater than zero"),
            "expected the reason to name the rule, got {error}"
        );
    }
}

#[test]
fn a_rate_too_small_to_be_a_period_is_an_error_not_a_panic() {
    // Finite and positive, so the first check passes, but 1/rate overflows
    // what a Duration can hold.
    let error = tick_period(f64::MIN_POSITIVE)
        .expect_err("a subnormal rate has no representable period")
        .to_string();
    assert!(
        error.contains("too small"),
        "expected the reason to name the overflow, got {error}"
    );
}

#[test]
fn build_rejects_an_invalid_rate_before_wiring_anything() {
    let error = match Engine::new().tick_hz(0.0).build() {
        Ok(_) => panic!("build must reject a zero tick rate"),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains("tick_hz"),
        "expected the error to name the setting, got {error}"
    );
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
    assert_eq!(
        only_call.extension_timeouts.load,
        ExtensionTimeouts::default().load
    );
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
