/// What a new project starts as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Game,
    Egui,
    Web,
    Embed,
}

const GAME_LIB: &str = include_str!("../templates/game/lib.rs");
const GAME_CHECK: &str = include_str!("../templates/game/check.rs");
const EGUI_LIB: &str = include_str!("../templates/egui/lib.rs");
const EGUI_CHECK: &str = include_str!("../templates/egui/check.rs");
const WEB_LIB: &str = include_str!("../templates/web/lib.rs");
const WEB_CHECK: &str = include_str!("../templates/web/check.rs");
const EMBED_APP_MANIFEST: &str = include_str!("../templates/embed/app.Cargo.toml");
const EMBED_MAIN: &str = include_str!("../templates/embed/main.rs");

impl Kind {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "game" => Some(Self::Game),
            "egui" => Some(Self::Egui),
            "web" => Some(Self::Web),
            "embed" => Some(Self::Embed),
            _ => None,
        }
    }

    /// The extension's source: its path under `templates/` and the
    /// compiled-in copy.
    pub fn extension(self) -> (&'static str, &'static str) {
        match self {
            Self::Game | Self::Embed => ("game/lib.rs", GAME_LIB),
            Self::Egui => ("egui/lib.rs", EGUI_LIB),
            Self::Web => ("web/lib.rs", WEB_LIB),
        }
    }

    /// The headless check of the extension, as `extension` gives it.
    pub fn check(self) -> (&'static str, &'static str) {
        match self {
            Self::Game | Self::Embed => ("game/check.rs", GAME_CHECK),
            Self::Egui => ("egui/check.rs", EGUI_CHECK),
            Self::Web => ("web/check.rs", WEB_CHECK),
        }
    }

    /// The native app's manifest and source, for the kind that has one, as
    /// `extension` gives them.
    pub fn app(self) -> Option<[(&'static str, &'static str); 2]> {
        (self == Self::Embed).then_some([
            ("embed/app.Cargo.toml", EMBED_APP_MANIFEST),
            ("embed/main.rs", EMBED_MAIN),
        ])
    }

    /// The `bones.toml` lines this kind needs beyond the shared ones.
    pub fn config(self) -> &'static str {
        match self {
            Self::Web => "web = true\n",
            Self::Game | Self::Egui | Self::Embed => "",
        }
    }
}
