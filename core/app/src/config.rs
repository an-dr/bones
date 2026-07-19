//! `bones.toml` in the working directory configures the default composition
//! (structure.md); every field defaults to today's hardcoded values, so
//! running with no config file is unchanged.

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub extensions_dir: String,
    pub window_title: String,
    pub window_width: u32,
    pub window_height: u32,
    pub renderer: bool,
    pub ui: bool,
    pub audio: bool,
    pub game_core: bool,
    pub saves_dir: String,
    pub persistence_read_only: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            extensions_dir: "extensions".to_string(),
            window_title: "bones".to_string(),
            window_width: 800,
            window_height: 600,
            renderer: true,
            ui: true,
            // Unlike renderer/ui, not every deployment target has a working
            // audio device (headless CI, some containers) — opt-in via
            // bones.toml rather than risking `Engine::build()` failing by
            // default on environments this scaffold hasn't been proven
            // against yet.
            audio: false,
            // Opt-in, same as audio: a general-purpose GUI project (the
            // README's other named use case) has no use for game-core's
            // ECS/collision/tilemap capability — enabling it unconditionally
            // would cost every non-game project a Bus service registration
            // and a rapier2d/hecs dependency it never touches.
            game_core: false,
            // persistence itself is unconditional (core/wasm-extensions —
            // creating a local directory always succeeds, unlike opening
            // an audio device, so there's no equivalent resource-scarcity
            // reason to make it opt-in). Only read-only mode — a policy
            // choice, not a resource one — is configurable, and defaults
            // off (extensions can save).
            saves_dir: "saves".to_string(),
            persistence_read_only: false,
        }
    }
}

impl Config {
    /// A missing file is `Config::default()`, not an error; anything else
    /// wrong with it (unreadable, malformed TOML, an unknown field — likely
    /// a typo) is, so a broken config a user actually wrote never silently
    /// no-ops.
    pub fn load(path: &str) -> Result<Self, String> {
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(err) => return Err(format!("reading {path}: {err}")),
        };
        toml::from_str(&contents).map_err(|err| format!("parsing {path}: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_file_is_the_default_config() {
        let config = Config::load("no/such/bones.toml").unwrap();
        assert_eq!(config.extensions_dir, "extensions");
        assert_eq!(config.window_title, "bones");
        assert_eq!((config.window_width, config.window_height), (800, 600));
        assert!(config.renderer);
        assert!(config.ui);
        assert!(!config.audio, "not every deployment target has a working audio device");
        assert!(!config.game_core, "not every project is a game");
        assert_eq!(config.saves_dir, "saves");
        assert!(!config.persistence_read_only, "extensions can save by default");
    }

    #[test]
    fn a_partial_file_fills_in_defaults_for_missing_fields() {
        let dir = std::env::temp_dir().join("bones-app-test-partial-config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bones.toml");
        std::fs::write(&path, "window_width = 1920\nwindow_height = 1080\n").unwrap();

        let config = Config::load(path.to_str().unwrap());

        std::fs::remove_dir_all(&dir).ok();

        let config = config.unwrap();
        assert_eq!((config.window_width, config.window_height), (1920, 1080));
        assert_eq!(config.extensions_dir, "extensions", "omitted field should default");
    }

    #[test]
    fn malformed_toml_is_an_error_not_a_silent_default() {
        let dir = std::env::temp_dir().join("bones-app-test-bad-config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bones.toml");
        std::fs::write(&path, "this is not valid toml =====").unwrap();

        let result = Config::load(path.to_str().unwrap());

        std::fs::remove_dir_all(&dir).ok();

        assert!(result.is_err());
    }

    #[test]
    fn an_unknown_field_is_an_error_not_silently_ignored() {
        let dir = std::env::temp_dir().join("bones-app-test-unknown-field-config");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bones.toml");
        std::fs::write(&path, "windo_width = 1920\n").unwrap();

        let result = Config::load(path.to_str().unwrap());

        std::fs::remove_dir_all(&dir).ok();

        assert!(result.is_err(), "a typo'd field name should not be silently ignored");
    }
}
