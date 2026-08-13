/// Event produced by a browser backend and consumed during `Web::render`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendEvent {
    PageMessage {
        owner: String,
        panel: String,
        json: String,
    },
    Closed {
        owner: String,
        panel: String,
    },
}
