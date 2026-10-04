//! Lifecycle rail Steps component with display and navigable inspection modes.
//!
//! Presents queued, running, done, skipped, failed, and blocked states without
//! scheduling or executing work.

use ratatui::crossterm::event::KeyCode;

use crate::termrock::collections::{child_item_key, reconcile_cursor};
use crate::termrock::identity::{Id, ItemKey, Keyed, Part, Revision};
use crate::termrock::layout::{Axis, Constraints, Rect, Size};
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, VisualState,
};
use crate::termrock::runtime::{AnimationSample, Cx, MeasureCx, Ui};
use crate::termrock::scroll::{ScrollRegion, ScrollState};
use crate::termrock::theme::StylePatch;

/// Lifecycle execution status of a single step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StepStatus {
    #[default]
    Queued,
    Running,
    Done,
    Skipped,
    Failed,
    Blocked,
}

impl StepStatus {
    pub const fn glyph(&self, spinner_phase: usize) -> &'static str {
        const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        match self {
            Self::Queued => "·",
            Self::Running => SPINNER[spinner_phase % SPINNER.len()],
            Self::Done => "✓",
            Self::Skipped => "–",
            Self::Failed => "✗",
            Self::Blocked => "·",
        }
    }
}

/// A borrowed step item in [`Steps`].
#[derive(Debug, Clone)]
pub struct StepItem<'a> {
    pub key: ItemKey,
    pub label: &'a str,
    pub status: StepStatus,
    pub meta: Option<&'a str>,
}

impl<'a> StepItem<'a> {
    pub fn new(key: ItemKey, label: &'a str, status: StepStatus) -> Self {
        Self {
            key,
            label,
            status,
            meta: None,
        }
    }

    pub fn meta(mut self, meta: Option<&'a str>) -> Self {
        self.meta = meta;
        self
    }
}

impl<'a> Keyed for StepItem<'a> {
    fn key(&self) -> ItemKey {
        self.key
    }
}

/// Presentation mode for [`Steps`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StepsMode {
    /// Purely presentational progress rail: no focus stop or user interaction.
    #[default]
    Display,
    /// Interactive inspection rail with keyboard/pointer navigation.
    Navigable,
}

/// Durable view state for [`Steps`] in navigable mode.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StepsState {
    pub cursor: Option<ItemKey>,
    pub scroll: ScrollState,
    pub last_revision: Option<Revision>,
}

impl StepsState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cursor(&self) -> Option<ItemKey> {
        self.cursor
    }

    pub fn scroll(&self) -> &ScrollState {
        &self.scroll
    }

    pub fn scroll_mut(&mut self) -> &mut ScrollState {
        &mut self.scroll
    }
}

/// Typed actions emitted by [`Steps`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepsAction {
    Select {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    Activate {
        key: ItemKey,
        origin: ActivationOrigin,
    },
}

/// Lifecycle status rail component.
pub struct Steps<'a> {
    pub id: Id,
    pub steps: &'a [StepItem<'a>],
    pub revision: Revision,
    pub mode: StepsMode,
    pub animation: Option<AnimationSample>,
    pub patch: StylePatch,
}

impl<'a> Steps<'a> {
    pub fn new(id: Id, steps: &'a [StepItem<'a>], revision: Revision) -> Self {
        Self {
            id,
            steps,
            revision,
            mode: StepsMode::Display,
            animation: None,
            patch: StylePatch::empty(),
        }
    }

    pub fn mode(mut self, mode: StepsMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn animation(mut self, animation: AnimationSample) -> Self {
        self.animation = Some(animation);
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = patch;
        self
    }

    pub fn reconcile(&self, state: &mut StepsState) {
        if state.last_revision != Some(self.revision) {
            let step_keys: Vec<ItemKey> = self.steps.iter().map(|s| s.key()).collect();
            state.cursor = reconcile_cursor(state.cursor, &step_keys);
            state.scroll.total = self.steps.len();
            state.scroll.clamp();
            state.last_revision = Some(self.revision);
        }
    }

    pub fn update(&self, cx: &mut Cx<'_>, state: &mut StepsState) -> Response<StepsAction> {
        if self.mode == StepsMode::Display {
            return Response::bubble(self.id.clone());
        }

        self.reconcile(state);

        let has_focus = cx.has_focus(&self.id);
        let intended = cx.intended_owner().cloned();
        let is_target = intended.as_ref() == Some(&self.id)
            || intended.as_ref().and_then(|id| id.parent()).as_ref() == Some(&self.id)
            || intended
                .as_ref()
                .map(|id| child_item_key(&self.id, id).is_some())
                .unwrap_or(false);

        let mut action = None;
        let mut changed = false;

        let find_cursor_idx = |key_opt: Option<ItemKey>| -> Option<usize> {
            let key = key_opt?;
            self.steps.iter().position(|s| s.key() == key)
        };

        let cause = cx.cause().clone();

        match cause {
            UpdateCause::Input(Input::Key(k), _) if has_focus => match k.code {
                KeyCode::Up | KeyCode::Char('k') if k.plain() => {
                    if let Some(cur_idx) = find_cursor_idx(state.cursor) {
                        if cur_idx > 0 {
                            let next_idx = cur_idx - 1;
                            state.cursor = Some(self.steps[next_idx].key());
                            state.scroll.ensure_visible(next_idx);
                            changed = true;
                        }
                    } else if !self.steps.is_empty() {
                        state.cursor = Some(self.steps[0].key());
                        state.scroll.ensure_visible(0);
                        changed = true;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') if k.plain() => {
                    if let Some(cur_idx) = find_cursor_idx(state.cursor) {
                        if cur_idx + 1 < self.steps.len() {
                            let next_idx = cur_idx + 1;
                            state.cursor = Some(self.steps[next_idx].key());
                            state.scroll.ensure_visible(next_idx);
                            changed = true;
                        }
                    } else if !self.steps.is_empty() {
                        state.cursor = Some(self.steps[0].key());
                        state.scroll.ensure_visible(0);
                        changed = true;
                    }
                }
                KeyCode::Enter => {
                    if let Some(key) = state.cursor {
                        action = Some(StepsAction::Activate {
                            key,
                            origin: ActivationOrigin::Keyboard,
                        });
                    }
                }
                _ => {}
            },
            UpdateCause::Input(Input::Mouse(m), _) => {
                if let Some(target_id) = intended {
                    if let Some(clicked_key) = child_item_key(&self.id, &target_id) {
                        cx.request_focus(self.id.clone());
                        state.cursor = Some(clicked_key);
                        if let Some(idx) = find_cursor_idx(Some(clicked_key)) {
                            state.scroll.ensure_visible(idx);
                        }
                        changed = true;

                        if matches!(m.kind, MouseKind::Up | MouseKind::Down) {
                            action = Some(StepsAction::Activate {
                                key: clicked_key,
                                origin: ActivationOrigin::Pointer,
                            });
                        }
                    } else if is_target {
                        match m.kind {
                            MouseKind::WheelUp => {
                                changed = state.scroll.scroll_by(-3);
                            }
                            MouseKind::WheelDown => {
                                changed = state.scroll.scroll_by(3);
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }

        if changed {
            cx.request_invalidate(Invalidate::Paint);
        }

        if let Some(act) = action {
            Response::action(self.id.clone(), act)
                .with_flow(Flow::Consumed)
                .with_invalidate(Invalidate::Paint)
        } else if changed || is_target {
            Response::consumed(self.id.clone()).with_invalidate(if changed {
                Invalidate::Paint
            } else {
                Invalidate::None
            })
        } else {
            Response::bubble(self.id.clone())
        }
    }

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &StepsState) -> Rect {
        if self.mode == StepsMode::Navigable {
            ui.register_focus(self.id.clone(), true);
        }
        ui.register_hit(self.id.clone(), area);

        if area.is_empty() {
            return area;
        }

        let mut scroll = state.scroll;
        scroll.total = self.steps.len();
        scroll.viewport = area.height as usize;
        scroll.clamp();

        let visible = scroll.visible_range();
        let scrollbar_needed = scroll.is_overflowing();
        let row_width = if scrollbar_needed {
            area.width.saturating_sub(1)
        } else {
            area.width
        };

        let _spinner_phase = self.animation.map(|a| a.index as usize).unwrap_or(0);

        for i in visible {
            let step = &self.steps[i];
            let key = step.key();
            let row_y = area.y + (i - scroll.offset) as u16;
            let row_area = Rect::new(area.x, row_y, row_width, 1);
            let row_id = self.id.child(key);

            if self.mode == StepsMode::Navigable {
                ui.register_hit(row_id.clone(), row_area);
            }

            let is_cursor = self.mode == StepsMode::Navigable && state.cursor == Some(key);

            let _visual = VisualState::empty()
                .focused(is_cursor)
                .busy(step.status == StepStatus::Running);

            ui.part(row_id, Part::new("row"), row_area, |_p| {});
        }

        if scrollbar_needed {
            let scroll_region = ScrollRegion::new(
                self.id.clone(),
                Axis::Vertical,
                self.steps.len(),
                area.height as usize,
            );
            scroll_region.draw(ui, area, &scroll);
        }

        area
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let h = self.steps.len() as u16;
        constraints.clamp(Size::new(20, h))
    }
}
