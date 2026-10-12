//! Where an application's releases are published.

/// This build's platform, fixed at compile time. It names the manifest a
/// build reads, so an installed Windows arm64 copy is never offered an x64
/// archive. `None` on a platform with no published releases.
pub const PLATFORM: Option<&str> = if cfg!(all(windows, target_arch = "x86_64")) {
    Some("windows-x64")
} else if cfg!(all(windows, target_arch = "aarch64")) {
    Some("windows-arm64")
} else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
    Some("linux-x64")
} else {
    None
};

/// The manifest an application checks for updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseSource {
    /// The latest GitHub release of `owner/repo`, whose assets include one
    /// `<app>-<platform>.json` manifest per platform (see the README's
    /// release convention).
    GitHub {
        owner: String,
        repo: String,
        app: String,
    },
    /// An explicit manifest URL, typically a user's override.
    Url(String),
}

impl ReleaseSource {
    /// `override_url` when it holds anything but whitespace, otherwise
    /// `default` — the rule an application applies to a settings value it
    /// lets the user change. An empty override means the default source.
    pub fn with_override(default: ReleaseSource, override_url: &str) -> ReleaseSource {
        match override_url.trim() {
            "" => default,
            url => ReleaseSource::Url(url.to_string()),
        }
    }

    /// The manifest URL to fetch, or `None` for a GitHub source on a
    /// platform that has no releases.
    pub fn manifest_url(&self) -> Option<String> {
        match self {
            ReleaseSource::GitHub { owner, repo, app } => PLATFORM.map(|platform| {
                format!("https://github.com/{owner}/{repo}/releases/latest/download/{app}-{platform}.json")
            }),
            ReleaseSource::Url(url) => Some(url.clone()),
        }
    }
}
