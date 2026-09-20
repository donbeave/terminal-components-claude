//! Flattened action programs. Pointer labels never resolve through buffer search.

use std::time::Duration;

use junie_tui::{Key, KeyCode, KeyModifiers};

use crate::color::TerminalSize;
use crate::error::AdapterError;

/// One checkpoint action from a scenario row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Initial presented frame.
    FreshDraw,
    /// Production `Input::Tick` at unchanged Instant/Moment.
    Tick,
    /// `n` production ticks.
    Ticks(usize),
    /// Set logical Instant elapsed time (and matching Moment).
    Time(Duration),
    /// Set Instant elapsed then deliver one Tick.
    TimeThenTick(Duration),
    /// Terminal resize.
    Resize(TerminalSize),
    /// Keyboard event from the scenario transcript.
    Key(Key),
    /// Frozen pointer cell. Coordinates must be supplied by oracle capture.
    Pointer {
        /// hover / down / up / click / wheel / drag.
        kind: PointerKind,
        /// Frozen column.
        x: u16,
        /// Frozen row.
        y: u16,
    },
    /// Labeled pointer/focus action without frozen geometry — fail closed.
    Unfrozen {
        /// Raw token.
        raw: String,
        /// Label inside parentheses, if any.
        label: Option<String>,
    },
}

/// Pointer kind for frozen-coordinate replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    /// Mouse move.
    Hover,
    /// Primary down.
    Down,
    /// Primary up.
    Up,
    /// Down then up at the same cell.
    Click,
    /// Vertical wheel notches (negative is up).
    Wheel(i16),
    /// Horizontal wheel notches.
    WheelH(i16),
}

/// Ordered checkpoint program for one expanded case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionProgram {
    /// Original `action_checkpoints` text.
    pub source: String,
    /// Flattened actions.
    pub steps: Vec<Action>,
}

impl ActionProgram {
    /// Parse the TSV `action_checkpoints` column.
    pub fn parse(source: &str) -> Self {
        let steps = source
            .split(';')
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(parse_token)
            .collect();
        Self {
            source: source.to_owned(),
            steps,
        }
    }
}

fn parse_token(token: &str) -> Action {
    let folded = token.trim();
    let lower = folded.to_ascii_lowercase();
    if lower == "fresh draw" || lower == "fresh draw at tick0" {
        return Action::FreshDraw;
    }
    if lower == "tick" {
        return Action::Tick;
    }
    if let Some(action) = parse_ticks(folded) {
        return action;
    }
    if let Some(action) = parse_time(folded) {
        return action;
    }
    if let Some(rest) = folded.strip_prefix("resize")
        && let Some(size) = TerminalSize::parse(rest)
    {
        return Action::Resize(size);
    }
    if let Some(key) = parse_key(folded) {
        return Action::Key(key);
    }
    if looks_labeled(folded) {
        return Action::Unfrozen {
            raw: folded.to_owned(),
            label: labeled_name(folded),
        };
    }
    Action::Unfrozen {
        raw: folded.to_owned(),
        label: None,
    }
}

fn parse_ticks(token: &str) -> Option<Action> {
    let rest = token.strip_prefix("ticks")?;
    if rest.is_empty() {
        return Some(Action::Tick);
    }
    let count = rest
        .trim_start_matches('(')
        .trim_end_matches(')')
        .parse()
        .ok()?;
    Some(Action::Ticks(count))
}

fn parse_time(token: &str) -> Option<Action> {
    let rest = token.strip_prefix("time")?;
    let rest = rest.trim_start_matches('(').trim_end_matches(')');
    let (amount, tick) = if let Some(amount) = rest.strip_suffix("+Tick") {
        (amount, true)
    } else if let Some(amount) = rest.strip_suffix("+tick") {
        (amount, true)
    } else {
        (rest, false)
    };
    let duration = parse_duration(amount)?;
    if tick {
        Some(Action::TimeThenTick(duration))
    } else {
        Some(Action::Time(duration))
    }
}

fn parse_duration(token: &str) -> Option<Duration> {
    if let Some(ms) = token.strip_suffix("ms") {
        let (base, nanos) = split_plus_ns(ms)?;
        return Some(Duration::from_millis(base).saturating_add(Duration::from_nanos(nanos)));
    }
    if let Some(ns) = token.strip_suffix("ns") {
        let n: u64 = ns.parse().ok()?;
        return Some(Duration::from_nanos(n));
    }
    None
}

fn split_plus_ns(token: &str) -> Option<(u64, u64)> {
    if let Some((ms, ns)) = token.split_once('+') {
        let nanos = ns.strip_suffix("ns").unwrap_or(ns);
        Some((ms.parse().ok()?, nanos.parse().ok()?))
    } else {
        Some((token.parse().ok()?, 0))
    }
}

fn parse_key(token: &str) -> Option<Key> {
    let (mods, name) = parse_mods(token)?;
    let code = match name {
        "Tab" => KeyCode::Tab,
        "BackTab" => KeyCode::BackTab,
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Space" => KeyCode::Char(' '),
        "Down" => KeyCode::Down,
        "Up" => KeyCode::Up,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageDown" => KeyCode::PageDown,
        "PageUp" => KeyCode::PageUp,
        "Delete" => KeyCode::Delete,
        "F2" => KeyCode::F(2),
        "F10" => KeyCode::F(10),
        other if other.chars().count() == 1 => KeyCode::Char(other.chars().next()?),
        _ => return None,
    };
    Some(Key { code, mods })
}

fn parse_mods(token: &str) -> Option<(KeyModifiers, &str)> {
    let mut mods = KeyModifiers::NONE;
    let mut rest = token;
    loop {
        if let Some(tail) = rest.strip_prefix("Ctrl+") {
            mods |= KeyModifiers::CONTROL;
            rest = tail;
            continue;
        }
        if let Some(tail) = rest.strip_prefix("Alt+") {
            mods |= KeyModifiers::ALT;
            rest = tail;
            continue;
        }
        if let Some(tail) = rest.strip_prefix("Shift+") {
            mods |= KeyModifiers::SHIFT;
            rest = tail;
            continue;
        }
        break;
    }
    if rest.is_empty() {
        None
    } else {
        Some((mods, rest))
    }
}

fn looks_labeled(token: &str) -> bool {
    token.contains('(')
        && (token.starts_with("click")
            || token.starts_with("hover")
            || token.starts_with("down")
            || token.starts_with("Down")
            || token.starts_with("up")
            || token.starts_with("Up")
            || token.starts_with("focus")
            || token.starts_with("wheel")
            || token.starts_with("drag")
            || token.starts_with("paste")
            || token.starts_with("type")
            || token.starts_with("Launch"))
}

fn labeled_name(token: &str) -> Option<String> {
    let start = token.find('(')?;
    let end = token.rfind(')')?;
    if end <= start {
        return None;
    }
    token
        .get(start.saturating_add(1)..end)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

/// Execute-time refusal for labeled actions without frozen coordinates.
#[must_use]
pub fn unfrozen_error(action: &Action) -> Option<AdapterError> {
    match action {
        Action::Unfrozen { raw, label } => Some(label.as_ref().map_or_else(
            || AdapterError::UninterpretedAction { raw: raw.clone() },
            |label| AdapterError::UnfrozenSelector {
                label: label.clone(),
            },
        )),
        _ => None,
    }
}
