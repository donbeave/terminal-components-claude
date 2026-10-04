//! Shared runtime layer stack, modal ownership, barriers, and focus restoration.
//!
//! Provides the single layer stack for Dialog, Picker, Menu, and Popovers,
//! enforcing Z-order, clipping, outside-pointer dismissal, and prior-focus restoration.

use std::fmt;

use crate::termrock::identity::{Id, Part};
use crate::termrock::layout::{Constraints, Position, Rect, Size};

/// Category of an overlay layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerKind {
    /// Modal overlay: topmost, blocks all input below, traps focus ring.
    Modal,
    /// Popover overlay: anchored, dismissable on outside click, does not trap focus.
    Popover,
    /// Menu overlay: command list, anchored, dismissable on Escape or outside click.
    Menu,
}

/// Placement anchor for an overlay.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Anchor {
    /// Centered or placed relative to the full screen viewport.
    Screen(Rect),
    /// Anchored to a specific control part.
    OwnerPart(Id, Part),
    /// Anchored to an explicit cell coordinate.
    Position(Position),
}

/// Size constraints for an overlay layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerSize {
    /// Exact fixed width and height.
    Fixed(Size),
    /// Min/max constrained size resolved against viewport.
    Constrained(Constraints),
    /// Fit content with bounds.
    Auto,
}

/// Dismissal policy for an overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DismissPolicy {
    /// Closes when Escape is pressed.
    Escape,
    /// Closes when a pointer click occurs outside the layer's rectangle.
    OutsidePointer,
    /// Closes on either Escape or outside pointer click.
    #[default]
    Both,
    /// Only closes programmatically by the caller/component.
    None,
}

impl DismissPolicy {
    pub const fn on_escape(&self) -> bool {
        matches!(self, Self::Escape | Self::Both)
    }

    pub const fn on_outside_pointer(&self) -> bool {
        matches!(self, Self::OutsidePointer | Self::Both)
    }
}

/// Backdrop rendering recipe behind an active layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Backdrop {
    /// Dim underlying cells according to theme recipe.
    #[default]
    Dim,
    /// Completely opaque solid background fill.
    Opaque,
    /// Transparent: underlying cells remain clearly visible without dimming.
    None,
}

/// Reason why an overlay was dismissed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DismissReason {
    /// Dismissed via Escape key.
    Escape,
    /// Dismissed via pointer click outside the layer area.
    OutsidePointer,
    /// Owner control was removed from the active tree.
    OwnerRemoved,
    /// Programmatic dismissal by application or component update.
    Programmatic,
}

/// Specification for opening an overlay layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerSpec {
    pub owner: Id,
    pub kind: LayerKind,
    pub anchor: Anchor,
    pub size: LayerSize,
    pub dismiss: DismissPolicy,
    pub backdrop: Backdrop,
    pub inert_below: bool,
}

impl LayerSpec {
    pub fn modal(owner: Id, size: LayerSize) -> Self {
        Self {
            owner,
            kind: LayerKind::Modal,
            anchor: Anchor::Screen(Rect::zero()),
            size,
            dismiss: DismissPolicy::Both,
            backdrop: Backdrop::Dim,
            inert_below: true,
        }
    }

    pub fn popover(owner: Id, anchor: Anchor, size: LayerSize) -> Self {
        Self {
            owner,
            kind: LayerKind::Popover,
            anchor,
            size,
            dismiss: DismissPolicy::Both,
            backdrop: Backdrop::None,
            inert_below: false,
        }
    }

    pub fn menu(owner: Id, anchor: Anchor, size: LayerSize) -> Self {
        Self {
            owner,
            kind: LayerKind::Menu,
            anchor,
            size,
            dismiss: DismissPolicy::Both,
            backdrop: Backdrop::None,
            inert_below: true,
        }
    }
}

/// Typed layer errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerError {
    /// Layer with this owner ID is already open.
    DuplicateLayer(Id),
    /// Layer with this owner ID was not found in the stack.
    LayerNotFound(Id),
    /// Anchor could not be resolved against current layout.
    InvalidAnchor,
}

impl fmt::Display for LayerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateLayer(id) => write!(f, "Layer with ID '{id}' is already open"),
            Self::LayerNotFound(id) => write!(f, "Layer with ID '{id}' was not found"),
            Self::InvalidAnchor => write!(f, "Invalid anchor for layer"),
        }
    }
}

impl std::error::Error for LayerError {}

/// An active entry in the runtime layer stack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerEntry {
    pub spec: LayerSpec,
    pub resolved_rect: Rect,
    pub saved_focus: Option<Id>,
}

/// Stack of active overlays managed by the Termrock runtime.
#[derive(Debug, Clone, Default)]
pub struct LayerStack {
    layers: Vec<LayerEntry>,
}

impl LayerStack {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    pub fn len(&self) -> usize {
        self.layers.len()
    }

    pub fn entries(&self) -> &[LayerEntry] {
        &self.layers
    }

    pub fn topmost(&self) -> Option<&LayerEntry> {
        self.layers.last()
    }

    pub fn find(&self, id: &Id) -> Option<&LayerEntry> {
        self.layers.iter().find(|l| &l.spec.owner == id)
    }

    pub fn contains(&self, id: &Id) -> bool {
        self.layers.iter().any(|l| &l.spec.owner == id)
    }

    /// Open a new layer on top of the stack, recording the currently focused control.
    pub fn push(
        &mut self,
        spec: LayerSpec,
        viewport: Rect,
        current_focus: Option<Id>,
    ) -> Result<Rect, LayerError> {
        if self.contains(&spec.owner) {
            return Err(LayerError::DuplicateLayer(spec.owner.clone()));
        }

        let resolved_rect = Self::resolve_placement(&spec, viewport);
        self.layers.push(LayerEntry {
            spec,
            resolved_rect,
            saved_focus: current_focus,
        });
        Ok(resolved_rect)
    }

    /// Close the topmost layer if it matches `id`, returning the saved focus owner.
    pub fn close(&mut self, id: &Id, _reason: DismissReason) -> Result<Option<Id>, LayerError> {
        if let Some(pos) = self.layers.iter().rposition(|l| &l.spec.owner == id) {
            let removed = self.layers.remove(pos);
            Ok(removed.saved_focus)
        } else {
            Err(LayerError::LayerNotFound(id.clone()))
        }
    }

    /// Handle Escape dismissal: closes the topmost layer if its policy allows.
    pub fn handle_escape(&mut self) -> Option<(Id, Option<Id>)> {
        if self
            .layers
            .last()
            .is_some_and(|top| top.spec.dismiss.on_escape())
        {
            let removed = self.layers.pop()?;
            return Some((removed.spec.owner, removed.saved_focus));
        }
        None
    }

    /// Handle outside pointer click: checks if click is outside the topmost layer.
    /// If so and dismissal is enabled, pops the layer and returns the dismissed layer ID and saved focus.
    pub fn handle_outside_click(&mut self, pos: Position) -> Option<(Id, Option<Id>)> {
        if self.layers.last().is_some_and(|top| {
            !top.resolved_rect.contains(pos) && top.spec.dismiss.on_outside_pointer()
        }) {
            let removed = self.layers.pop()?;
            return Some((removed.spec.owner, removed.saved_focus));
        }
        None
    }

    /// Check if coordinates are blocked by an inert barrier from an active overlay.
    pub fn is_point_blocked(&self, pos: Position) -> bool {
        for layer in self.layers.iter().rev() {
            if layer.spec.inert_below {
                if layer.resolved_rect.contains(pos) {
                    return false; // Point is inside the active layer
                }
                return true; // Point is outside an inert barrier
            }
        }
        false
    }

    /// Resolve declarative layer placement against the viewport rectangle.
    pub fn resolve_placement(spec: &LayerSpec, viewport: Rect) -> Rect {
        let (width, height) = match spec.size {
            LayerSize::Fixed(size) => (
                size.width.min(viewport.width),
                size.height.min(viewport.height),
            ),
            LayerSize::Constrained(c) => {
                let w = c.clamp(Size::new(viewport.width, viewport.height)).width;
                let h = c.clamp(Size::new(viewport.width, viewport.height)).height;
                (w.min(viewport.width), h.min(viewport.height))
            }
            LayerSize::Auto => (
                (viewport.width * 3 / 4).max(10).min(viewport.width),
                (viewport.height * 3 / 4).max(5).min(viewport.height),
            ),
        };

        match spec.anchor {
            Anchor::Screen(_) => {
                // Center in viewport
                let x = viewport.x + (viewport.width.saturating_sub(width)) / 2;
                let y = viewport.y + (viewport.height.saturating_sub(height)) / 2;
                Rect::new(x, y, width, height)
            }
            Anchor::Position(p) => {
                let x = p.x.min(viewport.x + viewport.width.saturating_sub(width));
                let y = p.y.min(viewport.y + viewport.height.saturating_sub(height));
                Rect::new(x, y, width, height)
            }
            Anchor::OwnerPart(_, _) => {
                // Default fallback centered
                let x = viewport.x + (viewport.width.saturating_sub(width)) / 2;
                let y = viewport.y + (viewport.height.saturating_sub(height)) / 2;
                Rect::new(x, y, width, height)
            }
        }
    }

    /// Re-resolve placement of all active layers after viewport resize.
    pub fn resize_viewport(&mut self, new_viewport: Rect) {
        for layer in &mut self.layers {
            layer.resolved_rect = Self::resolve_placement(&layer.spec, new_viewport);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_stack_push_pop_and_focus_restoration() {
        let mut stack = LayerStack::new();
        let viewport = Rect::new(0, 0, 80, 24);
        let root_focus = Id::new("button.1");
        let modal_id = Id::new("dialog.confirm");

        let spec = LayerSpec::modal(modal_id.clone(), LayerSize::Fixed(Size::new(40, 10)));
        let rect = stack
            .push(spec, viewport, Some(root_focus.clone()))
            .unwrap();

        assert_eq!(rect.width, 40);
        assert_eq!(rect.height, 10);
        assert_eq!(rect.x, 20); // centered: (80 - 40) / 2
        assert_eq!(rect.y, 7); // centered: (24 - 10) / 2

        assert_eq!(stack.len(), 1);
        assert!(stack.contains(&modal_id));

        // Duplicate open fails closed
        let dup = LayerSpec::modal(modal_id.clone(), LayerSize::Fixed(Size::new(40, 10)));
        assert!(stack.push(dup, viewport, None).is_err());

        // Close restores prior focus
        let restored = stack.close(&modal_id, DismissReason::Programmatic).unwrap();
        assert_eq!(restored, Some(root_focus));
        assert!(stack.is_empty());
    }

    #[test]
    fn layer_outside_click_dismissal_and_barriers() {
        let mut stack = LayerStack::new();
        let viewport = Rect::new(0, 0, 100, 40);
        let modal_id = Id::new("modal.test");

        let spec = LayerSpec::modal(modal_id.clone(), LayerSize::Fixed(Size::new(50, 20)));
        let rect = stack.push(spec, viewport, None).unwrap();

        // Point inside modal: not blocked
        assert!(!stack.is_point_blocked(Position::new(rect.x + 5, rect.y + 5)));

        // Point outside modal: blocked because inert_below = true
        assert!(stack.is_point_blocked(Position::new(5, 5)));

        // Click outside triggers dismissal
        let dismissed = stack.handle_outside_click(Position::new(5, 5));
        assert_eq!(dismissed.unwrap().0, modal_id);
        assert!(stack.is_empty());
    }

    #[test]
    fn layer_escape_dismissal_ladder() {
        let mut stack = LayerStack::new();
        let viewport = Rect::new(0, 0, 100, 50);

        let parent_modal = Id::new("modal.parent");
        let child_popover = Id::new("popover.child");

        stack
            .push(
                LayerSpec::modal(parent_modal.clone(), LayerSize::Fixed(Size::new(60, 30))),
                viewport,
                None,
            )
            .unwrap();

        stack
            .push(
                LayerSpec::popover(
                    child_popover.clone(),
                    Anchor::Position(Position::new(30, 20)),
                    LayerSize::Fixed(Size::new(20, 10)),
                ),
                viewport,
                None,
            )
            .unwrap();

        assert_eq!(stack.len(), 2);

        // Escape closes child popover first
        let (first_closed, _) = stack.handle_escape().unwrap();
        assert_eq!(first_closed, child_popover);
        assert_eq!(stack.len(), 1);

        // Second Escape closes parent modal
        let (second_closed, _) = stack.handle_escape().unwrap();
        assert_eq!(second_closed, parent_modal);
        assert!(stack.is_empty());
    }
}
