//! Keyed List component with shared navigation, selection, and scroll semantics.
//!
//! Preserves semantic identity across reorder/removal, bounds draw operations to
//! visible rows, and emits typed activation and selection actions.

use ratatui::crossterm::event::KeyCode;

use crate::termrock::author::RowUi;
use crate::termrock::collections::{
    Readiness, RowPainter, RowState, SelectionMode, SelectionRequest, child_item_key,
    reconcile_cursor,
};
use crate::termrock::identity::{Id, ItemKey, Keyed, Part, Revision};
use crate::termrock::layout::{Axis, Constraints, Rect, Size};
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::scroll::{ScrollRegion, ScrollState};
use crate::termrock::theme::StylePatch;

/// Durable view state for [`List`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListState {
    pub cursor: Option<ItemKey>,
    pub anchor: Option<ItemKey>,
    pub scroll: ScrollState,
    pub last_revision: Option<Revision>,
}

impl ListState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cursor(mut self, cursor: ItemKey) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn cursor(&self) -> Option<ItemKey> {
        self.cursor
    }

    pub fn anchor(&self) -> Option<ItemKey> {
        self.anchor
    }

    pub fn scroll(&self) -> &ScrollState {
        &self.scroll
    }

    pub fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }

    pub fn last_revision(&self) -> Option<Revision> {
        self.last_revision
    }
}

/// Typed actions emitted by [`List`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListAction {
    Activate {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    SelectionRequested(SelectionRequest),
}

/// Reusable keyed list component.
pub struct List<'a, T: Keyed> {
    pub id: Id,
    pub rows: &'a [T],
    pub revision: Revision,
    pub selected: &'a [ItemKey],
    pub selection_mode: SelectionMode,
    pub row_painter: Option<&'a RowPainter<T>>,
    pub readiness: Option<Readiness<'a>>,
    pub disabled: bool,
    pub patch: StylePatch,
    pub empty_text: Option<&'a str>,
}

impl<'a, T: Keyed> List<'a, T> {
    pub fn new(id: Id, rows: &'a [T], revision: Revision) -> Self {
        Self {
            id,
            rows,
            revision,
            selected: &[],
            selection_mode: SelectionMode::Single,
            row_painter: None,
            readiness: None,
            disabled: false,
            patch: StylePatch::empty(),
            empty_text: None,
        }
    }

    pub fn selected(mut self, selected: &'a [ItemKey]) -> Self {
        self.selected = selected;
        self
    }

    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    pub fn row(mut self, painter: &'a RowPainter<T>) -> Self {
        self.row_painter = Some(painter);
        self
    }

    pub fn readiness(mut self, readiness: Readiness<'a>) -> Self {
        self.readiness = Some(readiness);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = patch;
        self
    }

    pub fn empty_text(mut self, text: &'a str) -> Self {
        self.empty_text = Some(text);
        self
    }

    /// Reconcile durable state against current source slice and revision.
    pub fn reconcile(&self, state: &mut ListState) {
        if state.last_revision != Some(self.revision) {
            let item_keys: Vec<ItemKey> = self.rows.iter().map(|r| r.key()).collect();
            state.cursor = reconcile_cursor(state.cursor, &item_keys);
            state.scroll.total = self.rows.len();
            state.scroll.clamp();
            state.last_revision = Some(self.revision);
        }
    }

    pub fn update(&self, cx: &mut Cx<'_>, state: &mut ListState) -> Response<ListAction> {
        self.reconcile(state);

        if self.disabled {
            return Response::bubble(self.id.clone());
        }

        let has_focus = cx.has_focus(&self.id);
        let intended = cx.intended_owner().cloned();
        let is_target = intended.as_ref() == Some(&self.id)
            || intended.as_ref().and_then(|id| id.parent()).as_ref() == Some(&self.id)
            || intended
                .as_ref()
                .map(|id| child_item_key(&self.id, id).is_some())
                .unwrap_or(false);

        let mut action = None;
        let mut changed = false;

        let find_cursor_idx = |key_opt: Option<ItemKey>| -> Option<usize> {
            let key = key_opt?;
            self.rows.iter().position(|r| r.key() == key)
        };

        let cause = cx.cause().clone();

        match cause {
            UpdateCause::Input(Input::Key(k), _) if has_focus => {
                let cur_idx = find_cursor_idx(state.cursor).unwrap_or(0);
                match k.code {
                    KeyCode::Up | KeyCode::Char('k') if k.plain() => {
                        if cur_idx > 0 {
                            let next_idx = cur_idx - 1;
                            state.cursor = Some(self.rows[next_idx].key());
                            state.scroll.ensure_visible(next_idx);
                            changed = true;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') if k.plain() => {
                        if cur_idx + 1 < self.rows.len() {
                            let next_idx = cur_idx + 1;
                            state.cursor = Some(self.rows[next_idx].key());
                            state.scroll.ensure_visible(next_idx);
                            changed = true;
                        }
                    }
                    KeyCode::PageUp => {
                        let step = state.scroll.viewport.max(1);
                        let next_idx = cur_idx.saturating_sub(step);
                        if let Some(r) = self.rows.get(next_idx) {
                            state.cursor = Some(r.key());
                            state.scroll.ensure_visible(next_idx);
                            changed = true;
                        }
                    }
                    KeyCode::PageDown => {
                        let step = state.scroll.viewport.max(1);
                        let next_idx = (cur_idx + step).min(self.rows.len().saturating_sub(1));
                        if let Some(r) = self.rows.get(next_idx) {
                            state.cursor = Some(r.key());
                            state.scroll.ensure_visible(next_idx);
                            changed = true;
                        }
                    }
                    KeyCode::Home | KeyCode::Char('g') if k.plain() => {
                        if let Some(first) = self.rows.first() {
                            state.cursor = Some(first.key());
                            state.scroll.ensure_visible(0);
                            changed = true;
                        }
                    }
                    KeyCode::End | KeyCode::Char('G') if k.plain() => {
                        if let Some(last) = self.rows.last() {
                            let last_idx = self.rows.len().saturating_sub(1);
                            state.cursor = Some(last.key());
                            state.scroll.ensure_visible(last_idx);
                            changed = true;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(key) = state.cursor {
                            action = Some(ListAction::Activate {
                                key,
                                origin: ActivationOrigin::Keyboard,
                            });
                        }
                    }
                    KeyCode::Char(' ') if k.plain() => {
                        if let Some(key) = state.cursor {
                            match self.selection_mode {
                                SelectionMode::Single => {
                                    action = Some(ListAction::SelectionRequested(
                                        SelectionRequest::Select(key),
                                    ));
                                }
                                SelectionMode::Multiple => {
                                    action = Some(ListAction::SelectionRequested(
                                        SelectionRequest::Toggle(key),
                                    ));
                                }
                                SelectionMode::None => {
                                    action = Some(ListAction::Activate {
                                        key,
                                        origin: ActivationOrigin::Keyboard,
                                    });
                                }
                            }
                        }
                    }
                    KeyCode::Char('a')
                        if k.ctrl() && self.selection_mode == SelectionMode::Multiple =>
                    {
                        action = Some(ListAction::SelectionRequested(SelectionRequest::SelectAll));
                    }
                    _ => {}
                }
            }
            UpdateCause::Input(Input::Mouse(m), _) => {
                if let Some(target_id) = intended {
                    if let Some(clicked_key) = child_item_key(&self.id, &target_id) {
                        cx.request_focus(self.id.clone());
                        state.cursor = Some(clicked_key);
                        if let Some(idx) = find_cursor_idx(Some(clicked_key)) {
                            state.scroll.ensure_visible(idx);
                        }
                        changed = true;

                        if matches!(m.kind, MouseKind::Up | MouseKind::Down) {
                            match self.selection_mode {
                                SelectionMode::Multiple => {
                                    action = Some(ListAction::SelectionRequested(
                                        SelectionRequest::Toggle(clicked_key),
                                    ));
                                }
                                SelectionMode::Single => {
                                    action = Some(ListAction::Activate {
                                        key: clicked_key,
                                        origin: ActivationOrigin::Pointer,
                                    });
                                }
                                SelectionMode::None => {
                                    action = Some(ListAction::Activate {
                                        key: clicked_key,
                                        origin: ActivationOrigin::Pointer,
                                    });
                                }
                            }
                        }
                    } else if is_target {
                        match m.kind {
                            MouseKind::WheelUp => {
                                changed = state.scroll.scroll_by(-3);
                            }
                            MouseKind::WheelDown => {
                                changed = state.scroll.scroll_by(3);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }

        if changed {
            cx.request_invalidate(Invalidate::Paint);
        }

        if let Some(act) = action {
            Response::action(self.id.clone(), act)
                .with_flow(Flow::Consumed)
                .with_invalidate(Invalidate::Paint)
        } else if changed || is_target {
            Response::consumed(self.id.clone()).with_invalidate(if changed {
                Invalidate::Paint
            } else {
                Invalidate::None
            })
        } else {
            Response::bubble(self.id.clone())
        }
    }

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &ListState) -> Rect {
        ui.register_focus(self.id.clone(), !self.disabled);
        ui.register_hit(self.id.clone(), area);

        if area.is_empty() {
            return area;
        }

        // Empty or non-ready presentation
        if self.readiness.is_some() || self.rows.is_empty() {
            let empty_id = self.id.sub("empty");
            ui.register_hit(empty_id.clone(), area);
            ui.part(empty_id, Part::new("empty"), area, |_p| {});
            return area;
        }

        let mut scroll = state.scroll;
        scroll.total = self.rows.len();
        scroll.viewport = area.height as usize;
        scroll.clamp();

        let visible = scroll.visible_range();
        let scrollbar_needed = scroll.is_overflowing();
        let row_width = if scrollbar_needed {
            area.width.saturating_sub(1)
        } else {
            area.width
        };

        // Bounded iteration: draw strictly visible rows
        for i in visible {
            let row = &self.rows[i];
            let key = row.key();
            let row_y = area.y + (i - scroll.offset) as u16;
            let row_area = Rect::new(area.x, row_y, row_width, 1);
            let row_id = self.id.child(key);

            ui.register_hit(row_id.clone(), row_area);

            let is_selected = self.selected.contains(&key);
            let is_cursor = state.cursor == Some(key);

            let visual = VisualState::empty()
                .selected(is_selected)
                .focused(is_cursor)
                .disabled(self.disabled);

            let row_state = RowState::new(visual, is_selected, is_cursor, i);

            if let Some(painter) = self.row_painter {
                let mut row_ui = RowUi {
                    key,
                    area: row_area,
                    surface: ui.current_surface,
                    theme: ui.theme,
                    selected: is_selected,
                    focused: is_cursor,
                };
                painter(&mut row_ui, row_area, row, row_state);
            } else {
                ui.part(row_id, Part::new("row"), row_area, |_p| {});
            }
        }

        // Render scrollbar on right edge if overflowing
        if scrollbar_needed {
            let scroll_region = ScrollRegion::new(
                self.id.clone(),
                Axis::Vertical,
                self.rows.len(),
                area.height as usize,
            );
            scroll_region.draw(ui, area, &scroll);
        }

        area
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let h = self.rows.len() as u16;
        constraints.clamp(Size::new(20, h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct Item {
        key: ItemKey,
        label: &'static str,
    }

    impl Keyed for Item {
        fn key(&self) -> ItemKey {
            self.key
        }
    }

    #[test]
    fn test_list_reconciles_cursor_on_revision_change() {
        let id = Id::new("list");
        let items_v0 = vec![
            Item {
                key: ItemKey::new(1),
                label: "One",
            },
            Item {
                key: ItemKey::new(2),
                label: "Two",
            },
        ];
        let mut state = ListState::new().with_cursor(ItemKey::new(2));

        let list_v0 = List::new(id.clone(), &items_v0, Revision::new(0));
        list_v0.reconcile(&mut state);
        assert_eq!(state.cursor, Some(ItemKey::new(2)));
        assert_eq!(state.last_revision, Some(Revision::new(0)));

        // Remove item 2 in v1
        let items_v1 = vec![Item {
            key: ItemKey::new(1),
            label: "One",
        }];
        let list_v1 = List::new(id, &items_v1, Revision::new(1));
        list_v1.reconcile(&mut state);
        assert_eq!(state.cursor, Some(ItemKey::new(1)));
        assert_eq!(state.last_revision, Some(Revision::new(1)));
    }
}
