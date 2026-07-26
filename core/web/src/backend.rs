use bones_messages::web::PanelSource;

use crate::BackendEvent;

/// Browser-runtime boundary used by the web protocol core.
pub trait Backend: Send {
    fn open(&mut self, owner: &str, panel: &str, source: PanelSource<'_>) -> Result<(), String>;
    fn close(&mut self, owner: &str, panel: &str) -> Result<(), String>;
    fn navigate(&mut self, owner: &str, panel: &str, url: &str) -> Result<(), String>;
    fn send_json(&mut self, owner: &str, panel: &str, json: &str) -> Result<(), String>;
    fn drain_events(&mut self) -> Vec<BackendEvent>;
}
