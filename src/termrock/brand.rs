//! Termrock Brand identity lockup control.
//!
//! Provides static decorative product identity lockups and optional interactive
//! click-capable lockups sharing standard Button activation timing.

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
use crate::termrock::theme::{ColorLevel, StylePatch};

/// Application identity lockup component.
#[derive(Debug, Clone)]
pub struct Brand<'a> {
    pub id: Id,
    pub label: &'a str,
    pub meta: Option<&'a str>,
    pub interactive: bool,
    pub compact: bool,
    pub patch: Option<StylePatch>,
}

impl<'a> Brand<'a> {
    pub fn new(id: Id, label: &'a str) -> Self {
        Self {
            id,
            label,
            meta: None,
            interactive: false,
            compact: false,
            patch: None,
        }
    }

    pub fn meta(mut self, meta: &'a str) -> Self {
        self.meta = Some(meta);
        self
    }

    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = Some(patch);
        self
    }

    pub fn update(&self, cx: &mut Cx<'_>) -> Response<Activated> {
        if !self.interactive {
            return Response::bubble(self.id.clone());
        }

        let has_focus = cx.has_focus(&self.id);

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
                    .with_state(VisualState::empty().focused(true).activation_feedback(true));
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
                                .with_state(VisualState::empty().focused(has_focus).pressed(true));
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
                                        .activation_feedback(true),
                                );
                            } else {
                                return Response::consumed(self.id.clone())
                                    .with_invalidate(Invalidate::Paint);
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
                                    .activation_feedback(true),
                            );
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        Response::bubble(self.id.clone()).with_state(VisualState::empty().focused(has_focus))
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let label_w = width(self.label);
        let meta_w = self.meta.map_or(0, |m| width(m) + 1);
        let padding = if self.compact { 0 } else { 2 };
        let total_w = (label_w + meta_w + padding) as u16;
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

        if self.interactive {
            ui.register_focus(self.id.clone(), true);
            ui.register_hit(self.id.clone(), rect);
        }

        let theme = ui.theme;
        let hovered = self.interactive && ui.is_hovered(&self.id);
        let pressed = self.interactive && ui.is_pressed(&self.id);
        let feedback = self.interactive && ui.is_feedback(&self.id);
        let active_press = pressed || feedback;

        let fill_bg = if active_press {
            theme.tokens.accent_pressed
        } else if hovered {
            theme.tokens.accent_hover
        } else {
            theme.tokens.accent
        };

        let mut base_style = Style::new()
            .fg(theme.tokens.text_on_accent)
            .bg(fill_bg)
            .add_modifier(Modifier::BOLD);

        if let Some(patch) = &self.patch {
            base_style = theme.apply_patch(base_style, patch, ui.current_surface);
        }

        ui.fill_rect(rect, base_style);
        ui.part(self.id.clone(), Part::new("container"), rect, |_| {});

        let mut curr_x = if self.compact {
            rect.x
        } else {
            rect.x.saturating_add(1)
        };
        let max_x = if self.compact {
            rect.x.saturating_add(rect.width)
        } else {
            rect.x.saturating_add(rect.width).saturating_sub(1)
        };

        if curr_x < max_x {
            let label_avail = (max_x - curr_x) as usize;
            let truncated_label = crate::termrock::text::truncate(self.label, label_avail);
            let label_w = width(&truncated_label) as u16;
            ui.set_string(curr_x, rect.y, &truncated_label, base_style);
            ui.part(
                self.id.clone(),
                Part::LABEL,
                Rect::new(curr_x, rect.y, label_w, 1),
                |_| {},
            );
            curr_x = curr_x.saturating_add(label_w);
        }

        if let Some(m) = self.meta.filter(|_| curr_x < max_x) {
            let meta_avail = (max_x - curr_x) as usize;
            let text = format!(" {m}");
            let truncated_meta = crate::termrock::text::truncate(&text, meta_avail);
            let meta_w = width(&truncated_meta) as u16;
            let meta_style = base_style
                .fg(theme.tokens.text_on_accent)
                .remove_modifier(Modifier::BOLD);
            ui.set_string(curr_x, rect.y, &truncated_meta, meta_style);
            ui.part(
                self.id.clone(),
                Part::new("meta"),
                Rect::new(curr_x, rect.y, meta_w, 1),
                |_| {},
            );
        }

        rect
    }
}
