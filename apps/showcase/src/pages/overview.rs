//! Overview screen: the public-facade contract and its stable sample data.

use tui_next::{
    Brand, Chord, DerivedHintBar, Empty, EmptyState, HelpOverlay, HelpOverlayState, HelpSection,
    Hint, HintBar, HintLayer, Id, ItemKey, KeyCode, KeyHint, Props, PropsList, PropsState, Rect,
    Response, RowUi, TooSmall, Ui, id, layout,
};

use super::{Page, author::AuthorBadge, frame, lines};

const BRAND: Id = id!("overview.brand");
const AUTHOR: Id = id!("overview.author");
const EMPTY: Id = id!("overview.empty");
const KEY_HINT: Id = id!("overview.key-hint");
const HINT_BAR: Id = id!("overview.hint-bar");
const DERIVED_HINT_BAR: Id = id!("overview.derived-hint-bar");
const HELP: Id = id!("overview.help");
const TOO_SMALL: Id = id!("overview.too-small");
const PROPS_ID: Id = id!("overview.props");
const PROPS: [(&str, &str); 6] = [
    ("Library", "tui-next"),
    ("Ownership", "application state"),
    ("Input", "runtime intents"),
    ("Rendering", "public Ui facade"),
    ("Binary", "showcase"),
    ("Tokens", "surface · accent · focus"),
];
const PROPS_SUMMARY: [(&str, &str); 1] = [("Mode", "interactive")];

type OverviewPropsList = PropsList<
    'static,
    (&'static str, &'static str),
    fn(&(&'static str, &'static str)) -> ItemKey,
    fn(&(&'static str, &'static str), &mut RowUi<'_>),
>;

fn prop_key((label, _value): &(&'static str, &'static str)) -> ItemKey {
    ItemKey::text(label)
}

fn prop_row((label, value): &(&'static str, &'static str), row: &mut RowUi<'_>) {
    row.label(label);
    row.meta(value);
}

fn props_list() -> OverviewPropsList {
    PropsList::new(PROPS_ID)
        .key(prop_key as fn(&(&'static str, &'static str)) -> ItemKey)
        .row(prop_row as fn(&(&'static str, &'static str), &mut RowUi<'_>))
}
const FULL_COPY: [&str; 4] = [
    "A complete app-owned migration of the legacy showcase.",
    "Each page owns durable state and talks to tui-next through",
    "the same public facade available to downstream binaries.",
    "Tab focuses controls · Enter activates · Esc returns home.",
];
const COMPACT_COPY: [&str; 4] = [
    "App-owned migration via tui-next.",
    "Pages own state; one public facade.",
    "Downstream binaries share that facade.",
    "Tab focus · Enter activate · Esc home.",
];

fn brand() -> Brand<'static> {
    Brand::new(BRAND, "Junie").tagline("component showcase")
}

fn inventory_hints() -> HintLayer {
    HintLayer {
        hints: vec![Hint {
            chord: Chord::key(KeyCode::Char('i')),
            label: "inspect",
            priority: 50,
        }],
        badge: Some("API"),
        status: None,
        centered: false,
    }
}

fn derived_hint_bar() -> DerivedHintBar<'static> {
    let derived: DerivedHintBar<'static> = HintBar::derived(DERIVED_HINT_BAR);
    derived
}

/// The landing page has one caller-owned property-list cursor; its content is
/// deliberately useful as a smoke test for themes, clipping and public exports.
#[derive(Debug)]
pub(crate) struct OverviewPage {
    author: AuthorBadge,
    help_state: HelpOverlayState,
    props_state: PropsState,
}

impl OverviewPage {
    pub(crate) fn new() -> Self {
        Self {
            author: AuthorBadge::new(AUTHOR),
            help_state: HelpOverlayState::default(),
            props_state: PropsState::default(),
        }
    }
}

impl Default for OverviewPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Page for OverviewPage {
    fn title(&self) -> &'static str {
        "Overview"
    }

    fn update(&mut self, cx: &mut tui_next::Cx<'_>) -> Response<()> {
        let mut response = brand().update(cx).erase();
        response |= self.author.update(cx);
        let hints = HintLayer::empty();
        let sections = [HelpSection::new("Overview", &hints)];
        response |= HelpOverlay::new(HELP, "overview", &sections)
            .update(cx, &mut self.help_state)
            .erase();
        response |= props_list()
            .update(cx, &mut self.props_state, &PROPS)
            .erase();
        let _ = Empty::new(
            EMPTY,
            EmptyState::Empty {
                title: "No optional content",
                hint: Some("the app owns this empty state"),
            },
        );
        let _ = KeyHint::new(KEY_HINT, Chord::key(KeyCode::Char('i')), "inspect");
        let _ = HintBar::new(HINT_BAR, &hints);
        let _ = derived_hint_bar();
        let _ = TooSmall::new(TOO_SMALL, "showcase");
        response
    }

    fn draw(&self, ui: &mut Ui<'_>, area: Rect) {
        frame(ui, area, self.title(), "public tui-next API", |ui, body| {
            let (intro, rest) = layout::split_v(body, 4);
            brand().draw(ui, intro);
            // The property column needs 28 cells for its widest label/value
            // pair. Keep a two-cell gutter so clipped copy cannot visually
            // merge with the metadata at compact terminal sizes.
            let props_width = 28.min(rest.width.saturating_sub(2));
            let copy_width = rest.width.saturating_sub(props_width).saturating_sub(2);
            let (copy, after_copy) = layout::split_h(rest, copy_width);
            let (_, props) = layout::split_h(after_copy, 2);
            let (props_list_area, props_summary_area) =
                layout::split_v(props, props.height.saturating_sub(1));
            let (copy_text, author_area) = layout::split_v(copy, copy.height.saturating_sub(2));
            let (copy_text, inventory) =
                layout::split_v(copy_text, copy_text.height.saturating_sub(7));
            let copy_lines: &[&str] = if copy.width >= 59 {
                &FULL_COPY
            } else {
                &COMPACT_COPY
            };
            lines(ui, copy_text, copy_lines);
            self.author.draw(ui, author_area);
            props_list().draw(ui, props_list_area, &self.props_state, &PROPS);
            Props::new(&PROPS_SUMMARY).draw(ui, props_summary_area);

            let hints = inventory_hints();
            let sections = [HelpSection::new("Overview", &hints)];
            let inventory_rows = super::rows(inventory, 5);
            if let Some(row) = inventory_rows.first().copied() {
                KeyHint::new(KEY_HINT, Chord::key(KeyCode::Char('i')), "inspect").draw(ui, row);
            }
            if let Some(row) = inventory_rows.get(1).copied() {
                HintBar::new(HINT_BAR, &hints).draw(ui, row);
            }
            if let Some(row) = inventory_rows.get(2).copied() {
                derived_hint_bar().global(&hints).draw(ui, row);
            }
            if let Some(row) = inventory_rows.get(3).copied() {
                Empty::new(
                    EMPTY,
                    EmptyState::Empty {
                        title: "No optional content",
                        hint: Some("the app owns this empty state"),
                    },
                )
                .draw(ui, row);
            }
            if let Some(row) = inventory_rows.get(4).copied() {
                HelpOverlay::new(HELP, "overview", &sections).draw(ui, row, &self.help_state);
            }
        });
        if area.width < 72 || area.height < 20 {
            TooSmall::new(TOO_SMALL, "showcase").draw(ui, area);
        }
    }
}
