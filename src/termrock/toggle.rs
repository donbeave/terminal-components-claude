//! Controlled Toggle switch component with baseline switch glyph, label, and state text.
//!
//! Caller owns the boolean on/off value; Toggle reports the requested next value
//! via `ValueChanged<bool>` and never mutates a domain model or retains an uncontrolled copy.

use std::time::Duration;

use ratatui::crossterm::event::KeyCode;
use ratatui::style::Style;

use crate::termrock::button::ControlStatus;
use crate::termrock::identity::{Id, Part};
use crate::termrock::layout::{Constraints, Position, Rect, Size};
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, ValueChanged,
    VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::text::width;
use crate::termrock::theme::{Role, StylePatch};

/// A controlled on/off toggle switch control.
#[derive(Debug, Clone)]
pub struct Toggle<'a> {
    pub id: Id,
    pub label: &'a str,
    pub on: bool,
    pub status: ControlStatus,
    pub patch: Option<StylePatch>,
    pub part_patches: Vec<(Part, StylePatch)>,
}

impl<'a> Toggle<'a> {
    /// Creates a new controlled `Toggle`.
    pub fn new(id: Id, label: &'a str, on: bool) -> Self {
        Self {
            id,
            label,
            on,
            status: ControlStatus::Normal,
            patch: None,
            part_patches: Vec::new(),
        }
    }

    /// Sets the disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        if disabled {
            self.status = ControlStatus::Disabled;
        } else if self.status == ControlStatus::Disabled {
            self.status = ControlStatus::Normal;
        }
        self
    }

    /// Sets the operational control status.
    pub fn status(mut self, status: ControlStatus) -> Self {
        self.status = status;
        self
    }

    /// Applies a root style patch.
    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = Some(patch);
        self
    }

    /// Applies a part-specific style patch.
    pub fn patch_part(mut self, part: Part, patch: StylePatch) -> Self {
        self.part_patches.push((part, patch));
        self
    }

    /// Returns `true` if the toggle is enabled.
    pub fn is_enabled(&self) -> bool {
        self.status == ControlStatus::Normal
    }

    /// Returns `true` if the toggle is disabled.
    pub fn is_disabled(&self) -> bool {
        self.status == ControlStatus::Disabled
    }

    /// Updates the toggle state in response to user input.
    pub fn update(&self, cx: &mut Cx<'_>) -> Response<ValueChanged<bool>> {
        let has_focus = cx.has_focus(&self.id);

        if !self.is_enabled() {
            if cx.pointer_capture.as_ref() == Some(&self.id) {
                cx.release_capture();
            }
            let vs = VisualState::empty()
                .focused(has_focus)
                .disabled(self.is_disabled())
                .checked(self.on);

            match cx.cause() {
                UpdateCause::Input(Input::Key(k), _) if has_focus => {
                    if k.code == KeyCode::Enter || k.code == KeyCode::Char(' ') {
                        return Response::consumed(self.id.clone()).with_state(vs);
                    }
                }
                UpdateCause::Input(Input::Mouse(_), _) if cx.intended_owner() == Some(&self.id) => {
                    return Response::consumed(self.id.clone()).with_state(vs);
                }
                _ => {}
            }
            return Response::bubble(self.id.clone()).with_state(vs);
        }

        match cx.cause() {
            UpdateCause::Input(Input::Key(k), _) => {
                if has_focus && (k.code == KeyCode::Enter || k.code == KeyCode::Char(' ')) {
                    cx.trigger_feedback(self.id.clone(), Duration::from_millis(140));
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::action(
                        self.id.clone(),
                        ValueChanged::new(!self.on, ActivationOrigin::Keyboard),
                    )
                    .with_flow(Flow::Consumed)
                    .with_invalidate(Invalidate::Paint)
                    .with_state(
                        VisualState::empty()
                            .focused(true)
                            .activation_feedback(true)
                            .checked(self.on),
                    );
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
                                .with_state(
                                    VisualState::empty()
                                        .focused(has_focus)
                                        .pressed(true)
                                        .checked(self.on),
                                );
                        }
                    }
                    MouseKind::Up => {
                        if cx.pointer_capture.as_ref() == Some(&self.id) {
                            cx.release_capture();
                            cx.request_invalidate(Invalidate::Paint);
                            if cx.contains_point(&self.id, pos) {
                                cx.trigger_feedback(self.id.clone(), Duration::from_millis(140));
                                return Response::action(
                                    self.id.clone(),
                                    ValueChanged::new(!self.on, ActivationOrigin::Pointer),
                                )
                                .with_flow(Flow::Consumed)
                                .with_invalidate(Invalidate::Paint)
                                .with_state(
                                    VisualState::empty()
                                        .focused(has_focus)
                                        .activation_feedback(true)
                                        .checked(self.on),
                                );
                            } else {
                                // Released outside: cancelled
                                return Response::consumed(self.id.clone())
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(
                                        VisualState::empty().focused(has_focus).checked(self.on),
                                    );
                            }
                        } else if (cx.intended_owner() == Some(&self.id)
                            || cx.contains_point(&self.id, pos))
                            && cx.contains_point(&self.id, pos)
                        {
                            cx.trigger_feedback(self.id.clone(), Duration::from_millis(140));
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(
                                self.id.clone(),
                                ValueChanged::new(!self.on, ActivationOrigin::Pointer),
                            )
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint)
                            .with_state(
                                VisualState::empty()
                                    .focused(has_focus)
                                    .activation_feedback(true)
                                    .checked(self.on),
                            );
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        let vs = VisualState::empty().focused(has_focus).checked(self.on);
        Response::bubble(self.id.clone()).with_state(vs)
    }

    /// Measures the dimensions of the toggle row.
    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let label_w = width(self.label);
        // Gutter (1) + space (1) + glyph (3) + space (1) + label + space (1) + state (3) = label_w + 10
        let total_w = (label_w as u16).saturating_add(10);
        constraints.clamp(Size::new(total_w, 1))
    }

    /// Draws the toggle inside the allocated area.
    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect {
        let clipped = ui.clip_area().intersect(area);
        let rect = Rect::new(clipped.x, clipped.y, clipped.width, 1.min(clipped.height));

        if rect.is_empty() {
            return Rect::zero();
        }

        let is_enabled = self.is_enabled();
        ui.register_focus(self.id.clone(), is_enabled);
        ui.register_hit(self.id.clone(), rect);

        let focused = ui.is_focused(&self.id);
        let hovered = ui.is_hovered(&self.id) && is_enabled;
        let pressed = ui.is_pressed(&self.id) && is_enabled;
        let feedback = ui.is_feedback(&self.id);
        let active_press = pressed || feedback;

        let theme = ui.theme;
        let bg_color = theme.resolve_role(Role::Background, ui.current_surface);

        let mut row_style = if self.is_disabled() {
            Style::new().fg(theme.tokens.disabled).bg(bg_color)
        } else if hovered || active_press {
            Style::new()
                .fg(theme.tokens.text_primary)
                .bg(theme.tokens.surface_overlay)
        } else {
            Style::new().fg(theme.tokens.text_primary).bg(bg_color)
        };

        if let Some(patch) = &self.patch {
            row_style = theme.apply_patch(row_style, patch, ui.current_surface);
        }

        // Fill background container
        ui.fill_rect(rect, row_style);
        ui.part(self.id.clone(), Part::new("container"), rect, |_| {});

        // Gutter
        let gutter_fg = if !focused || !is_enabled {
            row_style.bg.unwrap_or(bg_color)
        } else {
            theme.tokens.focus
        };
        let gutter_style = Style::new()
            .fg(gutter_fg)
            .bg(row_style.bg.unwrap_or(bg_color));
        let gutter_sym = if focused && is_enabled { "▎" } else { " " };
        ui.set_string(rect.x, rect.y, gutter_sym, gutter_style);
        ui.part(
            self.id.clone(),
            Part::new("gutter"),
            Rect::new(rect.x, rect.y, 1, 1),
            |_| {},
        );

        // Marker (switch glyph)
        let is_narrow = rect.width < 4;
        let switch_sym = if is_narrow {
            if self.on { "●" } else { "○" }
        } else if self.on {
            "──●"
        } else {
            "○──"
        };

        let switch_style = if self.is_disabled() {
            row_style
        } else if self.on {
            row_style.fg(theme.tokens.accent)
        } else {
            row_style.fg(theme.tokens.text_muted)
        };

        let switch_x = rect.x.saturating_add(1);
        let switch_w = width(switch_sym) as u16;
        if switch_x < rect.x.saturating_add(rect.width) {
            ui.set_string(switch_x, rect.y, switch_sym, switch_style);
            ui.part(
                self.id.clone(),
                Part::new("marker"),
                Rect::new(switch_x, rect.y, switch_w, 1),
                |_| {},
            );
        }

        // Label
        let label_x = if is_narrow {
            switch_x.saturating_add(switch_w)
        } else {
            rect.x.saturating_add(5)
        };

        let max_x = rect.x.saturating_add(rect.width);
        if label_x < max_x {
            let avail_w = (max_x - label_x) as usize;
            let truncated = crate::termrock::text::truncate(self.label, avail_w);
            let label_w = width(&truncated) as u16;
            ui.set_string(label_x, rect.y, &truncated, row_style);
            ui.part(
                self.id.clone(),
                Part::new("label"),
                Rect::new(label_x, rect.y, label_w, 1),
                |_| {},
            );
        }

        // State text ("on" / "off")
        let state_text = if self.on { "on" } else { "off" };
        let state_offset = 6usize.saturating_add(width(self.label));
        if state_offset.saturating_add(3) < usize::from(rect.width) {
            let state_x = rect.x.saturating_add(state_offset as u16);
            let state_style = row_style.fg(if self.is_disabled() {
                theme.tokens.disabled
            } else {
                theme.tokens.text_muted
            });
            ui.set_string(state_x, rect.y, state_text, state_style);
            ui.part(
                self.id.clone(),
                Part::new("state"),
                Rect::new(state_x, rect.y, 3, 1),
                |_| {},
            );
        }

        rect
    }
}
