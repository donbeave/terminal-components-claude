//! Termrock semantic theme, surfaces, roles, and capability resolution.
//!
//! Provides capability-aware semantic style resolution across TrueColor,
//! Ansi256, Ansi16, and Mono output, reproducing exact visual-baseline
//! tokens while supporting scoped family/variant/part patches.

use ratatui::style::{Color, Modifier, Style};
use std::collections::BTreeMap;

/// Terminal color capability level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum ColorLevel {
    #[default]
    TrueColor,
    Ansi256,
    Ansi16,
    Mono,
}

impl ColorLevel {
    /// Detect color capability from environment variables (NO_COLOR, COLORTERM, TERM).
    pub fn detect() -> Self {
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            return ColorLevel::Mono;
        }
        let colorterm = std::env::var("COLORTERM").unwrap_or_default();
        if colorterm == "truecolor" || colorterm == "24bit" {
            return ColorLevel::TrueColor;
        }
        let term = std::env::var("TERM").unwrap_or_default();
        if term.contains("256color") || term.contains("ghostty") || term.contains("kitty") {
            return ColorLevel::Ansi256;
        }
        ColorLevel::Ansi16
    }

    pub fn label(self) -> &'static str {
        match self {
            ColorLevel::TrueColor => "truecolor",
            ColorLevel::Ansi256 => "256 colors",
            ColorLevel::Ansi16 => "16 colors",
            ColorLevel::Mono => "no color",
        }
    }
}

impl From<crate::theme::ColorLevel> for ColorLevel {
    fn from(level: crate::theme::ColorLevel) -> Self {
        match level {
            crate::theme::ColorLevel::TrueColor => Self::TrueColor,
            crate::theme::ColorLevel::Ansi256 => Self::Ansi256,
            crate::theme::ColorLevel::Ansi16 => Self::Ansi16,
            crate::theme::ColorLevel::Mono => Self::Mono,
        }
    }
}

impl From<ColorLevel> for crate::theme::ColorLevel {
    fn from(level: ColorLevel) -> Self {
        match level {
            ColorLevel::TrueColor => Self::TrueColor,
            ColorLevel::Ansi256 => Self::Ansi256,
            ColorLevel::Ansi16 => Self::Ansi16,
            ColorLevel::Mono => Self::Mono,
        }
    }
}

/// Semantic surface tier hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum Surface {
    #[default]
    Canvas,
    Surface,
    Elevated,
    Overlay,
    Popover,
    Field,
    FieldHover,
}

/// Semantic role tokens for foreground, border, and accents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    Primary,
    Secondary,
    Accent,
    Danger,
    Success,
    Warning,
    Muted,
    Subtle,
    Border,
    BorderFocused,
    BorderSelected,
    Selection,
    Text,
    TextMuted,
    TextDisabled,
    Background,
    BackdropDim,
}

/// Semantic text tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum Tone {
    #[default]
    Normal,
    Primary,
    Accent,
    Success,
    Warning,
    Danger,
    Muted,
}

/// Component family classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Family {
    Button,
    Panel,
    Split,
    List,
    Grid,
    Input,
    Dialog,
    Menu,
    Status,
    Custom(&'static str),
}

/// Visual presentation variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum Variant {
    Primary,
    Secondary,
    Subtle,
    Danger,
    Toggle,
    Quiet,
    Ghost,
    #[default]
    Default,
}

use crate::termrock::identity::Part;

/// Patch slot state distinguishing inheritance, replacement, and explicit clearing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum PatchSlot<T> {
    #[default]
    Inherit,
    Set(T),
    Clear,
}

impl<T> PatchSlot<T> {
    pub const fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit)
    }

    pub const fn is_set(&self) -> bool {
        matches!(self, Self::Set(_))
    }

    pub const fn is_clear(&self) -> bool {
        matches!(self, Self::Clear)
    }

    pub fn as_option(&self) -> Option<&T> {
        match self {
            Self::Set(v) => Some(v),
            _ => None,
        }
    }
}

/// Modifier patch adding and removing bitflags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ModifierPatch {
    pub add: Modifier,
    pub remove: Modifier,
}

impl ModifierPatch {
    pub const fn empty() -> Self {
        Self {
            add: Modifier::empty(),
            remove: Modifier::empty(),
        }
    }

    pub const fn add(modifier: Modifier) -> Self {
        Self {
            add: modifier,
            remove: Modifier::empty(),
        }
    }

    pub const fn remove(modifier: Modifier) -> Self {
        Self {
            add: Modifier::empty(),
            remove: modifier,
        }
    }

    pub fn apply(&self, mut modifier: Modifier) -> Modifier {
        modifier.insert(self.add);
        modifier.remove(self.remove);
        modifier
    }

    pub fn apply_to_style(&self, mut style: Style) -> Style {
        style.add_modifier = self.apply(style.add_modifier);
        style
    }
}

/// Style patch specifying foreground, background, and modifier changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StylePatch {
    pub foreground: PatchSlot<Role>,
    pub background: PatchSlot<Role>,
    pub modifiers: ModifierPatch,
}

impl StylePatch {
    pub const fn empty() -> Self {
        Self {
            foreground: PatchSlot::Inherit,
            background: PatchSlot::Inherit,
            modifiers: ModifierPatch::empty(),
        }
    }

    pub const fn fg(role: Role) -> Self {
        Self {
            foreground: PatchSlot::Set(role),
            background: PatchSlot::Inherit,
            modifiers: ModifierPatch::empty(),
        }
    }

    pub const fn bg(role: Role) -> Self {
        Self {
            foreground: PatchSlot::Inherit,
            background: PatchSlot::Set(role),
            modifiers: ModifierPatch::empty(),
        }
    }

    pub const fn fg_bg(fg: Role, bg: Role) -> Self {
        Self {
            foreground: PatchSlot::Set(fg),
            background: PatchSlot::Set(bg),
            modifiers: ModifierPatch::empty(),
        }
    }
}

/// Raw palette resolution structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorTokens {
    pub canvas: Color,
    pub surface: Color,
    pub surface_elevated: Color,
    pub surface_overlay: Color,
    pub field: Color,
    pub field_hover: Color,
    pub popover: Color,
    pub highlight: Color,
    pub highlight_danger: Color,
    pub error_soft: Color,
    pub border_subtle: Color,
    pub border_strong: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_faint: Color,
    pub text_ghost: Color,
    pub text_on_accent: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_pressed: Color,
    pub accent_bg: Color,
    pub accent_bg_subtle: Color,
    pub focus: Color,
    pub disabled: Color,
    pub danger: Color,
    pub danger_bg: Color,
    pub warning: Color,
    pub success: Color,
    pub info: Color,
}

pub const fn rgb(hex: u32) -> Color {
    Color::Rgb(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

impl ColorTokens {
    /// Exact baseline tokens matching `crate::theme::Theme::junie()`.
    pub const fn termrock() -> Self {
        Self {
            canvas: rgb(0x000000),
            surface: rgb(0x111111),
            surface_elevated: rgb(0x18181b),
            surface_overlay: rgb(0x27272a),
            field: rgb(0x1e1e22),
            field_hover: rgb(0x232328),
            popover: rgb(0x3f3f46),
            highlight: rgb(0x2f5aa8),
            highlight_danger: rgb(0x7a2a2a),
            error_soft: rgb(0xd98a8a),
            border_subtle: rgb(0x262626),
            border_strong: rgb(0x4d4d4d),
            text_primary: rgb(0xffffff),
            text_secondary: rgb(0xb3b3b3),
            text_muted: rgb(0x808080),
            text_faint: rgb(0x4d4d4d),
            text_ghost: rgb(0x262626),
            text_on_accent: rgb(0x19191c),
            accent: rgb(0x48e054),
            accent_hover: rgb(0x3ab343),
            accent_pressed: rgb(0x2b8632),
            accent_bg: rgb(0x0f2e13),
            accent_bg_subtle: rgb(0x0a1c0c),
            focus: rgb(0x48e054),
            disabled: rgb(0x4d4d4d),
            danger: rgb(0xe44545),
            danger_bg: rgb(0x2e0f0f),
            warning: rgb(0xf59e09),
            success: rgb(0x48e054),
            info: rgb(0x8787ff),
        }
    }

    /// Paper light-theme sentinel palette for testing hardcoded color independence.
    pub const fn paper() -> Self {
        Self {
            canvas: rgb(0xffffff),
            surface: rgb(0xf4f4f5),
            surface_elevated: rgb(0xe4e4e7),
            surface_overlay: rgb(0xd4d4d8),
            field: rgb(0xf4f4f5),
            field_hover: rgb(0xe4e4e7),
            popover: rgb(0xffffff),
            highlight: rgb(0xbfdbfe),
            highlight_danger: rgb(0xfecaca),
            error_soft: rgb(0xf87171),
            border_subtle: rgb(0xe4e4e7),
            border_strong: rgb(0xa1a1aa),
            text_primary: rgb(0x09090b),
            text_secondary: rgb(0x52525b),
            text_muted: rgb(0x71717a),
            text_faint: rgb(0xa1a1aa),
            text_ghost: rgb(0xd4d4d8),
            text_on_accent: rgb(0xffffff),
            accent: rgb(0x16a34a),
            accent_hover: rgb(0x15803d),
            accent_pressed: rgb(0x166534),
            accent_bg: rgb(0xdcfce7),
            accent_bg_subtle: rgb(0xf0fdf4),
            focus: rgb(0x16a34a),
            disabled: rgb(0xa1a1aa),
            danger: rgb(0xdc2626),
            danger_bg: rgb(0xfee2e2),
            warning: rgb(0xd97706),
            success: rgb(0x16a34a),
            info: rgb(0x4f46e5),
        }
    }

    /// Downgrades all tokens to match the specified ColorLevel capability.
    pub fn downgrade_for_level(&mut self, level: ColorLevel) {
        if level == ColorLevel::TrueColor {
            return;
        }
        self.canvas = downgrade(self.canvas, level);
        self.surface = downgrade(self.surface, level);
        self.surface_elevated = downgrade(self.surface_elevated, level);
        self.surface_overlay = downgrade(self.surface_overlay, level);
        self.field = downgrade(self.field, level);
        self.field_hover = downgrade(self.field_hover, level);
        self.popover = downgrade(self.popover, level);
        self.highlight = downgrade(self.highlight, level);
        self.highlight_danger = downgrade(self.highlight_danger, level);
        self.error_soft = downgrade(self.error_soft, level);
        self.border_subtle = downgrade(self.border_subtle, level);
        self.border_strong = downgrade(self.border_strong, level);
        self.text_primary = downgrade(self.text_primary, level);
        self.text_secondary = downgrade(self.text_secondary, level);
        self.text_muted = downgrade(self.text_muted, level);
        self.text_faint = downgrade(self.text_faint, level);
        self.text_ghost = downgrade(self.text_ghost, level);
        self.text_on_accent = downgrade(self.text_on_accent, level);
        self.accent = downgrade(self.accent, level);
        self.accent_hover = downgrade(self.accent_hover, level);
        self.accent_pressed = downgrade(self.accent_pressed, level);
        self.accent_bg = downgrade(self.accent_bg, level);
        self.accent_bg_subtle = downgrade(self.accent_bg_subtle, level);
        self.focus = downgrade(self.focus, level);
        self.disabled = downgrade(self.disabled, level);
        self.danger = downgrade(self.danger, level);
        self.danger_bg = downgrade(self.danger_bg, level);
        self.warning = downgrade(self.warning, level);
        self.success = downgrade(self.success, level);
        self.info = downgrade(self.info, level);
    }
}

/// Semantic Theme containing capability level, resolved tokens, and scoped patches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub level: ColorLevel,
    pub tokens: ColorTokens,
    patches: BTreeMap<(Family, Variant, Part), StylePatch>,
}

impl Default for Theme {
    fn default() -> Self {
        Self::termrock()
    }
}

impl Theme {
    /// Reproduces exact baseline palette from `crate::theme::Theme::junie()`.
    pub fn termrock() -> Self {
        Self {
            level: ColorLevel::TrueColor,
            tokens: ColorTokens::termrock(),
            patches: BTreeMap::new(),
        }
    }

    /// Extension light-theme sentinel for verifying theme decoupling.
    pub fn paper() -> Self {
        Self {
            level: ColorLevel::TrueColor,
            tokens: ColorTokens::paper(),
            patches: BTreeMap::new(),
        }
    }

    /// Returns the theme adapted for the specified ColorLevel.
    pub fn for_level(level: ColorLevel) -> Self {
        let mut t = Self::termrock();
        t.level = level;
        t.tokens.downgrade_for_level(level);
        t
    }

    /// Configures the theme with a custom palette at the specified capability level.
    pub fn with_palette(mut self, level: ColorLevel, mut tokens: ColorTokens) -> Self {
        self.level = level;
        tokens.downgrade_for_level(level);
        self.tokens = tokens;
        self
    }

    /// Attaches a scoped style patch for a family, variant, and part.
    pub fn patch(
        mut self,
        family: Family,
        variant: Variant,
        part: Part,
        patch: StylePatch,
    ) -> Self {
        self.patches.insert((family, variant, part), patch);
        self
    }

    /// Look up an advertised part patch if registered.
    pub fn get_patch(&self, family: Family, variant: Variant, part: Part) -> Option<&StylePatch> {
        self.patches.get(&(family, variant, part))
    }

    /// Resolves a semantic role on a given surface to a concrete Ratatui color.
    pub fn resolve_role(&self, role: Role, surface: Surface) -> Color {
        match role {
            Role::Primary => self.tokens.text_primary,
            Role::Secondary => self.tokens.text_secondary,
            Role::Accent => self.tokens.accent,
            Role::Danger => self.tokens.danger,
            Role::Success => self.tokens.success,
            Role::Warning => self.tokens.warning,
            Role::Muted => self.tokens.text_muted,
            Role::Subtle => self.tokens.text_faint,
            Role::Border => self.tokens.border_subtle,
            Role::BorderFocused => self.tokens.border_strong,
            Role::BorderSelected => self.tokens.focus,
            Role::Selection => match self.level {
                ColorLevel::Mono => self.tokens.text_primary,
                _ => self.tokens.highlight,
            },
            Role::Text => self.tokens.text_primary,
            Role::TextMuted => self.tokens.text_muted,
            Role::TextDisabled => self.tokens.disabled,
            Role::Background => match surface {
                Surface::Canvas => self.tokens.canvas,
                Surface::Surface => self.tokens.surface,
                Surface::Elevated => self.tokens.surface_elevated,
                Surface::Overlay => self.tokens.surface_overlay,
                Surface::Popover => self.tokens.popover,
                Surface::Field => self.tokens.field,
                Surface::FieldHover => self.tokens.field_hover,
            },
            Role::BackdropDim => self.tokens.text_ghost,
        }
    }

    /// Resolves a semantic role on a given surface into a fully styled `ratatui::style::Style`.
    pub fn resolve_style(&self, role: Role, surface: Surface) -> Style {
        let bg_color = self.resolve_role(Role::Background, surface);
        match role {
            Role::Background => Style::new().bg(bg_color),
            Role::Selection => {
                let mut style = Style::new()
                    .fg(self.tokens.text_primary)
                    .bg(self.tokens.popover);
                if self.level == ColorLevel::Mono {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                style
            }
            Role::TextDisabled => {
                let mut style = Style::new().fg(self.tokens.disabled).bg(bg_color);
                if self.level == ColorLevel::Mono {
                    style = style.add_modifier(Modifier::DIM);
                }
                style
            }
            Role::BorderFocused => Style::new()
                .fg(self.resolve_role(Role::BorderFocused, surface))
                .bg(bg_color)
                .add_modifier(Modifier::BOLD),
            _ => Style::new()
                .fg(self.resolve_role(role, surface))
                .bg(bg_color),
        }
    }

    /// Converts Tone into semantic Role.
    pub fn tone_role(&self, tone: Tone) -> Role {
        match tone {
            Tone::Normal => Role::Text,
            Tone::Primary => Role::Primary,
            Tone::Accent => Role::Accent,
            Tone::Success => Role::Success,
            Tone::Warning => Role::Warning,
            Tone::Danger => Role::Danger,
            Tone::Muted => Role::TextMuted,
        }
    }

    /// Resolves Tone directly into color.
    pub fn tone_color(&self, tone: Tone) -> Color {
        self.resolve_role(self.tone_role(tone), Surface::Canvas)
    }

    /// Applies a style patch onto a base style in the context of a surface.
    pub fn apply_patch(&self, base: Style, patch: &StylePatch, surface: Surface) -> Style {
        let mut st = base;
        match patch.foreground {
            PatchSlot::Set(role) => {
                st = st.fg(self.resolve_role(role, surface));
            }
            PatchSlot::Clear => {
                st.fg = None;
            }
            PatchSlot::Inherit => {}
        }
        match patch.background {
            PatchSlot::Set(role) => {
                st = st.bg(self.resolve_role(role, surface));
            }
            PatchSlot::Clear => {
                st.bg = None;
            }
            PatchSlot::Inherit => {}
        }
        patch.modifiers.apply_to_style(st)
    }
}

// Capability color conversion helpers

fn downgrade(c: Color, level: ColorLevel) -> Color {
    let Color::Rgb(r, g, b) = c else { return c };
    match level {
        ColorLevel::TrueColor => c,
        ColorLevel::Ansi256 => Color::Indexed(nearest_256(r, g, b)),
        ColorLevel::Ansi16 => nearest_16(r, g, b),
        ColorLevel::Mono => match (r as u32 + g as u32 + b as u32) / 3 {
            0..=40 => Color::Black,
            41..=110 => Color::DarkGray,
            111..=190 => Color::Gray,
            _ => Color::White,
        },
    }
}

fn nearest_256(r: u8, g: u8, b: u8) -> u8 {
    let step = |v: u8| -> u8 { ((v as u32 * 5 + 127) / 255) as u8 };
    let cube = 16 + 36 * step(r) + 6 * step(g) + step(b);
    let cube_val = |i: u8| -> i32 { if i == 0 { 0 } else { 55 + i as i32 * 40 } };
    let (cr, cg, cb) = (cube_val(step(r)), cube_val(step(g)), cube_val(step(b)));
    let cube_err = (cr - r as i32).pow(2) + (cg - g as i32).pow(2) + (cb - b as i32).pow(2);
    let avg = (r as i32 + g as i32 + b as i32) / 3;
    let gi = ((avg - 8).max(0) / 10).min(23);
    let gv = 8 + gi * 10;
    let gray_err = (gv - r as i32).pow(2) + (gv - g as i32).pow(2) + (gv - b as i32).pow(2);
    if gray_err < cube_err {
        232 + gi as u8
    } else {
        cube
    }
}

fn nearest_16(r: u8, g: u8, b: u8) -> Color {
    let lum = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;
    let max = r.max(g).max(b) as u32;
    let min = r.min(g).min(b) as u32;
    if max - min < 40 {
        return match lum {
            0..=30 => Color::Black,
            31..=110 => Color::DarkGray,
            111..=200 => Color::Gray,
            _ => Color::White,
        };
    }
    let bright = max > 180;
    match (r >= g && r >= b, g >= r && g >= b, b >= r && b >= g) {
        (true, _, _) if g > 120 && b < 80 => Color::Yellow,
        (true, _, _) => {
            if bright {
                Color::LightRed
            } else {
                Color::Red
            }
        }
        (_, true, _) => {
            if bright {
                Color::LightGreen
            } else {
                Color::Green
            }
        }
        _ => {
            if bright {
                Color::LightBlue
            } else {
                Color::Blue
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_termrock_matches_baseline_junie() {
        let baseline = crate::theme::Theme::junie();
        let termrock = Theme::termrock();

        assert_eq!(termrock.tokens.canvas, baseline.canvas);
        assert_eq!(termrock.tokens.surface, baseline.surface);
        assert_eq!(termrock.tokens.surface_elevated, baseline.surface_elevated);
        assert_eq!(termrock.tokens.surface_overlay, baseline.surface_overlay);
        assert_eq!(termrock.tokens.field, baseline.field);
        assert_eq!(termrock.tokens.field_hover, baseline.field_hover);
        assert_eq!(termrock.tokens.popover, baseline.popover);
        assert_eq!(termrock.tokens.highlight, baseline.highlight);
        assert_eq!(termrock.tokens.highlight_danger, baseline.highlight_danger);
        assert_eq!(termrock.tokens.error_soft, baseline.error_soft);
        assert_eq!(termrock.tokens.border_subtle, baseline.border_subtle);
        assert_eq!(termrock.tokens.border_strong, baseline.border_strong);
        assert_eq!(termrock.tokens.text_primary, baseline.text_primary);
        assert_eq!(termrock.tokens.text_secondary, baseline.text_secondary);
        assert_eq!(termrock.tokens.text_muted, baseline.text_muted);
        assert_eq!(termrock.tokens.text_faint, baseline.text_faint);
        assert_eq!(termrock.tokens.text_ghost, baseline.text_ghost);
        assert_eq!(termrock.tokens.text_on_accent, baseline.text_on_accent);
        assert_eq!(termrock.tokens.accent, baseline.accent);
        assert_eq!(termrock.tokens.accent_hover, baseline.accent_hover);
        assert_eq!(termrock.tokens.accent_pressed, baseline.accent_pressed);
        assert_eq!(termrock.tokens.accent_bg, baseline.accent_bg);
        assert_eq!(termrock.tokens.accent_bg_subtle, baseline.accent_bg_subtle);
        assert_eq!(termrock.tokens.focus, baseline.focus);
        assert_eq!(termrock.tokens.disabled, baseline.disabled);
        assert_eq!(termrock.tokens.danger, baseline.error);
        assert_eq!(termrock.tokens.danger_bg, baseline.error_bg);
        assert_eq!(termrock.tokens.warning, baseline.warning);
        assert_eq!(termrock.tokens.success, baseline.success);
        assert_eq!(termrock.tokens.info, baseline.info);
    }

    #[test]
    fn theme_paper_sentinel_differs_from_termrock() {
        let termrock = Theme::termrock();
        let paper = Theme::paper();

        assert_ne!(termrock.tokens.canvas, paper.tokens.canvas);
        assert_ne!(termrock.tokens.text_primary, paper.tokens.text_primary);
        assert_ne!(termrock.tokens.surface, paper.tokens.surface);
    }

    #[test]
    fn theme_patch_and_resolution() {
        let patch = StylePatch {
            foreground: PatchSlot::Set(Role::Accent),
            background: PatchSlot::Set(Role::Background),
            modifiers: ModifierPatch::add(Modifier::BOLD),
        };

        let container_part = Part::new("container");
        let theme =
            Theme::termrock().patch(Family::Button, Variant::Primary, container_part, patch);

        let retrieved = theme.get_patch(Family::Button, Variant::Primary, container_part);
        assert_eq!(retrieved, Some(&patch));

        let resolved = theme.apply_patch(Style::new(), &patch, Surface::Elevated);
        assert_eq!(resolved.fg, Some(theme.tokens.accent));
        assert_eq!(resolved.bg, Some(theme.tokens.surface_elevated));
        assert!(resolved.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn theme_color_level_downgrades() {
        let t256 = Theme::for_level(ColorLevel::Ansi256);
        assert!(matches!(t256.tokens.accent, Color::Indexed(_)));

        let t16 = Theme::for_level(ColorLevel::Ansi16);
        assert_eq!(t16.tokens.accent, Color::LightGreen);
        assert_eq!(t16.tokens.danger, Color::LightRed);
        assert_eq!(t16.tokens.canvas, Color::Black);

        let tmono = Theme::for_level(ColorLevel::Mono);
        assert_eq!(tmono.tokens.canvas, Color::Black);
        assert_eq!(tmono.tokens.text_primary, Color::White);
    }
}
