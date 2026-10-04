//! Termrock static Props display-only component, PropsRow, PropsValue, and two-column layout.

use ratatui::style::Style;

use crate::termrock::author::StyledText;
use crate::termrock::identity::{Id, ItemKey, Part};
use crate::termrock::layout::{Constraints, Rect, Size};
use crate::termrock::runtime::{MeasureCx, Ui};
use crate::termrock::text::width;
use crate::termrock::theme::{Role, StylePatch, Tone};

/// Display value for a property row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropsValue<'a> {
    Text(&'a str),
    Styled(StyledText<'a>),
    Empty,
    Protected(&'a str),
}

impl<'a> PropsValue<'a> {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Text(s) => s,
            Self::Styled(st) => st.text,
            Self::Empty => "",
            Self::Protected(s) => s,
        }
    }

    pub fn is_protected(&self) -> bool {
        matches!(self, Self::Protected(_))
    }
}

/// Specification for a single row in a property sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropsRow<'a> {
    pub key: ItemKey,
    pub label: &'a str,
    pub value: PropsValue<'a>,
    pub tone: Tone,
    pub wrap: bool,
    pub copyable: bool,
}

impl<'a> PropsRow<'a> {
    pub fn new(key: ItemKey, label: &'a str, value: PropsValue<'a>) -> Self {
        Self {
            key,
            label,
            value,
            tone: Tone::Normal,
            wrap: false,
            copyable: false,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn copyable(mut self, copyable: bool) -> Self {
        self.copyable = copyable;
        self
    }
}

/// Policy for sizing the label column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LabelWidth {
    #[default]
    Auto,
    Exact(u16),
    Max(u16),
}

impl LabelWidth {
    pub fn resolve(&self, rows: &[PropsRow<'_>]) -> u16 {
        let max_label = rows.iter().map(|r| width(r.label)).max().unwrap_or(0) as u16;
        let natural = max_label.saturating_add(2);

        match self {
            Self::Auto => natural,
            Self::Exact(w) => *w,
            Self::Max(m) => natural.min(*m),
        }
    }
}

/// Static display-only property sheet.
///
/// Has NO update method because the surface has no durable interaction.
#[derive(Debug, Clone)]
pub struct Props<'a> {
    pub id: Id,
    pub rows: &'a [PropsRow<'a>],
    pub label_width_policy: LabelWidth,
    pub patch: Option<StylePatch>,
    pub part_patches: Vec<(Part, StylePatch)>,
}

impl<'a> Props<'a> {
    pub fn new(id: Id, rows: &'a [PropsRow<'a>]) -> Self {
        Self {
            id,
            rows,
            label_width_policy: LabelWidth::Auto,
            patch: None,
            part_patches: Vec::new(),
        }
    }

    pub fn label_width(mut self, width: LabelWidth) -> Self {
        self.label_width_policy = width;
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

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let label_w = self.label_width_policy.resolve(self.rows);
        let avail_w = constraints.max.width.saturating_sub(label_w) as usize;
        let mut total_lines: u16 = 0;

        for row in self.rows {
            if row.wrap && avail_w >= 4 {
                let lines = crate::ui::text::wrap(row.value.as_str(), avail_w);
                total_lines = total_lines.saturating_add(lines.len().max(1) as u16);
            } else {
                total_lines = total_lines.saturating_add(1);
            }
        }

        let needed = Size::new(constraints.max.width, total_lines);
        constraints.clamp(needed)
    }

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect {
        if area.is_empty() {
            return Rect::zero();
        }

        // Static display-only: NEVER register focus or hit testing!
        let label_w = self.label_width_policy.resolve(self.rows);
        let avail_val_w = (area.width.saturating_sub(label_w)) as usize;
        let theme = ui.theme;
        let bg_color = theme.resolve_role(Role::Background, ui.current_surface);
        let muted_fg = theme.resolve_role(Role::Muted, ui.current_surface);

        let mut y = area.y;
        let bottom = area.y.saturating_add(area.height);

        for row in self.rows {
            if y >= bottom {
                break;
            }

            let val_color = theme.tone_color(row.tone);
            let val_style = Style::new().fg(val_color).bg(bg_color);
            let label_style = Style::new().fg(muted_fg).bg(bg_color);

            if row.wrap && avail_val_w >= 4 {
                let lines = crate::ui::text::wrap(row.value.as_str(), avail_val_w);
                for (i, line) in lines.into_iter().enumerate() {
                    if y >= bottom {
                        break;
                    }
                    if i == 0 {
                        ui.set_string(area.x, y, row.label, label_style);
                        ui.part(
                            self.id.child(row.key),
                            Part::LABEL,
                            Rect::new(area.x, y, width(row.label) as u16, 1),
                            |_| {},
                        );
                    }
                    ui.set_string(area.x.saturating_add(label_w), y, &line, val_style);
                    ui.part(
                        self.id.child(row.key),
                        Part::new("value"),
                        Rect::new(area.x.saturating_add(label_w), y, width(&line) as u16, 1),
                        |_| {},
                    );
                    y += 1;
                }
            } else {
                ui.set_string(area.x, y, row.label, label_style);
                ui.part(
                    self.id.child(row.key),
                    Part::LABEL,
                    Rect::new(area.x, y, width(row.label) as u16, 1),
                    |_| {},
                );

                let val_str = crate::termrock::text::truncate(row.value.as_str(), avail_val_w);
                ui.set_string(area.x.saturating_add(label_w), y, &val_str, val_style);
                ui.part(
                    self.id.child(row.key),
                    Part::new("value"),
                    Rect::new(area.x.saturating_add(label_w), y, width(&val_str) as u16, 1),
                    |_| {},
                );
                y += 1;
            }
        }

        Rect::new(area.x, area.y, area.width, (y - area.y).min(area.height))
    }
}
