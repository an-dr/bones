//! Application-side upgrading, owned by one native module (ADR-039).
//!
//! Version semantics are shared with the launcher through bones-versioning.
//! Update operations are private implementation, invoked by module methods
//! and the existing `upgrader/*` bus contract.

mod checksum;
mod companion;
mod download;
mod fetch;
mod https_fetch;
mod identity;
mod install;
mod installation;
mod manifest;
mod release_source;
mod rollback;
mod updater;
mod upgrade_config;
mod upgrade_module;
mod versions;

pub use companion::{Outcome as CompanionOutcome, Policy as CompanionPolicy};
pub use fetch::Fetch;
pub use https_fetch::HttpsFetch;
pub use identity::AppIdentity;
pub use release_source::ReleaseSource;
pub use upgrade_config::UpgradeConfig;
pub use upgrade_module::UpgradeModule;

#[cfg(test)]
use bones_versioning::is_newer;
use checksum::verify_checksum;
use download::{download_asset_verified, fetch_manifest};
#[cfg(test)]
use identity::host_identity;
#[cfg(test)]
use install::remove_version_dir;
use install::{copy_version_from_dir, extract_version, install_fresh};
use installation::{
    default_install_dir, is_installed_version_dir, is_version_folder, shared_or_exe_relative,
};
use manifest::{parse_manifest, Manifest};
#[cfg(test)]
use release_source::PLATFORM;
use versions::{current_version_dir, previous_version_dir};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod implementation_tests;
