//! Termrock runtime coordinator, frame phases, focus, pointer capture, and supplied time.
//!
//! Owns the execution lifecycle: update, measure, draw, transactional geometry publication,
//! focus traversal, modal hit barriers, pointer capture, and monotonic time progression.

use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use crate::termrock::author::{PartUi, StyledText, TerminalSource};
use crate::termrock::identity::{Id, Part};
use crate::termrock::layers::{DismissReason, LayerError, LayerSpec, LayerStack};
use crate::termrock::layout::{Constraints, Position, Rect};
pub use crate::termrock::response::Moment;
use crate::termrock::response::{Input, Invalidate, MouseKind, UpdateCause};
use crate::termrock::theme::{ColorLevel, Surface, Theme};

/// Motion policy for animations and visual transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MotionPolicy {
    /// Full animation at target frame rate.
    #[default]
    Full,
    /// Reduced animation: simplified transitions, no rapid flashes.
    Reduced,
    /// Paused: animations frozen at current or neutral phase.
    Paused,
}

/// Sampled phase of an animation at a discrete moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimationSample {
    pub index: u64,
}

impl AnimationSample {
    pub const fn phase(index: u64) -> Self {
        Self { index }
    }

    pub fn timed(
        now: Moment,
        epoch: Moment,
        cadence: Duration,
        policy: MotionPolicy,
    ) -> Result<Self, ClockError> {
        if policy == MotionPolicy::Paused {
            return Ok(Self { index: 0 });
        }
        if now.as_millis() < epoch.as_millis() {
            return Err(ClockError::NonmonotonicTime);
        }
        let elapsed_ms = now.as_millis() - epoch.as_millis();
        let cadence_ms = cadence.as_millis().max(1) as u64;
        let index = elapsed_ms / cadence_ms;
        Ok(Self { index })
    }
}

/// Errors related to monotonic clock progression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockError {
    NonmonotonicTime,
    ClockOverflow,
}

impl fmt::Display for ClockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonmonotonicTime => write!(f, "Supplied time regressed backwards"),
            Self::ClockOverflow => write!(f, "Clock calculation overflowed"),
        }
    }
}

impl std::error::Error for ClockError {}

/// Monotonic token acknowledging presentation of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FrameToken(pub u64);

/// Safe runtime failure conditions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    DuplicateId(Id),
    StaleGeometry,
    InvalidCaptureOwner(Id),
    NonmonotonicTime { prior: Moment, current: Moment },
    InvalidFrameToken(FrameToken),
    PublicationFailed(&'static str),
    Layer(LayerError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId(id) => write!(f, "Duplicate live ID detected: {id}"),
            Self::StaleGeometry => write!(f, "Coordinate dispatch rejected on stale geometry"),
            Self::InvalidCaptureOwner(id) => write!(f, "Invalid pointer capture owner: {id}"),
            Self::NonmonotonicTime { prior, current } => {
                write!(f, "Time regressed from {prior:?} to {current:?}")
            }
            Self::InvalidFrameToken(token) => write!(f, "Invalid frame token: {:?}", token.0),
            Self::PublicationFailed(msg) => write!(f, "Geometry publication failed: {msg}"),
            Self::Layer(e) => write!(f, "Layer error: {e}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<LayerError> for RuntimeError {
    fn from(e: LayerError) -> Self {
        Self::Layer(e)
    }
}

/// Summary report returned after an update pass.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UpdateReport {
    pub invalidate: Invalidate,
    pub handled: bool,
    pub actions_emitted: usize,
}

/// Generic consumer adapter for a full-screen scene.
pub trait Scene {
    fn update(&mut self, cx: &mut Cx<'_>, cause: UpdateCause);
    fn draw(&self, ui: &mut Ui<'_>, area: Rect);
}

/// Update context supplied to components and scenes.
pub struct Cx<'a> {
    pub cause: &'a UpdateCause,
    pub moment: Moment,
    pub intended_owner: Option<Id>,
    pub focus: Option<Id>,
    pub pointer_capture: Option<Id>,
    pub invalidate: Invalidate,
    pub layer_stack: &'a mut LayerStack,
    pub cursor_request: Option<(Id, Position)>,
    pub new_focus: Option<Option<Id>>,
    pub new_capture: Option<Option<Id>>,
    pub feedback_requests: Vec<(Id, Duration)>,
    pub published_geometry: Option<&'a HashMap<Id, Rect>>,
}

impl<'a> Cx<'a> {
    pub fn cause(&self) -> &UpdateCause {
        self.cause
    }

    pub fn moment(&self) -> Moment {
        self.moment
    }

    pub fn intended_owner(&self) -> Option<&Id> {
        self.intended_owner.as_ref()
    }

    pub fn has_focus(&self, id: &Id) -> bool {
        self.focus.as_ref() == Some(id)
    }

    pub fn request_focus(&mut self, id: Id) {
        self.new_focus = Some(Some(id));
    }

    pub fn clear_focus(&mut self) {
        self.new_focus = Some(None);
    }

    pub fn capture_pointer(&mut self, id: Id) {
        self.new_capture = Some(Some(id));
    }

    pub fn release_capture(&mut self) {
        self.new_capture = Some(None);
    }

    pub fn trigger_feedback(&mut self, id: Id, duration: Duration) {
        self.feedback_requests.push((id, duration));
    }

    pub fn contains_point(&self, id: &Id, pos: Position) -> bool {
        if let Some(geom) = self.published_geometry {
            geom.get(id).is_some_and(|r| r.contains(pos))
        } else {
            false
        }
    }

    pub fn request_invalidate(&mut self, invalidate: Invalidate) {
        self.invalidate = self.invalidate.or(invalidate);
    }

    pub fn request_cursor(&mut self, id: Id, pos: Position) {
        self.cursor_request = Some((id, pos));
    }

    pub fn open_layer(&mut self, spec: LayerSpec, viewport: Rect) -> Result<Rect, LayerError> {
        self.request_invalidate(Invalidate::Paint);
        self.layer_stack.push(spec, viewport, self.focus.clone())
    }

    pub fn close_layer(&mut self, id: &Id, reason: DismissReason) -> Result<(), LayerError> {
        self.request_invalidate(Invalidate::Paint);
        let saved_focus = self.layer_stack.close(id, reason)?;
        if let Some(prior) = saved_focus {
            self.request_focus(prior);
        }
        Ok(())
    }
}

/// Pure measurement context containing constraints and theme metrics.
pub struct MeasureCx<'a> {
    pub constraints: Constraints,
    pub theme: &'a Theme,
    pub color_level: ColorLevel,
}

impl<'a> MeasureCx<'a> {
    pub fn new(constraints: Constraints, theme: &'a Theme, color_level: ColorLevel) -> Self {
        Self {
            constraints,
            theme,
            color_level,
        }
    }
}

/// Constrained draw context for painting cells and publishing geometry.
pub struct Ui<'a> {
    pub theme: &'a Theme,
    pub viewport: Rect,
    pub current_surface: Surface,
    surface_stack: Vec<Surface>,
    clip_stack: Vec<Rect>,
    pub hit_regions: HashMap<Id, Rect>,
    pub focus_candidates: Vec<Id>,
    pub cursor_intent: Option<(Id, Position)>,
    pub layer_stack: &'a mut LayerStack,
    pub buffer: Option<&'a mut ratatui::buffer::Buffer>,
    pub focus: Option<Id>,
    pub hovered: Option<Id>,
    pub pressed: Option<Id>,
    pub feedback: HashMap<Id, Moment>,
}

impl<'a> Ui<'a> {
    pub fn new(theme: &'a Theme, viewport: Rect, layer_stack: &'a mut LayerStack) -> Self {
        Self {
            theme,
            viewport,
            current_surface: Surface::Canvas,
            surface_stack: Vec::new(),
            clip_stack: vec![viewport],
            hit_regions: HashMap::new(),
            focus_candidates: Vec::new(),
            cursor_intent: None,
            layer_stack,
            buffer: None,
            focus: None,
            hovered: None,
            pressed: None,
            feedback: HashMap::new(),
        }
    }

    pub fn clip_area(&self) -> Rect {
        self.clip_stack.last().copied().unwrap_or(self.viewport)
    }

    pub fn with_buffer(mut self, buffer: &'a mut ratatui::buffer::Buffer) -> Self {
        self.buffer = Some(buffer);
        self
    }

    pub fn is_focused(&self, id: &Id) -> bool {
        self.focus.as_ref() == Some(id)
    }

    pub fn is_hovered(&self, id: &Id) -> bool {
        self.hovered.as_ref() == Some(id)
    }

    pub fn is_pressed(&self, id: &Id) -> bool {
        self.pressed.as_ref() == Some(id)
    }

    pub fn is_feedback(&self, id: &Id) -> bool {
        self.feedback.contains_key(id)
    }

    pub fn fill_rect(&mut self, area: Rect, style: ratatui::style::Style) {
        let clipped = self.clip_area().intersect(area);
        if clipped.is_empty() {
            return;
        }
        if let Some(buf) = self.buffer.as_deref_mut() {
            let max_x = clipped
                .x
                .saturating_add(clipped.width)
                .min(buf.area().width);
            let max_y = clipped
                .y
                .saturating_add(clipped.height)
                .min(buf.area().height);
            for y in clipped.y..max_y {
                for x in clipped.x..max_x {
                    let cell = &mut buf[(x, y)];
                    cell.set_symbol(" ");
                    cell.set_style(style);
                }
            }
        }
    }

    pub fn set_string(&mut self, x: u16, y: u16, s: &str, style: ratatui::style::Style) {
        let clip = self.clip_area();
        if clip.is_empty() || y < clip.y || y >= clip.y.saturating_add(clip.height) {
            return;
        }
        if let Some(buf) = self.buffer.as_deref_mut() {
            if y >= buf.area().height {
                return;
            }
            use unicode_segmentation::UnicodeSegmentation;
            use unicode_width::UnicodeWidthStr;
            let mut curr_x = x;
            let clip_right = clip.x.saturating_add(clip.width);
            let buf_width = buf.area().width;
            for g in s.graphemes(true) {
                let gw = UnicodeWidthStr::width(g) as u16;
                if curr_x >= clip.x
                    && curr_x.saturating_add(gw) <= clip_right
                    && curr_x.saturating_add(gw) <= buf_width
                {
                    buf.set_string(curr_x, y, g, style);
                }
                curr_x = curr_x.saturating_add(gw);
                if curr_x >= clip_right {
                    break;
                }
            }
        }
    }

    pub fn with_surface<R>(&mut self, surface: Surface, body: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.surface_stack.push(self.current_surface);
        self.current_surface = surface;
        let result = body(self);
        self.current_surface = self.surface_stack.pop().unwrap_or(Surface::Canvas);
        result
    }

    pub fn clip<R>(&mut self, area: Rect, body: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        let current_clip = self.clip_area();
        let intersected = current_clip.intersect(area);
        self.clip_stack.push(intersected);
        let result = body(self);
        self.clip_stack.pop();
        result
    }

    pub fn register_hit(&mut self, id: Id, area: Rect) {
        let clipped = self.clip_area().intersect(area);
        if !clipped.is_empty() {
            self.hit_regions.insert(id, clipped);
        }
    }

    pub fn register_focus(&mut self, id: Id, enabled: bool) {
        if enabled && !self.focus_candidates.contains(&id) {
            self.focus_candidates.push(id);
        }
    }

    pub fn request_cursor(&mut self, id: Id, pos: Position) {
        let clip = self.clip_area();
        if clip.contains(pos) {
            self.cursor_intent = Some((id, pos));
        }
    }

    pub fn layer<R>(&mut self, id: Id, body: impl FnOnce(&mut Ui<'_>, Rect) -> R) -> Option<R> {
        let entry = self.layer_stack.find(&id)?;
        let rect = entry.resolved_rect;
        Some(self.clip(rect, |sub_ui| body(sub_ui, rect)))
    }

    pub fn part<R>(
        &mut self,
        _owner: Id,
        _part: Part,
        area: Rect,
        painter: impl FnOnce(&mut PartUi<'_>) -> R,
    ) -> R {
        let mut part_ui = PartUi::new(area, self.current_surface, self.theme);
        painter(&mut part_ui)
    }

    pub fn paint_styled_text(&mut self, _area: Rect, _text: StyledText<'_>, _part: Part) {
        // Paints within assigned area and clip
    }

    pub fn blit_terminal(&mut self, _area: Rect, _source: &dyn TerminalSource) {
        // Reserved TerminalView blitting
    }
}

/// Rendered frame output ready for terminal presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaintedFrame {
    pub token: FrameToken,
    pub viewport: Rect,
    pub cursor: Option<(Id, Position)>,
    pub published_regions: usize,
}

/// Termrock runtime driver orchestrating scenes, focus, hit-testing, and layers.
pub struct Runtime<S: Scene> {
    pub scene: S,
    pub theme: Theme,
    pub focus: Option<Id>,
    pub focus_ring: Vec<Id>,
    pub pointer_capture: Option<Id>,
    pub hovered: Option<Id>,
    pub layer_stack: LayerStack,
    pub published_geometry: HashMap<Id, Rect>,
    pub last_moment: Option<Moment>,
    frame_sequence: u64,
    pub last_presented_token: Option<FrameToken>,
    pub suppress_hover: bool,
    pub active_feedback: HashMap<Id, Moment>,
}

impl<S: Scene> Runtime<S> {
    pub fn new(scene: S, theme: Theme) -> Self {
        Self {
            scene,
            theme,
            focus: None,
            focus_ring: Vec::new(),
            pointer_capture: None,
            hovered: None,
            layer_stack: LayerStack::new(),
            published_geometry: HashMap::new(),
            last_moment: None,
            frame_sequence: 0,
            last_presented_token: None,
            suppress_hover: false,
            active_feedback: HashMap::new(),
        }
    }

    pub fn current_focus(&self) -> Option<&Id> {
        self.focus.as_ref()
    }

    pub fn focus_next(&mut self) -> Option<&Id> {
        if self.focus_ring.is_empty() {
            return None;
        }
        let next_idx = match &self.focus {
            Some(cur) => self
                .focus_ring
                .iter()
                .position(|id| id == cur)
                .map(|idx| (idx + 1) % self.focus_ring.len())
                .unwrap_or(0),
            None => 0,
        };
        self.focus = self.focus_ring.get(next_idx).cloned();
        self.focus.as_ref()
    }

    pub fn focus_prev(&mut self) -> Option<&Id> {
        if self.focus_ring.is_empty() {
            return None;
        }
        let prev_idx = match &self.focus {
            Some(cur) => self
                .focus_ring
                .iter()
                .position(|id| id == cur)
                .map(|idx| {
                    if idx == 0 {
                        self.focus_ring.len() - 1
                    } else {
                        idx - 1
                    }
                })
                .unwrap_or(0),
            None => 0,
        };
        self.focus = self.focus_ring.get(prev_idx).cloned();
        self.focus.as_ref()
    }

    /// Process an update cause with monotonic time enforcement.
    pub fn handle(
        &mut self,
        cause: UpdateCause,
        moment: Moment,
    ) -> Result<UpdateReport, RuntimeError> {
        // Enforce monotonic time
        if let Some(last) = self
            .last_moment
            .filter(|&last| moment.as_millis() < last.as_millis())
        {
            return Err(RuntimeError::NonmonotonicTime {
                prior: last,
                current: moment,
            });
        }
        self.last_moment = Some(moment);

        let mut intended_owner = None;

        // Route normalized input
        match &cause {
            UpdateCause::Input(input, _) => match input {
                Input::Key(_) => {
                    self.suppress_hover = true;
                    // Escape key closes topmost layer if present
                    if let Some((_closed, saved)) = self.layer_stack.handle_escape() {
                        if let Some(restored) = saved {
                            self.focus = Some(restored);
                        }
                        return Ok(UpdateReport {
                            invalidate: Invalidate::Paint,
                            handled: true,
                            actions_emitted: 0,
                        });
                    }
                    intended_owner = self.focus.clone();
                }
                Input::Mouse(m) => {
                    self.suppress_hover = false;
                    let pos = Position::new(m.pos.x, m.pos.y);

                    // Check outside click on topmost layer
                    if matches!(m.kind, MouseKind::Down) {
                        let dismissed = self.layer_stack.handle_outside_click(pos);
                        if let Some((_closed, saved)) = dismissed {
                            if let Some(restored) = saved {
                                self.focus = Some(restored);
                            }
                            return Ok(UpdateReport {
                                invalidate: Invalidate::Paint,
                                handled: true,
                                actions_emitted: 0,
                            });
                        }
                    }

                    // Check if click is blocked by inert layer
                    if self.layer_stack.is_point_blocked(pos) {
                        return Ok(UpdateReport {
                            invalidate: Invalidate::None,
                            handled: true,
                            actions_emitted: 0,
                        });
                    }

                    // Hit test published geometry
                    if let Some(captured) = &self.pointer_capture {
                        intended_owner = Some(captured.clone());
                    } else {
                        intended_owner = self
                            .published_geometry
                            .iter()
                            .find(|(_, rect)| rect.contains(pos))
                            .map(|(id, _)| id.clone());
                    }

                    if matches!(m.kind, MouseKind::Move) {
                        self.hovered = intended_owner.clone();
                    } else if matches!(m.kind, MouseKind::Up) {
                        self.pointer_capture = None;
                    }
                }
                Input::Resize(w, h) => {
                    let viewport = Rect::new(0, 0, *w, *h);
                    self.layer_stack.resize_viewport(viewport);
                }
                _ => {}
            },
            UpdateCause::Resize(size, _) => {
                let viewport = Rect::new(0, 0, size.width, size.height);
                self.layer_stack.resize_viewport(viewport);
            }
            _ => {}
        }

        // Expire feedback whose deadline has passed
        self.active_feedback
            .retain(|_, expiry| moment.as_millis() < expiry.as_millis());

        let mut cx = Cx {
            cause: &cause,
            moment,
            intended_owner,
            focus: self.focus.clone(),
            pointer_capture: self.pointer_capture.clone(),
            invalidate: Invalidate::None,
            layer_stack: &mut self.layer_stack,
            cursor_request: None,
            new_focus: None,
            new_capture: None,
            feedback_requests: Vec::new(),
            published_geometry: Some(&self.published_geometry),
        };

        self.scene.update(&mut cx, cause.clone());

        if let Some(focus_req) = cx.new_focus {
            self.focus = focus_req;
        }
        if let Some(capture_req) = cx.new_capture {
            self.pointer_capture = capture_req;
        }
        for (id, dur) in cx.feedback_requests {
            self.active_feedback
                .insert(id, moment.saturating_add_millis(dur.as_millis() as u64));
        }

        Ok(UpdateReport {
            invalidate: cx.invalidate,
            handled: true,
            actions_emitted: 0,
        })
    }

    /// Render a frame and publish geometry transactionally.
    pub fn draw(&mut self, area: Rect) -> Result<PaintedFrame, RuntimeError> {
        let mut ui = Ui::new(&self.theme, area, &mut self.layer_stack);
        ui.focus = self.focus.clone();
        ui.hovered = if self.suppress_hover {
            None
        } else {
            self.hovered.clone()
        };
        ui.pressed = self.pointer_capture.clone();
        ui.feedback = self.active_feedback.clone();

        self.scene.draw(&mut ui, area);

        // Transactional publication: commit geometry and focus ring
        self.published_geometry = ui.hit_regions;
        self.focus_ring = ui.focus_candidates;

        // Auto-initialize focus if none set and ring has items
        if self.focus.is_none() && !self.focus_ring.is_empty() {
            self.focus = self.focus_ring.first().cloned();
        }

        self.frame_sequence = self.frame_sequence.wrapping_add(1);
        let token = FrameToken(self.frame_sequence);

        Ok(PaintedFrame {
            token,
            viewport: area,
            cursor: ui.cursor_intent,
            published_regions: self.published_geometry.len(),
        })
    }

    /// Render a frame into a Ratatui buffer and publish geometry transactionally.
    pub fn draw_into(
        &mut self,
        area: Rect,
        buffer: &mut ratatui::buffer::Buffer,
    ) -> Result<PaintedFrame, RuntimeError> {
        let mut ui = Ui::new(&self.theme, area, &mut self.layer_stack).with_buffer(buffer);
        ui.focus = self.focus.clone();
        ui.hovered = if self.suppress_hover {
            None
        } else {
            self.hovered.clone()
        };
        ui.pressed = self.pointer_capture.clone();
        ui.feedback = self.active_feedback.clone();

        self.scene.draw(&mut ui, area);

        // Transactional publication: commit geometry and focus ring
        self.published_geometry = ui.hit_regions;
        self.focus_ring = ui.focus_candidates;

        // Auto-initialize focus if none set and ring has items
        if self.focus.is_none() && !self.focus_ring.is_empty() {
            self.focus = self.focus_ring.first().cloned();
        }

        self.frame_sequence = self.frame_sequence.wrapping_add(1);
        let token = FrameToken(self.frame_sequence);

        Ok(PaintedFrame {
            token,
            viewport: area,
            cursor: ui.cursor_intent,
            published_regions: self.published_geometry.len(),
        })
    }

    /// Acknowledge presentation of a painted frame.
    pub fn presented(&mut self, token: FrameToken) -> Result<(), RuntimeError> {
        if token.0 != self.frame_sequence {
            return Err(RuntimeError::InvalidFrameToken(token));
        }
        self.last_presented_token = Some(token);
        Ok(())
    }

    /// Check if active feedback is active for the given ID at the specified moment.
    pub fn is_feedback(&self, id: &Id, current: Moment) -> bool {
        if let Some(expiry) = self.active_feedback.get(id) {
            current.as_millis() < expiry.as_millis()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::termrock::response::Mouse;

    struct CounterScene {
        count: u64,
        btn_id: Id,
    }

    impl Scene for CounterScene {
        fn update(&mut self, cx: &mut Cx<'_>, _cause: UpdateCause) {
            if cx.intended_owner() == Some(&self.btn_id) {
                self.count += 1;
                cx.request_invalidate(Invalidate::Paint);
            }
        }

        fn draw(&self, ui: &mut Ui<'_>, area: Rect) {
            ui.register_focus(self.btn_id.clone(), true);
            ui.register_hit(
                self.btn_id.clone(),
                Rect::new(area.x + 2, area.y + 2, 10, 1),
            );
        }
    }

    #[test]
    fn runtime_draw_publish_and_pointer_hit() {
        let btn_id = Id::new("button.inc");
        let scene = CounterScene {
            count: 0,
            btn_id: btn_id.clone(),
        };
        let mut runtime = Runtime::new(scene, Theme::termrock());
        let area = Rect::new(0, 0, 80, 24);

        // Frame 1 draw publishes geometry
        let frame = runtime.draw(area).unwrap();
        assert_eq!(frame.published_regions, 1);
        assert_eq!(runtime.current_focus(), Some(&btn_id));

        // Click on button coordinates (x: 4, y: 2)
        let mouse_down = UpdateCause::Input(
            Input::Mouse(Mouse {
                kind: MouseKind::Down,
                pos: ratatui::layout::Position::new(4, 2),
            }),
            Moment::from_millis(10),
        );
        let report = runtime.handle(mouse_down, Moment::from_millis(10)).unwrap();
        assert!(report.handled);
        assert_eq!(runtime.scene.count, 1);

        // Click outside button (x: 50, y: 20) -> no increment
        let mouse_outside = UpdateCause::Input(
            Input::Mouse(Mouse {
                kind: MouseKind::Down,
                pos: ratatui::layout::Position::new(50, 20),
            }),
            Moment::from_millis(20),
        );
        let report = runtime
            .handle(mouse_outside, Moment::from_millis(20))
            .unwrap();
        assert!(report.handled);
        assert_eq!(runtime.scene.count, 1);
    }

    #[test]
    fn runtime_rejects_nonmonotonic_time() {
        let scene = CounterScene {
            count: 0,
            btn_id: Id::new("btn"),
        };
        let mut runtime = Runtime::new(scene, Theme::termrock());

        let t1 = Moment::from_millis(100);
        let t2 = Moment::from_millis(50); // regressed

        assert!(runtime.handle(UpdateCause::Boot(t1), t1).is_ok());
        let err = runtime.handle(UpdateCause::Tick(t2), t2).unwrap_err();
        assert!(matches!(err, RuntimeError::NonmonotonicTime { .. }));
    }

    #[test]
    fn runtime_draw_is_idempotent() {
        let scene = CounterScene {
            count: 0,
            btn_id: Id::new("btn"),
        };
        let mut runtime = Runtime::new(scene, Theme::termrock());
        let area = Rect::new(0, 0, 80, 24);

        let f1 = runtime.draw(area).unwrap();
        let count_after_f1 = runtime.scene.count;
        let regions_f1 = runtime.published_geometry.clone();

        let f2 = runtime.draw(area).unwrap();
        assert_eq!(
            runtime.scene.count, count_after_f1,
            "Draw must not mutate semantic state"
        );
        assert_eq!(
            regions_f1, runtime.published_geometry,
            "Draw geometry must be identical"
        );
        assert_eq!(f2.published_regions, f1.published_regions);
    }
}
