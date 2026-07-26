//! Backend-independent web panel ownership and bus protocol.

mod backend;
mod backend_event;
mod web;

pub use backend::Backend;
pub use backend_event::BackendEvent;
pub use web::Web;
