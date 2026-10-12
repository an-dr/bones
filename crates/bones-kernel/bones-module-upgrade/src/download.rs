use crate::{parse_manifest, verify_checksum, Fetch, Manifest};
use base64::Engine;

/// Splits a [`Fetch::fetch_url`] body into its content type and decoded bytes.
pub(crate) fn decode_fetch_result(value: &str) -> Result<(String, Vec<u8>), String> {
    let (content_type, encoded) = value.split_once(";base64,").ok_or_else(|| {
        String::from("fetch_url result is not in the expected content-type;base64,data form")
    })?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| error.to_string())?;
    Ok((content_type.to_string(), bytes))
}

/// Fetches and parses the manifest at `url`. Takes the backend as a
/// trait object so a test can inject a stub instead of hitting the network.
pub(crate) fn fetch_manifest(backend: &dyn Fetch, url: &str) -> Result<Manifest, String> {
    let fetched = backend
        .fetch_url(url)?
        .ok_or_else(|| format!("no manifest found at {url}"))?;
    let (_, bytes) = decode_fetch_result(&fetched)?;
    let body = String::from_utf8(bytes).map_err(|error| error.to_string())?;
    parse_manifest(&body)
}

/// Downloads the asset a manifest points at, refusing it outright if a
/// supplied checksum does not match — a truncated or tampered download is
/// treated identically to a network failure, never silently accepted.
pub(crate) fn download_asset_verified(
    backend: &dyn Fetch,
    manifest: &Manifest,
) -> Result<Vec<u8>, String> {
    let fetched = backend
        .fetch_url(&manifest.url)?
        .ok_or_else(|| format!("no asset found at {}", manifest.url))?;
    let (_, bytes) = decode_fetch_result(&fetched)?;
    if let Some(expected) = &manifest.sha256 {
        if !verify_checksum(&bytes, expected) {
            return Err(String::from(
                "downloaded asset does not match the manifest's sha256",
            ));
        }
    }
    Ok(bytes)
}
