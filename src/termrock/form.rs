//! Form component composing fields, actions, validation, and child controls.

use std::collections::HashMap;

use ratatui::crossterm::event::KeyCode;
use ratatui::style::Style;

use crate::core::event::{Input, MouseKind};
use crate::termrock::button::{Button, ButtonVariant};
use crate::termrock::identity::{ActionKey, FieldKey, Id};
use crate::termrock::layout::{Constraints, Rect, Size};
use crate::termrock::response::{Flow, Invalidate, Response, UpdateCause};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::secret::FieldError;
use crate::termrock::theme::StylePatch;

/// Field declaration within a form.
#[derive(Debug, Clone)]
pub struct FieldSpec<'a> {
    pub key: FieldKey,
    pub child_id: Id,
    pub label: &'a str,
    pub required: bool,
    pub disabled: bool,
    pub hidden: bool,
}

impl<'a> FieldSpec<'a> {
    pub fn new(key: FieldKey, child_id: Id, label: &'a str) -> Self {
        Self {
            key,
            child_id,
            label,
            required: false,
            disabled: false,
            hidden: false,
        }
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }
}

pub use crate::termrock::empty::ActionMeta;

/// Typed action emitted by a form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormAction {
    Submit,
    Cancel,
    Auxiliary { action: ActionKey },
}

/// Durable caller-owned state for a form component.
#[derive(Debug, Default, Clone)]
pub struct FormState {
    pub submitted: bool,
    pub focused_field: Option<FieldKey>,
    pub invalid_fields: Vec<FieldKey>,
}

impl FormState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Container for child control values and states.
#[derive(Debug, Default, Clone)]
pub struct FormControls {
    pub values: HashMap<FieldKey, String>,
}

impl FormControls {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_value(&mut self, key: FieldKey, value: impl Into<String>) {
        self.values.insert(key, value.into());
    }

    pub fn get_value(&self, key: &FieldKey) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }
}

/// Form composite component.
pub struct Form<'a> {
    pub id: Id,
    pub fields: &'a [FieldSpec<'a>],
    pub actions: &'a [ActionMeta<'a>],
    pub validation: &'a [FieldError],
    pub dirty: bool,
    pub busy: bool,
    pub patch: StylePatch,
}

impl<'a> Form<'a> {
    pub fn new(id: Id, fields: &'a [FieldSpec<'a>], actions: &'a [ActionMeta<'a>]) -> Self {
        Self {
            id,
            fields,
            actions,
            validation: &[],
            dirty: false,
            busy: false,
            patch: StylePatch::default(),
        }
    }

    pub fn validation(mut self, validation: &'a [FieldError]) -> Self {
        self.validation = validation;
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

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = patch;
        self
    }

    fn visible_enabled_fields(&self) -> Vec<&FieldSpec<'a>> {
        self.fields
            .iter()
            .filter(|f| !f.hidden && !f.disabled)
            .collect()
    }

    /// Measure the overall form geometry.
    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let field_count = self.fields.iter().filter(|f| !f.hidden).count();
        let fields_height = (field_count as u16).saturating_mul(3); // 3 rows per field
        let actions_height = if self.actions.is_empty() { 0 } else { 2 };
        let total_h = fields_height.saturating_add(actions_height);
        constraints.clamp(Size::new(constraints.min.width.max(30), total_h))
    }

    /// Process input and handle form traversal or submission.
    pub fn update(
        &self,
        cx: &mut Cx<'_>,
        state: &mut FormState,
        controls: &mut FormControls,
    ) -> Response<FormAction> {
        let visible_fields = self.visible_enabled_fields();

        // Reconcile focused field if hidden or disabled
        if let Some(fk) = state.focused_field
            && !visible_fields.iter().any(|f| f.key == fk)
        {
            state.focused_field = visible_fields.first().map(|f| f.key);
            if let Some(first) = visible_fields.first() {
                cx.request_focus(first.child_id.clone());
            }
        }

        let cause = cx.cause().clone();

        match cause {
            UpdateCause::Input(Input::Key(k), _) => {
                match k.code {
                    KeyCode::Tab => {
                        // Focus traversal
                        if !visible_fields.is_empty() {
                            let next_idx = match state
                                .focused_field
                                .and_then(|fk| visible_fields.iter().position(|f| f.key == fk))
                            {
                                Some(cur_idx) => {
                                    if k.shift() {
                                        if cur_idx == 0 {
                                            visible_fields.len() - 1
                                        } else {
                                            cur_idx - 1
                                        }
                                    } else {
                                        (cur_idx + 1) % visible_fields.len()
                                    }
                                }
                                None => {
                                    if k.shift() {
                                        visible_fields.len() - 1
                                    } else {
                                        0
                                    }
                                }
                            };
                            let next_field = visible_fields[next_idx];
                            state.focused_field = Some(next_field.key);
                            cx.request_focus(next_field.child_id.clone());
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::consumed(self.id.clone())
                                .with_invalidate(Invalidate::Paint);
                        }
                    }
                    KeyCode::Enter => {
                        // Enter triggers submit action if present
                        if let Some(submit_meta) = self
                            .actions
                            .iter()
                            .find(|a| a.variant == ButtonVariant::Primary)
                        {
                            return self.trigger_action(cx, state, controls, submit_meta);
                        }
                    }
                    KeyCode::Esc => {
                        // Cancel
                        return Response::action(self.id.clone(), FormAction::Cancel)
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint);
                    }
                    _ => {}
                }
            }
            UpdateCause::Input(Input::Mouse(m), _) => {
                if matches!(m.kind, MouseKind::Up)
                    && let Some(target) = cx.intended_owner()
                {
                    // Check action buttons
                    for action_meta in self.actions {
                        let action_id = self.id.sub(&format!("act.{}", action_meta.key.as_str()));
                        if *target == action_id {
                            return self.trigger_action(cx, state, controls, action_meta);
                        }
                    }
                }
            }
            _ => {}
        }

        Response::bubble(self.id.clone())
    }

    fn trigger_action(
        &self,
        cx: &mut Cx<'_>,
        state: &mut FormState,
        controls: &FormControls,
        meta: &ActionMeta<'a>,
    ) -> Response<FormAction> {
        if meta.variant == ButtonVariant::Primary {
            // Submit attempt
            if self.busy {
                return Response::consumed(self.id.clone());
            }

            let mut invalid = Vec::new();

            // Validate all visible fields
            for f in self.fields.iter().filter(|f| !f.hidden) {
                if f.required {
                    let val = controls.get_value(&f.key).unwrap_or("");
                    if val.trim().is_empty() {
                        invalid.push(f.key);
                    }
                }
                if self.validation.iter().any(|fe| fe.field == f.key) && !invalid.contains(&f.key) {
                    invalid.push(f.key);
                }
            }

            if !invalid.is_empty() {
                // Focus the FIRST invalid visible field
                let first_invalid = invalid[0];
                state.invalid_fields = invalid;
                state.focused_field = Some(first_invalid);
                if let Some(f) = self.fields.iter().find(|f| f.key == first_invalid) {
                    cx.request_focus(f.child_id.clone());
                }
                cx.request_invalidate(Invalidate::Paint);
                // DO NOT emit Submit!
                return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
            }

            state.invalid_fields.clear();
            state.submitted = true;
            Response::action(self.id.clone(), FormAction::Submit)
                .with_flow(Flow::Consumed)
                .with_invalidate(Invalidate::Paint)
        } else if meta.variant == ButtonVariant::Ghost || meta.label == "Cancel" {
            Response::action(self.id.clone(), FormAction::Cancel)
                .with_flow(Flow::Consumed)
                .with_invalidate(Invalidate::Paint)
        } else {
            Response::action(self.id.clone(), FormAction::Auxiliary { action: meta.key })
                .with_flow(Flow::Consumed)
                .with_invalidate(Invalidate::Paint)
        }
    }

    /// Render form layout, fields, and action buttons.
    pub fn draw(
        &self,
        ui: &mut Ui<'_>,
        area: Rect,
        state: &FormState,
        controls: &FormControls,
    ) -> Rect {
        if area.is_empty() {
            return Rect::zero();
        }

        let theme = ui.theme;
        let mut curr_y = area.y;
        let mut rem_h = area.height;

        // Draw visible fields
        for f in self.fields.iter().filter(|f| !f.hidden) {
            if rem_h < 3 {
                break;
            }
            let field_area = Rect::new(area.x, curr_y, area.width, 3);
            let has_focus = ui.is_focused(&f.child_id);
            let is_invalid = state.invalid_fields.contains(&f.key);

            let lbl_style = Style::new().fg(theme.tokens.text_primary);
            ui.set_string(field_area.x, field_area.y, f.label, lbl_style);
            if f.required {
                let lx = field_area.x.saturating_add(f.label.len() as u16);
                ui.set_string(lx, field_area.y, " *", Style::new().fg(theme.tokens.danger));
            }

            // Input box
            let input_y = field_area.y.saturating_add(1);
            let input_rect = Rect::new(field_area.x, input_y, field_area.width, 1);
            let bg_color = theme.tokens.surface;
            ui.fill_rect(input_rect, Style::new().bg(bg_color));

            let gutter = if has_focus { "▎" } else { " " };
            let gutter_style = if is_invalid {
                Style::new().fg(theme.tokens.danger).bg(bg_color)
            } else if has_focus {
                Style::new().fg(theme.tokens.focus).bg(bg_color)
            } else {
                Style::new().fg(theme.tokens.text_muted).bg(bg_color)
            };
            ui.set_string(field_area.x, input_y, gutter, gutter_style);

            let val = controls.get_value(&f.key).unwrap_or("");
            ui.set_string(
                field_area.x.saturating_add(2),
                input_y,
                val,
                Style::new().fg(theme.tokens.text_primary).bg(bg_color),
            );

            ui.register_hit(f.child_id.clone(), input_rect);
            ui.register_focus(f.child_id.clone(), !f.disabled);

            curr_y = curr_y.saturating_add(3);
            rem_h = rem_h.saturating_sub(3);
        }

        // Draw actions row at the bottom
        if !self.actions.is_empty() && rem_h >= 1 {
            let mut curr_x = area.x;
            let measure_cx = MeasureCx::new(
                Constraints::unbounded(),
                theme,
                crate::termrock::theme::ColorLevel::TrueColor,
            );
            for action_meta in self.actions {
                let action_id = self.id.sub(&format!("act.{}", action_meta.key.as_str()));
                let btn = Button::new(action_id, action_meta.label).variant(action_meta.variant);
                let btn_size = btn.measure(&measure_cx, Constraints::unbounded());
                let btn_rect = Rect::new(curr_x, curr_y, btn_size.width, 1);
                btn.draw(ui, btn_rect);
                curr_x = curr_x.saturating_add(btn_size.width).saturating_add(2);
            }
        }

        area
    }
}
