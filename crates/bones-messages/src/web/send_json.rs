/// Sends opaque JSON from an extension to its panel page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendJson<'a> {
    /// The open panel to send to.
    pub panel: &'a str,
    /// The JSON to deliver to its page.
    pub json: &'a str,
}
