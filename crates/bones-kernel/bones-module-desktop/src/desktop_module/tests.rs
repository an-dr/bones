use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{Integration, Locations};
use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext, ServiceRegistry};
use bones_messages::desktop::{Action, Outcome, Request, Result as DesktopResult};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

use crate::DesktopModule;

/// Scratch locations, so nothing reaches the real desktop. On Windows the
/// classes key is a scratch registry key, removed again on drop.
struct Scratch {
    dir: tempfile::TempDir,
    #[cfg(windows)]
    classes_key: String,
}

impl Scratch {
    fn new(name: &str) -> Self {
        Self {
            dir: tempfile::tempdir().unwrap(),
            #[cfg(windows)]
            classes_key: format!(
                r"Software\BonesModuleDesktopTest-{name}-{}",
                std::process::id()
            ),
        }
    }

    fn locations(&self) -> Locations {
        #[cfg(windows)]
        return Locations {
            classes_key: self.classes_key.clone(),
            start_menu: self.dir.path().join("Programs"),
            desktop: self.dir.path().join("Desktop"),
        };
        #[cfg(target_os = "linux")]
        return Locations {
            data_home: self.dir.path().to_path_buf(),
            refresh_caches: false,
        };
    }

    /// An executable that exists, so registering it is allowed.
    fn installed(&self) -> PathBuf {
        let executable = self.dir.path().join("app").join("sample.exe");
        std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
        std::fs::write(&executable, b"binary").unwrap();
        executable
    }
}

#[cfg(windows)]
impl Drop for Scratch {
    fn drop(&mut self) {
        let unregister = Integration {
            executable: PathBuf::new(),
            ..integration(Path::new(""))
        };
        let _ = crate::unregister(&unregister, &self.locations());
    }
}

fn integration(executable: &Path) -> Integration<'static> {
    Integration {
        id: "bones-module-desktop-sample",
        name: "BonesModuleDesktopSample",
        comment: "A sample application",
        open_label: "Open in Sample",
        categories: "Development;",
        executable: executable.to_path_buf(),
        icon_png: b"\x89PNG not really",
    }
}

/// Sends each action in turn over a real bus and collects the outcomes.
fn run(module: DesktopModule, actions: &[Action]) -> Vec<(u32, Result<bool, String>)> {
    let bus = Bus::new();
    let mut services = ServiceRegistry::new();
    services.provide(bus.clone()).unwrap();
    let mut module = module;
    module.init(&mut ModuleContext::new(&mut services)).unwrap();
    let results = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&results);
    let endpoint = bus.register("guest", move |envelope: &Envelope| {
        let result = DesktopResult::decode(&envelope.payload).unwrap();
        assert_eq!(envelope.correlation, Some(u64::from(result.request_id)));
        sink.lock().unwrap().push((
            result.request_id,
            match result.outcome {
                Outcome::Registered(registered) => Ok(registered),
                Outcome::Failed(reason) => Err(reason.to_string()),
            },
        ));
    });
    endpoint.subscribe(DesktopResult::TOPIC);

    for (index, action) in actions.iter().enumerate() {
        let request_id = index as u32 + 1;
        let started = Instant::now();
        module.handle(&Envelope {
            topic: Request::TOPIC.into(),
            sender: "guest".into(),
            correlation: None,
            payload: Request {
                request_id,
                action: *action,
            }
            .encode(),
        });
        assert!(
            started.elapsed() < Duration::from_millis(50),
            "handling a request must not wait for the work"
        );
        // One at a time, so each action sees the previous one's effect.
        let deadline = Instant::now() + Duration::from_secs(5);
        while results.lock().unwrap().len() < request_id as usize {
            assert!(
                Instant::now() < deadline,
                "request {request_id} never answered"
            );
            std::thread::sleep(Duration::from_millis(5));
            bus.dispatch();
        }
    }
    let results = results.lock().unwrap().clone();
    results
}

#[test]
fn registers_reads_back_and_unregisters_through_the_bus() {
    let scratch = Scratch::new("roundtrip");
    let module =
        DesktopModule::with_locations(integration(&scratch.installed()), Some(scratch.locations()));

    let results = run(
        module,
        &[
            Action::Status,
            Action::Register,
            Action::Status,
            Action::Unregister,
            Action::Status,
        ],
    );

    assert_eq!(
        results,
        [
            (1, Ok(false)),
            (2, Ok(true)),
            (3, Ok(true)),
            (4, Ok(false)),
            (5, Ok(false)),
        ]
    );
}

#[test]
fn refuses_to_register_an_executable_that_is_not_there() {
    let scratch = Scratch::new("missing");
    let missing = scratch.dir.path().join("nowhere").join("sample.exe");
    let module = DesktopModule::with_locations(integration(&missing), Some(scratch.locations()));

    let results = run(module, &[Action::Register]);

    let (_, outcome) = &results[0];
    assert!(outcome.as_ref().unwrap_err().contains("not installed"));
}

#[test]
fn answers_every_request_even_without_locations() {
    let module = DesktopModule::with_locations(integration(Path::new("x")), None);

    let results = run(module, &[Action::Status]);

    assert!(results[0].1.as_ref().unwrap_err().contains("not available"));
}

#[test]
fn the_crate_api_does_the_same_directly() {
    let scratch = Scratch::new("direct");
    let app = integration(&scratch.installed());
    let locations = scratch.locations();

    crate::register(&app, &locations).unwrap();
    assert!(crate::is_registered(&app, &locations));
    crate::unregister(&app, &locations).unwrap();
    assert!(!crate::is_registered(&app, &locations));
}
