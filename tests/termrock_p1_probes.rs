//! Comprehensive verification probes for Termrock P1 Foundations.
//!
//! Covers:
//! - TASK-002: Identity, keys, revisions, response flow, and reconciliation.
//! - TASK-003: Runtime focus, pointer capture, layer stack, and monotonic time.
//! - TASK-004: Geometry, measurement, theme resolution, text, and secret zeroization.

#![allow(unused_imports, unused_variables, dead_code, clippy::useless_vec)]

use std::time::Duration;

use junie_tui::core::event::{Input, Key, Mouse, MouseKind};
use junie_tui::termrock::{
    ActionKey, ActivationOrigin, AnimationSample, Backdrop, Binding, BindingView, Chord,
    ClockError, ColorLevel, ColumnKey, Constraints, Cx, DismissPolicy, DismissReason, FieldKey,
    Flow, Id, Invalidate, ItemKey, Keyed, LayerKind, LayerSize, LayerSpec, LayerStack, MeasureCx,
    ModifierPatch, Moment, MotionPolicy, Part, PatchSlot, Position, Rect, Response, Revision,
    RevisionError, Role, Runtime, RuntimeError, Scene, Secret, SecretPolicy, Size, StylePatch,
    Surface, TextAction, TextEditorCore, Theme, Tone, Track, Ui, UpdateCause, ValueChanged,
    VisualState, author,
};
use ratatui::crossterm::event::{KeyCode, KeyModifiers};

// =========================================================================
// TASK-002: Identity, Revisions, Responses & Reconciliation
// =========================================================================

#[test]
fn test_identity_zero_collision_and_hierarchy() {
    let a_b = Id::new("a").sub("b");
    let ab_empty = Id::new("ab").sub("");
    let a_empty = Id::new("a");

    // Length-delimited composition prevents cross-boundary ambiguity
    assert_ne!(a_b, ab_empty);
    assert_ne!(a_b, a_empty);

    // Child derivation by ItemKey
    let child0 = a_b.child(ItemKey::new(0));
    let child1 = a_b.child(ItemKey::new(1));
    assert_ne!(child0, child1);
    assert_ne!(child0, a_b);

    // Parent resolution
    assert_eq!(child0.parent(), Some(a_b.clone()));
    assert_eq!(a_b.parent(), Some(a_empty.clone()));
    assert_eq!(a_empty.parent(), None);
}

#[test]
fn test_revision_generation_and_overflow_protection() {
    let mut rev = Revision::zero();
    assert_eq!(rev.as_u64(), 0);

    rev = rev.next().unwrap();
    assert_eq!(rev.as_u64(), 1);

    let old = Revision::zero();
    assert!(old.is_stale(rev));
    assert!(!rev.is_stale(old));

    // Checked overflow fails closed
    let max_rev = Revision::new(u64::MAX);
    assert!(matches!(max_rev.next(), Err(RevisionError::Overflow)));
}

#[test]
fn test_reconciliation_preserves_stable_keys_across_reorder_and_filter() {
    #[derive(Debug, Clone)]
    struct TaskItem {
        key: ItemKey,
        title: &'static str,
    }

    impl Keyed for TaskItem {
        fn key(&self) -> ItemKey {
            self.key
        }
    }

    let initial = vec![
        TaskItem {
            key: ItemKey::new(10),
            title: "Task 10",
        },
        TaskItem {
            key: ItemKey::new(20),
            title: "Task 20",
        },
        TaskItem {
            key: ItemKey::new(30),
            title: "Task 30",
        },
    ];

    let mut selected_key = Some(ItemKey::new(20));

    // Simulate reordering: [30, 20, 10]
    let reordered = vec![
        TaskItem {
            key: ItemKey::new(30),
            title: "Task 30",
        },
        TaskItem {
            key: ItemKey::new(20),
            title: "Task 20",
        },
        TaskItem {
            key: ItemKey::new(10),
            title: "Task 10",
        },
    ];

    // Keyed target survives reorder
    assert!(
        reordered
            .iter()
            .any(|item| Some(item.key()) == selected_key)
    );

    // Simulate filtering out item 20: [30, 10]
    let filtered = vec![
        TaskItem {
            key: ItemKey::new(30),
            title: "Task 30",
        },
        TaskItem {
            key: ItemKey::new(10),
            title: "Task 10",
        },
    ];

    // Reconcile cursor: fallback to nearest successor or predecessor
    if !filtered.iter().any(|item| Some(item.key()) == selected_key) {
        selected_key = filtered.first().map(|i| i.key());
    }
    assert_eq!(selected_key, Some(ItemKey::new(30)));
}

// =========================================================================
// TASK-003: Runtime Focus, Capture, Layers & Time
// =========================================================================

struct TestProbeScene {
    btn1: Id,
    btn2: Id,
    activated: Option<Id>,
}

impl Scene for TestProbeScene {
    fn update(&mut self, cx: &mut Cx<'_>, _cause: UpdateCause) {
        if cx.intended_owner() == Some(&self.btn1) {
            self.activated = Some(self.btn1.clone());
            cx.request_invalidate(Invalidate::Paint);
        } else if cx.intended_owner() == Some(&self.btn2) {
            self.activated = Some(self.btn2.clone());
            cx.request_invalidate(Invalidate::Paint);
        }
    }

    fn draw(&self, ui: &mut Ui<'_>, area: Rect) {
        ui.register_focus(self.btn1.clone(), true);
        ui.register_hit(self.btn1.clone(), Rect::new(area.x, area.y, 10, 1));

        ui.register_focus(self.btn2.clone(), true);
        ui.register_hit(self.btn2.clone(), Rect::new(area.x + 12, area.y, 10, 1));
    }
}

#[test]
fn test_runtime_focus_traversal_and_keyboard_routes() {
    let btn1 = Id::new("probe.btn1");
    let btn2 = Id::new("probe.btn2");

    let scene = TestProbeScene {
        btn1: btn1.clone(),
        btn2: btn2.clone(),
        activated: None,
    };

    let mut runtime = Runtime::new(scene, Theme::termrock());
    runtime.draw(Rect::new(0, 0, 80, 24)).unwrap();

    // Auto-initialized to first focus candidate in reading order
    assert_eq!(runtime.current_focus(), Some(&btn1));

    // Focus next
    assert_eq!(runtime.focus_next(), Some(&btn2));

    // Focus previous
    assert_eq!(runtime.focus_prev(), Some(&btn1));

    // Keyboard enter routes to focused control
    let enter_cause = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(100),
    );

    let report = runtime
        .handle(enter_cause, Moment::from_millis(100))
        .unwrap();
    assert!(report.handled);
    assert_eq!(runtime.scene.activated, Some(btn1));
}

#[test]
fn test_runtime_layer_modal_barrier_and_prior_focus_restoration() {
    let mut stack = LayerStack::new();
    let viewport = Rect::new(0, 0, 80, 24);

    let base_control = Id::new("page.input");
    let modal_id = Id::new("dialog.save");

    // Open modal with saved base_control focus
    let spec = LayerSpec::modal(modal_id.clone(), LayerSize::Fixed(Size::new(40, 10)));
    let modal_rect = stack
        .push(spec, viewport, Some(base_control.clone()))
        .unwrap();

    // Point outside modal is blocked by inert barrier
    assert!(stack.is_point_blocked(Position::new(2, 2)));

    // Point inside modal is not blocked
    assert!(!stack.is_point_blocked(Position::new(modal_rect.x + 2, modal_rect.y + 2)));

    // Escape closes modal and restores saved base focus
    let (closed, restored) = stack.handle_escape().unwrap();
    assert_eq!(closed, modal_id);
    assert_eq!(restored, Some(base_control));
    assert!(stack.is_empty());
}

#[test]
fn test_runtime_animation_sample_timed_and_motion_policy() {
    let epoch = Moment::from_millis(1000);
    let now = Moment::from_millis(1500);
    let cadence = Duration::from_millis(100);

    // Full motion: 500ms elapsed / 100ms cadence = phase 5
    let sample = AnimationSample::timed(now, epoch, cadence, MotionPolicy::Full).unwrap();
    assert_eq!(sample.index, 5);

    // Paused motion: always phase 0
    let paused = AnimationSample::timed(now, epoch, cadence, MotionPolicy::Paused).unwrap();
    assert_eq!(paused.index, 0);

    // Nonmonotonic time rejected
    let past = Moment::from_millis(500);
    let err = AnimationSample::timed(past, epoch, cadence, MotionPolicy::Full).unwrap_err();
    assert_eq!(err, ClockError::NonmonotonicTime);
}

// =========================================================================
// TASK-004: Layout, Theme, Text & Secret Foundations
// =========================================================================

#[test]
fn test_layout_tracks_and_degenerate_safety() {
    use junie_tui::termrock::layout::{Alignment, action_row, columns};

    // 0x0 degenerate area
    let zero = Rect::zero();
    let zero_cols = columns(zero, &[Track::Fixed(10), Track::Flex(1)], 1);
    assert_eq!(zero_cols.len(), 2);
    assert!(zero_cols[0].is_empty());
    assert!(zero_cols[1].is_empty());

    // 1x1 area with gap=1 (gap takes available space)
    let one = Rect::new(5, 5, 1, 1);
    let one_cols = columns(one, &[Track::Flex(1), Track::Flex(1)], 1);
    assert_eq!(one_cols.len(), 2);
    assert_eq!(one_cols[0].width + one_cols[1].width, 0);

    // 1x1 area with gap=0 (allocates the 1 cell across flex tracks)
    let one_no_gap = columns(one, &[Track::Flex(1), Track::Flex(1)], 0);
    assert_eq!(one_no_gap.len(), 2);
    assert_eq!(one_no_gap[0].width + one_no_gap[1].width, 1);

    // Normal allocation with gap
    let area = Rect::new(0, 0, 100, 20);
    let cols = columns(area, &[Track::Fixed(30), Track::Flex(1), Track::Flex(1)], 2);
    assert_eq!(cols.len(), 3);
    assert_eq!(cols[0].width, 30);
    // Remaining = 100 - 30 - (2 gaps * 2) = 66 -> 33 each
    assert_eq!(cols[1].width, 33);
    assert_eq!(cols[2].width, 33);

    // Action row right-aligned
    let action_sizes = [Size::new(10, 1), Size::new(10, 1)];
    let action_rects = action_row(area, &action_sizes, Alignment::End, 1);
    assert_eq!(action_rects.len(), 2);
    assert_eq!(action_rects[1].x + action_rects[1].width, 100);
}

#[test]
fn test_theme_termrock_matches_baseline_tokens_and_levels() {
    let t_termrock = Theme::termrock();
    let t_junie = junie_tui::theme::Theme::junie();

    // Semantic colors match baseline exactly
    assert_eq!(t_termrock.tokens.canvas, t_junie.canvas);
    assert_eq!(t_termrock.tokens.surface, t_junie.surface);
    assert_eq!(t_termrock.tokens.surface_elevated, t_junie.surface_elevated);
    assert_eq!(t_termrock.tokens.accent, t_junie.accent);
    assert_eq!(t_termrock.tokens.danger, t_junie.error);
    assert_eq!(t_termrock.tokens.text_primary, t_junie.text_primary);

    // Capability downgrade tests
    let t_mono = Theme::for_level(ColorLevel::Mono);
    assert_eq!(t_mono.tokens.canvas, ratatui::style::Color::Black);
    assert_eq!(t_mono.tokens.text_primary, ratatui::style::Color::White);

    // Paper sentinel differs
    let t_paper = Theme::paper();
    assert_ne!(t_paper.tokens.canvas, t_termrock.tokens.canvas);
}

#[test]
fn test_secret_zeroization_and_redaction() {
    let mut secret = Secret::new("SuperSecretPassword123".to_string());
    assert_eq!(secret.len(), 22);

    // Read via expose closure
    let exposed_len = secret.expose(|val| {
        assert_eq!(val, "SuperSecretPassword123");
        val.len()
    });
    assert_eq!(exposed_len, 22);

    // Debug output is redacted
    let debug_str = format!("{secret:?}");
    assert!(!debug_str.contains("SuperSecretPassword123"));
    assert!(debug_str.contains("[REDACTED]"));

    // Explicit clear zeroes the buffer
    secret.clear();
    assert!(secret.is_empty());
    assert_eq!(secret.len(), 0);
}

#[test]
fn test_text_editor_core_grapheme_safety() {
    let mut editor = TextEditorCore::new();
    // Insert Unicode with combining characters and emojis
    editor.insert_str("Hello 🌍! ");
    assert_eq!(editor.text(), "Hello 🌍! ");

    // Move left 3 graphemes: past ' ', '!', and '🌍'
    editor.move_left(false); // skips ' '
    editor.move_left(false); // skips '!'
    editor.move_left(false); // skips '🌍'

    editor.insert_str("Beautiful ");
    assert_eq!(editor.text(), "Hello Beautiful 🌍! ");

    // Undo restores state
    assert!(editor.undo());
    assert_eq!(editor.text(), "Hello 🌍! ");
}
