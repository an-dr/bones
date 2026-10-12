//! The `devtools` native module: an optional local channel for driving a
//! running application from outside it.
//!
//! An application's interface is a web page that talks to the host only
//! through bus messages. Clicking it from a script is fragile — a webview
//! under Xwayland ignores synthetic X input, and pixel positions drift — so
//! this channel works at the message layer instead: it posts page messages as
//! if the page had sent them, and records the bus so a test can read back
//! what a message caused.
//!
//! Off unless the variable the composition names holds a port number. It is
//! an unauthenticated hole into the running application — anything that can
//! reach the port can act as the user — so it binds to loopback only, refuses
//! browser requests, and a shipped launch never sets the variable.

mod devtools;
mod recording;
mod request;
mod server;

pub use devtools::Devtools;

#[cfg(test)]
mod tests;
