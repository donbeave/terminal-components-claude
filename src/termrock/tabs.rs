//! Horizontal Tabs component with keyed navigation, close affordances, and overflow indicators.
//!
//! Provides tab strip presentation and typed activation, close, and reorder requests
//! without owning hosted document or pane content.

use ratatui::crossterm::event::KeyCode;

use crate::termrock::collections::{child_item_key, reconcile_cursor};
use crate::termrock::identity::{Id, ItemKey, Keyed, Part, Revision};
use crate::termrock::layout::{Constraints, Rect, Size};
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::theme::StylePatch;

/// A borrowed tab descriptor in [`Tabs`].
#[derive(Debug, Clone)]
pub struct TabItem<'a> {
    pub key: ItemKey,
    pub label: &'a str,
    pub closable: bool,
    pub disabled: bool,
    pub prefix: Option<&'a str>,
    pub suffix: Option<&'a str>,
    pub dirty: bool,
    pub busy: bool,
    pub error: bool,
}

impl<'a> TabItem<'a> {
    pub fn new(key: ItemKey, label: &'a str) -> Self {
        Self {
            key,
            label,
            closable: false,
            disabled: false,
            prefix: None,
            suffix: None,
            dirty: false,
            busy: false,
            error: false,
        }
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn prefix(mut self, prefix: Option<&'a str>) -> Self {
        self.prefix = prefix;
        self
    }

    pub fn suffix(mut self, suffix: Option<&'a str>) -> Self {
        self.suffix = suffix;
        self
    }

    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }
}

impl<'a> Keyed for TabItem<'a> {
    fn key(&self) -> ItemKey {
        self.key
    }
}

/// Durable view state for [`Tabs`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TabsState {
    pub active: Option<ItemKey>,
    pub cursor: Option<ItemKey>,
    pub first_visible: usize,
    pub last_revision: Option<Revision>,
}

impl TabsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_active(mut self, active: ItemKey) -> Self {
        self.active = Some(active);
        self.cursor = Some(active);
        self
    }

    pub fn active(&self) -> Option<ItemKey> {
        self.active
    }

    pub fn cursor(&self) -> Option<ItemKey> {
        self.cursor
    }

    pub fn first_visible(&self) -> usize {
        self.first_visible
    }
}

/// Typed actions emitted by [`Tabs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabsAction {
    Select {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    Activate {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    Close {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    Reorder {
        key: ItemKey,
        before: Option<ItemKey>,
    },
}

/// Keyed tab strip component.
pub struct Tabs<'a> {
    pub id: Id,
    pub tabs: &'a [TabItem<'a>],
    pub revision: Revision,
    pub active: Option<ItemKey>,
    pub closable: bool,
    pub reorderable: bool,
    pub patch: StylePatch,
}

impl<'a> Tabs<'a> {
    pub fn new(id: Id, tabs: &'a [TabItem<'a>], revision: Revision) -> Self {
        Self {
            id,
            tabs,
            revision,
            active: None,
            closable: false,
            reorderable: false,
            patch: StylePatch::empty(),
        }
    }

    pub fn active(mut self, key: Option<ItemKey>) -> Self {
        self.active = key;
        self
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    pub fn reorderable(mut self, reorderable: bool) -> Self {
        self.reorderable = reorderable;
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = patch;
        self
    }

    pub fn reconcile(&self, state: &mut TabsState) {
        if state.last_revision != Some(self.revision) {
            let tab_keys: Vec<ItemKey> = self.tabs.iter().map(|t| t.key()).collect();
            state.active = reconcile_cursor(state.active, &tab_keys);
            state.cursor = reconcile_cursor(state.cursor, &tab_keys);
            if state.first_visible >= self.tabs.len() && !self.tabs.is_empty() {
                state.first_visible = self.tabs.len().saturating_sub(1);
            }
            state.last_revision = Some(self.revision);
        }
    }

    pub fn update(&self, cx: &mut Cx<'_>, state: &mut TabsState) -> Response<TabsAction> {
        self.reconcile(state);

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

        let cur_idx = state
            .cursor
            .and_then(|k| self.tabs.iter().position(|t| t.key() == k))
            .unwrap_or(0);

        let cause = cx.cause().clone();

        match cause {
            UpdateCause::Input(Input::Key(k), _) if has_focus => match k.code {
                KeyCode::Left | KeyCode::Char('h') if k.plain() => {
                    if cur_idx > 0 {
                        let next_idx = cur_idx - 1;
                        state.cursor = Some(self.tabs[next_idx].key());
                        if next_idx < state.first_visible {
                            state.first_visible = next_idx;
                        }
                        changed = true;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') if k.plain() => {
                    if cur_idx + 1 < self.tabs.len() {
                        let next_idx = cur_idx + 1;
                        state.cursor = Some(self.tabs[next_idx].key());
                        changed = true;
                    }
                }
                KeyCode::Char(c) if ('1'..='9').contains(&c) && k.plain() => {
                    let idx = (c as usize) - ('1' as usize);
                    if let Some(tab) = self.tabs.get(idx).filter(|t| !t.disabled) {
                        state.cursor = Some(tab.key());
                        state.active = Some(tab.key());
                        action = Some(TabsAction::Select {
                            key: tab.key(),
                            origin: ActivationOrigin::Keyboard,
                        });
                        changed = true;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') if k.plain() => {
                    if let Some(tab) = self.tabs.get(cur_idx).filter(|t| !t.disabled) {
                        state.active = Some(tab.key());
                        action = Some(TabsAction::Select {
                            key: tab.key(),
                            origin: ActivationOrigin::Keyboard,
                        });
                        changed = true;
                    }
                }
                KeyCode::Char('x') if k.plain() => {
                    if let Some(tab) = self
                        .tabs
                        .get(cur_idx)
                        .filter(|t| (t.closable || self.closable) && !t.disabled)
                    {
                        action = Some(TabsAction::Close {
                            key: tab.key(),
                            origin: ActivationOrigin::Keyboard,
                        });
                    }
                }
                KeyCode::Delete => {
                    if let Some(tab) = self
                        .tabs
                        .get(cur_idx)
                        .filter(|t| (t.closable || self.closable) && !t.disabled)
                    {
                        action = Some(TabsAction::Close {
                            key: tab.key(),
                            origin: ActivationOrigin::Keyboard,
                        });
                    }
                }
                _ => {}
            },
            UpdateCause::Input(Input::Mouse(m), _) => {
                if let Some(target_id) = intended {
                    if let Some(clicked_key) = child_item_key(&self.id, &target_id) {
                        let is_close_btn = target_id.as_str().ends_with("/s:5:close");

                        if matches!(m.kind, MouseKind::Up | MouseKind::Down) {
                            if is_close_btn {
                                if let Some(_tab) = self.tabs.iter().find(|t| {
                                    t.key() == clicked_key
                                        && (t.closable || self.closable)
                                        && !t.disabled
                                }) {
                                    action = Some(TabsAction::Close {
                                        key: clicked_key,
                                        origin: ActivationOrigin::Pointer,
                                    });
                                }
                            } else if let Some(_tab) = self
                                .tabs
                                .iter()
                                .find(|t| t.key() == clicked_key && !t.disabled)
                            {
                                cx.request_focus(self.id.clone());
                                state.cursor = Some(clicked_key);
                                state.active = Some(clicked_key);
                                action = Some(TabsAction::Select {
                                    key: clicked_key,
                                    origin: ActivationOrigin::Pointer,
                                });
                                changed = true;
                            }
                        }
                    } else if is_target {
                        match m.kind {
                            MouseKind::WheelLeft | MouseKind::WheelUp
                                if state.first_visible > 0 =>
                            {
                                state.first_visible -= 1;
                                changed = true;
                            }
                            MouseKind::WheelRight | MouseKind::WheelDown
                                if state.first_visible + 1 < self.tabs.len() =>
                            {
                                state.first_visible += 1;
                                changed = true;
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

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &TabsState) -> Rect {
        ui.register_focus(self.id.clone(), true);
        ui.register_hit(self.id.clone(), area);

        if area.is_empty() {
            return area;
        }

        let mut current_x = area.x;
        let effective_active = self.active.or(state.active);

        // Optional left overflow indicator
        if state.first_visible > 0 {
            let left_ov_id = self.id.sub("overflow_left");
            let left_ov_area = Rect::new(current_x, area.y, 1, 1);
            ui.register_hit(left_ov_id.clone(), left_ov_area);
            ui.part(left_ov_id, Part::new("overflow"), left_ov_area, |_p| {});
            current_x = current_x.saturating_add(2);
        }

        for tab in self.tabs.iter().skip(state.first_visible) {
            let key = tab.key();
            // Calculate width: label + padding (2) + prefix (2) + suffix (2) + close (2)
            let mut tab_width = tab.label.len() as u16 + 2;
            if tab.prefix.is_some() {
                tab_width += 2;
            }
            if tab.suffix.is_some() {
                tab_width += 2;
            }
            if tab.closable || self.closable {
                tab_width += 2;
            }

            if current_x + tab_width > area.x + area.width {
                // Right overflow indicator
                let right_ov_id = self.id.sub("overflow_right");
                let right_ov_area = Rect::new(area.x + area.width.saturating_sub(1), area.y, 1, 1);
                ui.register_hit(right_ov_id.clone(), right_ov_area);
                ui.part(right_ov_id, Part::new("overflow"), right_ov_area, |_p| {});
                break;
            }

            let tab_area = Rect::new(current_x, area.y, tab_width, 1);
            let tab_id = self.id.child(key);
            ui.register_hit(tab_id.clone(), tab_area);

            let is_active = effective_active == Some(key);
            let is_cursor = state.cursor == Some(key);

            let _visual = VisualState::empty()
                .selected(is_active)
                .focused(is_cursor)
                .disabled(tab.disabled)
                .busy(tab.busy);

            ui.part(tab_id.clone(), Part::new("tab"), tab_area, |_p| {});

            // If closable, register close button hit region (last 2 cells)
            if (tab.closable || self.closable) && !tab.disabled {
                let close_x = tab_area.x + tab_area.width.saturating_sub(2);
                let close_area = Rect::new(close_x, area.y, 2, 1);
                let close_id = tab_id.sub("close");
                ui.register_hit(close_id.clone(), close_area);
                ui.part(close_id, Part::new("close"), close_area, |_p| {});
            }

            // Draw underline if area height >= 2 and tab is active
            if area.height >= 2 && is_active {
                let underline_area = Rect::new(current_x, area.y + 1, tab_width, 1);
                ui.part(
                    tab_id.sub("underline"),
                    Part::new("underline"),
                    underline_area,
                    |_p| {},
                );
            }

            current_x = current_x.saturating_add(tab_width + 1);
        }

        area
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let total_w: u16 = self.tabs.iter().map(|t| (t.label.len() as u16) + 4).sum();
        constraints.clamp(Size::new(total_w, 2))
    }
}
