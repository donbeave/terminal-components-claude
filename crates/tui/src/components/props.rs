//! `Props` — a two-column label / value list (`COMPONENT_ARCHITECTURE.md`
//! §12.4, §17.0 A7).

use core::fmt;
use core::marker::PhantomData;

use ratatui_core::layout::Rect;

use super::scroll_region::ScrollRegion;
use super::{Acc, Overrides, SlotFn, cell_at};
use crate::collection::{
    ByIndex, CollectionCore, DefaultRow, KeyFn, Reconcile, Reconciliation, RowFn, RowUi,
};
use crate::event::{Chord, KeyCode};
use crate::focus::Focusability;
use crate::id::{Id, ItemKey, Part, PartRef};
use crate::intent::{Intent, Phase};
use crate::keymap::{Binding, BindingState, Bindings};
use crate::measure::{Constraints, Size};
use crate::response::{Response, StateFlags};
use crate::scroll::ScrollState;
use crate::text::width;
use crate::theme::{Family, StylePatch, Variant};
use crate::ui::{Cx, FrameRead, Ui};

/// What an interactive property surface reports.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropsAction {
    /// Copy the caller-owned value identified by this row key.
    Copy(ItemKey),
}

/// The const-constructible commands of [`PropsList`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropsCmd {
    /// Move to the previous row.
    Up,
    /// Move to the next row.
    Down,
    /// Move up one viewport.
    PageUp,
    /// Move down one viewport.
    PageDown,
    /// Move to the first row.
    Home,
    /// Move to the last row.
    End,
    /// Copy the current row when it is copyable.
    Copy,
}

const fn binding(
    action: &'static str,
    chord: Chord,
    cmd: PropsCmd,
    label: &'static str,
    visible: bool,
) -> Binding<PropsCmd> {
    Binding {
        action: crate::ActionKey::custom(action),
        chord: Some(chord),
        cmd,
        label,
        priority: if visible { 60 } else { 10 },
        visible,
    }
}

const PROPS_BINDINGS: [Binding<PropsCmd>; 12] = [
    binding(
        "props.up",
        Chord::key(KeyCode::Up),
        PropsCmd::Up,
        "Up",
        true,
    ),
    binding(
        "props.down",
        Chord::key(KeyCode::Down),
        PropsCmd::Down,
        "Down",
        true,
    ),
    binding(
        "props.up-vim",
        Chord::key(KeyCode::Char('k')),
        PropsCmd::Up,
        "Up",
        false,
    ),
    binding(
        "props.down-vim",
        Chord::key(KeyCode::Char('j')),
        PropsCmd::Down,
        "Down",
        false,
    ),
    binding(
        "props.page-up",
        Chord::key(KeyCode::PageUp),
        PropsCmd::PageUp,
        "Page up",
        false,
    ),
    binding(
        "props.page-down",
        Chord::key(KeyCode::PageDown),
        PropsCmd::PageDown,
        "Page down",
        false,
    ),
    binding(
        "props.home",
        Chord::key(KeyCode::Home),
        PropsCmd::Home,
        "First",
        false,
    ),
    binding(
        "props.home-vim",
        Chord::key(KeyCode::Char('g')),
        PropsCmd::Home,
        "First",
        false,
    ),
    binding(
        "props.end",
        Chord::key(KeyCode::End),
        PropsCmd::End,
        "Last",
        false,
    ),
    binding(
        "props.end-vim",
        Chord::key(KeyCode::Char('G')),
        PropsCmd::End,
        "Last",
        false,
    ),
    binding(
        "props.copy",
        Chord::key(KeyCode::Char('y')),
        PropsCmd::Copy,
        "Copy",
        true,
    ),
    binding(
        "props.copy-enter",
        Chord::key(KeyCode::Enter),
        PropsCmd::Copy,
        "Copy",
        false,
    ),
];

/// Caller-owned interaction state for [`PropsList`].
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PropsState {
    core: CollectionCore,
}

impl PropsState {
    /// The current row key.
    pub const fn cursor(&self) -> Option<ItemKey> {
        self.core.cursor()
    }

    /// The current row index from the last reconciliation.
    pub const fn cursor_index(&self) -> usize {
        self.core.cursor_index()
    }

    /// The vertical scroll state.
    pub const fn scroll(&self) -> &ScrollState {
        self.core.scroll()
    }

    /// Point the cursor at a keyed row.
    pub fn set_cursor(&mut self, index: usize, key: ItemKey) {
        self.core.set_cursor(index, key);
    }
}

impl Reconcile for PropsState {
    fn reconcile(&mut self, len: usize, key: impl Fn(usize) -> ItemKey) -> Reconciliation {
        self.core.reconcile(len, key)
    }

    fn invalidate(&mut self) {
        self.core.invalidate();
    }
}

/// A keyed, scrollable two-column list over caller-borrowed items.
///
/// Items are passed to each phase. `.key` supplies durable identity and
/// `.row` paints the two columns through [`RowUi::label`] and
/// [`RowUi::meta`]. `.copyable_item` is only an authorization predicate;
/// [`PropsAction::Copy`] carries a key, never the value. Secret owners must
/// paint through [`crate::Secret::write_mask`] and leave that predicate false.
pub struct PropsList<'a, T, K = ByIndex, R = DefaultRow> {
    id: Id,
    key: K,
    row: R,
    copyable_item: Option<&'a dyn Fn(&T) -> bool>,
    patch: Option<&'a StylePatch>,
    parts: &'a [(Part, StylePatch)],
    ov: Overrides<'a>,
    _item: PhantomData<fn(&T)>,
}

impl<T, K, R> fmt::Debug for PropsList<'_, T, K, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropsList")
            .field("id", &self.id)
            .field("copyable_item", &self.copyable_item.is_some())
            .finish_non_exhaustive()
    }
}

impl<T> PropsList<'_, T, ByIndex, DefaultRow> {
    /// A property list keyed by index and painted through `Display`.
    pub const fn new(id: Id) -> Self {
        PropsList {
            id,
            key: ByIndex,
            row: DefaultRow,
            copyable_item: None,
            patch: None,
            parts: &[],
            ov: Overrides::new(),
            _item: PhantomData,
        }
    }
}

impl<'a, T, K, R> PropsList<'a, T, K, R> {
    /// The parts this component styles or registers.
    pub const PARTS: &'static [Part] = &[
        Part::CONTAINER,
        Part::GUTTER,
        Part::LABEL,
        Part::META,
        Part::ROW,
        Part::TRACK,
        Part::THUMB,
    ];

    /// A stable key accessor.
    pub fn key<K2: Fn(&T) -> ItemKey>(self, key: K2) -> PropsList<'a, T, K2, R> {
        PropsList {
            id: self.id,
            key,
            row: self.row,
            copyable_item: self.copyable_item,
            patch: self.patch,
            parts: self.parts,
            ov: self.ov,
            _item: PhantomData,
        }
    }

    /// A two-column row painter.
    pub fn row<R2: Fn(&T, &mut RowUi<'_>)>(self, row: R2) -> PropsList<'a, T, K, R2> {
        PropsList {
            id: self.id,
            key: self.key,
            row,
            copyable_item: self.copyable_item,
            patch: self.patch,
            parts: self.parts,
            ov: self.ov,
            _item: PhantomData,
        }
    }

    /// Authorize copy for selected items. Secret-bearing rows must return
    /// `false`; the component never reads or returns item values.
    #[must_use]
    pub fn copyable_item(mut self, copyable: &'a dyn Fn(&T) -> bool) -> Self {
        self.copyable_item = Some(copyable);
        self
    }

    /// Apply an instance patch to every part.
    #[must_use]
    pub fn patch(mut self, patch: &'a StylePatch) -> Self {
        self.patch = Some(patch);
        self.ov = self.ov.patch(patch);
        self
    }

    /// Apply instance patches to named parts.
    #[must_use]
    pub fn patch_part(mut self, patches: &'a [(Part, StylePatch)]) -> Self {
        self.parts = patches;
        self.ov = self.ov.patch_part(patches);
        self
    }

    /// Replace `GUTTER`, `TRACK`, or `THUMB` painting without changing
    /// layout or hit regions.
    #[must_use]
    pub fn slot(mut self, part: Part, painter: SlotFn<'a>) -> Self {
        if matches!(part, Part::GUTTER | Part::TRACK | Part::THUMB) {
            self.ov = self.ov.slot(part, painter);
        }
        self
    }

    fn scrollbar(&self) -> ScrollRegion<'a> {
        let mut scrollbar = ScrollRegion::new(self.id)
            .inherit_family(Family::PROPS)
            .patch_part(self.parts);
        if let Some(patch) = self.patch {
            scrollbar = scrollbar.patch(patch);
        }
        if let Some(painter) = self.ov.slot_for(Part::TRACK) {
            scrollbar = scrollbar.slot(Part::TRACK, painter);
        }
        if let Some(painter) = self.ov.slot_for(Part::THUMB) {
            scrollbar = scrollbar.slot(Part::THUMB, painter);
        }
        scrollbar
    }

    fn can_copy(&self, item: &T) -> bool {
        self.copyable_item.is_some_and(|copyable| copyable(item))
    }
}

struct PaintRow<'a> {
    id: Id,
    flags: StateFlags,
    key: ItemKey,
    show_gutter: bool,
    gutter: Option<SlotFn<'a>>,
    ov: Overrides<'a>,
}

fn paint_two_columns<T, R: RowFn<T>>(
    ui: &mut Ui<'_>,
    area: Rect,
    item: &T,
    row: &R,
    paint: PaintRow<'_>,
) {
    {
        let mut row_ui = RowUi::new_with_column_patches(
            ui,
            paint.id,
            Family::PROPS,
            Variant::DEFAULT,
            paint.flags,
            paint.key,
            area,
            paint.ov.part_patch(Part::CONTAINER),
            paint.ov.part_patch(Part::LABEL),
            paint.ov.part_patch(Part::META),
        );
        if paint.show_gutter {
            row_ui.gutter();
        }
        row.row(item, &mut row_ui);
    }
    if paint.show_gutter
        && let Some(gutter) = paint.gutter
    {
        gutter(ui, cell_at(area, area.x));
    }
}

impl<T, K: KeyFn<T>, R: RowFn<T>> PropsList<'_, T, K, R> {
    fn key_at(&self, items: &[T], index: usize) -> ItemKey {
        items
            .get(index)
            .map_or(ItemKey::index(index), |item| self.key.key(item, index))
    }

    fn index_of(&self, items: &[T], key: ItemKey, hint: Option<usize>) -> Option<usize> {
        if let Some(index) = hint
            && index < items.len()
            && self.key_at(items, index) == key
        {
            return Some(index);
        }
        (0..items.len()).find(|&index| self.key_at(items, index) == key)
    }

    fn move_cursor(
        &self,
        st: &mut PropsState,
        items: &[T],
        target: usize,
        acc: &mut Acc<PropsAction>,
    ) {
        let Some(index) = (!items.is_empty()).then(|| target.min(items.len().saturating_sub(1)))
        else {
            acc.consumed();
            return;
        };
        let key = self.key_at(items, index);
        if st.core.cursor() == Some(key) {
            acc.consumed();
            return;
        }
        st.core.set_cursor(index, key);
        acc.changed();
    }

    fn copy_cursor(&self, st: &PropsState, items: &[T], acc: &mut Acc<PropsAction>) {
        let Some(key) = st.core.cursor() else {
            acc.consumed();
            return;
        };
        let Some(index) = self.index_of(items, key, Some(st.core.cursor_index())) else {
            acc.consumed();
            return;
        };
        if items.get(index).is_some_and(|item| self.can_copy(item)) {
            acc.action(PropsAction::Copy(key));
        } else {
            acc.consumed();
        }
    }

    /// Reconcile keyed rows, then drain navigation, copy, pointer, wheel and
    /// scrollbar intents.
    pub fn update(
        &self,
        cx: &mut Cx<'_>,
        st: &mut PropsState,
        items: &[T],
    ) -> Response<PropsAction> {
        let _ = st
            .core
            .reconcile(items.len(), |index| self.key_at(items, index));
        if st.core.cursor().is_none() && !items.is_empty() {
            st.core.set_cursor(0, self.key_at(items, 0));
        }
        let mut acc = Acc::new();
        let scroll = self
            .scrollbar()
            .update(cx, st.core.scroll_mut(), items.len());
        acc.fold(&scroll);
        let viewport = st.core.scroll().viewport_len().max(1);
        for intent in cx.intents(self.id) {
            match intent {
                Intent::Binding(action) => match Binding::command(&PROPS_BINDINGS, action) {
                    Some(PropsCmd::Up) => self.move_cursor(
                        st,
                        items,
                        st.core.cursor_index().saturating_sub(1),
                        &mut acc,
                    ),
                    Some(PropsCmd::Down) => self.move_cursor(
                        st,
                        items,
                        st.core.cursor_index().saturating_add(1),
                        &mut acc,
                    ),
                    Some(PropsCmd::PageUp) => self.move_cursor(
                        st,
                        items,
                        st.core.cursor_index().saturating_sub(viewport),
                        &mut acc,
                    ),
                    Some(PropsCmd::PageDown) => self.move_cursor(
                        st,
                        items,
                        st.core.cursor_index().saturating_add(viewport),
                        &mut acc,
                    ),
                    Some(PropsCmd::Home) => self.move_cursor(st, items, 0, &mut acc),
                    Some(PropsCmd::End) => self.move_cursor(st, items, usize::MAX, &mut acc),
                    Some(PropsCmd::Copy) => self.copy_cursor(st, items, &mut acc),
                    None => {}
                },
                Intent::Pointer {
                    phase,
                    part:
                        PartRef {
                            part: Part::ROW,
                            item: Some(key),
                        },
                    ..
                } => {
                    let hint = self.index_of(items, key, Some(st.core.cursor_index()));
                    let Some(index) = hint else {
                        acc.consumed();
                        continue;
                    };
                    match phase {
                        Phase::Press => self.move_cursor(st, items, index, &mut acc),
                        Phase::Click | Phase::DoubleClick => {
                            if st.core.cursor() != Some(key) {
                                st.core.set_cursor(index, key);
                                acc.changed();
                            }
                            self.copy_cursor(st, items, &mut acc);
                        }
                        _ => acc.consumed(),
                    }
                }
                Intent::Pointer { .. } => acc.consumed(),
                _ => {}
            }
        }
        acc.finish(self.id)
    }

    /// Draw only visible rows; the row renderer is never invoked offscreen.
    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, st: &PropsState, items: &[T]) -> Rect {
        if area.is_empty() {
            return area;
        }
        if !ui.is_inert() {
            ui.register_control(self.id, area, Focusability::Focusable);
        }
        let live = Overrides::flags(ui.state(self.id), StateFlags::empty());
        if !ui.is_inert() {
            ui.publish_bindings(self.id, live, &PROPS_BINDINGS);
        }
        let root = self.ov.style(
            ui,
            self.id,
            Family::PROPS,
            Variant::DEFAULT,
            Part::CONTAINER,
            live.difference(StateFlags::FOCUSED | StateFlags::PRESSED),
        );
        ui.fill(area, root.style);
        let content = self
            .scrollbar()
            .draw(ui, area, st.core.scroll(), items.len());
        let view = ScrollRegion::view(st.core.scroll(), content, items.len());
        let hovered = ui.hovered_part(self.id);
        let pressed = ui.pressed_part(self.id);
        for (visible, index) in view.visible_range().enumerate() {
            let Some(item) = items.get(index) else { break };
            let key = self.key.key(item, index);
            let row_part = PartRef::item(Part::ROW, key);
            let is_cursor = st.core.cursor() == Some(key);
            let mut flags = StateFlags::empty();
            if is_cursor {
                flags |= live & (StateFlags::FOCUSED | StateFlags::FOCUS_VISIBLE);
            }
            if hovered == Some(row_part) {
                flags |= StateFlags::HOVERED;
            }
            if pressed == Some(row_part)
                || (pressed.is_none() && is_cursor && live.contains(StateFlags::PRESSED))
            {
                flags |= StateFlags::PRESSED;
            }
            let row_area = Rect {
                x: content.x,
                y: content
                    .y
                    .saturating_add(visible.min(usize::from(u16::MAX)) as u16),
                width: content.width,
                height: 1,
            };
            paint_two_columns(
                ui,
                row_area,
                item,
                &self.row,
                PaintRow {
                    id: self.id,
                    flags,
                    key,
                    show_gutter: true,
                    gutter: self.ov.slot_for(Part::GUTTER),
                    ov: self.ov,
                },
            );
            if !ui.is_inert() {
                ui.register_part(self.id, row_part, row_area);
            }
        }
        area
    }

    /// A conservative natural size for a property list.
    pub fn measure(&self, _ui: &Ui<'_>, constraints: Constraints) -> Size {
        Size {
            min: (12, 1),
            preferred: (32, constraints.max.1),
        }
        .fit(constraints)
    }
}

impl<T, K, R> Bindings for PropsList<'_, T, K, R> {
    type Cmd = PropsCmd;

    fn bindings(&self, _state: BindingState) -> &'static [Binding<PropsCmd>] {
        &PROPS_BINDINGS
    }
}

fn paint_pair(row: &(&str, &str), ui: &mut RowUi<'_>) {
    ui.label(row.0);
    ui.meta(row.1);
}

/// Label / value rows: muted labels in a column sized to the widest, values
/// beside them.
///
/// ## Construction
/// `Props::new(rows)` over `&[(&str, &str)]`.
///
/// ## Ownership
/// Stateless; the rows are borrowed.
///
/// ## Configuration
/// `.patch_part`.
///
/// ## Variants
/// `Family::PROPS`, `DEFAULT` only.
///
/// ## States
/// None.
///
/// ## Actions
/// None.
///
/// ## Focus
/// Never a focus stop; registers nothing (it has no id).
///
/// ## Keyboard
/// None.
///
/// ## Mouse
/// None.
///
/// ## Layout
/// One row per pair; `measure` is `(widest label + 2 + widest value,
/// rows)`; `draw` returns the rows painted, clipped to `area`.
///
/// ## Parts
/// `META` (the label column), `LABEL` (the value column).
///
/// ## Overrides
/// `.patch_part` on both parts.
///
/// ## Identity
/// None.
///
/// ## Testing
/// `PropsCase` with no capabilities; `render::components::dialog::*` covers
/// it inside a body.
///
/// ## Invariants
/// Never writes outside `area`; never allocates.
pub struct Props<'a> {
    rows: &'a [(&'a str, &'a str)],
    ov: Overrides<'a>,
}

impl fmt::Debug for Props<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Props")
            .field("rows", &self.rows.len())
            .finish_non_exhaustive()
    }
}

impl<'a> Props<'a> {
    /// The parts this component styles.
    pub const PARTS: &'static [Part] = &[Part::META, Part::LABEL];

    /// Rows of `(label, value)`.
    pub const fn new(rows: &'a [(&'a str, &'a str)]) -> Self {
        Props {
            rows,
            ov: Overrides::new(),
        }
    }

    /// Per-part instance patches.
    #[must_use]
    pub const fn patch_part(mut self, ps: &'a [(Part, StylePatch)]) -> Self {
        self.ov = self.ov.patch_part(ps);
        self
    }

    fn label_width(&self) -> u16 {
        self.rows.iter().map(|(k, _)| width(k)).max().unwrap_or(0)
    }

    /// The draw phase.
    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect {
        if area.is_empty() {
            return area;
        }
        let mut painted = 0u16;
        for (index, (row_area, row)) in area.rows().zip(self.rows.iter()).enumerate() {
            paint_two_columns(
                ui,
                row_area,
                row,
                &paint_pair,
                PaintRow {
                    id: Id::root("tui.props"),
                    flags: StateFlags::empty(),
                    key: ItemKey::index(index),
                    show_gutter: false,
                    gutter: None,
                    ov: self.ov,
                },
            );
            painted = painted.saturating_add(1);
        }
        Rect {
            height: painted,
            ..area
        }
    }

    /// The natural size.
    pub fn measure(&self, _ui: &Ui<'_>, c: Constraints) -> Size {
        let vw = self.rows.iter().map(|(_, v)| width(v)).max().unwrap_or(0);
        let w = self.label_width().saturating_add(2).saturating_add(vw);
        let h = self.rows.len().min(usize::from(u16::MAX)) as u16;
        Size::exact(w, h).fit(c)
    }
}
