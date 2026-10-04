//! Public component-author extension surface and constrained slot/part painting.
//!
//! Provides the extension API for generic reusable components, constrained
//! part/row/cell painters, and the narrow prepared-cell `TerminalView` adapter.

use ratatui::style::{Color, Modifier, Style};

use crate::termrock::identity::{Id, ItemKey, Part, Revision};
use crate::termrock::layout::{Position, Rect, Size};
use crate::termrock::text::StyleSpan;
use crate::termrock::theme::{Surface, Theme};

/// Specification for registering a component's interactive region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionSpec {
    pub id: Id,
    pub area: Rect,
}

impl RegionSpec {
    pub fn new(id: Id, area: Rect) -> Self {
        Self { id, area }
    }
}

/// Cursor presentation request from a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorSpec {
    pub pos: Position,
    pub visible: bool,
}

impl CursorSpec {
    pub fn new(pos: Position) -> Self {
        Self { pos, visible: true }
    }

    pub fn hidden() -> Self {
        Self {
            pos: Position::new(0, 0),
            visible: false,
        }
    }
}

/// Borrowed logical text slice with associated style spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledText<'a> {
    pub text: &'a str,
    pub spans: &'a [StyleSpan],
}

impl<'a> StyledText<'a> {
    pub fn plain(text: &'a str) -> Self {
        Self { text, spans: &[] }
    }

    pub fn styled(text: &'a str, spans: &'a [StyleSpan]) -> Self {
        Self { text, spans }
    }
}

/// A prepared terminal cell for `TerminalView`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCell<'a> {
    pub symbol: &'a str,
    pub width: u8,
    pub continuation: bool,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub modifier: Modifier,
}

impl<'a> TerminalCell<'a> {
    pub fn new(symbol: &'a str) -> Self {
        Self {
            symbol,
            width: 1,
            continuation: false,
            fg: None,
            bg: None,
            modifier: Modifier::empty(),
        }
    }

    pub fn empty() -> Self {
        Self::new(" ")
    }
}

/// Terminal cursor description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCursor {
    pub pos: Position,
    pub visible: bool,
}

/// Caller-provided prepared-cell terminal source trait.
pub trait TerminalSource {
    fn revision(&self) -> Revision;
    fn size(&self) -> Size;
    fn cell(&self, position: Position) -> Option<TerminalCell<'_>>;
    fn cursor(&self) -> Option<TerminalCursor>;
}

/// Constrained painter context for an advertised component part.
pub struct PartUi<'a> {
    pub area: Rect,
    pub surface: Surface,
    pub theme: &'a Theme,
}

impl<'a> PartUi<'a> {
    pub fn new(area: Rect, surface: Surface, theme: &'a Theme) -> Self {
        Self {
            area,
            surface,
            theme,
        }
    }

    pub fn style(&self) -> Style {
        Style::new()
    }
}

/// Constrained painter context for a collection row.
pub struct RowUi<'a> {
    pub key: ItemKey,
    pub area: Rect,
    pub surface: Surface,
    pub theme: &'a Theme,
    pub selected: bool,
    pub focused: bool,
}

/// Constrained painter context for a grid cell.
pub struct CellUi<'a> {
    pub row: ItemKey,
    pub area: Rect,
    pub surface: Surface,
    pub theme: &'a Theme,
    pub selected: bool,
    pub focused: bool,
}

/// Top-level public authoring functions.
#[allow(clippy::module_inception)]
pub mod author {
    use super::*;
    use crate::termrock::runtime::Ui;

    /// Register an interactive component region with the frame.
    pub fn register(ui: &mut Ui<'_>, region: RegionSpec) {
        ui.register_hit(region.id, region.area);
    }

    /// Paint text into a reserved component part with clipping.
    pub fn paint(ui: &mut Ui<'_>, area: Rect, text: StyledText<'_>, part: Part) {
        ui.paint_styled_text(area, text, part);
    }

    /// Request cursor positioning from a custom component.
    pub fn request_cursor(ui: &mut Ui<'_>, owner: Id, cursor: CursorSpec) {
        if cursor.visible {
            ui.request_cursor(owner, cursor.pos);
        }
    }

    /// Blit prepared terminal cells for `TerminalView`.
    pub fn blit_terminal(ui: &mut Ui<'_>, area: Rect, source: &dyn TerminalSource) {
        ui.blit_terminal(area, source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTerminal {
        rev: Revision,
        size: Size,
    }

    impl TerminalSource for MockTerminal {
        fn revision(&self) -> Revision {
            self.rev
        }
        fn size(&self) -> Size {
            self.size
        }
        fn cell(&self, _pos: Position) -> Option<TerminalCell<'_>> {
            Some(TerminalCell::new("X"))
        }
        fn cursor(&self) -> Option<TerminalCursor> {
            Some(TerminalCursor {
                pos: Position::new(0, 0),
                visible: true,
            })
        }
    }

    #[test]
    fn terminal_source_contract() {
        let mock = MockTerminal {
            rev: Revision::zero(),
            size: Size::new(80, 24),
        };
        assert_eq!(mock.revision(), Revision::zero());
        assert_eq!(mock.size(), Size::new(80, 24));
        let cell = mock.cell(Position::new(0, 0)).unwrap();
        assert_eq!(cell.symbol, "X");
        assert_eq!(cell.width, 1);
        let cursor = mock.cursor().unwrap();
        assert!(cursor.visible);
        assert_eq!(cursor.pos, Position::new(0, 0));
    }

    #[test]
    fn region_and_cursor_specs() {
        let id = Id::new("custom.control");
        let area = Rect::new(5, 5, 20, 2);
        let reg = RegionSpec::new(id, area);
        assert_eq!(reg.area, area);

        let cur = CursorSpec::new(Position::new(6, 6));
        assert!(cur.visible);
        assert_eq!(cur.pos, Position::new(6, 6));

        let hidden = CursorSpec::hidden();
        assert!(!hidden.visible);
    }
}
