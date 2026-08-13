/// Sends opaque JSON from an extension to its panel page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendJson<'a> {
    pub panel: &'a str,
    pub json: &'a str,
}
