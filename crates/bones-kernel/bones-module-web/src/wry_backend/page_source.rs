//! Serves `PanelSource::Html` through a custom protocol instead of handing the
//! markup straight to the webview.
//!
//! - A document set from a string loads at an *opaque* origin: `localStorage`
//!   and `sessionStorage` throw on access, and every same-origin check fails.
//!   Any page that keeps state — which is most of them — breaks on load, and
//!   an embedder's only workaround is to serve the page itself over HTTP.
//! - A custom protocol gives the same markup a real origin, at the cost of one
//!   handler per panel.
//! - The URL is written in the `{scheme}://localhost/` form on every platform;
//!   wry rewrites it to `http://{scheme}.localhost/` where the platform needs
//!   that (Windows, Android).

use std::borrow::Cow;

use wry::http::{header::CONTENT_TYPE, Response};

/// Scheme the panel's own markup is served under.
pub const PROTOCOL: &str = "bones";

/// Address of the served page, in wry's portable custom-protocol form.
pub const URL: &str = "bones://localhost/";

/// Builds the response carrying the panel's markup.
pub fn response(html: &[u8]) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .header(CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Cow::Owned(html.to_vec()))
        // The status, the one header and the body are all fixed here, so there
        // is nothing left for the builder to reject.
        .expect("panel page response is well formed")
}
