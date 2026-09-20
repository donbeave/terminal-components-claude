//! Finite TP scenario axes owned by this adapter.

use junie_tui::ColorLevel;

/// TP-001 parent identity.
pub const SCENARIO_TP001: &str = "TP-001";
/// Preset C: Connections launch with no `--connect`.
pub const PRESET_C: &str = "C";
/// Named first-frame checkpoint of TP-001.
pub const CHECKPOINT_INITIAL: &str = "initial";

/// `ALL` sizes from the `TablePro` scenario contract: 80×24, 100×30, 120×40, 160×50.
pub const ALL_SIZES: [(u16, u16); 4] = [(80, 24), (100, 30), (120, 40), (160, 50)];

/// Color-profile expansion: truecolor, 256, 16, explicit mono/`none`, and `NO_COLOR`.
pub const COLOR_PROFILES: [ColorProfile; 5] = [
    ColorProfile::Truecolor,
    ColorProfile::Ansi256,
    ColorProfile::Ansi16,
    ColorProfile::None,
    ColorProfile::NoColor,
];

/// One finite color axis of an ALL-row expansion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColorProfile {
    /// `--color truecolor` (and alias `24bit`).
    Truecolor,
    /// `--color 256` (and alias `ansi256`).
    Ansi256,
    /// `--color 16` (and alias `ansi16`).
    Ansi16,
    /// `--color none` (and alias `mono`).
    None,
    /// `NO_COLOR` present with a non-empty value; not a `--color` flag.
    NoColor,
}

impl ColorProfile {
    /// Stable profile label used in observation identity.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Truecolor => "truecolor",
            Self::Ansi256 => "256",
            Self::Ansi16 => "16",
            Self::None => "none",
            Self::NoColor => "nocolor",
        }
    }

    /// `--color` value that selects this profile, if the profile is a CLI flag.
    #[must_use]
    pub const fn cli_color_value(self) -> Option<&'static str> {
        match self {
            Self::Truecolor => Some("truecolor"),
            Self::Ansi256 => Some("256"),
            Self::Ansi16 => Some("16"),
            Self::None => Some("none"),
            Self::NoColor => None,
        }
    }

    /// Production color level for this profile.
    ///
    /// `NoColor` uses the same `ColorLevel::from_env` table as `ColorLevel::detect`.
    #[must_use]
    pub fn color_level(self) -> ColorLevel {
        match self {
            Self::Truecolor => ColorLevel::TrueColor,
            Self::Ansi256 => ColorLevel::Ansi256,
            Self::Ansi16 => ColorLevel::Ansi16,
            Self::None => ColorLevel::Mono,
            Self::NoColor => ColorLevel::from_env(
                None,
                Some(std::ffi::OsStr::new("1")),
                Some("xterm-256color"),
                Some("truecolor"),
                true,
            ),
        }
    }
}
