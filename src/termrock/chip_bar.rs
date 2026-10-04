//! Termrock ChipBar control, ChipItem, typed actions, and keyboard/pointer navigation.

use ratatui::crossterm::event::KeyCode;
use ratatui::style::{Modifier, Style};

use crate::termrock::identity::{Id, ItemKey, Part, Revision};
use crate::termrock::layout::{Constraints, Rect, Size};
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::text::width;
use crate::termrock::theme::{Role, StylePatch};

/// An individual chip descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipItem<'a> {
    pub key: ItemKey,
    pub label: &'a str,
    pub enabled: bool,
    pub closable: bool,
    pub checked: Option<bool>,
    pub error: bool,
}

impl<'a> ChipItem<'a> {
    pub fn new(key: ItemKey, label: &'a str) -> Self {
        Self {
            key,
            label,
            enabled: true,
            closable: true,
            checked: None,
            error: false,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    pub fn checked(mut self, checked: Option<bool>) -> Self {
        self.checked = checked;
        self
    }

    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }
}

/// Semantic actions emitted by `ChipBar`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChipAction {
    Activate {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    SetChecked {
        key: ItemKey,
        checked: bool,
        origin: ActivationOrigin,
    },
    Close {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    Add {
        origin: ActivationOrigin,
    },
    Lead {
        origin: ActivationOrigin,
    },
    ClearAll {
        origin: ActivationOrigin,
    },
}

/// Caller-owned durable state for `ChipBar`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChipBarState {
    pub cursor: usize,
    pub scroll_offset: usize,
}

impl ChipBarState {
    pub fn new() -> Self {
        Self {
            cursor: 0,
            scroll_offset: 0,
        }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
    }

    pub fn reconcile(&mut self, chip_count: usize, has_add: bool) {
        let stops = chip_count + usize::from(has_add);
        if stops == 0 {
            self.cursor = 0;
        } else if self.cursor >= stops {
            self.cursor = stops - 1;
        }
    }
}

/// A horizontal keyed strip of chips with shared focus and disjoint hit targets.
#[derive(Debug, Clone)]
pub struct ChipBar<'a> {
    pub id: Id,
    pub chips: &'a [ChipItem<'a>],
    pub revision: Revision,
    pub add_action: Option<&'a str>,
    pub lead: Option<&'a str>,
    pub disabled: bool,
    pub patch: Option<StylePatch>,
}

impl<'a> ChipBar<'a> {
    pub fn new(id: Id, chips: &'a [ChipItem<'a>], revision: Revision) -> Self {
        Self {
            id,
            chips,
            revision,
            add_action: Some("+ Add filter"),
            lead: None,
            disabled: false,
            patch: None,
        }
    }

    pub fn add_action(mut self, action: Option<&'a str>) -> Self {
        self.add_action = action;
        self
    }

    pub fn lead(mut self, lead: Option<&'a str>) -> Self {
        self.lead = lead;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = Some(patch);
        self
    }

    pub fn chip_id(&self, key: ItemKey) -> Id {
        self.id.child(key)
    }

    pub fn close_id(&self, key: ItemKey) -> Id {
        self.id.child(key).sub("close")
    }

    pub fn add_id(&self) -> Id {
        self.id.sub("add")
    }

    pub fn lead_id(&self) -> Id {
        self.id.sub("lead")
    }

    pub fn update(&self, cx: &mut Cx<'_>, state: &mut ChipBarState) -> Response<ChipAction> {
        state.reconcile(self.chips.len(), self.add_action.is_some());
        let has_focus = cx.has_focus(&self.id);

        if self.disabled {
            if has_focus && matches!(cx.cause(), UpdateCause::Input(Input::Key(_), _)) {
                return Response::consumed(self.id.clone())
                    .with_state(VisualState::empty().focused(true).disabled(true));
            }
            return Response::bubble(self.id.clone())
                .with_state(VisualState::empty().focused(has_focus).disabled(true));
        }

        let total_stops = self.chips.len() + usize::from(self.add_action.is_some());

        // Handle keyboard navigation
        let cause = cx.cause().clone();
        match cause {
            UpdateCause::Input(Input::Key(k), _) if has_focus => match k.code {
                KeyCode::Left | KeyCode::Char('h') if !k.ctrl() => {
                    state.cursor = state.cursor.saturating_sub(1);
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
                }
                KeyCode::Right | KeyCode::Char('l') if !k.ctrl() => {
                    if total_stops > 0 {
                        state.cursor = (state.cursor + 1).min(total_stops - 1);
                    }
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::consumed(self.id.clone()).with_invalidate(Invalidate::Paint);
                }
                KeyCode::Enter => {
                    cx.request_invalidate(Invalidate::Paint);
                    if state.cursor < self.chips.len() {
                        let chip = &self.chips[state.cursor];
                        if chip.enabled {
                            return Response::action(
                                self.id.clone(),
                                ChipAction::Activate {
                                    key: chip.key,
                                    origin: ActivationOrigin::Keyboard,
                                },
                            )
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint);
                        }
                    } else if self.add_action.is_some() {
                        return Response::action(
                            self.id.clone(),
                            ChipAction::Add {
                                origin: ActivationOrigin::Keyboard,
                            },
                        )
                        .with_flow(Flow::Consumed)
                        .with_invalidate(Invalidate::Paint);
                    }
                }
                KeyCode::Char(' ') => {
                    if state.cursor < self.chips.len() {
                        let chip = &self.chips[state.cursor];
                        if chip.enabled {
                            cx.request_invalidate(Invalidate::Paint);
                            if let Some(checked) = chip.checked {
                                return Response::action(
                                    self.id.clone(),
                                    ChipAction::SetChecked {
                                        key: chip.key,
                                        checked: !checked,
                                        origin: ActivationOrigin::Keyboard,
                                    },
                                )
                                .with_flow(Flow::Consumed)
                                .with_invalidate(Invalidate::Paint);
                            } else {
                                return Response::action(
                                    self.id.clone(),
                                    ChipAction::Activate {
                                        key: chip.key,
                                        origin: ActivationOrigin::Keyboard,
                                    },
                                )
                                .with_flow(Flow::Consumed)
                                .with_invalidate(Invalidate::Paint);
                            }
                        }
                    }
                }
                KeyCode::Delete | KeyCode::Backspace | KeyCode::Char('x') => {
                    if state.cursor < self.chips.len() {
                        let chip = &self.chips[state.cursor];
                        if chip.enabled && chip.closable {
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(
                                self.id.clone(),
                                ChipAction::Close {
                                    key: chip.key,
                                    origin: ActivationOrigin::Keyboard,
                                },
                            )
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint);
                        }
                    }
                }
                KeyCode::Char('+') => {
                    if self.add_action.is_some() {
                        cx.request_invalidate(Invalidate::Paint);
                        return Response::action(
                            self.id.clone(),
                            ChipAction::Add {
                                origin: ActivationOrigin::Keyboard,
                            },
                        )
                        .with_flow(Flow::Consumed)
                        .with_invalidate(Invalidate::Paint);
                    }
                }
                KeyCode::Char('X') => {
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::action(
                        self.id.clone(),
                        ChipAction::ClearAll {
                            origin: ActivationOrigin::Keyboard,
                        },
                    )
                    .with_flow(Flow::Consumed)
                    .with_invalidate(Invalidate::Paint);
                }
                _ => {}
            },
            _ => {}
        }

        // Handle pointer targeting
        match (cx.cause(), cx.intended_owner()) {
            (UpdateCause::Input(Input::Mouse(m), _), Some(target))
                if matches!(m.kind, MouseKind::Up) =>
            {
                if *target == self.lead_id() {
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::action(
                        self.id.clone(),
                        ChipAction::Lead {
                            origin: ActivationOrigin::Pointer,
                        },
                    )
                    .with_flow(Flow::Consumed)
                    .with_invalidate(Invalidate::Paint);
                } else if *target == self.add_id() {
                    state.cursor = self.chips.len();
                    cx.request_invalidate(Invalidate::Paint);
                    return Response::action(
                        self.id.clone(),
                        ChipAction::Add {
                            origin: ActivationOrigin::Pointer,
                        },
                    )
                    .with_flow(Flow::Consumed)
                    .with_invalidate(Invalidate::Paint);
                } else {
                    for (i, chip) in self.chips.iter().enumerate() {
                        if *target == self.close_id(chip.key) && chip.enabled && chip.closable {
                            cx.request_invalidate(Invalidate::Paint);
                            // Close click NEVER activates chip body
                            return Response::action(
                                self.id.clone(),
                                ChipAction::Close {
                                    key: chip.key,
                                    origin: ActivationOrigin::Pointer,
                                },
                            )
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint);
                        } else if *target == self.chip_id(chip.key) && chip.enabled {
                            state.cursor = i;
                            cx.request_invalidate(Invalidate::Paint);
                            return Response::action(
                                self.id.clone(),
                                ChipAction::Activate {
                                    key: chip.key,
                                    origin: ActivationOrigin::Pointer,
                                },
                            )
                            .with_flow(Flow::Consumed)
                            .with_invalidate(Invalidate::Paint);
                        }
                    }
                }
            }
            _ => {}
        }

        Response::bubble(self.id.clone()).with_state(VisualState::empty().focused(has_focus))
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let mut total_w: u16 = 0;

        if let Some(lead) = self.lead {
            total_w = total_w.saturating_add(width(lead) as u16 + 2 + 1); // lead + padding + gap
        }

        for chip in self.chips {
            let label_w = width(chip.label) as u16;
            let close_w = if chip.closable { 2 } else { 0 };
            let marker_w = if chip.checked.is_some() { 2 } else { 0 };
            let chip_w = 1 + label_w + 1 + marker_w + close_w + 1; // gutter + label + marker + close + pad
            total_w = total_w.saturating_add(chip_w + 1); // +1 gap
        }

        if let Some(add) = self.add_action {
            total_w = total_w.saturating_add(width(add) as u16 + 2);
        }

        constraints.clamp(Size::new(total_w, 1))
    }

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &ChipBarState) -> Rect {
        if area.is_empty() {
            return Rect::zero();
        }

        ui.register_focus(self.id.clone(), !self.disabled);

        let focused = ui.is_focused(&self.id);
        let theme = ui.theme;
        let bg_color = theme.resolve_role(Role::Background, ui.current_surface);
        let mut curr_x = area.x;
        let right_limit = area.x.saturating_add(area.width);

        // Lead affordance
        if let Some(lead) = self.lead {
            let lead_id = self.lead_id();
            let text = format!(" {lead} ");
            let lead_w = width(&text) as u16;
            if curr_x.saturating_add(lead_w) <= right_limit {
                let rect = Rect::new(curr_x, area.y, lead_w, 1);
                ui.register_hit(lead_id.clone(), rect);
                let hovered = ui.is_hovered(&lead_id);
                let st = if hovered {
                    Style::new()
                        .fg(theme.tokens.text_primary)
                        .bg(theme.tokens.surface_elevated)
                } else {
                    Style::new().fg(theme.tokens.text_muted).bg(bg_color)
                };
                ui.set_string(curr_x, area.y, &text, st);
                ui.part(lead_id, Part::new("lead"), rect, |_| {});
                curr_x = curr_x.saturating_add(lead_w + 1);
            }
        }

        // Chips
        for (i, chip) in self.chips.iter().enumerate() {
            let cid = self.chip_id(chip.key);
            let xid = self.close_id(chip.key);
            let label_w = width(chip.label) as u16;
            let close_w = if chip.closable { 2 } else { 0 };
            let marker_w = if chip.checked.is_some() { 2 } else { 0 };
            let chip_w = 1 + label_w + 1 + marker_w + close_w + 1;

            if curr_x.saturating_add(chip_w) > right_limit {
                if curr_x < right_limit {
                    ui.set_string(
                        curr_x,
                        area.y,
                        "…",
                        Style::new().fg(theme.tokens.text_muted).bg(bg_color),
                    );
                    ui.part(
                        self.id.clone(),
                        Part::new("overflow"),
                        Rect::new(curr_x, area.y, 1, 1),
                        |_| {},
                    );
                }
                return Rect::new(area.x, area.y, area.width, 1.min(area.height));
            }

            let chip_rect = Rect::new(curr_x, area.y, chip_w, 1);
            if chip.enabled && !self.disabled {
                ui.register_hit(cid.clone(), chip_rect);
            }

            let is_cursor = focused && i == state.cursor;
            let is_hovered = ui.is_hovered(&cid) || ui.is_hovered(&xid);

            let mut st = Style::new()
                .fg(theme.tokens.text_primary)
                .bg(theme.tokens.surface_overlay);
            if is_hovered {
                st = st.bg(theme.tokens.popover);
            }
            if is_cursor {
                st = st.add_modifier(Modifier::BOLD);
            }
            if !chip.enabled || self.disabled {
                st = Style::new()
                    .fg(theme.tokens.disabled)
                    .bg(theme.tokens.surface_overlay);
            } else if chip.error {
                st = st.fg(theme.tokens.danger);
            }

            ui.fill_rect(chip_rect, st);
            ui.part(cid.clone(), Part::new("container"), chip_rect, |_| {});

            // Focus gutter
            let gutter_sym = if is_cursor && chip.enabled && !self.disabled {
                "▎"
            } else {
                " "
            };
            let gutter_style = Style::new()
                .fg(theme.tokens.focus)
                .bg(st.bg.unwrap_or(bg_color));
            ui.set_string(curr_x, area.y, gutter_sym, gutter_style);
            ui.part(
                cid.clone(),
                Part::new("gutter"),
                Rect::new(curr_x, area.y, 1, 1),
                |_| {},
            );

            let mut draw_x = curr_x.saturating_add(1);

            // Checked marker
            if let Some(checked) = chip.checked {
                let m_sym = if checked { "● " } else { "○ " };
                let m_style = if chip.enabled && !self.disabled {
                    st.fg(if checked {
                        theme.tokens.accent
                    } else {
                        theme.tokens.text_muted
                    })
                } else {
                    st
                };
                ui.set_string(draw_x, area.y, m_sym, m_style);
                ui.part(
                    cid.clone(),
                    Part::new("marker"),
                    Rect::new(draw_x, area.y, 2, 1),
                    |_| {},
                );
                draw_x = draw_x.saturating_add(2);
            }

            // Label
            ui.set_string(draw_x, area.y, chip.label, st);
            ui.part(
                cid.clone(),
                Part::LABEL,
                Rect::new(draw_x, area.y, label_w, 1),
                |_| {},
            );
            draw_x = draw_x.saturating_add(label_w);

            // Close affordance
            if chip.closable {
                let close_x = draw_x.saturating_add(1);
                let close_rect = Rect::new(close_x, area.y, 1, 1);
                if chip.enabled && !self.disabled {
                    ui.register_hit(xid.clone(), close_rect);
                }
                let close_hover = ui.is_hovered(&xid);
                let close_style = if close_hover {
                    st.fg(theme.tokens.text_primary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    st.fg(theme.tokens.text_muted)
                };
                ui.set_string(close_x, area.y, "×", close_style);
                ui.part(xid, Part::new("close"), close_rect, |_| {});
            }

            curr_x = curr_x.saturating_add(chip_w + 1);
        }

        // Add affordance
        if let Some(add) = self.add_action {
            let aid = self.add_id();
            let add_w = width(add) as u16 + 2;
            if curr_x.saturating_add(add_w) <= right_limit {
                let add_rect = Rect::new(curr_x, area.y, add_w, 1);
                if !self.disabled {
                    ui.register_hit(aid.clone(), add_rect);
                }
                let is_cursor = focused && state.cursor == self.chips.len();
                let is_hover = ui.is_hovered(&aid);
                let mut st = Style::new().fg(theme.tokens.text_secondary).bg(bg_color);
                if is_hover {
                    st = st
                        .fg(theme.tokens.text_primary)
                        .bg(theme.tokens.surface_elevated);
                }
                if is_cursor {
                    st = st
                        .fg(theme.tokens.text_primary)
                        .add_modifier(Modifier::BOLD);
                }
                ui.fill_rect(add_rect, st);
                let gutter_sym = if is_cursor && !self.disabled {
                    "▎"
                } else {
                    " "
                };
                ui.set_string(
                    curr_x,
                    area.y,
                    gutter_sym,
                    Style::new().fg(theme.tokens.focus).bg(bg_color),
                );
                ui.set_string(curr_x.saturating_add(1), area.y, add, st);
                ui.part(aid, Part::new("add"), add_rect, |_| {});
            }
        }

        Rect::new(area.x, area.y, area.width, 1.min(area.height))
    }
}
