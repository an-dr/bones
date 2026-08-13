use bones_messages::web::PanelSource;

use crate::BackendEvent;

/// Browser-runtime boundary used by the web protocol core.
pub trait Backend: Send {
    fn open(&mut self, owner: &str, panel: &str, source: PanelSource<'_>) -> Result<(), String>;
    fn close(&mut self, owner: &str, panel: &str) -> Result<(), String>;
    fn navigate(&mut self, owner: &str, panel: &str, url: &str) -> Result<(), String>;
    fn send_json(&mut self, owner: &str, panel: &str, json: &str) -> Result<(), String>;
    /// Synchronizes native state once per render frame.
    fn update(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn drain_events(&mut self) -> Vec<BackendEvent>;
}
