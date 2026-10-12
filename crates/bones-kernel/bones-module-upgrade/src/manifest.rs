/// A hosted update announcement: the newest available version, where to
/// download it, and an optional integrity check. Checksum is optional
/// because not every manifest publisher can commit to one immediately, not
/// because it is unimportant — `download_asset_verified` refuses to accept
/// a mismatch when one is supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub version: String,
    pub url: String,
    pub sha256: Option<String>,
}

/// Parses a manifest from its JSON body. Manual field extraction rather than
/// `serde`'s derive macros, matching this workspace's existing
/// `serde_json::Value` convention (no other crate here pulls in `serde`
/// derive).
pub(crate) fn parse_manifest(body: &str) -> Result<Manifest, String> {
    let value: serde_json::Value = serde_json::from_str(body).map_err(|error| error.to_string())?;
    let version = value
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| String::from("manifest is missing a string \"version\""))?
        .to_string();
    let url = value
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| String::from("manifest is missing a string \"url\""))?
        .to_string();
    let sha256 = value
        .get("sha256")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(Manifest {
        version,
        url,
        sha256,
    })
}
