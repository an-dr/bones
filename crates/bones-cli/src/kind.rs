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

    /// The extension's source.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Game | Self::Embed => GAME_LIB,
            Self::Egui => EGUI_LIB,
            Self::Web => WEB_LIB,
        }
    }

    /// The headless check of the extension.
    pub fn check(self) -> &'static str {
        match self {
            Self::Game | Self::Embed => GAME_CHECK,
            Self::Egui => EGUI_CHECK,
            Self::Web => WEB_CHECK,
        }
    }

    /// The native app's manifest and source, for the kind that has one.
    pub fn app(self) -> Option<(&'static str, &'static str)> {
        (self == Self::Embed).then_some((EMBED_APP_MANIFEST, EMBED_MAIN))
    }

    /// The `bones.toml` lines this kind needs beyond the shared ones.
    pub fn config(self) -> &'static str {
        match self {
            Self::Web => "web = true\n",
            Self::Game | Self::Egui | Self::Embed => "",
        }
    }
}
