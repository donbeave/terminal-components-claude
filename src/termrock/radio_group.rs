//! Controlled RadioGroup component with keyed mutually exclusive choices.
//!
//! Caller owns options and controlled selected key; RadioGroup owns candidate cursor
//! and scroll state. Arrow navigation changes cursor only; Enter/Space/Click commits choice.

use std::time::Duration;

use ratatui::crossterm::event::KeyCode;
use ratatui::style::Style;

use crate::termrock::collections::{child_item_key, reconcile_cursor};
use crate::termrock::identity::{Id, ItemKey, Keyed, Part, Revision};
use crate::termrock::layout::{Axis, Constraints, Position, Rect, Size};
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::scroll::ScrollState;
use crate::termrock::text::width;
use crate::termrock::theme::{Role, StylePatch};

/// An option record in a choice control or select dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceItem<'a> {
    pub key: ItemKey,
    pub label: &'a str,
    pub disabled: bool,
}

impl<'a> ChoiceItem<'a> {
    /// Creates a new `ChoiceItem` with a key and display label.
    pub fn new(key: impl Into<ItemKey>, label: &'a str) -> Self {
        Self {
            key: key.into(),
            label,
            disabled: false,
        }
    }

    /// Sets whether this specific choice is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a> Keyed for ChoiceItem<'a> {
    fn key(&self) -> ItemKey {
        self.key
    }
}

/// Durable view state for [`RadioGroup`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RadioGroupState {
    pub cursor: Option<ItemKey>,
    pub scroll: ScrollState,
    pub last_revision: Option<Revision>,
}

impl RadioGroupState {
    /// Creates a new `RadioGroupState`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the initial cursor key.
    pub fn with_cursor(mut self, cursor: ItemKey) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Returns the current candidate cursor key.
    pub fn cursor(&self) -> Option<ItemKey> {
        self.cursor
    }

    /// Returns a reference to the scroll state.
    pub fn scroll(&self) -> &ScrollState {
        &self.scroll
    }

    /// Returns a mutable reference to the scroll state.
    pub fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }

    /// Returns the last revision reconciled against.
    pub fn last_revision(&self) -> Option<Revision> {
        self.last_revision
    }
}

/// Typed action emitted by [`RadioGroup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RadioAction {
    /// An option was explicitly chosen via Enter, Space, or completed pointer click.
    Choose {
        key: ItemKey,
        origin: ActivationOrigin,
    },
}

/// Controlled mutually exclusive choice group.
#[derive(Debug, Clone)]
pub struct RadioGroup<'a> {
    pub id: Id,
    pub label: Option<&'a str>,
    pub options: &'a [ChoiceItem<'a>],
    pub revision: Revision,
    pub selected: Option<ItemKey>,
    pub orientation: Axis,
    pub disabled: bool,
    pub patch: Option<StylePatch>,
}

impl<'a> RadioGroup<'a> {
    /// Creates a new `RadioGroup`.
    pub fn new(id: Id, options: &'a [ChoiceItem<'a>], revision: Revision) -> Self {
        Self {
            id,
            label: None,
            options,
            revision,
            selected: None,
            orientation: Axis::Vertical,
            disabled: false,
            patch: None,
        }
    }

    /// Sets an optional group label displayed above options.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the controlled selected item key.
    pub fn selected(mut self, selected: Option<ItemKey>) -> Self {
        self.selected = selected;
        self
    }

    /// Sets the layout orientation of the options (vertical or horizontal).
    pub fn orientation(mut self, axis: Axis) -> Self {
        self.orientation = axis;
        self
    }

    /// Sets whether the entire radio group is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Applies a style patch to the group.
    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = Some(patch);
        self
    }

    /// Returns the stable child identity for an individual option.
    pub fn option_id(&self, key: ItemKey) -> Id {
        self.id.child(key)
    }

    /// Updates the radio group, moving candidate cursor on arrow keys and emitting
    /// `RadioAction::Choose` on explicit Enter, Space, or pointer click.
    pub fn update(&self, cx: &mut Cx<'_>, state: &mut RadioGroupState) -> Response<RadioAction> {
        let all_keys: Vec<ItemKey> = self.options.iter().map(|o| o.key).collect();
        let enabled_keys: Vec<ItemKey> = self
            .options
            .iter()
            .filter(|o| !o.disabled)
            .map(|o| o.key)
            .collect();

        // Deterministically reconcile candidate cursor
        if state.cursor.is_none() || !all_keys.contains(&state.cursor.unwrap()) {
            if let Some(sel) = self.selected.filter(|s| all_keys.contains(s)) {
                state.cursor = Some(sel);
            } else {
                state.cursor = reconcile_cursor(state.cursor, &all_keys);
            }
        }
        state.last_revision = Some(self.revision);

        let has_focus = cx.has_focus(&self.id);

        if self.disabled || self.options.is_empty() || enabled_keys.is_empty() {
            if let Some(cap) = &cx.pointer_capture
                && (cap == &self.id || child_item_key(&self.id, cap).is_some())
            {
                cx.release_capture();
            }
            let vs = VisualState::empty()
                .focused(has_focus)
                .disabled(self.disabled);

            match cx.cause() {
                UpdateCause::Input(Input::Key(k), _) if has_focus => {
                    if k.code == KeyCode::Enter || k.code == KeyCode::Char(' ') {
                        return Response::consumed(self.id.clone()).with_state(vs);
                    }
                }
                UpdateCause::Input(Input::Mouse(_), _) => {
                    if let Some(owner) = cx.intended_owner()
                        && (owner == &self.id || child_item_key(&self.id, owner).is_some())
                    {
                        return Response::consumed(self.id.clone()).with_state(vs);
                    }
                }
                _ => {}
            }
            return Response::bubble(self.id.clone()).with_state(vs);
        }

        match cx.cause() {
            UpdateCause::Input(Input::Key(k), _) if has_focus => {
                let is_up = matches!(
                    (self.orientation, k.code),
                    (Axis::Vertical, KeyCode::Up | KeyCode::Char('k'))
                        | (Axis::Horizontal, KeyCode::Left | KeyCode::Char('h'))
                );
                let is_down = matches!(
                    (self.orientation, k.code),
                    (Axis::Vertical, KeyCode::Down | KeyCode::Char('j'))
                        | (Axis::Horizontal, KeyCode::Right | KeyCode::Char('l'))
                );

                if is_up {
                    // Move cursor to previous enabled option
                    if let Some(current_key) = state.cursor {
                        if let Some(curr_idx) = enabled_keys.iter().position(|&k| k == current_key)
                        {
                            let prev_idx = curr_idx.saturating_sub(1);
                            state.cursor = Some(enabled_keys[prev_idx]);
                        } else {
                            state.cursor = enabled_keys.first().copied();
                        }
                    } else {
                        state.cursor = enabled_keys.first().copied();
                    }
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone())
                        .with_invalidate(Invalidate::Paint)
                        .with_state(VisualState::empty().focused(true));
                }

                if is_down {
                    // Move cursor to next enabled option
                    if let Some(current_key) = state.cursor {
                        if let Some(curr_idx) = enabled_keys.iter().position(|&k| k == current_key)
                        {
                            let next_idx = (curr_idx + 1).min(enabled_keys.len().saturating_sub(1));
                            state.cursor = Some(enabled_keys[next_idx]);
                        } else {
                            state.cursor = enabled_keys.first().copied();
                        }
                    } else {
                        state.cursor = enabled_keys.first().copied();
                    }
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone())
                        .with_invalidate(Invalidate::Paint)
                        .with_state(VisualState::empty().focused(true));
                }

                if (k.code == KeyCode::Enter || k.code == KeyCode::Char(' '))
                    && let Some(key) = state.cursor
                    && enabled_keys.contains(&key)
                {
                    cx.trigger_feedback(self.id.clone(), Duration::from_millis(140));
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::action(
                        self.id.clone(),
                        RadioAction::Choose {
                            key,
                            origin: ActivationOrigin::Keyboard,
                        },
                    )
                    .with_flow(Flow::Consumed)
                    .with_invalidate(Invalidate::Paint)
                    .with_state(VisualState::empty().focused(true).activation_feedback(true));
                }
            }
            UpdateCause::Input(Input::Mouse(m), _) => {
                let pos = Position::new(m.pos.x, m.pos.y);
                match m.kind {
                    MouseKind::Down => {
                        for opt in self.options {
                            let opt_id = self.option_id(opt.key);
                            if cx.intended_owner() == Some(&opt_id)
                                || cx.contains_point(&opt_id, pos)
                            {
                                cx.capture_pointer(opt_id.clone());
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::consumed(self.id.clone())
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(has_focus));
                            }
                        }
                    }
                    MouseKind::Up => {
                        for opt in self.options {
                            let opt_id = self.option_id(opt.key);
                            if cx.pointer_capture.as_ref() == Some(&opt_id) {
                                cx.release_capture();
                                cx.request_invalidate(Invalidate::Paint);
                                if cx.contains_point(&opt_id, pos) && !opt.disabled {
                                    state.cursor = Some(opt.key);
                                    cx.trigger_feedback(
                                        self.id.clone(),
                                        Duration::from_millis(140),
                                    );
                                    return Response::action(
                                        self.id.clone(),
                                        RadioAction::Choose {
                                            key: opt.key,
                                            origin: ActivationOrigin::Pointer,
                                        },
                                    )
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(
                                        VisualState::empty()
                                            .focused(has_focus)
                                            .activation_feedback(true),
                                    );
                                } else {
                                    return Response::consumed(self.id.clone())
                                        .with_invalidate(Invalidate::Paint)
                                        .with_state(VisualState::empty().focused(has_focus));
                                }
                            } else if (cx.intended_owner() == Some(&opt_id)
                                || cx.contains_point(&opt_id, pos))
                                && cx.contains_point(&opt_id, pos)
                                && !opt.disabled
                            {
                                state.cursor = Some(opt.key);
                                cx.trigger_feedback(self.id.clone(), Duration::from_millis(140));
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::action(
                                    self.id.clone(),
                                    RadioAction::Choose {
                                        key: opt.key,
                                        origin: ActivationOrigin::Pointer,
                                    },
                                )
                                .with_flow(Flow::Consumed)
                                .with_invalidate(Invalidate::Paint)
                                .with_state(
                                    VisualState::empty()
                                        .focused(has_focus)
                                        .activation_feedback(true),
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        let vs = VisualState::empty().focused(has_focus);
        Response::bubble(self.id.clone()).with_state(vs)
    }

    /// Measures the dimensions of the radio group.
    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let label_rows = if self.label.is_some() { 1 } else { 0 };
        match self.orientation {
            Axis::Vertical => {
                let height = (self.options.len() as u16).saturating_add(label_rows);
                let mut max_w = self.label.map(|l| width(l) as u16 + 2).unwrap_or(0);
                for opt in self.options {
                    let opt_w = (width(opt.label) as u16).saturating_add(6);
                    if opt_w > max_w {
                        max_w = opt_w;
                    }
                }
                constraints.clamp(Size::new(max_w, height))
            }
            Axis::Horizontal => {
                let height = 1u16.saturating_add(label_rows);
                let mut total_w = 0u16;
                for opt in self.options {
                    total_w = total_w.saturating_add((width(opt.label) as u16).saturating_add(7));
                }
                if let Some(l) = self.label {
                    let lw = (width(l) as u16).saturating_add(2);
                    if lw > total_w {
                        total_w = lw;
                    }
                }
                constraints.clamp(Size::new(total_w, height))
            }
        }
    }

    /// Draws the radio group inside the allocated area.
    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &RadioGroupState) -> Rect {
        let clipped = ui.clip_area().intersect(area);
        if clipped.is_empty() {
            return Rect::zero();
        }

        let is_enabled = !self.disabled && !self.options.is_empty();
        ui.register_focus(self.id.clone(), is_enabled);

        let focused = ui.is_focused(&self.id);
        let theme = ui.theme;
        let bg_color = theme.resolve_role(Role::Background, ui.current_surface);

        let mut curr_y = clipped.y;
        let bottom = clipped.y.saturating_add(clipped.height);

        // Optional group label
        if let Some(lbl) = self.label
            && curr_y < bottom
        {
            let lbl_style = if self.disabled {
                Style::new().fg(theme.tokens.disabled).bg(bg_color)
            } else if focused {
                Style::new().fg(theme.tokens.accent).bg(bg_color)
            } else {
                Style::new().fg(theme.tokens.text_secondary).bg(bg_color)
            };
            let avail_w = clipped.width.saturating_sub(2) as usize;
            let truncated = crate::termrock::text::truncate(lbl, avail_w);
            ui.set_string(clipped.x.saturating_add(2), curr_y, &truncated, lbl_style);
            ui.part(
                self.id.clone(),
                Part::new("label"),
                Rect::new(clipped.x, curr_y, clipped.width, 1),
                |_| {},
            );
            curr_y = curr_y.saturating_add(1);
        }

        let mut curr_x = clipped.x;
        let right = clipped.x.saturating_add(clipped.width);

        for opt in self.options {
            let opt_id = self.option_id(opt.key);
            let opt_disabled = self.disabled || opt.disabled;

            let is_selected = self.selected == Some(opt.key);
            let is_cursor = state.cursor == Some(opt.key);
            let is_focused_row = focused && is_cursor && !opt_disabled;
            let is_hovered = ui.is_hovered(&opt_id) && !opt_disabled;

            match self.orientation {
                Axis::Vertical => {
                    if curr_y >= bottom {
                        break;
                    }
                    let row_rect = Rect::new(clipped.x, curr_y, clipped.width, 1);
                    if !opt_disabled {
                        ui.register_hit(opt_id.clone(), row_rect);
                    }

                    let row_style = if opt_disabled {
                        Style::new().fg(theme.tokens.disabled).bg(bg_color)
                    } else if is_hovered {
                        Style::new()
                            .fg(theme.tokens.text_primary)
                            .bg(theme.tokens.surface_overlay)
                    } else {
                        Style::new().fg(theme.tokens.text_primary).bg(bg_color)
                    };

                    ui.fill_rect(row_rect, row_style);
                    ui.part(opt_id.clone(), Part::new("container"), row_rect, |_| {});

                    // Gutter
                    let gutter_fg = if is_focused_row {
                        theme.tokens.focus
                    } else {
                        row_style.bg.unwrap_or(bg_color)
                    };
                    let gutter_style = Style::new()
                        .fg(gutter_fg)
                        .bg(row_style.bg.unwrap_or(bg_color));
                    let gutter_sym = if is_focused_row { "▎" } else { " " };
                    ui.set_string(row_rect.x, row_rect.y, gutter_sym, gutter_style);
                    ui.part(
                        opt_id.clone(),
                        Part::new("gutter"),
                        Rect::new(row_rect.x, row_rect.y, 1, 1),
                        |_| {},
                    );

                    // Marker
                    let is_narrow = row_rect.width < 4;
                    let mark_sym = if is_narrow {
                        if is_selected { "●" } else { "○" }
                    } else if is_selected {
                        "(●)"
                    } else {
                        "( )"
                    };

                    let mark_style = if opt_disabled {
                        row_style
                    } else if is_selected {
                        row_style.fg(theme.tokens.accent)
                    } else {
                        row_style.fg(theme.tokens.text_muted)
                    };

                    let mark_x = row_rect.x.saturating_add(1);
                    let mark_w = width(mark_sym) as u16;
                    if mark_x < row_rect.x.saturating_add(row_rect.width) {
                        ui.set_string(mark_x, row_rect.y, mark_sym, mark_style);
                        ui.part(
                            opt_id.clone(),
                            Part::new("marker"),
                            Rect::new(mark_x, row_rect.y, mark_w, 1),
                            |_| {},
                        );
                    }

                    // Label
                    let label_x = if is_narrow {
                        mark_x.saturating_add(mark_w)
                    } else {
                        row_rect.x.saturating_add(5)
                    };

                    let max_x = row_rect.x.saturating_add(row_rect.width);
                    if label_x < max_x {
                        let avail_w = (max_x - label_x) as usize;
                        let truncated = crate::termrock::text::truncate(opt.label, avail_w);
                        let label_w = width(&truncated) as u16;
                        ui.set_string(label_x, row_rect.y, &truncated, row_style);
                        ui.part(
                            opt_id.clone(),
                            Part::new("label"),
                            Rect::new(label_x, row_rect.y, label_w, 1),
                            |_| {},
                        );
                    }

                    curr_y = curr_y.saturating_add(1);
                }
                Axis::Horizontal => {
                    if curr_x >= right || curr_y >= bottom {
                        break;
                    }
                    let opt_w = (width(opt.label) as u16)
                        .saturating_add(6)
                        .min(right - curr_x);
                    let item_rect = Rect::new(curr_x, curr_y, opt_w, 1);
                    if !opt_disabled {
                        ui.register_hit(opt_id.clone(), item_rect);
                    }

                    let item_style = if opt_disabled {
                        Style::new().fg(theme.tokens.disabled).bg(bg_color)
                    } else if is_hovered {
                        Style::new()
                            .fg(theme.tokens.text_primary)
                            .bg(theme.tokens.surface_overlay)
                    } else {
                        Style::new().fg(theme.tokens.text_primary).bg(bg_color)
                    };

                    ui.fill_rect(item_rect, item_style);
                    ui.part(opt_id.clone(), Part::new("container"), item_rect, |_| {});

                    // Marker
                    let mark_sym = if is_selected { "(●)" } else { "( )" };
                    let mark_style = if opt_disabled {
                        item_style
                    } else if is_selected {
                        item_style.fg(theme.tokens.accent)
                    } else {
                        item_style.fg(theme.tokens.text_muted)
                    };

                    ui.set_string(item_rect.x, item_rect.y, mark_sym, mark_style);
                    ui.part(
                        opt_id.clone(),
                        Part::new("marker"),
                        Rect::new(item_rect.x, item_rect.y, 3, 1),
                        |_| {},
                    );

                    // Label
                    let label_x = item_rect.x.saturating_add(4);
                    let max_x = item_rect.x.saturating_add(item_rect.width);
                    if label_x < max_x {
                        let avail_w = (max_x - label_x) as usize;
                        let truncated = crate::termrock::text::truncate(opt.label, avail_w);
                        let label_w = width(&truncated) as u16;
                        ui.set_string(label_x, item_rect.y, &truncated, item_style);
                        ui.part(
                            opt_id.clone(),
                            Part::new("label"),
                            Rect::new(label_x, item_rect.y, label_w, 1),
                            |_| {},
                        );
                    }

                    curr_x = curr_x.saturating_add(opt_w).saturating_add(1);
                }
            }
        }

        clipped
    }
}
