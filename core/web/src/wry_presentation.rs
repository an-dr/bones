use bus::{Bus, ModuleRegistration, Registry, ServiceRegistry};
use logging::Logger;
use platform::Platform;

use crate::{Web, WryBackend};

/// A detachable SDL window and wry-backed `web` module on an existing bus.
///
/// The owning engine may remain headless: presentation resources exist only
/// between `open` and `close`, and the endpoint can be attached again later.
pub struct WryPresentation {
    bus: Bus,
    platform: Option<Platform>,
    web: ModuleRegistration,
}

impl WryPresentation {
    /// Opens a resizable native window and attaches the `web` endpoint.
    pub fn open(
        bus: Bus,
        registry: Registry,
        logger: Logger,
        title: impl AsRef<str>,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let mut platform = Platform::new(title.as_ref(), width, height)?;
        let window = platform
            .take_window()
            .ok_or_else(|| "presentation window is unavailable".to_string())?;
        let backend = WryBackend::new(&window)?;
        drop(window);
        let mut services = ServiceRegistry::new();
        let web = ModuleRegistration::attach(
            bus.clone(),
            registry,
            &mut services,
            Web::new(bus.clone(), logger, backend),
        )?;
        Ok(Self {
            bus,
            platform: Some(platform),
            web,
        })
    }

    /// Polls native events and advances the web backend once.
    ///
    /// Returns `true` after the native window asks to close.
    pub fn update(&mut self) -> bool {
        let Some(platform) = self.platform.as_mut() else {
            return true;
        };
        platform.poll_events(&self.bus, "platform");
        self.web.render();
        platform.quit_requested()
    }

    /// Closes every panel and detaches the `web` endpoint. Idempotent.
    pub fn close(&mut self) {
        self.web.detach();
        self.platform = None;
    }

    /// Reports whether the presentation endpoint is attached.
    pub fn is_open(&self) -> bool {
        self.web.is_attached() && self.platform.is_some()
    }
}
