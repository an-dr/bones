//! The public builder API (design/modules.md): discovers WASM extensions
//! and runs them. `bones::Engine` in the design sketch — lives here as
//! `runner::Engine` until a top-level facade crate exists to re-export it.
//! No `.module(...)` yet — that arrives once a real native module
//! (renderer) exists to shape that part of the API (rung 8's job).

use std::path::{Path, PathBuf};
use std::time::Duration;

use host::Host;
use logging::Logger;

use crate::{Runner, TICK_TOPIC};

const DEFAULT_TICK_HZ: f64 = 60.0;

pub struct Engine {
    extensions_dir: Option<PathBuf>,
    logger: Logger,
    tick_hz: f64,
}

impl Engine {
    pub fn builder() -> Self {
        Self {
            extensions_dir: None,
            logger: Logger::default(),
            tick_hz: DEFAULT_TICK_HZ,
        }
    }

    pub fn extensions_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.extensions_dir = Some(path.into());
        self
    }

    pub fn logger(mut self, logger: Logger) -> Self {
        self.logger = logger;
        self
    }

    /// Target rate for `run`'s loop. Extensions that never subscribe to
    /// `core/tick` (ADR-004) are unaffected by this either way.
    pub fn tick_hz(mut self, hz: f64) -> Self {
        self.tick_hz = hz;
        self
    }

    /// Wires the bus and every `.wasm` file found in `extensions_dir`, each
    /// registered as a bus endpoint named after its file stem and
    /// subscribed to `core/tick`. A file that fails to load is logged and
    /// skipped — one bad extension never stops the others (or startup)
    /// from proceeding. Exposed publicly (not just used by `run`) since a
    /// future driver (platform's own event pump, rung 2) will want the
    /// wired-up `Runner` without this module's sleep-loop attached to it.
    pub fn build(self) -> wasmtime::Result<Runner> {
        let bus = bus::Bus::new();
        let wasm_engine = host::new_engine()?;

        if let Some(dir) = &self.extensions_dir {
            for path in find_wasm_files(dir) {
                let name = derive_extension_name(&path);
                match Host::load(&wasm_engine, &path.to_string_lossy(), self.logger.clone()) {
                    Ok(extension) => {
                        let ep = bus.register(name.clone(), extension);
                        ep.subscribe(TICK_TOPIC);
                        self.logger
                            .info("engine", &format!("loaded '{name}' from {}", path.display()));
                    }
                    Err(err) => {
                        self.logger
                            .error("engine", &format!("failed to load {}: {err}", path.display()));
                    }
                }
            }
        }

        Ok(Runner::new(bus, self.logger))
    }

    /// Runs forever at `tick_hz` until the process is killed. A thin
    /// wrapper around `Runner::step` (ADR-014's "run forever is a thin
    /// wrapper, not the primitive tests use") — real wall-clock timing,
    /// unlike the headless runner's own bounded/virtual-clock API.
    ///
    /// `dt` passed to `step` is the *measured* time since the previous
    /// tick, not the nominal period — if a tick runs long, extensions see
    /// that in `dt` rather than being told a fixed value that didn't happen.
    pub fn run(self) -> wasmtime::Result<()> {
        let period = Duration::from_secs_f64(1.0 / self.tick_hz);
        let runner = self.build()?;

        let mut last = std::time::Instant::now() - period;
        loop {
            let now = std::time::Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;

            runner.step(dt);

            let elapsed = now.elapsed();
            if elapsed < period {
                std::thread::sleep(period - elapsed);
            }
        }
    }
}

/// `.wasm` files directly inside `dir`, sorted for deterministic load order.
/// A missing directory is "no extensions," not an error.
fn find_wasm_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "wasm"))
        .collect();
    files.sort();
    files
}

fn derive_extension_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::RecordingSink;
    use std::sync::Arc;

    // Built by extensions/hello/build.ps1 (see its README).
    const HELLO_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../extensions/hello/target/wasm32-wasip2/release"
    );

    #[test]
    fn tick_hz_defaults_to_60_and_is_overridable() {
        assert_eq!(Engine::builder().tick_hz, DEFAULT_TICK_HZ);
        assert_eq!(Engine::builder().tick_hz(30.0).tick_hz, 30.0);
    }

    #[test]
    fn find_wasm_files_finds_only_wasm_extensions_sorted() {
        let files = find_wasm_files(Path::new(HELLO_DIR));
        assert!(
            files.iter().all(|f| f.extension().unwrap() == "wasm"),
            "expected only .wasm files, got {files:?}"
        );
        assert!(
            files.iter().any(|f| f.file_stem().unwrap() == "hello"),
            "expected hello.wasm in {files:?} — run extensions/hello/build.ps1 first"
        );
        let mut sorted = files.clone();
        sorted.sort();
        assert_eq!(files, sorted);
    }

    #[test]
    fn find_wasm_files_on_a_missing_directory_is_empty_not_an_error() {
        assert_eq!(find_wasm_files(Path::new("no/such/directory")), Vec::<PathBuf>::new());
    }

    #[test]
    fn derive_extension_name_is_the_file_stem() {
        assert_eq!(derive_extension_name(Path::new("/a/b/hello.wasm")), "hello");
    }

    #[test]
    fn build_discovers_loads_and_registers_a_real_extension() {
        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));

        let runner = Engine::builder()
            .extensions_dir(HELLO_DIR)
            .logger(logger)
            .build()
            .expect("build extensions/hello first: pwsh extensions/hello/build.ps1");

        runner.step(1.0 / 60.0);

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("loaded 'hello'")),
            "expected a load confirmation, got {records:?}"
        );
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("tick")),
            "expected the extension's tick log line, got {records:?}"
        );
    }

    #[test]
    fn build_skips_a_component_that_fails_to_load_without_failing_the_engine() {
        let dir = std::env::temp_dir().join("bones-engine-test-bad-extension");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.wasm"), b"not a real component").unwrap();

        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));

        let runner = Engine::builder()
            .extensions_dir(&dir)
            .logger(logger)
            .build()
            .expect("a bad component must be skipped, not fail the whole engine");
        runner.step(1.0 / 60.0);

        std::fs::remove_dir_all(&dir).ok();

        let records = sink.records();
        assert!(
            records.iter().any(|(_, _, msg)| msg.contains("failed to load")),
            "expected a load failure to be logged, got {records:?}"
        );
    }
}
