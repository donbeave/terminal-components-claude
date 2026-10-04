//! Shared form chrome wrapper.
//!
//! Owns label, required indicator, help and error rows, and child allocation.
//! Does not own focus, hit detection, or editing state.

use ratatui::style::Style;

use crate::termrock::identity::Id;
use crate::termrock::layout::{Constraints, Rect, Size};
use crate::termrock::runtime::{MeasureCx, Ui};
use crate::termrock::secret::ValidationMessage;
use crate::termrock::text::width;
use crate::termrock::theme::StylePatch;

/// Chrome wrapper for form inputs and fields.
#[derive(Debug, Clone)]
pub struct Field<'a> {
    pub label: Option<&'a str>,
    pub help: Option<&'a str>,
    pub error: Option<&'a ValidationMessage>,
    pub required: bool,
    pub plain_label: bool,
    pub child_id: Option<Id>,
    pub patch: StylePatch,
}

impl<'a> Field<'a> {
    pub fn new(label: &'a str) -> Self {
        Self {
            label: if label.is_empty() { None } else { Some(label) },
            help: None,
            error: None,
            required: false,
            plain_label: false,
            child_id: None,
            patch: StylePatch::default(),
        }
    }

    pub fn help(mut self, help: &'a str) -> Self {
        self.help = if help.is_empty() { None } else { Some(help) };
        self
    }

    pub fn error(mut self, error: Option<&'a ValidationMessage>) -> Self {
        self.error = error;
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn plain_label(mut self, plain_label: bool) -> Self {
        self.plain_label = plain_label;
        self
    }

    pub fn child_id(mut self, child_id: Id) -> Self {
        self.child_id = Some(child_id);
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = patch;
        self
    }

    /// Measure the total field size wrapping the child size.
    pub fn measure(&self, _cx: &MeasureCx<'_>, child_size: Size, constraints: Constraints) -> Size {
        let label_h = if self.label.is_some() { 1 } else { 0 };
        let footer_h = if self.error.is_some() || self.help.is_some() {
            1
        } else {
            0
        };
        let total_h = child_size
            .height
            .saturating_add(label_h)
            .saturating_add(footer_h);

        let label_w = if let Some(lbl) = self.label {
            let base_w = width(lbl) as u16;
            if self.required {
                base_w.saturating_add(2)
            } else {
                base_w
            }
        } else {
            0
        };

        let footer_w = if let Some(err) = self.error {
            width(&err.display) as u16
        } else if let Some(hlp) = self.help {
            width(hlp) as u16
        } else {
            0
        };

        let total_w = label_w.max(child_size.width).max(footer_w);
        constraints.clamp(Size::new(total_w, total_h))
    }

    /// Draw the field chrome around the child painter.
    pub fn draw<R>(
        &self,
        ui: &mut Ui<'_>,
        area: Rect,
        painter: impl FnOnce(&mut Ui<'_>, Rect) -> R,
    ) -> R {
        if area.is_empty() {
            return painter(ui, Rect::zero());
        }

        let theme = ui.theme;
        let mut curr_y = area.y;
        let mut rem_h = area.height;

        // 1. Draw Label if present
        if let Some(lbl) = self.label
            && rem_h >= 1
        {
            let lbl_style = Style::new().fg(theme.tokens.text_primary);
            ui.set_string(area.x, curr_y, lbl, lbl_style);
            if self.required {
                let lx = area.x.saturating_add(width(lbl) as u16);
                ui.set_string(lx, curr_y, " *", Style::new().fg(theme.tokens.danger));
            }
            curr_y = curr_y.saturating_add(1);
            rem_h = rem_h.saturating_sub(1);
        }

        // Check if footer (error or help) is present
        let has_footer = self.error.is_some() || self.help.is_some();
        let reserve_footer = has_footer && rem_h > 1;

        let child_h = if reserve_footer {
            rem_h.saturating_sub(1)
        } else {
            rem_h
        };

        let child_area = Rect::new(area.x, curr_y, area.width, child_h);
        let result = painter(ui, child_area);

        curr_y = curr_y.saturating_add(child_h);
        rem_h = rem_h.saturating_sub(child_h);

        // 2. Draw Help or Error row in the footer
        if rem_h >= 1 {
            if let Some(err) = self.error {
                let err_style = Style::new().fg(theme.tokens.danger);
                ui.set_string(area.x, curr_y, &err.display, err_style);
            } else if let Some(hlp) = self.help {
                let help_style = Style::new().fg(theme.tokens.text_muted);
                ui.set_string(area.x, curr_y, hlp, help_style);
            }
        }

        result
    }
}
