use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use bones_kernel::bus::Envelope;
use bones_kernel::logging::{Logger, RecordingSink};
use bones_kernel::{BuiltKernel, Kernel};
use bones_messages::gfx::Command;
use bones_messages::{DecodeMessage, EncodeMessage};

use crate::recorder::Recorder;
use crate::recording::Recording;
use crate::Error;

/// The `dt` of every step: one frame at 60 Hz.
const DT: f32 = 1.0 / 60.0;

/// The sender of messages a test sends, the name the engine's own input
/// carries, so an extension filtering on it cannot tell the difference.
const SENDER: &str = "platform";

/// One extension running on a headless kernel, with everything published on
/// the bus recorded.
pub struct App {
    kernel: BuiltKernel,
    recording: Arc<Mutex<Recording>>,
    log: RecordingSink,
    name: String,
    envelopes: Vec<Envelope>,
    frame: BTreeMap<String, Vec<Envelope>>,
    /// The save directory `load` made, removed on drop.
    scratch_saves: Option<std::path::PathBuf>,
    /// The `request_id` of the last screenshot asked for.
    #[cfg(feature = "render")]
    last_capture: u32,
    /// Held while the renderer lives; declared last so it is released only
    /// after the kernel, and SDL with it, has shut down.
    #[cfg(feature = "render")]
    _rendering: Option<std::sync::MutexGuard<'static, ()>>,
}

impl App {
    /// Loads the `.wasm` at `path` into a headless kernel and runs its
    /// `init`. Its name is the file stem, as in the engine. It saves into an
    /// empty directory of its own, removed when the `App` is dropped, so
    /// tests running side by side never see each other's state.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let saves = scratch_saves_dir();
        let mut app = Self::with_kernel(Kernel::new().saves_dir(&saves), path)?;
        app.scratch_saves = Some(saves);
        Ok(app)
    }

    /// As [`App::load`], but with the real renderer drawing into an
    /// offscreen window of `width` × `height`, so [`App::screenshot`]
    /// returns the pixels a player would see. Needs no display.
    ///
    /// SDL runs one window per process, so rendered `App`s take turns: a
    /// second one waits until the first is dropped. SDL's offscreen driver
    /// is selected for the whole test process.
    #[cfg(feature = "render")]
    pub fn rendered(path: impl AsRef<Path>, width: u32, height: u32) -> Result<Self, Error> {
        static RENDERING: Mutex<()> = Mutex::new(());
        // A test that panicked while rendering left nothing to recover.
        let turn = RENDERING
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let saves = scratch_saves_dir();
        let kernel = Kernel::new()
            .saves_dir(&saves)
            .window("bones-test", width, height)
            .offscreen()
            .renderer();
        let mut app = Self::with_kernel(kernel, path)?;
        app.scratch_saves = Some(saves);
        app._rendering = Some(turn);
        Ok(app)
    }

    /// As [`App::load`], on a kernel you configured: permissions, budgets,
    /// timeouts, or native modules. A window must be `.offscreen()`, as
    /// [`App::rendered`] makes it; without the `render` feature there is
    /// none. Its logger is replaced so [`App::logs`] can read the log.
    pub fn with_kernel(kernel: Kernel, path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        if !path.is_file() {
            return Err(format!(
                "{} does not exist; build the extension first, for example with bones_test::build",
                path.display()
            ));
        }
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .ok_or_else(|| format!("{} has no file name", path.display()))?;
        let log = RecordingSink::new();
        let recording = Arc::new(Mutex::new(Recording::default()));
        let kernel = kernel
            .logger(Logger::new(Arc::new(log.clone())))
            .module(Recorder {
                recording: recording.clone(),
            })
            .catalog_extension(name.clone(), path)
            .build()
            .map_err(|error| error.to_string())?;
        let mut app = Self {
            kernel,
            recording,
            log,
            name,
            envelopes: Vec::new(),
            frame: BTreeMap::new(),
            scratch_saves: None,
            #[cfg(feature = "render")]
            last_capture: 0,
            #[cfg(feature = "render")]
            _rendering: None,
        };
        if !app.kernel.registry.contains(&app.name) {
            return Err(format!(
                "'{}' did not load:\n{}",
                app.name,
                app.logs().join("\n")
            ));
        }
        app.flush();
        Ok(app)
    }

    /// Publishes a core message as the engine's platform would; it arrives
    /// during the next [`App::step`].
    pub fn send<M: EncodeMessage>(&mut self, message: &M) -> &mut Self {
        self.send_raw(M::TOPIC, message.encode())
    }

    /// Publishes any payload on any topic, for an extension's own topics.
    pub fn send_raw(&mut self, topic: &str, payload: Vec<u8>) -> &mut Self {
        self.kernel.runner.bus().publish(Envelope {
            topic: topic.to_string(),
            sender: SENDER.to_string(),
            correlation: None,
            payload,
        });
        self
    }

    /// Makes a direct `send` to the extension, as another extension would,
    /// and returns its reply; the extension sees an empty topic and the
    /// sender `bones-test`.
    pub fn request(&mut self, payload: &[u8]) -> Result<Vec<u8>, Error> {
        let reply = self
            .kernel
            .registry
            .call("bones-test", &self.name, payload)
            .map_err(|error| format!("send to '{}' failed: {error:?}", self.name));
        self.flush();
        reply
    }

    /// Runs `frames` frames of 1/60 s each, then delivers what the last one
    /// published, so the next read sees it.
    pub fn step(&mut self, frames: u32) -> &mut Self {
        for _ in 0..frames {
            self.kernel.step(DT);
        }
        self.flush();
        self
    }

    /// Every recorded message of type `M`, oldest first, from any sender.
    /// A recorded payload that does not decode is skipped; [`App::logs`]
    /// carries the host's report of it.
    pub fn published<'a, M: DecodeMessage<'a>>(&'a self) -> Vec<M> {
        self.envelopes
            .iter()
            .filter(|envelope| envelope.topic == M::TOPIC)
            .filter_map(|envelope| M::decode(&envelope.payload).ok())
            .collect()
    }

    /// Every recorded envelope except ticks, oldest first.
    pub fn messages(&self) -> &[Envelope] {
        &self.envelopes
    }

    /// Forgets the recorded messages; the last frame stays.
    pub fn clear(&mut self) -> &mut Self {
        self.envelopes.clear();
        self
    }

    /// The last frame as text: a `[sender]` line, then each `gfx/*` command
    /// that sender drew, one per line.
    pub fn frame_text(&self) -> String {
        let mut text = String::new();
        for (sender, batch) in &self.frame {
            text.push_str(&format!("[{sender}]\n"));
            for envelope in batch {
                let line = match Command::decode(&envelope.topic, &envelope.payload) {
                    Ok(Some(command)) => format!("{command:?}"),
                    Ok(None) => envelope.topic.clone(),
                    Err(error) => format!("{} (does not decode: {error})", envelope.topic),
                };
                text.push_str(&line);
                text.push('\n');
            }
        }
        text
    }

    /// The frame as the renderer draws it now, without advancing the
    /// simulation: the extension's last `gfx/*` batch, composited.
    ///
    /// Fails on an `App` without a renderer, and with the renderer's own
    /// error when the capture failed.
    #[cfg(feature = "render")]
    pub fn screenshot(&mut self) -> Result<crate::Screenshot, Error> {
        use bones_messages::renderer::{Capture, Captured, Scope};
        if self.kernel.is_headless() {
            return Err("this App has no window to capture; load it with App::rendered".into());
        }
        self.last_capture += 1;
        let request_id = self.last_capture;
        self.send(&Capture {
            request_id,
            scope: Scope::Canvas,
        });
        // A frame without a tick, as the engine runs one while paused: the
        // capture reaches the renderer, which answers it while presenting.
        self.kernel.runner.begin_frame();
        self.kernel.runner.bus().dispatch();
        self.kernel.present_frame();
        self.flush();
        let captured = self
            .published::<Captured>()
            .into_iter()
            .rev()
            .find(|captured| captured.request_id == request_id)
            .map(|captured| (captured.width, captured.height, captured.png.to_vec()));
        match captured {
            Some((0, _, _)) | Some((_, 0, _)) => Err(format!(
                "the renderer could not capture the frame:\n{}",
                self.logs().join("\n")
            )),
            Some((width, height, png)) => crate::Screenshot::from_png(width, height, png),
            None => Err("no renderer answered the capture; compose one with .renderer()".into()),
        }
    }

    /// Every log line so far, as `LEVEL category: message`.
    pub fn logs(&self) -> Vec<String> {
        self.log
            .records()
            .into_iter()
            .map(|(level, category, message)| {
                format!(
                    "{} {category}: {message}",
                    format!("{level:?}").to_uppercase()
                )
            })
            .collect()
    }

    /// Whether the extension still runs: `false` once it was quarantined.
    pub fn is_running(&self) -> bool {
        self.kernel.registry.contains(&self.name)
    }

    /// Delivers what is queued, then moves what the recorder heard here.
    fn flush(&mut self) {
        self.kernel.runner.bus().dispatch();
        let mut recording = self.recording.lock().unwrap();
        self.envelopes.append(&mut recording.envelopes);
        self.frame = recording.latest_frame();
    }
}

impl Drop for App {
    /// Calls the extension's `shutdown`, as closing the engine would.
    fn drop(&mut self) {
        self.kernel.shutdown();
        if let Some(saves) = &self.scratch_saves {
            let _ = std::fs::remove_dir_all(saves);
        }
    }
}

/// A save directory no other `App` in any process uses.
fn scratch_saves_dir() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "bones-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
