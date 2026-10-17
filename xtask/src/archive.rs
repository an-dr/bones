//! Zip archives and their SHA-256 checksums, for release bundles and the
//! self-update example's asset.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::Result;

/// Zips the contents of `source` into `archive`, each entry under `prefix`
/// (empty for none).
pub fn write_zip(source: &Path, prefix: &str, archive: &Path) -> Result {
    let file = File::create(archive)
        .map_err(|err| format!("could not create {}: {err}", archive.display()))?;
    let mut zip = ZipWriter::new(file);
    add_dir(&mut zip, source, prefix)
        .and_then(|()| zip.finish().map(|_| ()).map_err(io::Error::other))
        .map_err(|err| format!("could not write {}: {err}", archive.display()))
}

fn add_dir(zip: &mut ZipWriter<File>, dir: &Path, prefix: &str) -> io::Result<()> {
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
    // Sorted so an archive lists its files in the same order on every machine.
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        if entry.path().is_dir() {
            zip.add_directory(path.as_str(), options)
                .map_err(io::Error::other)?;
            add_dir(zip, &entry.path(), &path)?;
        } else {
            zip.start_file(path.as_str(), options)
                .map_err(io::Error::other)?;
            zip.write_all(&fs::read(entry.path())?)?;
        }
    }
    Ok(())
}

/// The lowercase hex SHA-256 of the file at `path`.
pub fn sha256_hex(path: &Path) -> Result<String> {
    let bytes =
        fs::read(path).map_err(|err| format!("could not read {}: {err}", path.display()))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
