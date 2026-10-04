//! Semantic update responses, flow control, bindings, and update causes.
//!
//! Provides the canonical [`Response<A>`] type, independent [`Flow`] and [`Invalidate`]
//! axes, key chord parsing and scoping, and input event normalisation.

use std::fmt;
use std::str::FromStr;

use ratatui::crossterm::event::{KeyCode, KeyModifiers};

pub use crate::core::event::{Input, Key, Mouse, MouseKind};
use crate::termrock::identity::{ActionKey, Id};
use crate::termrock::layout::Size;

/// Routing flow policy for an update response.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Flow {
    /// Allow subsequent handlers to observe the event.
    #[default]
    Bubble,
    /// Stop further routing of the event.
    Consumed,
}

impl Flow {
    pub const fn is_consumed(&self) -> bool {
        matches!(self, Self::Consumed)
    }
}

/// Redraw and geometry publication invalidation policy.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Invalidate {
    /// No redraw or layout change needed.
    #[default]
    None,
    /// Requires repaint without changing layout geometry.
    Paint,
    /// Requires re-layout and repaint.
    Layout,
}

impl Invalidate {
    pub const fn requires_paint(&self) -> bool {
        matches!(self, Self::Paint | Self::Layout)
    }

    pub const fn requires_layout(&self) -> bool {
        matches!(self, Self::Layout)
    }

    pub const fn or(&self, other: Invalidate) -> Invalidate {
        match (*self, other) {
            (Invalidate::Layout, _) | (_, Invalidate::Layout) => Invalidate::Layout,
            (Invalidate::Paint, _) | (_, Invalidate::Paint) => Invalidate::Paint,
            _ => Invalidate::None,
        }
    }
}

/// Derived visual state flags for styling and rendering.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct VisualState {
    pub focused: bool,
    pub focus_visible: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub activation_feedback: bool,
    pub selected: bool,
    pub checked: bool,
    pub disabled: bool,
    pub editing: bool,
    pub invalid: bool,
    pub busy: bool,
    pub loading: bool,
}

impl VisualState {
    pub const fn empty() -> Self {
        Self {
            focused: false,
            focus_visible: false,
            hovered: false,
            pressed: false,
            activation_feedback: false,
            selected: false,
            checked: false,
            disabled: false,
            editing: false,
            invalid: false,
            busy: false,
            loading: false,
        }
    }

    pub const fn focused(mut self, v: bool) -> Self {
        self.focused = v;
        self
    }
    pub const fn focus_visible(mut self, v: bool) -> Self {
        self.focus_visible = v;
        self
    }
    pub const fn hovered(mut self, v: bool) -> Self {
        self.hovered = v;
        self
    }
    pub const fn pressed(mut self, v: bool) -> Self {
        self.pressed = v;
        self
    }
    pub const fn activation_feedback(mut self, v: bool) -> Self {
        self.activation_feedback = v;
        self
    }
    pub const fn selected(mut self, v: bool) -> Self {
        self.selected = v;
        self
    }
    pub const fn checked(mut self, v: bool) -> Self {
        self.checked = v;
        self
    }
    pub const fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
    pub const fn editing(mut self, v: bool) -> Self {
        self.editing = v;
        self
    }
    pub const fn invalid(mut self, v: bool) -> Self {
        self.invalid = v;
        self
    }
    pub const fn busy(mut self, v: bool) -> Self {
        self.busy = v;
        self
    }
    pub const fn loading(mut self, v: bool) -> Self {
        self.loading = v;
        self
    }
}

/// The origin of an activation or value change.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ActivationOrigin {
    Keyboard,
    Pointer,
    Programmatic,
}

/// Semantic activation marker carrying origin.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Activated {
    pub origin: ActivationOrigin,
}

impl Activated {
    pub const fn new(origin: ActivationOrigin) -> Self {
        Self { origin }
    }
}

/// Controlled next-value request carrying value and origin.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ValueChanged<T> {
    pub value: T,
    pub origin: ActivationOrigin,
}

impl<T> ValueChanged<T> {
    pub const fn new(value: T, origin: ActivationOrigin) -> Self {
        Self { value, origin }
    }
}

/// Canonical typed update response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response<A> {
    pub id: Id,
    pub flow: Flow,
    pub invalidate: Invalidate,
    pub state: VisualState,
    pub action: Option<A>,
}

impl<A> Response<A> {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            flow: Flow::Bubble,
            invalidate: Invalidate::None,
            state: VisualState::empty(),
            action: None,
        }
    }

    pub fn action(id: Id, action: A) -> Self {
        Self {
            id,
            flow: Flow::Bubble,
            invalidate: Invalidate::None,
            state: VisualState::empty(),
            action: Some(action),
        }
    }

    pub fn consumed(id: Id) -> Self {
        Self {
            id,
            flow: Flow::Consumed,
            invalidate: Invalidate::None,
            state: VisualState::empty(),
            action: None,
        }
    }

    pub fn bubble(id: Id) -> Self {
        Self {
            id,
            flow: Flow::Bubble,
            invalidate: Invalidate::None,
            state: VisualState::empty(),
            action: None,
        }
    }

    pub fn with_action(mut self, action: A) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_flow(mut self, flow: Flow) -> Self {
        self.flow = flow;
        self
    }

    pub fn with_invalidate(mut self, invalidate: Invalidate) -> Self {
        self.invalidate = invalidate;
        self
    }

    pub fn with_state(mut self, state: VisualState) -> Self {
        self.state = state;
        self
    }

    pub fn map_action<B>(self, f: impl FnOnce(A) -> B) -> Response<B> {
        Response {
            id: self.id,
            flow: self.flow,
            invalidate: self.invalidate,
            state: self.state,
            action: self.action.map(f),
        }
    }
}

/// A parsed keyboard chord (modifiers + key code).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub code: KeyCode,
    pub mods: KeyModifiers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChordParseError {
    Empty,
    UnknownKey(String),
    UnknownModifier(String),
}

impl fmt::Display for ChordParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty chord string"),
            Self::UnknownKey(k) => write!(f, "unknown key: {k}"),
            Self::UnknownModifier(m) => write!(f, "unknown modifier: {m}"),
        }
    }
}

impl std::error::Error for ChordParseError {}

impl Chord {
    pub const fn new(code: KeyCode, mods: KeyModifiers) -> Self {
        Self { code, mods }
    }

    pub fn parse(s: &str) -> Result<Self, ChordParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ChordParseError::Empty);
        }

        let (mod_part, key_str) = if s.len() > 1 && s.ends_with("++") {
            (&s[..s.len() - 2], "+")
        } else if s == "+" {
            ("", "+")
        } else {
            match s.rsplit_once('+') {
                Some((m, k)) => (m, k),
                None => ("", s),
            }
        };

        let mut mods = KeyModifiers::NONE;
        if !mod_part.is_empty() {
            for token in mod_part.split('+') {
                let m = token.trim().to_lowercase();
                match m.as_str() {
                    "ctrl" | "control" => mods |= KeyModifiers::CONTROL,
                    "alt" | "opt" | "option" => mods |= KeyModifiers::ALT,
                    "shift" => mods |= KeyModifiers::SHIFT,
                    "super" | "cmd" | "command" | "win" => mods |= KeyModifiers::SUPER,
                    _ => return Err(ChordParseError::UnknownModifier(token.to_string())),
                }
            }
        }

        let k_lower = key_str.trim().to_lowercase();
        let parsed_code = match k_lower.as_str() {
            "enter" | "return" => KeyCode::Enter,
            "esc" | "escape" => KeyCode::Esc,
            "backspace" => KeyCode::Backspace,
            "tab" => KeyCode::Tab,
            "backtab" => KeyCode::BackTab,
            "delete" | "del" => KeyCode::Delete,
            "insert" | "ins" => KeyCode::Insert,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" | "page_up" | "pgup" => KeyCode::PageUp,
            "pagedown" | "page_down" | "pgdn" => KeyCode::PageDown,
            "space" => KeyCode::Char(' '),
            s if s.starts_with('f')
                && s.len() > 1
                && s[1..].chars().all(|c| c.is_ascii_digit()) =>
            {
                let num: u8 = s[1..]
                    .parse()
                    .map_err(|_| ChordParseError::UnknownKey(key_str.to_string()))?;
                if (1..=24).contains(&num) {
                    KeyCode::F(num)
                } else {
                    return Err(ChordParseError::UnknownKey(key_str.to_string()));
                }
            }
            _ => {
                let mut chars = key_str.trim().chars();
                if let Some(c) = chars.next() {
                    if chars.next().is_none() {
                        KeyCode::Char(c)
                    } else {
                        return Err(ChordParseError::UnknownKey(key_str.to_string()));
                    }
                } else {
                    return Err(ChordParseError::Empty);
                }
            }
        };

        Ok(Chord {
            code: parsed_code,
            mods,
        })
    }

    pub fn matches(&self, key: &Key) -> bool {
        self.code == key.code && self.mods == key.mods
    }
}

impl FromStr for Chord {
    type Err = ChordParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Chord::parse(s)
    }
}

impl From<Key> for Chord {
    fn from(k: Key) -> Self {
        Self {
            code: k.code,
            mods: k.mods,
        }
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mods.contains(KeyModifiers::CONTROL) {
            write!(f, "ctrl+")?;
        }
        if self.mods.contains(KeyModifiers::ALT) {
            write!(f, "alt+")?;
        }
        if self.mods.contains(KeyModifiers::SHIFT) {
            write!(f, "shift+")?;
        }
        if self.mods.contains(KeyModifiers::SUPER) {
            write!(f, "super+")?;
        }
        match self.code {
            KeyCode::Enter => write!(f, "enter"),
            KeyCode::Esc => write!(f, "esc"),
            KeyCode::Backspace => write!(f, "backspace"),
            KeyCode::Tab => write!(f, "tab"),
            KeyCode::BackTab => write!(f, "backtab"),
            KeyCode::Delete => write!(f, "delete"),
            KeyCode::Insert => write!(f, "insert"),
            KeyCode::Left => write!(f, "left"),
            KeyCode::Right => write!(f, "right"),
            KeyCode::Up => write!(f, "up"),
            KeyCode::Down => write!(f, "down"),
            KeyCode::Home => write!(f, "home"),
            KeyCode::End => write!(f, "end"),
            KeyCode::PageUp => write!(f, "pageup"),
            KeyCode::PageDown => write!(f, "pagedown"),
            KeyCode::F(n) => write!(f, "f{n}"),
            KeyCode::Char(' ') => write!(f, "space"),
            KeyCode::Char(c) => write!(f, "{c}"),
            _ => write!(f, "{:?}", self.code),
        }
    }
}

/// Identifier for an input binding scope.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ScopeId(&'static str);

impl ScopeId {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

impl fmt::Display for ScopeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A command binding specification.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Binding<'a> {
    pub action: ActionKey,
    pub chord: Option<Chord>,
    pub label: &'a str,
    pub enabled: bool,
    pub visible: bool,
    pub priority: u16,
}

impl<'a> Binding<'a> {
    pub const fn new(action: ActionKey, label: &'a str) -> Self {
        Self {
            action,
            chord: None,
            label,
            enabled: true,
            visible: true,
            priority: 0,
        }
    }

    pub const fn chord(mut self, chord: Chord) -> Self {
        self.chord = Some(chord);
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub const fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub const fn priority(mut self, priority: u16) -> Self {
        self.priority = priority;
        self
    }
}

/// Resolved binding in a resolved scope.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ResolvedBinding {
    pub action: ActionKey,
    pub chord: Option<Chord>,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    pub priority: u16,
}

/// Resolved effective bindings for an input scope.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BindingView {
    pub scope: ScopeId,
    pub bindings: Vec<ResolvedBinding>,
}

impl BindingView {
    pub fn resolve(scope: ScopeId, bindings: &[Binding<'_>]) -> Self {
        let mut resolved: Vec<ResolvedBinding> = bindings
            .iter()
            .map(|b| ResolvedBinding {
                action: b.action,
                chord: b.chord,
                label: b.label.to_string(),
                enabled: b.enabled,
                visible: b.visible,
                priority: b.priority,
            })
            .collect();

        resolved.sort_by_key(|a| std::cmp::Reverse(a.priority));

        Self {
            scope,
            bindings: resolved,
        }
    }

    pub fn find_by_chord(&self, chord: &Chord) -> Option<&ResolvedBinding> {
        self.bindings
            .iter()
            .find(|b| b.enabled && b.chord.as_ref() == Some(chord))
    }

    pub fn find_by_action(&self, action: ActionKey) -> Option<&ResolvedBinding> {
        self.bindings.iter().find(|b| b.action == action)
    }

    pub fn scope(&self) -> ScopeId {
        self.scope
    }

    pub fn bindings(&self) -> &[ResolvedBinding] {
        &self.bindings
    }
}

/// Monotonic timestamp in milliseconds.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Moment(pub u64);

impl Moment {
    pub const fn from_millis(ms: u64) -> Self {
        Self(ms)
    }

    pub const fn as_millis(&self) -> u64 {
        self.0
    }

    pub const fn saturating_add_millis(&self, ms: u64) -> Self {
        Self(self.0.saturating_add(ms))
    }

    pub const fn saturating_sub_millis(&self, ms: u64) -> Self {
        Self(self.0.saturating_sub(ms))
    }
}

impl fmt::Debug for Moment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Moment({}ms)", self.0)
    }
}

impl fmt::Display for Moment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ms", self.0)
    }
}

impl std::ops::Add<std::time::Duration> for Moment {
    type Output = Self;
    fn add(self, rhs: std::time::Duration) -> Self {
        Self(self.0.saturating_add(rhs.as_millis() as u64))
    }
}

impl std::ops::Sub<std::time::Duration> for Moment {
    type Output = Self;
    fn sub(self, rhs: std::time::Duration) -> Self {
        Self(self.0.saturating_sub(rhs.as_millis() as u64))
    }
}

impl std::ops::Sub<Moment> for Moment {
    type Output = std::time::Duration;
    fn sub(self, rhs: Moment) -> std::time::Duration {
        std::time::Duration::from_millis(self.0.saturating_sub(rhs.0))
    }
}

/// The triggering cause of a component or system update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCause {
    Boot(Moment),
    Input(Input, Moment),
    Tick(Moment),
    ModelChanged(Moment),
    Resize(Size, Moment),
}

impl UpdateCause {
    pub fn moment(&self) -> Moment {
        match self {
            Self::Boot(m) => *m,
            Self::Input(_, m) => *m,
            Self::Tick(m) => *m,
            Self::ModelChanged(m) => *m,
            Self::Resize(_, m) => *m,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_and_invalidation() {
        assert!(!Flow::Bubble.is_consumed());
        assert!(Flow::Consumed.is_consumed());

        assert!(!Invalidate::None.requires_paint());
        assert!(Invalidate::Paint.requires_paint());
        assert!(Invalidate::Layout.requires_paint());
        assert!(!Invalidate::Paint.requires_layout());
        assert!(Invalidate::Layout.requires_layout());

        assert_eq!(Invalidate::None.or(Invalidate::Paint), Invalidate::Paint);
        assert_eq!(Invalidate::Paint.or(Invalidate::Layout), Invalidate::Layout);
        assert_eq!(Invalidate::Layout.or(Invalidate::Paint), Invalidate::Layout);
    }

    #[test]
    fn chord_parsing_and_formatting() {
        let c1 = Chord::parse("ctrl+c").unwrap();
        assert_eq!(c1.code, KeyCode::Char('c'));
        assert_eq!(c1.mods, KeyModifiers::CONTROL);
        assert_eq!(c1.to_string(), "ctrl+c");

        let c2 = Chord::parse("alt+shift+x").unwrap();
        assert_eq!(c2.code, KeyCode::Char('x'));
        assert!(c2.mods.contains(KeyModifiers::ALT));
        assert!(c2.mods.contains(KeyModifiers::SHIFT));

        let c3 = Chord::parse("enter").unwrap();
        assert_eq!(c3.code, KeyCode::Enter);
        assert_eq!(c3.mods, KeyModifiers::NONE);
        assert_eq!(c3.to_string(), "enter");

        let c4 = Chord::parse("f12").unwrap();
        assert_eq!(c4.code, KeyCode::F(12));

        let c5 = Chord::parse("ctrl++").unwrap();
        assert_eq!(c5.code, KeyCode::Char('+'));
        assert_eq!(c5.mods, KeyModifiers::CONTROL);

        assert!(Chord::parse("").is_err());
        assert!(Chord::parse("fake+c").is_err());
    }

    #[test]
    fn bindings_and_view_resolution() {
        let scope = ScopeId::new("editor");
        let act1 = ActionKey::new("save");
        let act2 = ActionKey::new("close");
        let chord1 = Chord::parse("ctrl+s").unwrap();
        let chord2 = Chord::parse("ctrl+w").unwrap();

        let b1 = Binding::new(act1, "Save").chord(chord1).priority(10);
        let b2 = Binding::new(act2, "Close").chord(chord2).priority(20);

        let view = BindingView::resolve(scope, &[b1, b2]);
        assert_eq!(view.scope().as_str(), "editor");
        assert_eq!(view.bindings()[0].action, act2); // higher priority first

        let found = view.find_by_chord(&chord1).unwrap();
        assert_eq!(found.action, act1);
        assert_eq!(found.label, "Save");
    }

    #[test]
    fn response_construction_and_mapping() {
        let id = Id::new("button");
        let resp = Response::action(id.clone(), 42)
            .with_flow(Flow::Consumed)
            .with_invalidate(Invalidate::Paint)
            .with_state(VisualState::empty().focused(true));

        assert_eq!(resp.id, id);
        assert!(resp.flow.is_consumed());
        assert!(resp.invalidate.requires_paint());
        assert!(resp.state.focused);
        assert_eq!(resp.action, Some(42));

        let mapped = resp.map_action(|n| format!("number:{n}"));
        assert_eq!(mapped.action, Some("number:42".to_string()));
    }

    #[test]
    fn update_cause_moment() {
        let m = Moment::from_millis(1000);
        let boot = UpdateCause::Boot(m);
        assert_eq!(boot.moment(), m);

        let tick = UpdateCause::Tick(m.saturating_add_millis(50));
        assert_eq!(tick.moment().as_millis(), 1050);
    }
}
