//! Keyed FilterList component combining search input with a keyed list projection.
//!
//! Provides text query editing, Unicode-safe matching, and keyed list delegation.

use ratatui::crossterm::event::KeyCode;

use crate::termrock::collections::{
    MatchResult, Readiness, RowPainter, SelectionMode, SelectionRequest,
};
use crate::termrock::identity::{Id, ItemKey, Keyed, Part, Revision};
use crate::termrock::layout::{Constraints, Rect, Size};
use crate::termrock::list::{List, ListAction, ListState};
use crate::termrock::response::{ActivationOrigin, Flow, Input, Invalidate, Response, UpdateCause};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::theme::StylePatch;

/// Durable view state for [`FilterList`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilterListState {
    pub query: String,
    pub cursor: usize,
    pub list: ListState,
}

impl FilterListState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        let q = query.into();
        self.cursor = q.chars().count();
        self.query = q;
    }

    pub fn list_state(&self) -> &ListState {
        &self.list
    }

    pub fn list_state_mut(&mut self) -> &mut ListState {
        &mut self.list
    }
}

/// Typed actions emitted by [`FilterList`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterListAction {
    FilterChanged(String),
    QueryChanged(String),
    List(ListAction),
    Activate {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    SelectionRequested(SelectionRequest),
}

/// Filtered keyed list component.
#[allow(clippy::type_complexity)]
pub struct FilterList<'a, T: Keyed> {
    pub id: Id,
    pub rows: &'a [T],
    pub revision: Revision,
    pub query: Option<&'a str>,
    pub matcher: Option<&'a dyn Fn(&T, &str) -> MatchResult>,
    pub row_painter: Option<&'a RowPainter<T>>,
    pub readiness: Option<Readiness<'a>>,
    pub patch: StylePatch,
    pub selection_mode: SelectionMode,
    pub selected: &'a [ItemKey],
}

impl<'a, T: Keyed> FilterList<'a, T> {
    pub fn new(id: Id, rows: &'a [T], revision: Revision) -> Self {
        Self {
            id,
            rows,
            revision,
            query: None,
            matcher: None,
            row_painter: None,
            readiness: None,
            patch: StylePatch::empty(),
            selection_mode: SelectionMode::Single,
            selected: &[],
        }
    }

    pub fn query(mut self, query: &'a str) -> Self {
        self.query = Some(query);
        self
    }

    pub fn filter(mut self, matcher: &'a dyn Fn(&T, &str) -> MatchResult) -> Self {
        self.matcher = Some(matcher);
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

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = patch;
        self
    }

    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    pub fn selected(mut self, selected: &'a [ItemKey]) -> Self {
        self.selected = selected;
        self
    }

    /// Filter source rows according to the query.
    pub fn filter_rows(&self, query: &str) -> Vec<&'a T> {
        if query.is_empty() {
            return self.rows.iter().collect();
        }
        if let Some(matcher) = self.matcher {
            self.rows
                .iter()
                .filter(|item| matcher(item, query).matched)
                .collect()
        } else {
            // If no matcher provided, all rows pass
            self.rows.iter().collect()
        }
    }

    pub fn update(
        &self,
        cx: &mut Cx<'_>,
        state: &mut FilterListState,
    ) -> Response<FilterListAction> {
        let has_focus = cx.has_focus(&self.id);
        let query_id = self.id.sub("query");
        let list_id = self.id.sub("list");

        // Handle query text changes
        let cause = cx.cause().clone();
        match cause {
            UpdateCause::Input(Input::Key(k), _) if has_focus => match k.code {
                KeyCode::Char(c) if k.plain() => {
                    state.query.push(c);
                    state.cursor += 1;
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::action(
                        query_id,
                        FilterListAction::FilterChanged(state.query.clone()),
                    )
                    .with_flow(Flow::Consumed)
                    .with_invalidate(Invalidate::Paint);
                }
                KeyCode::Backspace if !state.query.is_empty() => {
                    state.query.pop();
                    state.cursor = state.cursor.saturating_sub(1);
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::action(
                        query_id,
                        FilterListAction::FilterChanged(state.query.clone()),
                    )
                    .with_flow(Flow::Consumed)
                    .with_invalidate(Invalidate::Paint);
                }
                _ => {}
            },
            _ => {}
        }

        // Delegate list navigation / selection to filtered rows
        let effective_query = self.query.unwrap_or(&state.query);
        let filtered = self.filter_rows(effective_query);

        // Map filtered references to Keyed wrapper
        struct RefRow<'b, T>(&'b T);
        impl<'b, T: Keyed> Keyed for RefRow<'b, T> {
            fn key(&self) -> ItemKey {
                self.0.key()
            }
        }
        let wrapped_rows: Vec<RefRow<'_, T>> = filtered.into_iter().map(RefRow).collect();

        let mut sub_list = List::new(list_id.clone(), &wrapped_rows, self.revision)
            .selection_mode(self.selection_mode)
            .selected(self.selected);

        if let Some(r) = self.readiness {
            sub_list = sub_list.readiness(r);
        }

        let list_resp = sub_list.update(cx, &mut state.list);

        if let Some(act) = list_resp.action {
            let mapped = match act {
                ListAction::Activate { key, origin } => FilterListAction::Activate { key, origin },
                ListAction::SelectionRequested(req) => FilterListAction::SelectionRequested(req),
            };
            Response::action(list_id, mapped)
                .with_flow(list_resp.flow)
                .with_invalidate(list_resp.invalidate)
        } else if list_resp.flow.is_consumed() {
            Response::consumed(self.id.clone()).with_invalidate(list_resp.invalidate)
        } else {
            Response::bubble(self.id.clone())
        }
    }

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &FilterListState) -> Rect {
        ui.register_focus(self.id.clone(), true);
        ui.register_hit(self.id.clone(), area);

        if area.is_empty() {
            return area;
        }

        let query_h = 1.min(area.height);
        let query_area = Rect::new(area.x, area.y, area.width, query_h);
        let list_h = area.height.saturating_sub(query_h);
        let list_area = Rect::new(area.x, area.y + query_h, area.width, list_h);

        // Draw query bar
        let query_id = self.id.sub("query");
        ui.register_hit(query_id.clone(), query_area);
        ui.part(query_id, Part::new("query"), query_area, |_p| {});

        // Draw list portion
        if list_h > 0 {
            let effective_query = self.query.unwrap_or(&state.query);
            let filtered = self.filter_rows(effective_query);

            struct RefRow<'b, T>(&'b T);
            impl<'b, T: Keyed> Keyed for RefRow<'b, T> {
                fn key(&self) -> ItemKey {
                    self.0.key()
                }
            }
            let wrapped_rows: Vec<RefRow<'_, T>> = filtered.into_iter().map(RefRow).collect();

            let list_id = self.id.sub("list");
            let mut sub_list = List::new(list_id, &wrapped_rows, self.revision)
                .selection_mode(self.selection_mode)
                .selected(self.selected);

            if let Some(r) = self.readiness {
                sub_list = sub_list.readiness(r);
            }

            sub_list.draw(ui, list_area, &state.list);
        }

        area
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let needed = Size::new(20, (self.rows.len() as u16).saturating_add(1));
        constraints.clamp(needed)
    }
}
