//! Termrock Empty state component, Readiness lifecycle, and child action button.

use ratatui::style::{Modifier, Style};

use crate::termrock::button::Button;
use crate::termrock::identity::{ActionKey, Id, Part};
use crate::termrock::layout::{Constraints, Rect, Size};
use crate::termrock::response::{ActivationOrigin, Flow, Invalidate, Response};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::text::width;
use crate::termrock::theme::{ColorLevel, Role, StylePatch};

/// Readiness state for an empty view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness<'a> {
    Empty,
    Loading,
    Partial,
    Error(&'a str),
}

/// Action metadata for an empty state retry or primary action affordance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionMeta<'a> {
    pub key: ActionKey,
    pub label: &'a str,
}

impl<'a> ActionMeta<'a> {
    pub const fn new(key: ActionKey, label: &'a str) -> Self {
        Self { key, label }
    }
}

/// Semantic action emitted by `Empty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmptyAction {
    Invoke {
        action: ActionKey,
        origin: ActivationOrigin,
    },
}

/// Empty state readiness presentation with optional child button action.
#[derive(Debug, Clone)]
pub struct Empty<'a> {
    pub id: Id,
    pub state: Readiness<'a>,
    pub title: Option<&'a str>,
    pub detail: Option<&'a str>,
    pub action: Option<ActionMeta<'a>>,
    pub patch: Option<StylePatch>,
}

impl<'a> Empty<'a> {
    pub fn new(id: Id, state: Readiness<'a>) -> Self {
        Self {
            id,
            state,
            title: None,
            detail: None,
            action: None,
            patch: None,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn detail(mut self, detail: &'a str) -> Self {
        self.detail = Some(detail);
        self
    }

    pub fn action(mut self, action: Option<ActionMeta<'a>>) -> Self {
        self.action = action;
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = Some(patch);
        self
    }

    pub fn update(&self, cx: &mut Cx<'_>) -> Response<EmptyAction> {
        if let Some(act) = self.action {
            let child_btn = Button::new(self.id.sub("action"), act.label);
            let resp = child_btn.update(cx);

            if let Some(activated) = resp.action {
                return Response::action(
                    self.id.clone(),
                    EmptyAction::Invoke {
                        action: act.key,
                        origin: activated.origin,
                    },
                )
                .with_flow(Flow::Consumed)
                .with_invalidate(Invalidate::Paint);
            }

            Response::new(self.id.clone())
                .with_flow(resp.flow)
                .with_invalidate(resp.invalidate)
                .with_state(resp.state)
        } else {
            // Message-only: NEVER registers focus, hit, or emits actions
            Response::bubble(self.id.clone())
        }
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let mut height: u16 = 1; // title
        if let Some(d) = self.detail {
            let lines =
                crate::ui::text::wrap(d, constraints.max.width.saturating_sub(4).max(8) as usize);
            height = height.saturating_add(1 + lines.len() as u16);
        }
        if self.action.is_some() {
            height = height.saturating_add(2); // gap + button
        }
        constraints.clamp(Size::new(constraints.max.width, height))
    }

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect) -> Rect {
        if area.is_empty() {
            return Rect::zero();
        }

        let theme = ui.theme;
        let bg_color = theme.resolve_role(Role::Background, ui.current_surface);

        let detail_lines: Vec<String> = self
            .detail
            .map(|d| crate::ui::text::wrap(d, area.width.saturating_sub(4).max(8) as usize))
            .unwrap_or_default();

        let mut total_h: u16 = 1;
        if !detail_lines.is_empty() {
            total_h = total_h.saturating_add(1 + detail_lines.len() as u16);
        }
        if self.action.is_some() {
            total_h = total_h.saturating_add(2);
        }

        let y0 = area.y + area.height.saturating_sub(total_h) / 2;
        let mut curr_y = y0;

        // Render Title
        let title_text = match (self.title, self.state) {
            (Some(t), _) => t.to_string(),
            (None, Readiness::Empty) => "No items".to_string(),
            (None, Readiness::Loading) => "Loading…".to_string(),
            (None, Readiness::Partial) => "Partial results".to_string(),
            (None, Readiness::Error(e)) => e.to_string(),
        };

        if curr_y < area.y.saturating_add(area.height) {
            match self.state {
                Readiness::Error(_) => {
                    let full_title = format!("! {title_text}");
                    let s = crate::termrock::text::truncate(&full_title, area.width as usize);
                    let title_w = width(&s) as u16;
                    let x = area.x + area.width.saturating_sub(title_w) / 2;
                    let err_style = Style::new().fg(theme.tokens.danger).bg(bg_color);
                    ui.set_string(x, curr_y, &s, err_style);
                    // Bold '!' marker
                    ui.set_string(x, curr_y, "!", err_style.add_modifier(Modifier::BOLD));
                    ui.part(
                        self.id.clone(),
                        Part::new("title"),
                        Rect::new(x, curr_y, title_w, 1),
                        |_| {},
                    );
                }
                Readiness::Loading => {
                    let full_title = format!("⠋ {title_text}");
                    let s = crate::termrock::text::truncate(&full_title, area.width as usize);
                    let title_w = width(&s) as u16;
                    let x = area.x + area.width.saturating_sub(title_w) / 2;
                    let load_style = Style::new().fg(theme.tokens.accent).bg(bg_color);
                    ui.set_string(x, curr_y, &s, load_style);
                    ui.part(
                        self.id.clone(),
                        Part::new("title"),
                        Rect::new(x, curr_y, title_w, 1),
                        |_| {},
                    );
                }
                _ => {
                    let s = crate::termrock::text::truncate(&title_text, area.width as usize);
                    let title_w = width(&s) as u16;
                    let x = area.x + area.width.saturating_sub(title_w) / 2;
                    let title_style = Style::new().fg(theme.tokens.text_muted).bg(bg_color);
                    ui.set_string(x, curr_y, &s, title_style);
                    ui.part(
                        self.id.clone(),
                        Part::new("title"),
                        Rect::new(x, curr_y, title_w, 1),
                        |_| {},
                    );
                }
            }
            curr_y += 1;
        }

        // Render Detail lines
        if !detail_lines.is_empty() {
            curr_y += 1; // blank line
            for line in &detail_lines {
                if curr_y >= area.y.saturating_add(area.height) {
                    break;
                }
                let line_str = crate::termrock::text::truncate(line, area.width as usize);
                let line_w = width(&line_str) as u16;
                let x = area.x + area.width.saturating_sub(line_w) / 2;
                let faint_style = Style::new().fg(theme.tokens.text_faint).bg(bg_color);
                ui.set_string(x, curr_y, &line_str, faint_style);
                ui.part(
                    self.id.clone(),
                    Part::new("detail"),
                    Rect::new(x, curr_y, line_w, 1),
                    |_| {},
                );
                curr_y += 1;
            }
        }

        // Render Action Button
        if let Some(act) = self.action {
            curr_y += 1; // blank line
            if curr_y < area.y.saturating_add(area.height) {
                let btn = Button::new(self.id.sub("action"), act.label);
                let measure_cx = MeasureCx::new(
                    Constraints::loose(Size::new(area.width, 1)),
                    ui.theme,
                    ColorLevel::TrueColor,
                );
                let btn_size =
                    btn.measure(&measure_cx, Constraints::loose(Size::new(area.width, 1)));
                let btn_w = btn_size.width.min(area.width);
                let btn_x = area.x + area.width.saturating_sub(btn_w) / 2;
                let btn_rect = Rect::new(btn_x, curr_y, btn_w, 1);
                btn.draw(ui, btn_rect);
            }
        }

        area
    }
}
