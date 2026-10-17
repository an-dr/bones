//! The application half: it knows its own version, and can replace itself.
//!
//! Everything here is what an application actually has to write. The version
//! arithmetic, the version folders and the entry-point replacement are
//! the upgrade module's; what an app supplies is its identity, a way to fetch,
//! and the decision about when to update.
//!
//! Run it through the launcher, not directly -- that is the whole point of
//! the two-binary shape, and `--help` explains why.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bones_kernel::bus::Envelope;
use bones_kernel::desktop::{Integration, Locations};
use bones_kernel::messages::upgrader::{Action, Outcome, Request, Result as UpgraderResult};
use bones_kernel::messages::{DecodeMessage, EncodeMessage, Message};
use bones_kernel::upgrade::{Fetch, ReleaseSource, UpgradeConfig, UpgradeModule};
use bones_kernel::Kernel;
mod app_identity;
use app_identity::app_identity;

/// This build's version. A real app uses `env!("CARGO_PKG_VERSION")`; the
/// example hardcodes it so `cargo xtask example self-updating-app` can produce two versions from one
/// source tree and show an update actually happening.
const VERSION: &str = match option_env!("DEMO_APP_VERSION") {
    Some(version) => version,
    None => "1.0.0",
};

/// A 1x1 transparent PNG: the icon a Linux application list shows. Windows
/// shortcuts take their icon from the executable instead.
const ICON_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae,
    0x42, 0x60, 0x82,
];

/// Reads a `file://` URL, so the example needs no network and no server.
///
/// A real host leaves `UpgradeConfig::fetch` unset and gets the module's
/// own HTTPS download. The trait is one method wide precisely so that a test
/// or a file share is just as easy.
struct FileFetch;

impl Fetch for FileFetch {
    fn fetch_url(&self, url: &str) -> Result<Option<String>, String> {
        let path = url
            .strip_prefix("file://")
            .ok_or_else(|| format!("this demo only reads file:// URLs, got {url}"))?;
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(format!(
                "application/octet-stream;base64,{}",
                base64(&bytes)
            ))),
            // A missing manifest is a normal "nothing published yet", not a
            // failure -- which is why the trait returns Option.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }
}

/// The example's own base64, so it carries no dependency beyond bones.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - i * 6)) & 0x3f) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--install") => report(install()),
        Some("--unregister") => report(unregister()),
        Some("--rollback") => report(upgrade_module().rollback()),
        Some("--version") => println!("demo-app {VERSION}"),
        Some("--help") => println!("demo [--install | --unregister | --version | --rollback]\nUse demo --previous --rollback to recover from a broken newest version."),
        _ => run(),
    }
}

fn report(outcome: Result<String, String>) {
    match outcome {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn run() {
    let identity = app_identity();
    println!("demo-app {VERSION}");
    println!("  install dir name: {}", identity.install_dir_name);
    println!("  launcher: {}", identity.launcher_exe());

    let Some(install_dir) = upgrade_module().install_dir() else {
        println!("  (no install dir resolved; running from a build tree)");
        return;
    };
    println!("  installed at:     {}", install_dir.display());
    if let Some(current) = upgrade_module().installed_version() {
        println!("  current version:  {}", current);
    }

    // The manifest lives beside the install in this example. A real app
    // passes `ReleaseSource::GitHub { .. }`, or a URL its user configured.
    let manifest_url = format!("file://{}", install_dir.join("manifest.json").display());
    match stage_through_the_engine(manifest_url) {
        Ok(Some(version)) => {
            println!("  staged {version}; the launcher will start it next run");
        }
        Ok(None) => println!("  up to date"),
        Err(error) => println!("  update check failed: {error}"),
    }
}

/// Asks the engine's `upgrader` module to stage a newer release, exactly as
/// a page or an extension would: publish `upgrader/request`, then wait for
/// the `upgrader/result` carrying the same id.
///
/// Staging rather than replacing is what makes this safe: the running app is
/// never overwritten, and the launcher picks the newest version folder on the
/// next start. A checksum mismatch is refused outright, and a failure here
/// costs the update, not the installation.
fn stage_through_the_engine(manifest_url: String) -> Result<Option<String>, String> {
    let engine = Kernel::new()
        .upgrade(
            UpgradeConfig::new(app_identity(), ReleaseSource::Url(manifest_url), VERSION)
                .fetch(Arc::new(FileFetch)),
        )
        .build()
        .map_err(|error| error.to_string())?;
    let bus = engine.runner.bus().clone();
    let answers = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&answers);
    let guest = bus.register("demo-guest", move |envelope: &Envelope| {
        sink.lock().unwrap().push(envelope.payload.clone());
    });
    guest.subscribe(UpgraderResult::TOPIC);

    bus.publish(Envelope {
        topic: Request::TOPIC.into(),
        sender: "demo-guest".into(),
        correlation: None,
        payload: Request {
            request_id: 1,
            action: Action::Stage,
        }
        .encode(),
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        bus.dispatch();
        if let Some(payload) = answers.lock().unwrap().first() {
            let result = UpgraderResult::decode(payload).map_err(|error| error.to_string())?;
            return match result.outcome {
                Outcome::Staged { version } => Ok(Some(version.to_string())),
                Outcome::UpToDate => Ok(None),
                Outcome::Failed { reason } => Err(reason.to_string()),
                other => Err(format!("unexpected answer {other:?}")),
            };
        }
        if Instant::now() > deadline {
            return Err("the upgrader did not answer".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// What a real application's `--install` does: place the files with
/// the upgrade module, then register the installed launcher with the desktop
/// with `bones-module-desktop`. Two crates, called one after the other, because
/// replacing versions and appearing in the application list are separate
/// jobs (ADR-036).
fn install() -> Result<String, String> {
    let fresh = upgrade_module().install()?;
    let (integration, locations) = registration()?;
    bones_kernel::desktop::register(&integration, &locations)?;
    Ok(format!(
        "installed {VERSION} ({}) and registered {}",
        if fresh {
            "fresh"
        } else {
            "beside the existing install"
        },
        integration.executable.display()
    ))
}

fn unregister() -> Result<String, String> {
    let (integration, locations) = registration()?;
    bones_kernel::desktop::unregister(&integration, &locations)?;
    Ok(String::from(
        "unregistered; the installed files are untouched",
    ))
}

/// The registration names the installed launcher, never a version folder's
/// executable: the launcher is the one path that survives updates.
fn registration() -> Result<(Integration<'static>, Locations), String> {
    let identity = app_identity();
    let install_dir = upgrade_module()
        .install_dir()
        .ok_or("could not resolve the install directory")?;
    let locations =
        Locations::for_current_user().ok_or("desktop registration is not available here")?;
    Ok((
        Integration {
            id: "bones-demo-app",
            name: "Bones Demo App",
            comment: "The bones self-updating example",
            open_label: "Open in Bones Demo App",
            categories: "Development;",
            executable: install_dir.join(identity.launcher_exe()),
            icon_png: ICON_PNG,
        },
        locations,
    ))
}

fn upgrade_module() -> UpgradeModule {
    UpgradeModule::new(UpgradeConfig::new(
        app_identity(),
        ReleaseSource::Url(String::new()),
        VERSION,
    ))
}
