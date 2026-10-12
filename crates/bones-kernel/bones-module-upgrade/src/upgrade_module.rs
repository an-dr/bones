use std::path::{Path, PathBuf};
use std::sync::Arc;

// Serializes installation mutations across module instances in this process.
static MUTATIONS: std::sync::Mutex<()> = std::sync::Mutex::new(());

use crate::{updater, AppIdentity, Fetch, ReleaseSource};
use bones_kernel_infra::bus::{Bus, Envelope, Handler, Module, ModuleContext, Registry};
use bones_messages::persistence::{Save, ENDPOINT as PERSISTENCE};
use bones_messages::upgrader::{Action, Outcome, Request, Result as UpgraderResult, ENDPOINT};
use bones_messages::{DecodeMessage, EncodeMessage, Message};

use crate::{HttpsFetch, UpgradeConfig};

/// `upgrader/request` in, `upgrader/result` out.
///
/// - Check, stage, and install each run on a thread of their own: a
///   download has no bound tight enough to wait for in a frame.
/// - Last-applied answers at once, from what the module recorded when the
///   application started.
pub struct UpgradeModule {
    source: Arc<ReleaseSource>,
    current_version: Arc<str>,
    fetch: Arc<dyn Fetch + Send + Sync>,
    identity: AppIdentity,
    bus: Option<Bus>,
    registry: Option<Registry>,
    /// The version recorded before this run, once [`Self::record_version`]
    /// has run: `Some(None)` for a first run or a run outside an install.
    previous: Option<Option<String>>,
}

impl UpgradeModule {
    /// Uses the application's explicitly supplied installation identity and source.
    pub fn new(config: UpgradeConfig) -> Self {
        Self {
            source: Arc::new(config.source),
            current_version: config.current_version.into(),
            fetch: config.fetch.unwrap_or_else(|| Arc::new(HttpsFetch)),
            identity: config.identity,
            bus: None,
            registry: None,
            previous: None,
        }
    }

    /// Installs the running build through this module's update implementation.
    pub fn install(&self) -> Result<bool, String> {
        install(&self.identity, &self.current_version)
    }

    /// Resolves this application's installation directory.
    pub fn install_dir(&self) -> Option<PathBuf> {
        crate::default_install_dir(&self.identity)
    }

    /// Reports the newest installed SemVer version, excluding its installation instance.
    pub fn installed_version(&self) -> Option<String> {
        let path = crate::current_version_dir(&self.install_dir()?)?;
        bones_versioning::parse_folder_name(path.file_name()?.to_str()?)
            .map(|version| version.to_string())
    }

    /// Resolves persistent application data beside an installed version tree.
    pub fn shared_path(&self, name: &str) -> Option<PathBuf> {
        crate::shared_or_exe_relative(&self.identity, name)
    }

    /// Refreshes the launcher from this running version's payload, when installed.
    pub fn refresh_launcher(&self) -> Result<bool, String> {
        let _mutation = MUTATIONS.lock().map_err(|error| error.to_string())?;
        crate::install::refresh_launcher_from_running_exe(&self.identity)
    }

    /// Removes the newest version only when a previous version is available.
    ///
    /// Run recovery from the previous version via the launcher's `--previous`.
    pub fn rollback(&self) -> Result<String, String> {
        let _mutation = MUTATIONS.lock().map_err(|error| error.to_string())?;
        let dir = self
            .install_dir()
            .ok_or("could not resolve the install directory")?;
        crate::rollback::rollback(&dir)
    }

    /// Installs a trusted companion payload; never accepts an identity from a guest.
    pub fn install_companion(
        &self,
        identity: &AppIdentity,
        archive: &Path,
        version: &str,
        expected_sha256: Option<&str>,
        policy: crate::CompanionPolicy,
    ) -> Result<crate::CompanionOutcome, String> {
        let _mutation = MUTATIONS.lock().map_err(|error| error.to_string())?;
        crate::companion::install_companion(identity, archive, version, expected_sha256, policy)
    }

    /// Reads the version saved by the previous run and saves this one, once
    /// per run.
    ///
    /// - Runs on the first frame or the first last-applied request, not in
    ///   `init`: the kernel infrastructure's persistence registers after the modules do, so
    ///   it cannot answer during `init`.
    /// - Only for a run from an installed version folder. A development build
    ///   was never "updated" by this mechanism, and recording its version
    ///   would make the next installed run claim it had been.
    fn record_version(&mut self) {
        if self.previous.is_some() {
            return;
        }
        if !running_dir().is_some_and(|dir| crate::is_installed_version_dir(&self.identity, &dir)) {
            self.previous = Some(None);
            return;
        }
        let saved = self
            .registry
            .as_ref()
            .and_then(|registry| registry.call(ENDPOINT, PERSISTENCE, &[]).ok())
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
        if let Some(bus) = &self.bus {
            bus.publish(Envelope {
                topic: Save::TOPIC.into(),
                sender: ENDPOINT.into(),
                correlation: None,
                payload: Save {
                    bytes: self.current_version.as_bytes(),
                }
                .encode(),
            });
        }
        self.previous = Some(saved);
    }

    fn start(&mut self, request: Request) {
        let Some(bus) = self.bus.clone() else { return };
        if request.action == Action::LastApplied {
            self.record_version();
            let previous = self.previous.clone().flatten();
            publish(
                &bus,
                request.request_id,
                Outcome::LastApplied {
                    previous: previous.as_deref(),
                    current: &self.current_version,
                },
            );
            return;
        }
        let source = Arc::clone(&self.source);
        let current = Arc::clone(&self.current_version);
        let fetch = Arc::clone(&self.fetch);
        let identity = self.identity;
        std::thread::spawn(move || {
            let fetch = fetch.as_ref();
            match request.action {
                Action::Check => match updater::check(fetch, &source, &current) {
                    Ok(Some(manifest)) => publish(
                        &bus,
                        request.request_id,
                        Outcome::Available {
                            version: &manifest.version,
                        },
                    ),
                    Ok(None) => publish(&bus, request.request_id, Outcome::UpToDate),
                    Err(reason) => fail(&bus, request.request_id, &reason),
                },
                Action::Stage => match stage(fetch, &source, &current, &identity) {
                    Ok(Some(version)) => publish(
                        &bus,
                        request.request_id,
                        Outcome::Staged { version: &version },
                    ),
                    Ok(None) => publish(&bus, request.request_id, Outcome::UpToDate),
                    Err(reason) => fail(&bus, request.request_id, &reason),
                },
                Action::Install => match install(&identity, &current) {
                    Ok(fresh) => publish(&bus, request.request_id, Outcome::Installed { fresh }),
                    Err(reason) => fail(&bus, request.request_id, &reason),
                },
                Action::LastApplied => {}
            }
        });
    }
}

/// Checks, then stages a newer release. `Ok(None)` when there is none.
fn stage(
    fetch: &dyn Fetch,
    source: &ReleaseSource,
    current: &str,
    identity: &AppIdentity,
) -> Result<Option<String>, String> {
    let _mutation = MUTATIONS.lock().map_err(|error| error.to_string())?;
    let Some(manifest) = updater::check(fetch, source, current)? else {
        return Ok(None);
    };
    let install_dir = crate::default_install_dir(identity)
        .ok_or_else(|| String::from("could not resolve the install directory"))?;
    updater::stage(fetch, &manifest, &install_dir)?;
    Ok(Some(manifest.version))
}

fn install(identity: &AppIdentity, current: &str) -> Result<bool, String> {
    let _mutation = MUTATIONS.lock().map_err(|error| error.to_string())?;
    let source_dir = running_dir()
        .ok_or_else(|| String::from("could not resolve the running executable's directory"))?;
    updater::install(identity, &source_dir, current)
}

fn running_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

fn publish(bus: &Bus, request_id: u32, outcome: Outcome) {
    let result = UpgraderResult {
        request_id,
        outcome,
    };
    let payload = result.try_encode().unwrap_or_else(|| {
        UpgraderResult {
            request_id,
            outcome: Outcome::Failed {
                reason: "the result was too large to send",
            },
        }
        .encode()
    });
    bus.publish(Envelope {
        topic: UpgraderResult::TOPIC.into(),
        sender: ENDPOINT.into(),
        correlation: Some(u64::from(request_id)),
        payload,
    });
}

fn fail(bus: &Bus, request_id: u32, reason: &str) {
    publish(bus, request_id, Outcome::Failed { reason });
}

impl Handler for UpgradeModule {
    fn handle(&mut self, envelope: &Envelope) {
        if envelope.topic != Request::TOPIC {
            return;
        }
        if let Ok(request) = Request::decode(&envelope.payload) {
            self.start(request);
        }
    }
}

impl Module for UpgradeModule {
    fn name(&self) -> &str {
        ENDPOINT
    }

    fn init(&mut self, context: &mut ModuleContext) -> Result<(), String> {
        context.subscribe(Request::TOPIC);
        self.bus = context.get_service::<Bus>().cloned();
        self.registry = context.get_service::<Registry>().cloned();
        if self.bus.is_none() {
            return Err("no Bus service available".into());
        }
        Ok(())
    }

    fn render(&mut self) {
        self.record_version();
    }
}
