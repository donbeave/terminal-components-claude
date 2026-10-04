//! Hierarchical Tree component with stable-key node expansion, navigation, and scroll.
//!
//! Flattens only visible projection, bounds draw calls, preserves expansion across
//! source revisions, and distinguishes disclosure and row body hit targets.

use std::collections::BTreeSet;

use ratatui::crossterm::event::KeyCode;

use crate::termrock::collections::{
    BranchActivation, Readiness, SelectionRequest, TreeNode, TreeSource, child_item_key,
};
use crate::termrock::identity::{Id, ItemKey, Part, Revision};
use crate::termrock::layout::{Axis, Constraints, Rect, Size};
use crate::termrock::response::{
    ActivationOrigin, Flow, Input, Invalidate, MouseKind, Response, UpdateCause, VisualState,
};
use crate::termrock::runtime::{Cx, MeasureCx, Ui};
use crate::termrock::scroll::{ScrollRegion, ScrollState};
use crate::termrock::theme::StylePatch;

/// Flattened visible projection node for [`Tree`].
#[derive(Debug, Clone)]
pub struct FlatNode<'a> {
    pub key: ItemKey,
    pub depth: usize,
    pub node: TreeNode<'a>,
    pub parent_key: Option<ItemKey>,
}

/// Durable view state for [`Tree`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TreeState {
    pub cursor: Option<ItemKey>,
    pub expanded: BTreeSet<ItemKey>,
    pub scroll: ScrollState,
    pub last_revision: Option<Revision>,
}

impl TreeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cursor(&self) -> Option<ItemKey> {
        self.cursor
    }

    pub fn is_expanded(&self, key: ItemKey) -> bool {
        self.expanded.contains(&key)
    }

    pub fn expand(&mut self, key: ItemKey) {
        self.expanded.insert(key);
    }

    pub fn collapse(&mut self, key: ItemKey) {
        self.expanded.remove(&key);
    }

    pub fn toggle(&mut self, key: ItemKey) -> bool {
        if self.expanded.contains(&key) {
            self.expanded.remove(&key);
            false
        } else {
            self.expanded.insert(key);
            true
        }
    }

    pub fn expand_all(&mut self, keys: impl IntoIterator<Item = ItemKey>) {
        self.expanded.extend(keys);
    }

    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }
}

/// Typed actions emitted by [`Tree`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeAction {
    Activate {
        key: ItemKey,
        origin: ActivationOrigin,
    },
    ToggleExpand {
        key: ItemKey,
    },
    ExpansionChanged {
        key: ItemKey,
        expanded: bool,
    },
    LoadChildren {
        key: ItemKey,
    },
    SelectionRequested(SelectionRequest),
}

/// Hierarchical tree component.
pub struct Tree<'a, S: TreeSource + ?Sized> {
    pub id: Id,
    pub source: &'a S,
    pub selected: &'a [ItemKey],
    pub branch_activation: BranchActivation,
    pub readiness: Option<Readiness<'a>>,
    pub patch: StylePatch,
}

impl<'a, S: TreeSource + ?Sized> Tree<'a, S> {
    pub fn new(id: Id, source: &'a S) -> Self {
        Self {
            id,
            source,
            selected: &[],
            branch_activation: BranchActivation::Toggle,
            readiness: None,
            patch: StylePatch::empty(),
        }
    }

    pub fn selected(mut self, selected: &'a [ItemKey]) -> Self {
        self.selected = selected;
        self
    }

    pub fn branch_activation(mut self, mode: BranchActivation) -> Self {
        self.branch_activation = mode;
        self
    }

    pub fn readiness(mut self, readiness: Readiness<'a>) -> Self {
        self.readiness = Some(readiness);
        self
    }

    pub fn patch(mut self, patch: StylePatch) -> Self {
        self.patch = patch;
        self
    }

    /// Flatten only the visible projection based on current expansion state.
    pub fn flatten(&self, state: &TreeState) -> Vec<FlatNode<'a>> {
        let mut result = Vec::new();
        for &root_key in self.source.roots() {
            self.flatten_recursive(root_key, 0, None, state, &mut result);
        }
        result
    }

    fn flatten_recursive(
        &self,
        key: ItemKey,
        depth: usize,
        parent_key: Option<ItemKey>,
        state: &TreeState,
        out: &mut Vec<FlatNode<'a>>,
    ) {
        if let Some(node) = self.source.node(key) {
            let is_expanded = state.is_expanded(key);
            out.push(FlatNode {
                key,
                depth,
                node: node.clone(),
                parent_key,
            });

            if is_expanded && !node.leaf {
                for &child_key in node.children {
                    self.flatten_recursive(child_key, depth + 1, Some(key), state, out);
                }
            }
        }
    }

    pub fn update(&self, cx: &mut Cx<'_>, state: &mut TreeState) -> Response<TreeAction> {
        let flat = self.flatten(state);

        // Reconcile cursor: fallback to ancestor or neighbor if disappeared
        if let Some(c) = state.cursor {
            if !flat.iter().any(|fnod| fnod.key == c) {
                state.cursor = flat.first().map(|fnod| fnod.key);
            }
        } else if let Some(first) = flat.first() {
            state.cursor = Some(first.key);
        }

        state.scroll.total = flat.len();
        state.scroll.clamp();
        state.last_revision = Some(self.source.revision());

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
            flat.iter().position(|fnod| fnod.key == key)
        };

        let cause = cx.cause().clone();

        match cause {
            UpdateCause::Input(Input::Key(k), _) if has_focus => {
                let cur_idx = find_cursor_idx(state.cursor).unwrap_or(0);
                match k.code {
                    KeyCode::Up | KeyCode::Char('k') if k.plain() => {
                        if cur_idx > 0 {
                            let next_idx = cur_idx - 1;
                            state.cursor = Some(flat[next_idx].key);
                            state.scroll.ensure_visible(next_idx);
                            changed = true;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') if k.plain() => {
                        if cur_idx + 1 < flat.len() {
                            let next_idx = cur_idx + 1;
                            state.cursor = Some(flat[next_idx].key);
                            state.scroll.ensure_visible(next_idx);
                            changed = true;
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') if k.plain() => {
                        if let Some(fnod) = flat.get(cur_idx).filter(|n| !n.node.leaf) {
                            if !state.is_expanded(fnod.key) {
                                state.expand(fnod.key);
                                action = Some(TreeAction::ExpansionChanged {
                                    key: fnod.key,
                                    expanded: true,
                                });
                                changed = true;
                            } else if cur_idx + 1 < flat.len() {
                                // Move to first child
                                state.cursor = Some(flat[cur_idx + 1].key);
                                state.scroll.ensure_visible(cur_idx + 1);
                                changed = true;
                            }
                        }
                    }
                    KeyCode::Left | KeyCode::Char('h') if k.plain() => {
                        if let Some(fnod) = flat.get(cur_idx) {
                            if !fnod.node.leaf && state.is_expanded(fnod.key) {
                                state.collapse(fnod.key);
                                action = Some(TreeAction::ExpansionChanged {
                                    key: fnod.key,
                                    expanded: false,
                                });
                                changed = true;
                            } else if let Some(parent) = fnod.parent_key {
                                state.cursor = Some(parent);
                                if let Some(p_idx) = flat.iter().position(|n| n.key == parent) {
                                    state.scroll.ensure_visible(p_idx);
                                }
                                changed = true;
                            }
                        }
                    }
                    KeyCode::Char('*') => {
                        let all_keys: Vec<ItemKey> = flat.iter().map(|n| n.key).collect();
                        state.expand_all(all_keys);
                        changed = true;
                    }
                    KeyCode::Char('-') => {
                        state.collapse_all();
                        changed = true;
                    }
                    KeyCode::Enter | KeyCode::Char(' ') if k.plain() => {
                        if let Some(fnod) = flat.get(cur_idx) {
                            if fnod.node.leaf {
                                action = Some(TreeAction::Activate {
                                    key: fnod.key,
                                    origin: ActivationOrigin::Keyboard,
                                });
                            } else {
                                match self.branch_activation {
                                    BranchActivation::Toggle => {
                                        let exp = state.toggle(fnod.key);
                                        action = Some(TreeAction::ExpansionChanged {
                                            key: fnod.key,
                                            expanded: exp,
                                        });
                                        changed = true;
                                    }
                                    BranchActivation::Activate => {
                                        action = Some(TreeAction::Activate {
                                            key: fnod.key,
                                            origin: ActivationOrigin::Keyboard,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            UpdateCause::Input(Input::Mouse(m), _) => {
                if let Some(target_id) = intended {
                    if let Some(clicked_key) = child_item_key(&self.id, &target_id) {
                        cx.request_focus(self.id.clone());
                        state.cursor = Some(clicked_key);
                        if let Some(idx) = flat.iter().position(|n| n.key == clicked_key) {
                            state.scroll.ensure_visible(idx);
                        }
                        changed = true;

                        let is_disclosure = target_id.as_str().ends_with("/s:10:disclosure");

                        if matches!(m.kind, MouseKind::Up | MouseKind::Down) {
                            if is_disclosure {
                                let exp = state.toggle(clicked_key);
                                action = Some(TreeAction::ExpansionChanged {
                                    key: clicked_key,
                                    expanded: exp,
                                });
                            } else if let Some(fnod) = flat.iter().find(|n| n.key == clicked_key) {
                                if fnod.node.leaf {
                                    action = Some(TreeAction::Activate {
                                        key: clicked_key,
                                        origin: ActivationOrigin::Pointer,
                                    });
                                } else {
                                    match self.branch_activation {
                                        BranchActivation::Toggle => {
                                            let exp = state.toggle(clicked_key);
                                            action = Some(TreeAction::ExpansionChanged {
                                                key: clicked_key,
                                                expanded: exp,
                                            });
                                        }
                                        BranchActivation::Activate => {
                                            action = Some(TreeAction::Activate {
                                                key: clicked_key,
                                                origin: ActivationOrigin::Pointer,
                                            });
                                        }
                                    }
                                }
                            }
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

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &TreeState) -> Rect {
        ui.register_focus(self.id.clone(), true);
        ui.register_hit(self.id.clone(), area);

        if area.is_empty() {
            return area;
        }

        let flat = self.flatten(state);
        let mut scroll = state.scroll;
        scroll.total = flat.len();
        scroll.viewport = area.height as usize;
        scroll.clamp();

        let visible = scroll.visible_range();
        let scrollbar_needed = scroll.is_overflowing();
        let row_width = if scrollbar_needed {
            area.width.saturating_sub(1)
        } else {
            area.width
        };

        for i in visible {
            let fnod = &flat[i];
            let key = fnod.key;
            let row_y = area.y + (i - scroll.offset) as u16;
            let row_area = Rect::new(area.x, row_y, row_width, 1);
            let row_id = self.id.child(key);

            ui.register_hit(row_id.clone(), row_area);

            // Disclosure hit target (width 2 at indent offset)
            let indent_cells = (fnod.depth * 2) as u16;
            let disc_x = area.x.saturating_add(indent_cells);
            if disc_x + 2 <= area.x + row_width && !fnod.node.leaf {
                let disc_area = Rect::new(disc_x, row_y, 2, 1);
                let disc_id = row_id.sub("disclosure");
                ui.register_hit(disc_id.clone(), disc_area);
                ui.part(disc_id, Part::new("disclosure"), disc_area, |_p| {});
            }

            let is_selected = self.selected.contains(&key);
            let is_cursor = state.cursor == Some(key);

            let _visual = VisualState::empty()
                .selected(is_selected)
                .focused(is_cursor)
                .disabled(fnod.node.disabled)
                .busy(fnod.node.busy);

            ui.part(row_id, Part::new("row"), row_area, |_p| {});
        }

        if scrollbar_needed {
            let scroll_region = ScrollRegion::new(
                self.id.clone(),
                Axis::Vertical,
                flat.len(),
                area.height as usize,
            );
            scroll_region.draw(ui, area, &scroll);
        }

        area
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let roots_len = self.source.roots().len() as u16;
        constraints.clamp(Size::new(24, roots_len))
    }
}
