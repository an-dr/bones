//! The application-shell features composed through the builder: each
//! registers its endpoint when asked for, and only then.
//!
//! Headless throughout: none of these needs a window, and a headless engine
//! builds without SDL's thread rules.

#[cfg(feature = "instance")]
#[test]
fn a_single_instance_engine_publishes_each_later_launch() {
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use bones_kernel::bus::Envelope;
    use bones_kernel::instance::{claim, Claim, Handoff};
    use bones_kernel::Kernel;
    use bones_messages::instance::Launched;
    use bones_messages::{DecodeMessage, Message};

    let app_uid = format!("bones-kernel-test-{}", std::process::id());
    let handoff = |args: &[&str]| Handoff {
        args: args.iter().map(|arg| arg.to_string()).collect(),
        cwd: std::env::temp_dir(),
    };
    let Claim::Primary(primary) = claim(&app_uid, handoff(&[]), Duration::from_secs(2)).unwrap()
    else {
        panic!("the app_uid should have been free");
    };
    let engine = Kernel::new().single_instance(primary).build().unwrap();
    assert!(engine.registry.contains("instance"));
    let launched = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&launched);
    let spy = engine
        .runner
        .bus()
        .register("spy", move |envelope: &Envelope| {
            sink.lock().unwrap().push(envelope.payload.clone());
        });
    spy.subscribe(Launched::TOPIC);

    let second = claim(&app_uid, handoff(&["repo"]), Duration::from_secs(2)).unwrap();

    assert!(matches!(second, Claim::HandedOff));
    let deadline = Instant::now() + Duration::from_secs(5);
    while launched.lock().unwrap().is_empty() {
        assert!(
            Instant::now() < deadline,
            "the launch never reached the bus"
        );
        engine.present_frame();
        engine.runner.bus().dispatch();
        std::thread::sleep(Duration::from_millis(10));
    }
    let payload = launched.lock().unwrap()[0].clone();
    assert_eq!(Launched::decode(&payload).unwrap().args, ["repo"]);
}

#[cfg(feature = "devtools")]
#[test]
fn the_devtools_channel_exists_only_when_its_variable_names_a_port() {
    use bones_kernel::Kernel;

    let unset = Kernel::new()
        .devtools_from_env("BONES_ENGINE_TEST_DEVTOOLS_UNSET")
        .build()
        .unwrap();
    assert!(!unset.registry.contains("devtools"));

    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    std::env::set_var("BONES_ENGINE_TEST_DEVTOOLS_PORT", port.to_string());
    let mut set = Kernel::new()
        .devtools_from_env("BONES_ENGINE_TEST_DEVTOOLS_PORT")
        .build()
        .unwrap();
    assert!(set.registry.contains("devtools"));
    set.shutdown();
}

#[cfg(feature = "desktop")]
#[test]
fn desktop_registers_its_endpoint() {
    use bones_kernel::desktop::Integration;
    use bones_kernel::Kernel;

    let engine = Kernel::new()
        .desktop(Integration {
            id: "bones-kernel-test",
            name: "Bones Kernel Test",
            comment: "",
            open_label: "Open in Bones Kernel Test",
            categories: "Development;",
            executable: std::env::temp_dir().join("nothing.exe"),
            icon_png: &[],
        })
        .build()
        .unwrap();

    assert!(engine.registry.contains("desktop"));
}

#[cfg(feature = "upgrade")]
#[test]
fn upgrader_registers_its_endpoint() {
    use bones_kernel::upgrade::{AppIdentity, ReleaseSource, UpgradeConfig};
    use bones_kernel::Kernel;

    let engine = Kernel::new()
        .upgrade(UpgradeConfig::new(
            AppIdentity {
                install_dir_name: ".test-upgrade",
                install_env_var: "TEST_UPGRADE_INSTALL_DIR",
                launcher_stem: "test",
                app_stem: "test-app",
            },
            ReleaseSource::Url("https://example.test/app.json".into()),
            "1.0.0",
        ))
        .build()
        .unwrap();

    assert!(engine.registry.contains("upgrader"));
}
