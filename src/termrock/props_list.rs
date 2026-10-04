//! Termrock interactive PropsList component, CopyPolicy, PropsAction, and keyed navigation.

use ratatui::crossterm::event::KeyCode;
use ratatui::style::Style;

use crate::termrock::identity::{Id, ItemKey, Part, Revision};
use crate::termrock::layout::{Constraints, Rect, Size};
use crate::termrock::props::{LabelWidth, PropsRow};
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::text::width;
use crate::termrock::theme::{Role, StylePatch};

/// Policy controlling whether and what property values may be copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CopyPolicy {
    AllowSafe,
    #[default]
    DenyProtected,
    DenyAll,
}

/// Semantic actions emitted by `PropsList`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropsAction {
    CopyRequested {
        key: ItemKey,
        value: String,
    },
    Activate {
        key: ItemKey,
        origin: ActivationOrigin,
    },
}

/// Caller-owned durable state for `PropsList`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PropsState {
    pub cursor: usize,
    pub scroll_offset: usize,
    pub last_revision: Revision,
    pub selected_key: Option<ItemKey>,
}

impl PropsState {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            scroll_offset: 0,
            last_revision: Revision::zero(),
            selected_key: None,
        }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn selected_key(&self) -> Option<ItemKey> {
        self.selected_key
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    pub fn reconcile(&mut self, rows: &[PropsRow<'_>], revision: Revision) {
        if self.last_revision != revision {
            self.last_revision = revision;
            if let Some(target_key) = self.selected_key {
                if let Some(idx) = rows.iter().position(|r| r.key == target_key) {
                    self.cursor = idx;
                } else if !rows.is_empty() {
                    self.cursor = self.cursor.min(rows.len() - 1);
                    self.selected_key = rows.get(self.cursor).map(|r| r.key);
                } else {
                    self.cursor = 0;
                    self.selected_key = None;
                }
            } else if !rows.is_empty() {
                self.cursor = self.cursor.min(rows.len() - 1);
                self.selected_key = rows.get(self.cursor).map(|r| r.key);
            }
        }
    }

    pub fn ensure_visible(&mut self, viewport_h: usize) {
        if viewport_h == 0 {
            return;
        }
        if self.cursor < self.scroll_offset {
            self.scroll_offset = self.cursor;
        } else if self.cursor >= self.scroll_offset + viewport_h {
            self.scroll_offset = self.cursor - viewport_h + 1;
        }
    }
}

/// Interactive property sheet with keyboard navigation and safe copy policies.
#[derive(Debug, Clone)]
pub struct PropsList<'a> {
    pub id: Id,
    pub rows: &'a [PropsRow<'a>],
    pub revision: Revision,
    pub label_width_policy: LabelWidth,
    pub copy_policy: CopyPolicy,
    pub patch: Option<StylePatch>,
}

impl<'a> PropsList<'a> {
    pub fn new(id: Id, rows: &'a [PropsRow<'a>], revision: Revision) -> Self {
        Self {
            id,
            rows,
            revision,
            label_width_policy: LabelWidth::Auto,
            copy_policy: CopyPolicy::DenyProtected,
            patch: None,
        }
    }

    pub fn label_width(mut self, width: LabelWidth) -> Self {
        self.label_width_policy = width;
        self
    }

    pub fn copy_policy(mut self, policy: CopyPolicy) -> Self {
        self.copy_policy = policy;
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = Some(patch);
        self
    }

    pub fn row_id(&self, key: ItemKey) -> Id {
        self.id.child(key)
    }

    pub fn update(&self, cx: &mut Cx<'_>, state: &mut PropsState) -> Response<PropsAction> {
        state.reconcile(self.rows, self.revision);
        let has_focus = cx.has_focus(&self.id);

        if self.rows.is_empty() {
            return Response::bubble(self.id.clone())
                .with_state(VisualState::empty().focused(has_focus));
        }

        let cause = cx.cause().clone();
        match cause {
            UpdateCause::Input(Input::Key(k), _) if has_focus => match k.code {
                KeyCode::Up | KeyCode::Char('k') if !k.ctrl() => {
                    state.cursor = state.cursor.saturating_sub(1);
                    state.selected_key = self.rows.get(state.cursor).map(|r| r.key);
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
                }
                KeyCode::Down | KeyCode::Char('j') if !k.ctrl() => {
                    state.cursor = (state.cursor + 1).min(self.rows.len() - 1);
                    state.selected_key = self.rows.get(state.cursor).map(|r| r.key);
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
                }
                KeyCode::Home | KeyCode::Char('g') if !k.ctrl() => {
                    state.cursor = 0;
                    state.selected_key = self.rows.get(state.cursor).map(|r| r.key);
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
                }
                KeyCode::End | KeyCode::Char('G') => {
                    state.cursor = self.rows.len() - 1;
                    state.selected_key = self.rows.get(state.cursor).map(|r| r.key);
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
                }
                KeyCode::PageUp => {
                    state.cursor = state.cursor.saturating_sub(5);
                    state.selected_key = self.rows.get(state.cursor).map(|r| r.key);
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
                }
                KeyCode::PageDown => {
                    state.cursor = (state.cursor + 5).min(self.rows.len() - 1);
                    state.selected_key = self.rows.get(state.cursor).map(|r| r.key);
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
                }
                KeyCode::Enter => {
                    if state.cursor < self.rows.len() {
                        let row = &self.rows[state.cursor];
                        cx.request_invalidate(Invalidate::Paint);
                        return Response::action(
                            self.id.clone(),
                            PropsAction::Activate {
                                key: row.key,
                                origin: ActivationOrigin::Keyboard,
                            },
                        )
                        .with_flow(Flow::Consumed)
                        .with_invalidate(Invalidate::Paint);
                    }
                }
                KeyCode::Char('y') if !k.ctrl() && state.cursor < self.rows.len() => {
                    let row = &self.rows[state.cursor];
                    if !row.copyable {
                        return Response::consumed(self.id.clone());
                    }
                    match self.copy_policy {
                        CopyPolicy::DenyAll => {
                            return Response::consumed(self.id.clone());
                        }
                        CopyPolicy::DenyProtected | CopyPolicy::AllowSafe => {
                            if row.value.is_protected() {
                                return Response::consumed(self.id.clone());
                            }
                        }
                    }
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::action(
                        self.id.clone(),
                        PropsAction::CopyRequested {
                            key: row.key,
                            value: row.value.as_str().to_string(),
                        },
                    )
                    .with_flow(Flow::Consumed)
                    .with_invalidate(Invalidate::Paint);
                }
                _ => {}
            },
            _ => {}
        }

        if let UpdateCause::Input(Input::Mouse(m), _) = cx.cause() {
            if matches!(m.kind, MouseKind::Up) {
                if let Some(target) = cx.intended_owner() {
                    for (i, row) in self.rows.iter().enumerate() {
                        if *target == self.row_id(row.key) {
                            state.cursor = i;
                            state.selected_key = Some(row.key);
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(
                                self.id.clone(),
                                PropsAction::Activate {
                                    key: row.key,
                                    origin: ActivationOrigin::Pointer,
                                },
                            )
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint);
                        }
                    }
                }
            } else if matches!(m.kind, MouseKind::WheelUp) {
                state.scroll_offset = state.scroll_offset.saturating_sub(1);
                cx.request_invalidate(Invalidate::Paint);
                return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
            } else if matches!(m.kind, MouseKind::WheelDown) {
                if state.scroll_offset + 1 < self.rows.len() {
                    state.scroll_offset += 1;
                }
                cx.request_invalidate(Invalidate::Paint);
                return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
            }
        }

        Response::bubble(self.id.clone()).with_state(VisualState::empty().focused(has_focus))
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let _label_w = self.label_width_policy.resolve(self.rows);
        let needed = Size::new(
            constraints.max.width,
            (self.rows.len() as u16).min(constraints.max.height),
        );
        constraints.clamp(needed)
    }

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &PropsState) -> Rect {
        if area.is_empty() {
            return Rect::zero();
        }

        ui.register_focus(self.id.clone(), true);

        let focused = ui.is_focused(&self.id);
        let theme = ui.theme;
        let bg_color = theme.resolve_role(Role::Background, ui.current_surface);
        let has_overflow = self.rows.len() > area.height as usize;
        let row_w = area.width.saturating_sub(u16::from(has_overflow));
        let label_w = self.label_width_policy.resolve(self.rows);

        let visible_h = area.height as usize;
        let start_idx = state.scroll_offset.min(self.rows.len());
        let end_idx = (start_idx + visible_h).min(self.rows.len());

        for (k, i) in (start_idx..end_idx).enumerate() {
            let row_idx = i;
            let row = &self.rows[row_idx];
            let rid = self.row_id(row.key);
            let y = area.y + k as u16;
            let row_rect = Rect::new(area.x, y, row_w, 1);
            ui.register_hit(rid.clone(), row_rect);

            let is_cursor = focused && row_idx == state.cursor;
            let is_hovered = ui.is_hovered(&rid);

            let mut st = Style::new().fg(theme.tokens.text_primary).bg(bg_color);
            if is_hovered {
                st = st.bg(theme.tokens.surface_elevated);
            }
            if is_cursor {
                st = st.bg(theme.tokens.surface_overlay);
            }

            ui.fill_rect(row_rect, st);
            ui.part(rid.clone(), Part::new("row"), row_rect, |_| {});

            // Focus gutter
            let gutter_sym = if is_cursor { "▎" } else { " " };
            ui.set_string(
                row_rect.x,
                y,
                gutter_sym,
                Style::new()
                    .fg(theme.tokens.focus)
                    .bg(st.bg.unwrap_or(bg_color)),
            );
            ui.part(
                rid.clone(),
                Part::new("gutter"),
                Rect::new(row_rect.x, y, 1, 1),
                |_| {},
            );

            // Label
            let label_x = row_rect.x.saturating_add(2);
            let label_style = Style::new()
                .fg(theme.tokens.text_muted)
                .bg(st.bg.unwrap_or(bg_color));
            ui.set_string(label_x, y, row.label, label_style);
            ui.part(
                rid.clone(),
                Part::LABEL,
                Rect::new(label_x, y, width(row.label) as u16, 1),
                |_| {},
            );

            // Value
            let val_x = row_rect.x.saturating_add(2).saturating_add(label_w);
            let hint_w = if row.copyable && is_cursor { 8 } else { 0 };
            let val_avail =
                (row_rect.x.saturating_add(row_rect.width)).saturating_sub(val_x + hint_w) as usize;
            let val_text = crate::termrock::text::truncate(row.value.as_str(), val_avail);
            let val_color = theme.tone_color(row.tone);
            let val_style = Style::new().fg(val_color).bg(st.bg.unwrap_or(bg_color));
            ui.set_string(val_x, y, &val_text, val_style);
            ui.part(
                rid.clone(),
                Part::new("value"),
                Rect::new(val_x, y, width(&val_text) as u16, 1),
                |_| {},
            );

            // Hint
            if hint_w > 0 {
                let hint_x = row_rect.x.saturating_add(row_rect.width).saturating_sub(7);
                ui.set_string(
                    hint_x,
                    y,
                    "y copy",
                    Style::new()
                        .fg(theme.tokens.text_faint)
                        .bg(st.bg.unwrap_or(bg_color)),
                );
            }
        }

        // Scrollbar track
        if has_overflow {
            let sb_x = area.x.saturating_add(area.width).saturating_sub(1);
            let sb_rect = Rect::new(sb_x, area.y, 1, area.height);
            ui.set_string(
                sb_x,
                area.y,
                "│",
                Style::new().fg(theme.tokens.border_subtle).bg(bg_color),
            );
            ui.part(self.id.clone(), Part::new("scrollbar"), sb_rect, |_| {});
        }

        area
    }
}
