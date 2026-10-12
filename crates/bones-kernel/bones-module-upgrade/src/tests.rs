use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{Fetch, ReleaseSource};
use base64::Engine;
use bones_kernel_infra::bus::{
    Bus, Envelope, Handler, Module, ModuleContext, ModuleRegistration, Registry, ServiceRegistry,
};
use bones_kernel_infra::wasm_extensions::persistence::Persistence;
use bones_messages::persistence::Save;
use bones_messages::upgrader::{Action, Outcome, Request, Result as UpgraderResult};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

use crate::{UpgradeConfig, UpgradeModule};

/// The install location is read from one process-wide variable, so the tests
/// that set it take turns.
static INSTALL_DIR_LOCK: Mutex<()> = Mutex::new(());

const MANIFEST_URL: &str = "https://example.test/app.json";
const ARCHIVE_URL: &str = "https://example.test/app-2.0.0.zip";

/// Canned responses, served after `delay` so a test can tell whether the
/// module waited for them.
struct StubFetch {
    responses: HashMap<String, Vec<u8>>,
    delay: Duration,
}

impl Fetch for StubFetch {
    fn fetch_url(&self, url: &str) -> Result<Option<String>, String> {
        std::thread::sleep(self.delay);
        let bytes = self
            .responses
            .get(url)
            .ok_or_else(|| format!("no stub response for {url}"))?;
        Ok(Some(format!(
            "application/octet-stream;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        )))
    }
}

fn release(version: &str, delay: Duration) -> Arc<StubFetch> {
    let mut archive = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut archive));
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
        writer.start_file("app.exe", options).unwrap();
        writer.write_all(version.as_bytes()).unwrap();
        writer.finish().unwrap();
    }
    let sha256: String = {
        use sha2::{Digest, Sha256};
        Sha256::digest(&archive)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    };
    let manifest =
        format!(r#"{{"version":"{version}","url":"{ARCHIVE_URL}","sha256":"{sha256}"}}"#);
    Arc::new(StubFetch {
        responses: HashMap::from([
            (MANIFEST_URL.to_string(), manifest.into_bytes()),
            (ARCHIVE_URL.to_string(), archive),
        ]),
        delay,
    })
}

struct Harness {
    bus: Bus,
    module: UpgradeModule,
    results: Arc<Mutex<Vec<Envelope>>>,
    _guest: bones_kernel_infra::bus::Endpoint,
    _persistence: Option<ModuleRegistration>,
}

fn harness(fetch: Arc<StubFetch>, current: &str, state_dir: Option<&std::path::Path>) -> Harness {
    let bus = Bus::new();
    let registry = Registry::new();
    let mut services = ServiceRegistry::new();
    services.provide(bus.clone()).unwrap();
    services.provide(registry.clone()).unwrap();
    let persistence = state_dir.map(|dir| {
        ModuleRegistration::attach(
            bus.clone(),
            registry.clone(),
            &mut services,
            Persistence::new(dir, false),
        )
        .unwrap()
    });
    let mut module = UpgradeModule::new(
        UpgradeConfig::new(
            crate::host_identity(),
            ReleaseSource::Url(MANIFEST_URL.into()),
            current,
        )
        .fetch(fetch),
    );
    module.init(&mut ModuleContext::new(&mut services)).unwrap();
    let results = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&results);
    let guest = bus.register("guest", move |envelope: &Envelope| {
        sink.lock().unwrap().push(envelope.clone());
    });
    guest.subscribe(UpgraderResult::TOPIC);
    Harness {
        bus,
        module,
        results,
        _guest: guest,
        _persistence: persistence,
    }
}

impl Harness {
    /// Sends one request and waits for its one result; returns the result's
    /// payload and how long `handle` itself took.
    fn ask(&mut self, request_id: u32, action: Action) -> (Vec<u8>, Duration) {
        let before = self.results.lock().unwrap().len();
        let started = Instant::now();
        self.module.handle(&Envelope {
            topic: Request::TOPIC.into(),
            sender: "guest".into(),
            correlation: None,
            payload: Request { request_id, action }.encode(),
        });
        let handled = started.elapsed();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            self.bus.dispatch();
            let results = self.results.lock().unwrap();
            if results.len() > before {
                assert_eq!(results.len(), before + 1, "one request, one result");
                let envelope = &results[before];
                assert_eq!(envelope.correlation, Some(u64::from(request_id)));
                assert_eq!(envelope.sender, "upgrader");
                return (envelope.payload.clone(), handled);
            }
            drop(results);
            assert!(
                Instant::now() < deadline,
                "request {request_id} never answered"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[test]
fn a_check_never_blocks_the_frame_and_answers_once_correlated() {
    let mut harness = harness(release("2.0.0", Duration::from_millis(200)), "1.0.0", None);

    let (payload, handled) = harness.ask(7, Action::Check);

    assert!(
        handled < Duration::from_millis(50),
        "handle took {handled:?}"
    );
    assert_eq!(
        UpgraderResult::decode(&payload).unwrap().outcome,
        Outcome::Available { version: "2.0.0" }
    );
}

#[test]
fn a_check_against_the_running_version_is_up_to_date() {
    let mut harness = harness(release("1.0.0", Duration::ZERO), "1.0.0", None);

    let (payload, _) = harness.ask(1, Action::Check);

    assert_eq!(
        UpgraderResult::decode(&payload).unwrap().outcome,
        Outcome::UpToDate
    );
}

#[test]
fn a_failed_fetch_is_reported_rather_than_dropped() {
    let fetch = Arc::new(StubFetch {
        responses: HashMap::new(),
        delay: Duration::ZERO,
    });
    let mut harness = harness(fetch, "1.0.0", None);

    let (payload, _) = harness.ask(1, Action::Check);

    assert!(matches!(
        UpgraderResult::decode(&payload).unwrap().outcome,
        Outcome::Failed { .. }
    ));
}

#[test]
fn staging_puts_the_release_in_its_own_version_folder() {
    let _lock = INSTALL_DIR_LOCK.lock().unwrap();
    let install = tempfile::tempdir().unwrap();
    std::env::set_var(crate::host_identity().install_env_var, install.path());
    let mut harness = harness(release("2.0.0", Duration::ZERO), "1.0.0", None);

    let (payload, _) = harness.ask(3, Action::Stage);

    assert_eq!(
        UpgraderResult::decode(&payload).unwrap().outcome,
        Outcome::Staged { version: "2.0.0" }
    );
    assert_eq!(
        std::fs::read(install.path().join("2.0.0/app.exe")).unwrap(),
        b"2.0.0"
    );
}

#[test]
fn a_run_outside_an_install_reports_no_previous_version_and_records_none() {
    let _lock = INSTALL_DIR_LOCK.lock().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    std::env::set_var(crate::host_identity().install_env_var, elsewhere.path());
    let state = tempfile::tempdir().unwrap();
    let mut harness = harness(
        release("1.0.0", Duration::ZERO),
        "1.0.0",
        Some(state.path()),
    );

    let (payload, _) = harness.ask(1, Action::LastApplied);
    harness.bus.dispatch();

    assert_eq!(
        UpgraderResult::decode(&payload).unwrap().outcome,
        Outcome::LastApplied {
            previous: None,
            current: "1.0.0"
        }
    );
    assert!(!state.path().join("upgrader.bin").exists());
}

#[test]
fn an_installed_run_reports_the_version_that_ran_before_it() {
    let _lock = INSTALL_DIR_LOCK.lock().unwrap();
    // The test binary's own folder stands in for a version folder: its parent
    // is made the install location.
    let running = std::env::current_exe().unwrap();
    let install_dir = running.parent().unwrap().parent().unwrap();
    std::env::set_var(crate::host_identity().install_env_var, install_dir);
    let state = tempfile::tempdir().unwrap();
    let mut harness = harness(
        release("1.0.0", Duration::ZERO),
        "1.1.0",
        Some(state.path()),
    );
    // What the previous run saved.
    harness.bus.publish(Envelope {
        topic: Save::TOPIC.into(),
        sender: "upgrader".into(),
        correlation: None,
        payload: Save { bytes: b"1.0.0" }.encode(),
    });
    harness.bus.dispatch();

    let (first, _) = harness.ask(1, Action::LastApplied);
    harness.bus.dispatch();
    let (again, _) = harness.ask(2, Action::LastApplied);

    let expected = Outcome::LastApplied {
        previous: Some("1.0.0"),
        current: "1.1.0",
    };
    assert_eq!(UpgraderResult::decode(&first).unwrap().outcome, expected);
    // The answer is the run's, not the request's: asking twice changes nothing.
    assert_eq!(UpgraderResult::decode(&again).unwrap().outcome, expected);
    // This run is recorded for the next one.
    assert_eq!(
        std::fs::read(state.path().join("upgrader.bin")).unwrap(),
        b"1.1.0"
    );
}
