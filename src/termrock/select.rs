//! Controlled Select component with anchored popup list, keyed options, and dismissal.
//!
//! Caller owns keyed options and committed choice; Select owns open/closed status,
//! candidate highlight, and popup placement. Choosing is explicit; highlighting an option
//! never commits it. Escape or outside clicks dismiss without mutating committed choice.

use std::time::Duration;

use ratatui::crossterm::event::KeyCode;
use ratatui::style::Style;

use crate::termrock::collections::{child_item_key, reconcile_cursor};
use crate::termrock::identity::{Id, ItemKey, Part, Revision};
use crate::termrock::layout::{Constraints, Position, Rect, Size};
use crate::termrock::radio_group::ChoiceItem;
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::scroll::ScrollState;
use crate::termrock::secret::ValidationMessage;
use crate::termrock::text::width;
use crate::termrock::theme::{Role, StylePatch};

/// Durable view state for [`Select`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectState {
    pub open: bool,
    pub highlighted: Option<ItemKey>,
    pub scroll: ScrollState,
    pub last_revision: Option<Revision>,
}

impl SelectState {
    /// Creates a new `SelectState`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the dropdown popup is currently open.
    pub fn open(&self) -> bool {
        self.open
    }

    /// Returns the currently highlighted candidate item key in the popup list.
    pub fn highlighted(&self) -> Option<ItemKey> {
        self.highlighted
    }

    /// Returns a reference to the popup scroll state.
    pub fn scroll(&self) -> &ScrollState {
        &self.scroll
    }

    /// Returns a mutable reference to the popup scroll state.
    pub fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }

    /// Returns the last revision reconciled against.
    pub fn last_revision(&self) -> Option<Revision> {
        self.last_revision
    }
}

/// Typed action emitted by [`Select`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectAction {
    /// An option was explicitly chosen.
    Choose {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    /// The dropdown popup was dismissed without committing a choice.
    Dismissed,
}

/// Controlled select dropdown with anchored popup list.
#[derive(Debug, Clone)]
pub struct Select<'a> {
    pub id: Id,
    pub options: &'a [ChoiceItem<'a>],
    pub revision: Revision,
    pub selected: Option<ItemKey>,
    pub label: &'a str,
    pub placeholder: &'a str,
    pub help: &'a str,
    pub validation: Option<&'a ValidationMessage>,
    pub disabled: bool,
    pub patch: Option<StylePatch>,
}

impl<'a> Select<'a> {
    /// Three-row field standard height.
    pub const HEIGHT: u16 = 3;

    /// Creates a new controlled `Select`.
    pub fn new(id: Id, options: &'a [ChoiceItem<'a>], revision: Revision) -> Self {
        Self {
            id,
            options,
            revision,
            selected: None,
            label: "",
            placeholder: "",
            help: "",
            validation: None,
            disabled: false,
            patch: None,
        }
    }

    /// Sets the controlled selected item key.
    pub fn selected(mut self, key: Option<ItemKey>) -> Self {
        self.selected = key;
        self
    }

    /// Sets the field label displayed above the input.
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = label;
        self
    }

    /// Sets the placeholder displayed when no option is selected.
    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = placeholder;
        self
    }

    /// Sets the help text displayed below the field.
    pub fn help(mut self, help: &'a str) -> Self {
        self.help = help;
        self
    }

    /// Sets an optional field validation message (e.g. error).
    pub fn validation(mut self, validation: Option<&'a ValidationMessage>) -> Self {
        self.validation = validation;
        self
    }

    /// Sets whether the select field is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Applies a style patch to the select field.
    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = Some(patch);
        self
    }

    /// Returns the stable child identity for an individual option.
    pub fn option_id(&self, key: ItemKey) -> Id {
        self.id.child(key)
    }

    /// Returns the display label of the currently selected option, or placeholder.
    pub fn value_label(&self) -> &str {
        if let Some(sel_key) = self.selected
            && let Some(opt) = self.options.iter().find(|o| o.key == sel_key)
        {
            return opt.label;
        }
        self.placeholder
    }

    /// Updates the select dropdown, handling open/closed state transitions, keyboard
    /// navigation, popup dismissal, and option selection.
    pub fn update(&self, cx: &mut Cx<'_>, state: &mut SelectState) -> Response<SelectAction> {
        let all_keys: Vec<ItemKey> = self.options.iter().map(|o| o.key).collect();
        let enabled_keys: Vec<ItemKey> = self
            .options
            .iter()
            .filter(|o| !o.disabled)
            .map(|o| o.key)
            .collect();

        // Reconcile highlighted option across revisions
        if state.highlighted.is_none() || !all_keys.contains(&state.highlighted.unwrap()) {
            if let Some(sel) = self.selected.filter(|s| all_keys.contains(s)) {
                state.highlighted = Some(sel);
            } else {
                state.highlighted = reconcile_cursor(state.highlighted, &all_keys);
            }
        }
        state.last_revision = Some(self.revision);

        let has_focus = cx.has_focus(&self.id);

        if self.disabled {
            if state.open {
                state.open = false;
                state.highlighted = self.selected;
                cx.request_invalidate(Invalidate::Paint);
            }
            if let Some(cap) = &cx.pointer_capture
                && (cap == &self.id || child_item_key(&self.id, cap).is_some())
            {
                cx.release_capture();
            }
            let vs = VisualState::empty()
                .focused(has_focus)
                .disabled(true)
                .invalid(self.validation.is_some());
            return Response::bubble(self.id.clone()).with_state(vs);
        }

        // Losing focus closes an open select
        if state.open && !has_focus {
            state.open = false;
            state.highlighted = self.selected;
            cx.request_invalidate(Invalidate::Paint);
            return Response::action(self.id.clone(), SelectAction::Dismissed)
                .with_flow(Flow::Consumed)
                .with_invalidate(Invalidate::Paint)
                .with_state(
                    VisualState::empty()
                        .focused(false)
                        .invalid(self.validation.is_some()),
                );
        }

        match cx.cause() {
            UpdateCause::Input(Input::Key(k), _) => {
                // Ignore modified chords (Ctrl, Alt) for plain actions
                if k.code != KeyCode::Esc
                    && (k
                        .mods
                        .contains(ratatui::crossterm::event::KeyModifiers::CONTROL)
                        || k.mods
                            .contains(ratatui::crossterm::event::KeyModifiers::ALT))
                {
                    return Response::bubble(self.id.clone());
                }

                if has_focus {
                    if state.open {
                        match k.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                if let Some(curr_key) = state.highlighted {
                                    if let Some(curr_idx) =
                                        enabled_keys.iter().position(|&k| k == curr_key)
                                    {
                                        let prev_idx = curr_idx.saturating_sub(1);
                                        state.highlighted = Some(enabled_keys[prev_idx]);
                                    } else {
                                        state.highlighted = enabled_keys.first().copied();
                                    }
                                } else {
                                    state.highlighted = enabled_keys.first().copied();
                                }
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::consumed(self.id.clone())
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(true));
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if let Some(curr_key) = state.highlighted {
                                    if let Some(curr_idx) =
                                        enabled_keys.iter().position(|&k| k == curr_key)
                                    {
                                        let next_idx = (curr_idx + 1)
                                            .min(enabled_keys.len().saturating_sub(1));
                                        state.highlighted = Some(enabled_keys[next_idx]);
                                    } else {
                                        state.highlighted = enabled_keys.first().copied();
                                    }
                                } else {
                                    state.highlighted = enabled_keys.first().copied();
                                }
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::consumed(self.id.clone())
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(true));
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                if let Some(key) = state.highlighted
                                    && enabled_keys.contains(&key)
                                {
                                    state.open = false;
                                    cx.trigger_feedback(
                                        self.id.clone(),
                                        Duration::from_millis(140),
                                    );
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(
                                        self.id.clone(),
                                        SelectAction::Choose {
                                            key,
                                            origin: ActivationOrigin::Keyboard,
                                        },
                                    )
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(
                                        VisualState::empty()
                                            .focused(true)
                                            .activation_feedback(true),
                                    );
                                }
                            }
                            KeyCode::Esc => {
                                state.open = false;
                                state.highlighted = self.selected;
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::action(self.id.clone(), SelectAction::Dismissed)
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(true));
                            }
                            _ => {
                                return Response::consumed(self.id.clone());
                            }
                        }
                    } else {
                        // Closed keyboard navigation
                        match k.code {
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                state.open = true;
                                state.highlighted =
                                    self.selected.or_else(|| enabled_keys.first().copied());
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::consumed(self.id.clone())
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(true));
                            }
                            KeyCode::Up | KeyCode::Left => {
                                // Request previous option directly without opening
                                if let Some(sel) = self.selected
                                    && let Some(curr_idx) =
                                        enabled_keys.iter().position(|&k| k == sel)
                                    && curr_idx > 0
                                {
                                    let prev_key = enabled_keys[curr_idx - 1];
                                    state.highlighted = Some(prev_key);
                                    cx.trigger_feedback(
                                        self.id.clone(),
                                        Duration::from_millis(140),
                                    );
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(
                                        self.id.clone(),
                                        SelectAction::Choose {
                                            key: prev_key,
                                            origin: ActivationOrigin::Keyboard,
                                        },
                                    )
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(true));
                                } else if let Some(&first_key) = enabled_keys.first() {
                                    state.highlighted = Some(first_key);
                                    cx.trigger_feedback(
                                        self.id.clone(),
                                        Duration::from_millis(140),
                                    );
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(
                                        self.id.clone(),
                                        SelectAction::Choose {
                                            key: first_key,
                                            origin: ActivationOrigin::Keyboard,
                                        },
                                    )
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(true));
                                }
                            }
                            KeyCode::Down | KeyCode::Right => {
                                // Request next option directly without opening
                                if let Some(sel) = self.selected
                                    && let Some(curr_idx) =
                                        enabled_keys.iter().position(|&k| k == sel)
                                    && curr_idx + 1 < enabled_keys.len()
                                {
                                    let next_key = enabled_keys[curr_idx + 1];
                                    state.highlighted = Some(next_key);
                                    cx.trigger_feedback(
                                        self.id.clone(),
                                        Duration::from_millis(140),
                                    );
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(
                                        self.id.clone(),
                                        SelectAction::Choose {
                                            key: next_key,
                                            origin: ActivationOrigin::Keyboard,
                                        },
                                    )
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(true));
                                } else if let Some(&first_key) = enabled_keys.first() {
                                    state.highlighted = Some(first_key);
                                    cx.trigger_feedback(
                                        self.id.clone(),
                                        Duration::from_millis(140),
                                    );
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(
                                        self.id.clone(),
                                        SelectAction::Choose {
                                            key: first_key,
                                            origin: ActivationOrigin::Keyboard,
                                        },
                                    )
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(true));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            UpdateCause::Input(Input::Mouse(m), _) => {
                let pos = Position::new(m.pos.x, m.pos.y);
                match m.kind {
                    MouseKind::Down => {
                        if cx.intended_owner() == Some(&self.id) || cx.contains_point(&self.id, pos)
                        {
                            cx.capture_pointer(self.id.clone());
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::consumed(self.id.clone())
                                .with_invalidate(Invalidate::Paint)
                                .with_state(VisualState::empty().focused(has_focus));
                        }
                        if state.open {
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
                    }
                    MouseKind::Up => {
                        let is_field_click = cx.pointer_capture.as_ref() == Some(&self.id)
                            || cx.intended_owner() == Some(&self.id)
                            || cx.contains_point(&self.id, pos);

                        if is_field_click {
                            if cx.pointer_capture.as_ref() == Some(&self.id) {
                                cx.release_capture();
                            }
                            if state.open {
                                state.open = false;
                                state.highlighted = self.selected;
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::action(self.id.clone(), SelectAction::Dismissed)
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(has_focus));
                            } else {
                                state.open = true;
                                state.highlighted =
                                    self.selected.or_else(|| enabled_keys.first().copied());
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::consumed(self.id.clone())
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(VisualState::empty().focused(has_focus));
                            }
                        }

                        if state.open {
                            // Check option clicks
                            for opt in self.options {
                                let opt_id = self.option_id(opt.key);
                                if cx.pointer_capture.as_ref() == Some(&opt_id) {
                                    cx.release_capture();
                                }
                                let is_opt_click = (cx.intended_owner() == Some(&opt_id)
                                    || cx.contains_point(&opt_id, pos))
                                    && cx.contains_point(&opt_id, pos);

                                if is_opt_click && !opt.disabled {
                                    state.open = false;
                                    state.highlighted = Some(opt.key);
                                    cx.trigger_feedback(
                                        self.id.clone(),
                                        Duration::from_millis(140),
                                    );
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(
                                        self.id.clone(),
                                        SelectAction::Choose {
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

                            // Released outside: dismiss popup without changing choice
                            state.open = false;
                            state.highlighted = self.selected;
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(self.id.clone(), SelectAction::Dismissed)
                                .with_flow(Flow::Consumed)
                                .with_invalidate(Invalidate::Paint)
                                .with_state(VisualState::empty().focused(has_focus));
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        let vs = VisualState::empty()
            .focused(has_focus)
            .invalid(self.validation.is_some());
        Response::bubble(self.id.clone()).with_state(vs)
    }

    /// Measures the dimensions of the closed select field.
    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let has_bottom_text = self.validation.is_some() || !self.help.is_empty();
        let height = if has_bottom_text { 3 } else { 2 };
        let mut max_w = width(self.label).max(width(self.placeholder));
        for opt in self.options {
            let ow = width(opt.label);
            if ow > max_w {
                max_w = ow;
            }
        }
        let total_w = (max_w as u16).saturating_add(6);
        constraints.clamp(Size::new(total_w, height))
    }

    /// Draws the select field and anchored popup list inside the UI frame.
    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &SelectState) -> Rect {
        let clipped = ui.clip_area().intersect(area);
        if clipped.is_empty() {
            return Rect::zero();
        }

        let is_enabled = !self.disabled;
        let focused = ui.is_focused(&self.id);
        let theme = ui.theme;
        let bg_color = theme.resolve_role(Role::Background, ui.current_surface);

        // Row 0: Label
        if !self.label.is_empty() && clipped.height >= 1 {
            let label_style = if self.disabled {
                Style::new().fg(theme.tokens.disabled).bg(bg_color)
            } else if focused {
                Style::new().fg(theme.tokens.accent).bg(bg_color)
            } else {
                Style::new().fg(theme.tokens.text_secondary).bg(bg_color)
            };
            let avail_w = clipped.width.saturating_sub(2) as usize;
            let truncated = crate::termrock::text::truncate(self.label, avail_w);
            ui.set_string(
                clipped.x.saturating_add(2),
                clipped.y,
                &truncated,
                label_style,
            );
            ui.part(
                self.id.clone(),
                Part::new("label"),
                Rect::new(clipped.x, clipped.y, clipped.width, 1),
                |_| {},
            );
        }

        if clipped.height < 2 {
            return clipped;
        }

        // Row 1: Field Box
        let field_y = clipped.y.saturating_add(1);
        let field_rect = Rect::new(clipped.x, field_y, clipped.width, 1);

        ui.register_focus(self.id.clone(), is_enabled);
        ui.register_hit(self.id.clone(), field_rect);

        let field_hovered = ui.is_hovered(&self.id) && is_enabled;
        let field_bg = if self.disabled {
            bg_color
        } else if field_hovered {
            theme.tokens.field_hover
        } else {
            theme.tokens.field
        };

        let mut field_style = if self.disabled {
            Style::new().fg(theme.tokens.disabled).bg(field_bg)
        } else {
            Style::new().fg(theme.tokens.text_primary).bg(field_bg)
        };

        if let Some(patch) = &self.patch {
            field_style = theme.apply_patch(field_style, patch, ui.current_surface);
        }

        ui.fill_rect(field_rect, field_style);
        ui.part(self.id.clone(), Part::new("container"), field_rect, |_| {});

        // Gutter
        let gutter_fg = if focused && is_enabled {
            theme.tokens.focus
        } else {
            field_bg
        };
        let gutter_style = Style::new().fg(gutter_fg).bg(field_bg);
        let gutter_sym = if focused && is_enabled { "▎" } else { " " };
        ui.set_string(field_rect.x, field_rect.y, gutter_sym, gutter_style);
        ui.part(
            self.id.clone(),
            Part::new("gutter"),
            Rect::new(field_rect.x, field_rect.y, 1, 1),
            |_| {},
        );

        // Selected value or placeholder
        let val_text = self.value_label();
        let val_x = field_rect.x.saturating_add(2);
        let disclosure_x = field_rect
            .x
            .saturating_add(field_rect.width)
            .saturating_sub(2);

        if val_x < disclosure_x {
            let avail_w = (disclosure_x - val_x) as usize;
            let truncated = crate::termrock::text::truncate(val_text, avail_w);
            let val_style = if self.selected.is_none() || self.disabled {
                field_style.fg(theme.tokens.text_muted)
            } else {
                field_style.fg(theme.tokens.text_primary)
            };
            ui.set_string(val_x, field_rect.y, &truncated, val_style);
            ui.part(
                self.id.clone(),
                Part::new("value"),
                Rect::new(val_x, field_rect.y, width(&truncated) as u16, 1),
                |_| {},
            );
        }

        // Disclosure indicator (▾ when closed, ▴ when open)
        if disclosure_x >= field_rect.x {
            let disc_sym = if state.open { "▴" } else { "▾" };
            let disc_style = field_style.fg(if self.disabled {
                theme.tokens.disabled
            } else {
                theme.tokens.text_secondary
            });
            ui.set_string(disclosure_x, field_rect.y, disc_sym, disc_style);
            ui.part(
                self.id.clone(),
                Part::new("disclosure"),
                Rect::new(disclosure_x, field_rect.y, 1, 1),
                |_| {},
            );
        }

        // Row 2: Validation message or help
        if clipped.height >= 3 {
            let msg_y = field_y.saturating_add(1);
            if let Some(val) = self.validation {
                let err_style = Style::new().fg(theme.tokens.error_soft).bg(bg_color);
                let avail_w = clipped.width.saturating_sub(2) as usize;
                let truncated = crate::termrock::text::truncate(&val.display, avail_w);
                ui.set_string(clipped.x.saturating_add(2), msg_y, &truncated, err_style);
                ui.part(
                    self.id.clone(),
                    Part::new("error"),
                    Rect::new(clipped.x, msg_y, clipped.width, 1),
                    |_| {},
                );
            } else if !self.help.is_empty() {
                let help_style = Style::new().fg(theme.tokens.text_muted).bg(bg_color);
                let avail_w = clipped.width.saturating_sub(2) as usize;
                let truncated = crate::termrock::text::truncate(self.help, avail_w);
                ui.set_string(clipped.x.saturating_add(2), msg_y, &truncated, help_style);
                ui.part(
                    self.id.clone(),
                    Part::new("help"),
                    Rect::new(clipped.x, msg_y, clipped.width, 1),
                    |_| {},
                );
            }
        }

        // Anchored popup list when open
        if state.open {
            let screen = ui.viewport;
            let desired_w = field_rect.width.clamp(12, 40);
            let desired_h = ((self.options.len() as u16).saturating_add(2)).min(10);

            // Compute vertical placement: place below, flip above if insufficient room
            let below_y = field_rect.y.saturating_add(field_rect.height);
            let room_below = screen.bottom().saturating_sub(below_y);

            let popup_y = if room_below >= desired_h {
                below_y
            } else if field_rect.y >= screen.y.saturating_add(desired_h) {
                field_rect.y.saturating_sub(desired_h)
            } else {
                screen.bottom().saturating_sub(desired_h)
            };

            let popup_x = field_rect
                .x
                .min(screen.right().saturating_sub(desired_w))
                .max(screen.x);

            let popup_rect = Rect::new(popup_x, popup_y, desired_w, desired_h);

            // Paint elevated surface and frame
            let popup_style = Style::new()
                .fg(theme.tokens.text_primary)
                .bg(theme.tokens.surface_elevated);
            ui.fill_rect(popup_rect, popup_style);

            // Frame border
            let border_style = Style::new()
                .fg(theme.tokens.border_strong)
                .bg(theme.tokens.surface_elevated);

            let max_px = popup_rect.x.saturating_add(popup_rect.width);
            let max_py = popup_rect.y.saturating_add(popup_rect.height);

            // Top and bottom borders
            for x in popup_rect.x..max_px {
                ui.set_string(x, popup_rect.y, "─", border_style);
                if max_py > popup_rect.y.saturating_add(1) {
                    ui.set_string(x, max_py - 1, "─", border_style);
                }
            }
            // Left and right borders
            for y in popup_rect.y..max_py {
                ui.set_string(popup_rect.x, y, "│", border_style);
                if max_px > popup_rect.x.saturating_add(1) {
                    ui.set_string(max_px - 1, y, "│", border_style);
                }
            }
            // Corners
            ui.set_string(popup_rect.x, popup_rect.y, "╭", border_style);
            if max_px > popup_rect.x.saturating_add(1) {
                ui.set_string(max_px - 1, popup_rect.y, "╮", border_style);
            }
            if max_py > popup_rect.y.saturating_add(1) {
                ui.set_string(popup_rect.x, max_py - 1, "╰", border_style);
                if max_px > popup_rect.x.saturating_add(1) {
                    ui.set_string(max_px - 1, max_py - 1, "╯", border_style);
                }
            }

            ui.part(self.id.clone(), Part::new("popup"), popup_rect, |_| {});

            let inner_x = popup_rect.x.saturating_add(1);
            let inner_y = popup_rect.y.saturating_add(1);
            let inner_w = popup_rect.width.saturating_sub(2);
            let inner_h = popup_rect.height.saturating_sub(2);

            for (i, opt) in self.options.iter().enumerate() {
                if (i as u16) >= inner_h {
                    break;
                }
                let row_y = inner_y.saturating_add(i as u16);
                let row_rect = Rect::new(inner_x, row_y, inner_w, 1);
                let opt_id = self.option_id(opt.key);

                if !opt.disabled {
                    ui.register_hit(opt_id.clone(), row_rect);
                }

                let is_highlighted = state.highlighted == Some(opt.key);
                let is_selected = self.selected == Some(opt.key);
                let is_hovered = ui.is_hovered(&opt_id) && !opt.disabled;

                let row_style = if opt.disabled {
                    Style::new()
                        .fg(theme.tokens.disabled)
                        .bg(theme.tokens.surface_elevated)
                } else if is_highlighted || is_hovered {
                    Style::new()
                        .fg(theme.tokens.text_primary)
                        .bg(theme.tokens.surface_overlay)
                } else {
                    Style::new()
                        .fg(theme.tokens.text_primary)
                        .bg(theme.tokens.surface_elevated)
                };

                ui.fill_rect(row_rect, row_style);
                ui.part(opt_id.clone(), Part::new("row"), row_rect, |_| {});

                // Gutter
                let gutter_fg = if is_highlighted && !opt.disabled {
                    theme.tokens.focus
                } else {
                    row_style.bg.unwrap_or(theme.tokens.surface_elevated)
                };
                let gutter_style = Style::new()
                    .fg(gutter_fg)
                    .bg(row_style.bg.unwrap_or(theme.tokens.surface_elevated));
                let gutter_sym = if is_highlighted && !opt.disabled {
                    "▎"
                } else {
                    " "
                };
                ui.set_string(row_rect.x, row_y, gutter_sym, gutter_style);

                // Selected marker ›
                if is_selected {
                    let marker_style = row_style.fg(theme.tokens.accent);
                    ui.set_string(row_rect.x.saturating_add(1), row_y, "›", marker_style);
                    ui.part(
                        opt_id.clone(),
                        Part::new("marker"),
                        Rect::new(row_rect.x.saturating_add(1), row_y, 1, 1),
                        |_| {},
                    );
                }

                // Label
                let opt_label_x = row_rect.x.saturating_add(3);
                let max_opt_x = row_rect.x.saturating_add(row_rect.width);
                if opt_label_x < max_opt_x {
                    let avail_w = (max_opt_x - opt_label_x) as usize;
                    let truncated = crate::termrock::text::truncate(opt.label, avail_w);
                    let label_w = width(&truncated) as u16;
                    ui.set_string(opt_label_x, row_y, &truncated, row_style);
                    ui.part(
                        opt_id.clone(),
                        Part::new("label"),
                        Rect::new(opt_label_x, row_y, label_w, 1),
                        |_| {},
                    );
                }
            }
        }

        clipped
    }
}
