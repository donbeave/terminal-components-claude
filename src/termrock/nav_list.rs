//! NavList sidebar navigation component with full and compact presentation modes.
//!
//! Provides route/content selection without owning the router, with section headers,
//! icons, badges, and shared scroll behavior.

use ratatui::crossterm::event::KeyCode;

use crate::termrock::collections::{child_item_key, reconcile_cursor};
use crate::termrock::identity::{Id, ItemKey, Keyed, Part, Revision};
use crate::termrock::layout::{Axis, Constraints, Rect, Size};
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::scroll::{ScrollRegion, ScrollState};
use crate::termrock::theme::StylePatch;

/// Presentation mode for [`NavList`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NavMode {
    /// Full sidebar presentation with labels, icons, and badges.
    #[default]
    Full,
    /// Icon-only compact presentation for collapsed sidebars.
    Compact,
}

/// A borrowed navigation item or section heading.
#[derive(Debug, Clone)]
pub struct NavItem<'a> {
    pub key: ItemKey,
    pub label: &'a str,
    pub icon: Option<&'a str>,
    pub badge: Option<&'a str>,
    pub disabled: bool,
    pub section: bool,
}

impl<'a> NavItem<'a> {
    pub fn new(key: ItemKey, label: &'a str) -> Self {
        Self {
            key,
            label,
            icon: None,
            badge: None,
            disabled: false,
            section: false,
        }
    }

    pub fn section(label: &'a str) -> Self {
        Self {
            key: ItemKey::from_str(label),
            label,
            icon: None,
            badge: None,
            disabled: true,
            section: true,
        }
    }

    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn badge(mut self, badge: &'a str) -> Self {
        self.badge = Some(badge);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub const fn is_selectable(&self) -> bool {
        !self.section && !self.disabled
    }
}

impl<'a> Keyed for NavItem<'a> {
    fn key(&self) -> ItemKey {
        self.key
    }
}

/// Durable view state for [`NavList`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NavListState {
    pub cursor: Option<ItemKey>,
    pub scroll: ScrollState,
    pub last_revision: Option<Revision>,
}

impl NavListState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cursor(&self) -> Option<ItemKey> {
        self.cursor
    }

    pub fn scroll(&self) -> &ScrollState {
        &self.scroll
    }

    pub fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }
}

/// Typed actions emitted by [`NavList`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NavAction {
    Navigate {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    EnterContent {
        key: ItemKey,
    },
    Select {
        key: ItemKey,
        origin: ActivationOrigin,
    },
}

/// Sidebar navigation list component.
pub struct NavList<'a> {
    pub id: Id,
    pub items: &'a [NavItem<'a>],
    pub revision: Revision,
    pub active: Option<ItemKey>,
    pub mode: NavMode,
    pub patch: StylePatch,
}

impl<'a> NavList<'a> {
    pub fn new(id: Id, items: &'a [NavItem<'a>], revision: Revision) -> Self {
        Self {
            id,
            items,
            revision,
            active: None,
            mode: NavMode::Full,
            patch: StylePatch::empty(),
        }
    }

    pub fn active(mut self, key: Option<ItemKey>) -> Self {
        self.active = key;
        self
    }

    pub fn mode(mut self, mode: NavMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = patch;
        self
    }

    pub fn reconcile(&self, state: &mut NavListState) {
        if state.last_revision != Some(self.revision) {
            let selectable_keys: Vec<ItemKey> = self
                .items
                .iter()
                .filter(|it| it.is_selectable())
                .map(|it| it.key())
                .collect();
            state.cursor = reconcile_cursor(state.cursor, &selectable_keys);
            state.scroll.total = self.items.len();
            state.scroll.clamp();
            state.last_revision = Some(self.revision);
        }
    }

    pub fn update(&self, cx: &mut Cx<'_>, state: &mut NavListState) -> Response<NavAction> {
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

        let selectable_indices: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, it)| it.is_selectable())
            .map(|(idx, _)| idx)
            .collect();

        let find_cursor_sel_pos = |key_opt: Option<ItemKey>| -> Option<usize> {
            let key = key_opt?;
            let idx = self.items.iter().position(|it| it.key() == key)?;
            selectable_indices.iter().position(|&s_idx| s_idx == idx)
        };

        let cause = cx.cause().clone();

        match cause {
            UpdateCause::Input(Input::Key(k), _) if has_focus => {
                let cur_pos = find_cursor_sel_pos(state.cursor).unwrap_or(0);
                match k.code {
                    KeyCode::Up | KeyCode::Char('k') if k.plain() => {
                        if cur_pos > 0 {
                            let item_idx = selectable_indices[cur_pos - 1];
                            state.cursor = Some(self.items[item_idx].key());
                            state.scroll.ensure_visible(item_idx);
                            changed = true;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') if k.plain() => {
                        if cur_pos + 1 < selectable_indices.len() {
                            let item_idx = selectable_indices[cur_pos + 1];
                            state.cursor = Some(self.items[item_idx].key());
                            state.scroll.ensure_visible(item_idx);
                            changed = true;
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(key) = state.cursor {
                            action = Some(NavAction::Navigate {
                                key,
                                origin: ActivationOrigin::Keyboard,
                            });
                        }
                    }
                    _ => {}
                }
            }
            UpdateCause::Input(Input::Mouse(m), _) => {
                if let Some(target_id) = intended {
                    if let Some(clicked_key) = child_item_key(&self.id, &target_id) {
                        if let Some(_item) = self
                            .items
                            .iter()
                            .find(|it| it.key() == clicked_key && it.is_selectable())
                        {
                            cx.request_focus(self.id.clone());
                            state.cursor = Some(clicked_key);
                            if let Some(idx) =
                                self.items.iter().position(|it| it.key() == clicked_key)
                            {
                                state.scroll.ensure_visible(idx);
                            }
                            changed = true;

                            if matches!(m.kind, MouseKind::Up | MouseKind::Down) {
                                action = Some(NavAction::Navigate {
                                    key: clicked_key,
                                    origin: ActivationOrigin::Pointer,
                                });
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

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &NavListState) -> Rect {
        ui.register_focus(self.id.clone(), true);
        ui.register_hit(self.id.clone(), area);

        if area.is_empty() {
            return area;
        }

        let mut scroll = state.scroll;
        scroll.total = self.items.len();
        scroll.viewport = area.height as usize;
        scroll.clamp();

        let visible = scroll.visible_range();
        let scrollbar_needed = scroll.is_overflowing();
        let row_width = if scrollbar_needed {
            area.width.saturating_sub(1)
        } else {
            area.width
        };

        for i in visible {
            let item = &self.items[i];
            let key = item.key();
            let row_y = area.y + (i - scroll.offset) as u16;
            let row_area = Rect::new(area.x, row_y, row_width, 1);
            let row_id = self.id.child(key);

            if item.section {
                // Section heading: presentation only, not selectable
                ui.part(row_id, Part::new("section"), row_area, |_p| {});
            } else {
                ui.register_hit(row_id.clone(), row_area);

                let is_active = self.active == Some(key);
                let is_cursor = state.cursor == Some(key);

                let _visual = VisualState::empty()
                    .selected(is_active)
                    .focused(is_cursor)
                    .disabled(item.disabled);

                ui.part(row_id, Part::new("row"), row_area, |_p| {});
            }
        }

        if scrollbar_needed {
            let scroll_region = ScrollRegion::new(
                self.id.clone(),
                Axis::Vertical,
                self.items.len(),
                area.height as usize,
            );
            scroll_region.draw(ui, area, &scroll);
        }

        area
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let width = match self.mode {
            NavMode::Full => 24,
            NavMode::Compact => 6,
        };
        let h = self.items.len() as u16;
        constraints.clamp(Size::new(width, h))
    }
}
