//! Single-line controlled text editor with plain and secret specializations.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Range;

use ratatui::crossterm::event::KeyCode;
use ratatui::style::Style;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::core::event::{Input, MouseKind};
use crate::termrock::field::Field;
use crate::termrock::identity::{Id, Revision};
use crate::termrock::layout::{Constraints, Rect, Size};
use crate::termrock::response::{Flow, Invalidate, Response, UpdateCause, VisualState};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::secret::{
    Plain, Secret, SecretPolicy, SecretText, TextAction, ValidationMessage, Validator,
};
use crate::termrock::text::{TextEditorCore, width};
use crate::termrock::theme::StylePatch;

/// Active editing phase of a text input or text area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditPhase {
    /// Navigation mode: cursor/selection/scroll without text modification.
    #[default]
    Navigation,
    /// Editing mode: keystrokes mutate draft buffer.
    Editing,
}

/// Action taken when an editable field loses focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlurPolicy {
    /// Commit active draft on blur (baseline default).
    #[default]
    Commit,
    /// Cancel active draft and restore snapshot on blur.
    Cancel,
    /// Keep active draft without committing on blur.
    KeepDraft,
}

/// Policy for handling external source revision changes while editing a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictPolicy {
    /// Preserve active draft and report `TextAction::Conflict` (default).
    #[default]
    PreserveDraftAndReport,
    /// Overwrite active draft with incoming external value.
    OverwriteWithExternal,
}

/// Explicit resolution chosen by caller when resolving a revision conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Keep current local draft and dismiss conflict state.
    KeepDraft,
    /// Discard local draft and reload external value.
    ReloadExternal,
}

/// Trait for text input mode (Plain vs SecretText).
pub trait TextMode: 'static {
    type Value: fmt::Debug;
    fn to_value(s: &str) -> Self::Value;
    fn from_value(val: &Self::Value) -> String;
    fn is_secret() -> bool;
}

impl TextMode for Plain {
    type Value = String;
    fn to_value(s: &str) -> Self::Value {
        s.to_string()
    }
    fn from_value(val: &Self::Value) -> String {
        val.clone()
    }
    fn is_secret() -> bool {
        false
    }
}

impl TextMode for SecretText {
    type Value = Secret;
    fn to_value(s: &str) -> Self::Value {
        Secret::new(s.to_string())
    }
    fn from_value(val: &Self::Value) -> String {
        val.expose(|s| s.to_string())
    }
    fn is_secret() -> bool {
        true
    }
}

/// Durable caller-owned state for a text input component.
pub struct TextInputState<Mode: TextMode = Plain> {
    pub(crate) phase: EditPhase,
    pub(crate) core: TextEditorCore,
    pub(crate) snapshot: String,
    pub(crate) last_revision: Option<Revision>,
    pub(crate) scroll_offset: usize,
    pub(crate) conflict: bool,
    pub(crate) error: Option<ValidationMessage>,
    pub(crate) was_focused: bool,
    pub(crate) _mode: PhantomData<Mode>,
}

impl<Mode: TextMode> Default for TextInputState<Mode> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Mode: TextMode> TextInputState<Mode> {
    pub fn new() -> Self {
        Self {
            phase: EditPhase::Navigation,
            core: TextEditorCore::new(),
            snapshot: String::new(),
            last_revision: None,
            scroll_offset: 0,
            conflict: false,
            error: None,
            was_focused: false,
            _mode: PhantomData,
        }
    }

    pub fn phase(&self) -> EditPhase {
        self.phase
    }

    pub fn is_editing(&self) -> bool {
        self.phase == EditPhase::Editing
    }

    pub fn caret(&self) -> usize {
        self.core.caret()
    }

    pub fn selection(&self) -> Option<Range<usize>> {
        self.core.selection()
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn error(&self) -> Option<&ValidationMessage> {
        self.error.as_ref()
    }

    pub fn has_conflict(&self) -> bool {
        self.conflict
    }

    pub fn resolve_conflict(&mut self, res: ConflictResolution, external: &str) {
        self.conflict = false;
        match res {
            ConflictResolution::KeepDraft => {}
            ConflictResolution::ReloadExternal => {
                self.core.set_text(external);
                self.snapshot = external.to_string();
            }
        }
    }

    pub fn reset(&mut self, text: &str) {
        self.core.set_text(text);
        self.snapshot = text.to_string();
        self.phase = EditPhase::Navigation;
    }
}

impl TextInputState<Plain> {
    /// Safe inspection of active plain text draft.
    pub fn draft(&self) -> &str {
        self.core.text()
    }

    /// Alias for draft inspection.
    pub fn text(&self) -> &str {
        self.core.text()
    }
}

impl Clone for TextInputState<Plain> {
    fn clone(&self) -> Self {
        Self {
            phase: self.phase,
            core: self.core.clone(),
            snapshot: self.snapshot.clone(),
            last_revision: self.last_revision,
            scroll_offset: self.scroll_offset,
            conflict: self.conflict,
            error: self.error.clone(),
            was_focused: self.was_focused,
            _mode: PhantomData,
        }
    }
}

impl fmt::Debug for TextInputState<Plain> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextInputState<Plain>")
            .field("phase", &self.phase)
            .field("draft", &self.core.text())
            .field("caret", &self.core.caret())
            .field("selection", &self.core.selection())
            .field("scroll_offset", &self.scroll_offset)
            .finish()
    }
}

impl fmt::Debug for TextInputState<SecretText> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextInputState<SecretText>")
            .field("phase", &self.phase)
            .field("draft", &"[REDACTED]")
            .field("caret", &self.core.caret())
            .field("selection", &self.core.selection())
            .finish()
    }
}

impl<Mode: TextMode> Drop for TextInputState<Mode> {
    fn drop(&mut self) {
        if Mode::is_secret() {
            let bytes = unsafe { self.snapshot.as_bytes_mut() };
            Secret::zeroize_buffer(bytes);
            self.snapshot.clear();
            self.core.set_text("");
        }
    }
}

/// Single-line controlled text editor.
pub struct TextInput<'a, Mode: TextMode = Plain> {
    pub id: Id,
    pub value_str: &'a str,
    pub secret: Option<&'a Secret>,
    pub revision: Revision,
    pub label: Option<&'a str>,
    pub placeholder: Option<&'a str>,
    pub help: Option<&'a str>,
    pub required: bool,
    pub disabled: bool,
    pub read_only: bool,
    pub validation: Option<&'a ValidationMessage>,
    pub validator: Option<&'a (dyn Validator + 'a)>,
    pub blur: BlurPolicy,
    pub secret_policy: SecretPolicy,
    pub patch: StylePatch,
    pub conflict_policy: ConflictPolicy,
    pub _mode: PhantomData<Mode>,
}

impl<'a> TextInput<'a, Plain> {
    pub fn new(id: Id, value: &'a str, revision: Revision) -> Self {
        Self {
            id,
            value_str: value,
            secret: None,
            revision,
            label: None,
            placeholder: None,
            help: None,
            required: false,
            disabled: false,
            read_only: false,
            validation: None,
            validator: None,
            blur: BlurPolicy::Commit,
            secret_policy: SecretPolicy::none(),
            patch: StylePatch::default(),
            conflict_policy: ConflictPolicy::PreserveDraftAndReport,
            _mode: PhantomData,
        }
    }
}

impl<'a> TextInput<'a, SecretText> {
    pub fn secret(id: Id, value: &'a Secret, revision: Revision) -> Self {
        Self {
            id,
            value_str: "",
            secret: Some(value),
            revision,
            label: None,
            placeholder: None,
            help: None,
            required: false,
            disabled: false,
            read_only: false,
            validation: None,
            validator: None,
            blur: BlurPolicy::Commit,
            secret_policy: SecretPolicy::none(),
            patch: StylePatch::default(),
            conflict_policy: ConflictPolicy::PreserveDraftAndReport,
            _mode: PhantomData,
        }
    }
}

impl<'a, Mode: TextMode> TextInput<'a, Mode> {
    pub fn label(mut self, label: &'a str) -> Self {
        self.label = if label.is_empty() { None } else { Some(label) };
        self
    }

    pub fn placeholder(mut self, placeholder: &'a str) -> Self {
        self.placeholder = if placeholder.is_empty() {
            None
        } else {
            Some(placeholder)
        };
        self
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.help = if help.is_empty() { None } else { Some(help) };
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn validation(mut self, validation: Option<&'a ValidationMessage>) -> Self {
        self.validation = validation;
        self
    }

    pub fn validator(mut self, validator: &'a (dyn Validator + 'a)) -> Self {
        self.validator = Some(validator);
        self
    }

    pub fn blur(mut self, blur: BlurPolicy) -> Self {
        self.blur = blur;
        self
    }

    pub fn secret_policy(mut self, policy: SecretPolicy) -> Self {
        self.secret_policy = policy;
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = patch;
        self
    }

    pub fn conflict_policy(mut self, conflict_policy: ConflictPolicy) -> Self {
        self.conflict_policy = conflict_policy;
        self
    }

    /// Reconcile state with props.
    pub fn reconcile(&self, state: &mut TextInputState<Mode>) {
        if state.last_revision != Some(self.revision) {
            if state.phase == EditPhase::Navigation {
                if !Mode::is_secret() {
                    state.core.set_text(self.value_str);
                    state.snapshot = self.value_str.to_string();
                } else if let Some(sec) = self.secret {
                    sec.expose(|s| {
                        state.core.set_text(s);
                        state.snapshot = s.to_string();
                    });
                }
                state.last_revision = Some(self.revision);
                state.conflict = false;
            } else if self.conflict_policy == ConflictPolicy::PreserveDraftAndReport {
                state.conflict = true;
            } else {
                if !Mode::is_secret() {
                    state.core.set_text(self.value_str);
                    state.snapshot = self.value_str.to_string();
                } else if let Some(sec) = self.secret {
                    sec.expose(|s| {
                        state.core.set_text(s);
                        state.snapshot = s.to_string();
                    });
                }
                state.last_revision = Some(self.revision);
            }
        }
    }

    /// Measure the text input geometry.
    pub fn measure(&self, cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let field = Field::new(self.label.unwrap_or(""))
            .help(self.help.unwrap_or(""))
            .error(self.validation)
            .required(self.required);

        let text_len = width(self.value_str)
            .max(width(self.placeholder.unwrap_or("")))
            .max(10) as u16;
        let child_size = Size::new(text_len.saturating_add(2), 1);
        field.measure(cx, child_size, constraints)
    }

    /// Update text input state and process events.
    pub fn update(
        &self,
        cx: &mut Cx<'_>,
        state: &mut TextInputState<Mode>,
    ) -> Response<TextAction<Mode::Value>> {
        self.reconcile(state);

        let has_focus = cx.has_focus(&self.id);

        if self.disabled {
            state.was_focused = false;
            if has_focus {
                return Response::consumed(self.id.clone())
                    .with_state(VisualState::empty().focused(true).disabled(true));
            }
            return Response::bubble(self.id.clone())
                .with_state(VisualState::empty().disabled(true));
        }

        // Handle blur (focus loss while editing)
        if state.was_focused && !has_focus && state.phase == EditPhase::Editing {
            state.was_focused = false;
            match self.blur {
                BlurPolicy::Commit => {
                    self.validate_text(state);
                    state.phase = EditPhase::Navigation;
                    let val = Mode::to_value(state.core.text());
                    return Response::action(self.id.clone(), TextAction::Commit { value: val })
                        .with_flow(Flow::Consumed)
                        .with_invalidate(Invalidate::Paint);
                }
                BlurPolicy::Cancel => {
                    state.core.set_text(&state.snapshot);
                    state.phase = EditPhase::Navigation;
                    state.error = None;
                    return Response::action(self.id.clone(), TextAction::Cancelled)
                        .with_flow(Flow::Consumed)
                        .with_invalidate(Invalidate::Paint);
                }
                BlurPolicy::KeepDraft => {
                    state.phase = EditPhase::Navigation;
                }
            }
        }
        state.was_focused = has_focus;

        if state.conflict {
            return Response::action(self.id.clone(), TextAction::Conflict)
                .with_flow(Flow::Consumed);
        }

        let cause = cx.cause().clone();

        match cause {
            UpdateCause::Input(Input::Mouse(m), _) => {
                let intended = cx.intended_owner() == Some(&self.id);
                if matches!(m.kind, MouseKind::Up) && intended {
                    cx.request_focus(self.id.clone());
                    if !self.read_only {
                        if state.phase == EditPhase::Navigation {
                            state.snapshot = state.core.text().to_string();
                            state.phase = EditPhase::Editing;
                        }
                        // Click-to-caret mapping
                        let click_x = m.pos.x;
                        let text_start_x = 1; // 1-cell gutter
                        let rel_x = click_x.saturating_sub(text_start_x) as usize;
                        let target_caret = self.col_to_caret(state, rel_x);
                        state.core.set_caret(target_caret);
                    }
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
                }
            }
            UpdateCause::Input(Input::Paste(text), _) => {
                if !self.disabled && !self.read_only {
                    if state.phase == EditPhase::Navigation {
                        state.snapshot = state.core.text().to_string();
                        state.phase = EditPhase::Editing;
                    }
                    // Filter out newlines for single-line input
                    let single_line = text.replace(['\r', '\n'], " ");
                    state.core.insert_str(&single_line);
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::action(self.id.clone(), TextAction::Edited)
                        .with_flow(Flow::Consumed)
                        .with_invalidate(Invalidate::Paint);
                }
            }
            UpdateCause::Input(Input::Key(k), _) if has_focus => {
                if state.phase == EditPhase::Navigation {
                    if (k.code == KeyCode::Enter || k.code == KeyCode::F(2)) && !self.read_only {
                        state.snapshot = state.core.text().to_string();
                        state.phase = EditPhase::Editing;
                        cx.request_invalidate(Invalidate::Paint);
                        return Response::consumed(self.id.clone())
                            .with_invalidate(Invalidate::Paint);
                    }
                } else if state.phase == EditPhase::Editing {
                    match k.code {
                        KeyCode::Enter => {
                            self.validate_text(state);
                            state.phase = EditPhase::Navigation;
                            let val = Mode::to_value(state.core.text());
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(
                                self.id.clone(),
                                TextAction::Commit { value: val },
                            )
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint);
                        }
                        KeyCode::Esc => {
                            state.core.set_text(&state.snapshot);
                            state.phase = EditPhase::Navigation;
                            state.error = None;
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(self.id.clone(), TextAction::Cancelled)
                                .with_flow(Flow::Consumed)
                                .with_invalidate(Invalidate::Paint);
                        }
                        KeyCode::Tab => {
                            self.validate_text(state);
                            state.phase = EditPhase::Navigation;
                            let val = Mode::to_value(state.core.text());
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(
                                self.id.clone(),
                                TextAction::Commit { value: val },
                            )
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint);
                        }
                        KeyCode::Backspace => {
                            if !self.read_only && state.core.delete_backward() {
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::action(self.id.clone(), TextAction::Edited)
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint);
                            }
                        }
                        KeyCode::Delete => {
                            if !self.read_only && state.core.delete_forward() {
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::action(self.id.clone(), TextAction::Edited)
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint);
                            }
                        }
                        KeyCode::Left => {
                            state.core.move_left(k.shift());
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::consumed(self.id.clone())
                                .with_invalidate(Invalidate::Paint);
                        }
                        KeyCode::Right => {
                            state.core.move_right(k.shift());
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::consumed(self.id.clone())
                                .with_invalidate(Invalidate::Paint);
                        }
                        KeyCode::Home => {
                            state.core.move_home(k.shift());
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::consumed(self.id.clone())
                                .with_invalidate(Invalidate::Paint);
                        }
                        KeyCode::End => {
                            state.core.move_end(k.shift());
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::consumed(self.id.clone())
                                .with_invalidate(Invalidate::Paint);
                        }
                        KeyCode::Char(c) if k.ctrl() => match c {
                            'a' => {
                                state.core.select_all();
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::consumed(self.id.clone())
                                    .with_invalidate(Invalidate::Paint);
                            }
                            'u' => {
                                if !self.read_only && state.core.delete_to_line_start() {
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(self.id.clone(), TextAction::Edited)
                                        .with_flow(Flow::Consumed)
                                        .with_invalidate(Invalidate::Paint);
                                }
                            }
                            'k' => {
                                if !self.read_only && state.core.delete_to_line_end() {
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(self.id.clone(), TextAction::Edited)
                                        .with_flow(Flow::Consumed)
                                        .with_invalidate(Invalidate::Paint);
                                }
                            }
                            'w' => {
                                if !self.read_only && state.core.delete_word_left() {
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(self.id.clone(), TextAction::Edited)
                                        .with_flow(Flow::Consumed)
                                        .with_invalidate(Invalidate::Paint);
                                }
                            }
                            'z' => {
                                if !self.read_only && state.core.undo() {
                                    cx.request_invalidate(Invalidate::Paint);
                                    return Response::action(self.id.clone(), TextAction::Edited)
                                        .with_flow(Flow::Consumed)
                                        .with_invalidate(Invalidate::Paint);
                                }
                            }
                            'y' if !self.read_only && state.core.redo() => {
                                cx.request_invalidate(Invalidate::Paint);
                                return Response::action(self.id.clone(), TextAction::Edited)
                                    .with_flow(Flow::Consumed)
                                    .with_invalidate(Invalidate::Paint);
                            }
                            _ => {}
                        },
                        KeyCode::Char(c) if !k.ctrl() && !k.alt() && !self.read_only => {
                            state.core.insert_char(c);
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(self.id.clone(), TextAction::Edited)
                                .with_flow(Flow::Consumed)
                                .with_invalidate(Invalidate::Paint);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        Response::bubble(self.id.clone()).with_state(VisualState::empty().focused(has_focus))
    }

    fn validate_text(&self, state: &mut TextInputState<Mode>) {
        if self.required && state.core.text().is_empty() {
            state.error = Some(ValidationMessage::new("required", "This field is required"));
            return;
        }
        if let Some(validator) = self.validator
            && let Err(msg) = validator.validate(state.core.text())
        {
            state.error = Some(msg);
            return;
        }
        state.error = None;
    }

    fn col_to_caret(&self, state: &TextInputState<Mode>, col: usize) -> usize {
        let text = state.core.text();
        if Mode::is_secret() {
            // In secret mode, 1 cell per grapheme
            let mut byte_idx = 0;
            for (i, (b_idx, _g)) in text.grapheme_indices(true).enumerate() {
                if i == col {
                    return b_idx;
                }
                byte_idx = b_idx + _g.len();
            }
            byte_idx
        } else {
            let mut curr_col = 0;
            let mut byte_idx = 0;
            for (b_idx, g) in text.grapheme_indices(true) {
                let g_w = UnicodeWidthStr::width(g);
                if curr_col + g_w > col {
                    return b_idx;
                }
                curr_col += g_w;
                byte_idx = b_idx + g.len();
            }
            byte_idx
        }
    }

    /// Render the text input component.
    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &TextInputState<Mode>) -> Rect {
        if area.is_empty() {
            return Rect::zero();
        }

        let theme = ui.theme;
        let has_focus = ui.is_focused(&self.id);

        let err = self.validation.or(state.error.as_ref());
        let field = Field::new(self.label.unwrap_or(""))
            .help(self.help.unwrap_or(""))
            .error(err)
            .required(self.required);

        field.draw(ui, area, |ui, child_area| {
            if child_area.is_empty() {
                return;
            }

            let bg_color = if self.disabled {
                theme.tokens.surface
            } else if state.phase == EditPhase::Editing {
                theme.tokens.field
            } else {
                theme.tokens.surface
            };

            ui.fill_rect(child_area, Style::new().bg(bg_color));

            // Focus gutter (1 column)
            let gutter_sym = if has_focus { "▎" } else { " " };
            let gutter_style = if has_focus {
                Style::new().fg(theme.tokens.focus).bg(bg_color)
            } else {
                Style::new().fg(theme.tokens.text_muted).bg(bg_color)
            };
            ui.set_string(child_area.x, child_area.y, gutter_sym, gutter_style);

            let text_x = child_area.x.saturating_add(1);
            let avail_w = child_area.width.saturating_sub(1) as usize;

            let text = state.core.text();

            if text.is_empty() && state.phase == EditPhase::Navigation {
                if let Some(placeholder) = self.placeholder {
                    let ph_style = Style::new().fg(theme.tokens.text_muted).bg(bg_color);
                    ui.set_string(text_x, child_area.y, placeholder, ph_style);
                }
            } else if Mode::is_secret() {
                // Secret bullet masking
                let graphemes: Vec<&str> = text.graphemes(true).collect();
                let total_g = graphemes.len();
                let mut masked = String::new();
                for (i, g) in graphemes.iter().enumerate() {
                    if state.phase == EditPhase::Navigation
                        && self.secret_policy.reveal_tail > 0
                        && i + (self.secret_policy.reveal_tail as usize) >= total_g
                    {
                        masked.push_str(g);
                    } else {
                        masked.push('●');
                    }
                }
                let text_style = if self.disabled {
                    Style::new().fg(theme.tokens.text_muted).bg(bg_color)
                } else {
                    Style::new().fg(theme.tokens.text_primary).bg(bg_color)
                };
                ui.set_string(text_x, child_area.y, &masked, text_style);
            } else {
                let text_style = if self.disabled {
                    Style::new().fg(theme.tokens.text_muted).bg(bg_color)
                } else {
                    Style::new().fg(theme.tokens.text_primary).bg(bg_color)
                };
                ui.set_string(text_x, child_area.y, text, text_style);
            }

            // Caret cursor if editing and focused
            if has_focus && state.phase == EditPhase::Editing {
                let caret_col = if Mode::is_secret() {
                    let text = state.core.text();
                    text[..state.core.caret().min(text.len())]
                        .graphemes(true)
                        .count()
                } else {
                    let text = state.core.text();
                    width(&text[..state.core.caret().min(text.len())])
                };
                let cx_pos = text_x.saturating_add(caret_col as u16);
                if (cx_pos as usize) < (text_x as usize) + avail_w {
                    ui.request_cursor(
                        self.id.clone(),
                        crate::termrock::layout::Position::new(cx_pos, child_area.y),
                    );
                }
            }

            // Register hit region and focus candidate
            ui.register_hit(self.id.clone(), child_area);
            ui.register_focus(self.id.clone(), !self.disabled);
        });

        area
    }
}
