//! Termrock Button control, variant styles, focus gutters, and activation feedback.

use ratatui::crossterm::event::KeyCode;
use ratatui::style::{Modifier, Style};
use std::time::Duration;

use crate::termrock::identity::{Id, Part};
use crate::termrock::layout::{Constraints, Position, Rect, Size};
use crate::termrock::response::{
    Activated, ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause,
    VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::text::width;
use crate::termrock::theme::{ColorLevel, Role, StylePatch, Surface};

/// Visual variant for a Button control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Subtle,
    Danger,
    Toggle,
    Quiet,
    Ghost,
}

/// Operational eligibility and activity status for a control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ControlStatus {
    #[default]
    Normal,
    Busy,
    Disabled,
}

/// A one-row action control with stable identity, semantic variants, and feedback timing.
#[derive(Debug, Clone)]
pub struct Button<'a> {
    pub id: Id,
    pub label: &'a str,
    pub variant: ButtonVariant,
    pub status: ControlStatus,
    pub checked: Option<bool>,
    pub icon: Option<&'a str>,
    pub autofocus: bool,
    pub patch: Option<StylePatch>,
    pub part_patches: Vec<(Part, StylePatch)>,
}

impl<'a> Button<'a> {
    pub fn new(id: Id, label: &'a str) -> Self {
        Self {
            id,
            label,
            variant: ButtonVariant::Secondary,
            status: ControlStatus::Normal,
            checked: None,
            icon: None,
            autofocus: false,
            patch: None,
            part_patches: Vec::new(),
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        if disabled {
            self.status = ControlStatus::Disabled;
        } else if self.status == ControlStatus::Disabled {
            self.status = ControlStatus::Normal;
        }
        self
    }

    pub fn status(mut self, status: ControlStatus) -> Self {
        self.status = status;
        self
    }

    pub fn busy(mut self, busy: bool) -> Self {
        if busy {
            self.status = ControlStatus::Busy;
        } else if self.status == ControlStatus::Busy {
            self.status = ControlStatus::Normal;
        }
        self
    }

    pub fn checked(mut self, checked: Option<bool>) -> Self {
        self.checked = checked;
        self
    }

    pub fn icon(mut self, icon: Option<&'a str>) -> Self {
        self.icon = icon;
        self
    }

    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = Some(patch);
        self
    }

    pub fn patch_part(mut self, part: Part, patch: StylePatch) -> Self {
        self.part_patches.push((part, patch));
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.status == ControlStatus::Normal
    }

    pub fn is_busy(&self) -> bool {
        self.status == ControlStatus::Busy
    }

    pub fn is_disabled(&self) -> bool {
        self.status == ControlStatus::Disabled
    }

    pub fn update(&self, cx: &mut Cx<'_>) -> Response<Activated> {
        let has_focus = cx.has_focus(&self.id);

        if self.autofocus && self.is_enabled() && cx.focus.is_none() {
            cx.request_focus(self.id.clone());
        }

        if !self.is_enabled() {
            if cx.pointer_capture.as_ref() == Some(&self.id) {
                cx.release_capture();
            }
            let vs = VisualState::empty()
                .focused(has_focus)
                .disabled(self.is_disabled())
                .busy(self.is_busy())
                .checked(self.checked.unwrap_or(false));

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
                        Activated::new(ActivationOrigin::Keyboard),
                    )
                    .with_flow(Flow::Consumed)
                    .with_invalidate(Invalidate::Paint)
                    .with_state(
                        VisualState::empty()
                            .focused(true)
                            .activation_feedback(true)
                            .checked(self.checked.unwrap_or(false)),
                    );
                }
            }
            UpdateCause::Input(Input::Mouse(m), _) => {
                let pos = Position::new(m.pos.x, m.pos.y);
                match m.kind {
                    MouseKind::Down => {
                        if cx.intended_owner() == Some(&self.id) {
                            cx.capture_pointer(self.id.clone());
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::consumed(self.id.clone())
                                .with_invalidate(Invalidate::Paint)
                                .with_state(
                                    VisualState::empty()
                                        .focused(has_focus)
                                        .pressed(true)
                                        .checked(self.checked.unwrap_or(false)),
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
                                    Activated::new(ActivationOrigin::Pointer),
                                )
                                .with_flow(Flow::Consumed)
                                .with_invalidate(Invalidate::Paint)
                                .with_state(
                                    VisualState::empty()
                                        .focused(has_focus)
                                        .activation_feedback(true)
                                        .checked(self.checked.unwrap_or(false)),
                                );
                            } else {
                                // Released outside: canceled
                                return Response::consumed(self.id.clone())
                                    .with_invalidate(Invalidate::Paint)
                                    .with_state(
                                        VisualState::empty()
                                            .focused(has_focus)
                                            .checked(self.checked.unwrap_or(false)),
                                    );
                            }
                        } else if cx.intended_owner() == Some(&self.id)
                            && cx.contains_point(&self.id, pos)
                        {
                            cx.trigger_feedback(self.id.clone(), Duration::from_millis(140));
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(
                                self.id.clone(),
                                Activated::new(ActivationOrigin::Pointer),
                            )
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint)
                            .with_state(
                                VisualState::empty()
                                    .focused(has_focus)
                                    .activation_feedback(true)
                                    .checked(self.checked.unwrap_or(false)),
                            );
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        let vs = VisualState::empty()
            .focused(has_focus)
            .checked(self.checked.unwrap_or(false));
        Response::bubble(self.id.clone()).with_state(vs)
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let label_w = width(self.label);
        let marker_w = if self.checked.is_some() || self.is_busy() {
            2
        } else {
            0
        };
        let icon_w = if let Some(ic) = self.icon {
            width(ic) + 1
        } else {
            0
        };
        let total_w = (label_w + 2 + marker_w + icon_w) as u16;
        constraints.clamp(Size::new(total_w, 1))
    }

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect {
        let measure_cx = MeasureCx::new(
            Constraints::loose(Size::new(area.width, area.height)),
            ui.theme,
            ColorLevel::TrueColor,
        );
        let measured = self.measure(
            &measure_cx,
            Constraints::loose(Size::new(area.width, area.height)),
        );
        let w = measured.width.min(area.width);
        let h = measured.height.min(area.height).min(1);
        let rect = Rect::new(area.x, area.y, w, h);

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

        // Variant base colors and states
        let mut style = match self.variant {
            ButtonVariant::Primary => {
                let fill = if active_press {
                    theme.tokens.accent_pressed
                } else if hovered {
                    theme.tokens.accent_hover
                } else {
                    theme.tokens.accent
                };
                Style::new()
                    .fg(theme.tokens.text_on_accent)
                    .bg(fill)
                    .add_modifier(Modifier::BOLD)
            }
            ButtonVariant::Secondary | ButtonVariant::Toggle => {
                let mut st = Style::new()
                    .fg(theme.tokens.text_primary)
                    .bg(theme.tokens.surface_overlay);
                if hovered {
                    st = st.bg(theme.tokens.popover);
                }
                if focused {
                    st = st.add_modifier(Modifier::BOLD);
                }
                if active_press {
                    st = Style::new()
                        .fg(theme.tokens.canvas)
                        .bg(theme.tokens.text_primary);
                }
                st
            }
            ButtonVariant::Subtle | ButtonVariant::Quiet | ButtonVariant::Ghost => {
                let mut st = Style::new().fg(theme.tokens.text_secondary).bg(bg_color);
                if hovered {
                    let lifted = match ui.current_surface {
                        Surface::Canvas => theme.tokens.surface_elevated,
                        Surface::Surface | Surface::Elevated => theme.tokens.surface_overlay,
                        Surface::Field => theme.tokens.field_hover,
                        _ => theme.tokens.popover,
                    };
                    st = st.fg(theme.tokens.text_primary).bg(lifted);
                }
                if focused {
                    st = st
                        .fg(theme.tokens.text_primary)
                        .add_modifier(Modifier::BOLD);
                }
                if active_press {
                    st = Style::new()
                        .fg(theme.tokens.canvas)
                        .bg(theme.tokens.text_primary);
                }
                st
            }
            ButtonVariant::Danger => {
                let mut st = Style::new()
                    .fg(theme.tokens.error_soft)
                    .bg(theme.tokens.surface_overlay);
                if hovered {
                    st = st.bg(theme.tokens.popover);
                }
                if focused {
                    st = st.add_modifier(Modifier::BOLD);
                }
                if active_press {
                    st = Style::new()
                        .fg(theme.tokens.text_primary)
                        .bg(theme.tokens.danger);
                }
                st
            }
        };

        if self.is_disabled() {
            style = Style::new().fg(theme.tokens.disabled).bg(
                if matches!(
                    self.variant,
                    ButtonVariant::Subtle | ButtonVariant::Quiet | ButtonVariant::Ghost
                ) {
                    bg_color
                } else {
                    theme.tokens.surface_overlay
                },
            );
        } else if self.is_busy() {
            style = style
                .fg(theme.tokens.text_secondary)
                .remove_modifier(Modifier::BOLD);
        }

        // Apply style patch if any
        if let Some(patch) = &self.patch {
            style = theme.apply_patch(style, patch, ui.current_surface);
        }

        // Container part
        ui.fill_rect(rect, style);
        ui.part(self.id.clone(), Part::new("container"), rect, |_| {});

        // Gutter
        let on_accent = self.variant == ButtonVariant::Primary && is_enabled;
        let gutter_fg = if !focused || !is_enabled {
            style.bg.unwrap_or(bg_color)
        } else if on_accent {
            theme.tokens.text_primary
        } else {
            theme.tokens.focus
        };
        let gutter_style = Style::new().fg(gutter_fg).bg(style.bg.unwrap_or(bg_color));
        let gutter_sym = if focused && is_enabled { "▎" } else { " " };
        ui.set_string(rect.x, rect.y, gutter_sym, gutter_style);
        ui.part(
            self.id.clone(),
            Part::new("gutter"),
            Rect::new(rect.x, rect.y, 1, 1),
            |_| {},
        );

        let mut curr_x = rect.x.saturating_add(1);
        let max_x = rect.x.saturating_add(rect.width).saturating_sub(1); // leave 1 space at end

        // Marker (busy spinner or checked toggle)
        if self.is_busy() && curr_x < max_x {
            let spin_style = style.fg(theme.tokens.accent);
            ui.set_string(curr_x, rect.y, "⠋ ", spin_style);
            ui.part(
                self.id.clone(),
                Part::new("marker"),
                Rect::new(curr_x, rect.y, 2, 1),
                |_| {},
            );
            curr_x = curr_x.saturating_add(2);
        } else if let Some(on) = self.checked.filter(|_| curr_x < max_x) {
            let marker_sym = if on { "● " } else { "○ " };
            let marker_style = if is_enabled && !active_press {
                style.fg(if on {
                    theme.tokens.accent
                } else {
                    theme.tokens.text_muted
                })
            } else {
                style
            };
            ui.set_string(curr_x, rect.y, marker_sym, marker_style);
            ui.part(
                self.id.clone(),
                Part::new("marker"),
                Rect::new(curr_x, rect.y, 2, 1),
                |_| {},
            );
            curr_x = curr_x.saturating_add(2);
        }

        // Icon
        if let Some(ic) = self.icon {
            let ic_w = width(ic) as u16;
            if curr_x.saturating_add(ic_w).saturating_add(1) <= max_x {
                ui.set_string(curr_x, rect.y, ic, style);
                ui.set_string(curr_x.saturating_add(ic_w), rect.y, " ", style);
                ui.part(
                    self.id.clone(),
                    Part::new("icon"),
                    Rect::new(curr_x, rect.y, ic_w, 1),
                    |_| {},
                );
                curr_x = curr_x.saturating_add(ic_w).saturating_add(1);
            }
        }

        // Label
        if curr_x < max_x {
            let avail_w = (max_x - curr_x) as usize;
            let truncated_label = crate::termrock::text::truncate(self.label, avail_w);
            let label_part_rect = Rect::new(curr_x, rect.y, width(&truncated_label) as u16, 1);
            let mut label_style = style;
            for (p, patch) in &self.part_patches {
                if *p == Part::LABEL || *p == Part::new("label") {
                    label_style = theme.apply_patch(label_style, patch, ui.current_surface);
                }
            }
            ui.set_string(curr_x, rect.y, &truncated_label, label_style);
            ui.part(self.id.clone(), Part::LABEL, label_part_rect, |_| {});
        }

        // Trailing padding space
        if rect.width > 1 {
            ui.set_string(
                rect.x.saturating_add(rect.width).saturating_sub(1),
                rect.y,
                " ",
                style,
            );
        }

        rect
    }
}

/// Lay out buttons in a row with a given gap, returning their areas.
pub fn row_layout(area: Rect, widths: &[u16], gap: u16) -> Vec<Rect> {
    let mut x = area.x;
    let mut out = Vec::new();
    let right_limit = area.x.saturating_add(area.width);
    for &w in widths {
        let max_w = right_limit.saturating_sub(x);
        let actual_w = w.min(max_w);
        out.push(Rect::new(x, area.y, actual_w, area.height.min(1)));
        x = x.saturating_add(actual_w).saturating_add(gap);
    }
    out
}

/// Right-aligned row layout for action bars.
pub fn row_layout_right(area: Rect, widths: &[u16], gap: u16) -> Vec<Rect> {
    let total_w: u16 =
        widths.iter().sum::<u16>() + gap.saturating_mul(widths.len().saturating_sub(1) as u16);
    let start_x = area
        .x
        .saturating_add(area.width)
        .saturating_sub(total_w)
        .max(area.x);
    row_layout(
        Rect::new(
            start_x,
            area.y,
            area.x.saturating_add(area.width).saturating_sub(start_x),
            area.height,
        ),
        widths,
        gap,
    )
}
