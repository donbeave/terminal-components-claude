//! Finite HO scenario expansion: every register row becomes a program.

use holla_app::Motion;

use crate::branches::{NAMED_BRANCHES, NamedBranch};
use crate::observe::ORACLE_SIZES;
use crate::tsv::Table;
use crate::worlds::intern_world;

/// Frozen scenario register.
const SCENARIOS_TSV: &str = include_str!("../data/holla-scenarios.tsv");

/// One immutable action in an expanded program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// `NEW(world, motion, tick)`.
    New {
        /// Oracle world name.
        world: String,
        /// Motion policy.
        motion: Motion,
        /// Oracle tick ordinal.
        tick: u64,
    },
    /// Draw the current frame.
    Draw,
    /// Deliver `n` virtual ticks.
    Ticks(u32),
    /// Type Unicode scalars.
    Type(String),
    /// Bracketed paste.
    Paste(String),
    /// OPEN(label) helper.
    Open(String),
    /// RUN(label) helper.
    Run(String),
    /// SETTLE(id) helper.
    Settle(String),
    /// SCAN(kind) helper.
    Scan(String),
    /// SWEEP expansion marker.
    Sweep,
    /// ROUTE(file::test) native seed.
    Route {
        /// Source path.
        file: String,
        /// Test symbol.
        test: String,
    },
    /// Key with optional modifiers, stored as source text.
    Key(String),
    /// Repeat the constructor in a fresh world.
    RepeatFresh,
    /// Unparsed remainder retained so no clause is dropped.
    Raw(String),
}

/// Expanded scenario program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioProgram {
    /// HO-* identity.
    pub id: String,
    /// Fixture world or native-test marker.
    pub fixture: String,
    /// Declared sizes.
    pub sizes: Vec<(u16, u16)>,
    /// Structured actions.
    pub actions: Vec<Action>,
    /// Named branches attached to this parent.
    pub branches: Vec<String>,
    /// Raw actions column.
    pub raw_actions: String,
}

/// Parse and expand the frozen scenario register.
#[must_use]
pub fn expand_scenarios() -> Vec<ScenarioProgram> {
    let Some(table) = Table::parse(SCENARIOS_TSV) else {
        return Vec::new();
    };
    let mut programs = Vec::with_capacity(table.len());
    let mut index = 0;
    while index < table.len() {
        let Some(id) = table.get(index, "id") else {
            break;
        };
        let fixture = table.get(index, "fixture").unwrap_or("").to_owned();
        let raw_actions = table.get(index, "actions").unwrap_or("").to_owned();
        let sizes = parse_sizes(table.get(index, "sizes").unwrap_or(""));
        let actions = parse_actions(&raw_actions);
        let branches = NAMED_BRANCHES
            .iter()
            .filter(|branch| branch.parent == id)
            .map(|branch| branch.id.to_owned())
            .collect();
        programs.push(ScenarioProgram {
            id: id.to_owned(),
            fixture,
            sizes,
            actions,
            branches,
            raw_actions,
        });
        index = index.saturating_add(1);
    }
    programs
}

/// Scenario IDs in register order.
#[must_use]
pub fn scenario_ids() -> Vec<String> {
    expand_scenarios()
        .into_iter()
        .map(|program| program.id)
        .collect()
}

fn parse_sizes(spec: &str) -> Vec<(u16, u16)> {
    let mut sizes = Vec::new();
    for token in spec.split(',') {
        let token = token.trim();
        if let Some((w, h)) = token.split_once('x')
            && let (Ok(width), Ok(height)) = (w.parse::<u16>(), h.parse::<u16>())
        {
            sizes.push((width, height));
        }
    }
    if sizes.is_empty() {
        ORACLE_SIZES.into_iter().collect()
    } else {
        sizes
    }
}

fn parse_actions(raw: &str) -> Vec<Action> {
    let mut out = Vec::new();
    for part in split_top_level(raw) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(inner) = part.strip_prefix("NEW(").and_then(|s| s.strip_suffix(')')) {
            out.push(parse_new(inner));
        } else if part == "DRAW" {
            out.push(Action::Draw);
        } else if let Some(rest) = part
            .strip_prefix("TICKS(")
            .and_then(|s| s.strip_suffix(')'))
        {
            out.push(Action::Ticks(rest.parse().unwrap_or(0)));
        } else if let Some(rest) = part.strip_prefix("TYPE(").and_then(|s| s.strip_suffix(')')) {
            out.push(Action::Type(rest.to_owned()));
        } else if let Some(rest) = part
            .strip_prefix("PASTE(")
            .and_then(|s| s.strip_suffix(')'))
        {
            out.push(Action::Paste(rest.to_owned()));
        } else if let Some(rest) = part.strip_prefix("OPEN(").and_then(|s| s.strip_suffix(')')) {
            out.push(Action::Open(rest.to_owned()));
        } else if let Some(rest) = part.strip_prefix("RUN(").and_then(|s| s.strip_suffix(')')) {
            out.push(Action::Run(rest.to_owned()));
        } else if let Some(rest) = part
            .strip_prefix("SETTLE(")
            .and_then(|s| s.strip_suffix(')'))
        {
            out.push(Action::Settle(rest.to_owned()));
        } else if let Some(rest) = part.strip_prefix("SCAN(").and_then(|s| s.strip_suffix(')')) {
            out.push(Action::Scan(rest.to_owned()));
        } else if part == "SWEEP" || part.starts_with("SWEEP ") || part.starts_with("SWEEP(") {
            out.push(Action::Sweep);
            if part != "SWEEP" {
                out.push(Action::Raw(part.to_owned()));
            }
        } else if let Some(rest) = part
            .strip_prefix("ROUTE(")
            .and_then(|s| s.strip_suffix(')'))
        {
            out.push(parse_route(rest));
        } else if let Some(rest) = part.strip_prefix("KEY(").and_then(|s| s.strip_suffix(')')) {
            out.push(Action::Key(rest.to_owned()));
        } else if part.contains("repeat fresh") {
            out.push(Action::RepeatFresh);
            out.push(Action::Raw(part.to_owned()));
        } else {
            out.push(Action::Raw(part.to_owned()));
        }
    }
    out
}

fn parse_new(inner: &str) -> Action {
    let mut bits = inner.split(',');
    let world = bits.next().unwrap_or("").trim();
    let motion_name = bits.next().unwrap_or("").trim();
    let tick = bits
        .next()
        .unwrap_or("0")
        .trim()
        .parse::<u64>()
        .unwrap_or(0);
    Action::New {
        world: intern_world(world).unwrap_or(world).to_owned(),
        motion: motion_from_name(motion_name),
        tick,
    }
}

fn parse_route(inner: &str) -> Action {
    match inner.rsplit_once("::") {
        Some((file, test)) => Action::Route {
            file: file.trim().to_owned(),
            test: test.trim().to_owned(),
        },
        None => Action::Route {
            file: inner.to_owned(),
            test: String::new(),
        },
    }
}

fn motion_from_name(name: &str) -> Motion {
    match name {
        "Paused" | "paused" => Motion::Paused,
        "Reduced" | "reduced" => Motion::Reduced,
        _ => Motion::Full,
    }
}

fn split_top_level(raw: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0i32;
    for (index, ch) in raw.char_indices() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => {
                out.push(raw.get(start..index).unwrap_or(""));
                start = index.saturating_add(1);
            }
            _ => {}
        }
    }
    if let Some(tail) = raw.get(start..) {
        out.push(tail);
    }
    out
}

/// Branches belonging to `parent`.
#[must_use]
pub fn branches_for(parent: &str) -> Vec<&'static NamedBranch> {
    NAMED_BRANCHES
        .iter()
        .filter(|branch| branch.parent == parent)
        .collect()
}
