//! Comprehensive Probes for Termrock P2 TASK-005: Basic Controls and Semantic Chrome.
//!
//! Validates:
//! - AC-001: Scoped control construction, measure, update, draw, and typed actions.
//! - AC-002: Disabled, read-only, narrow, focus, hover, and activation states.
//! - AC-003: Protected paths and application invariants.
//! - AC-004: Completion gate passing.

use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use std::time::Duration;

use junie_tui::core::event::{Input, Key, Mouse, MouseKind};
use junie_tui::termrock::{
    ActionKey, ActionMeta, Activated, ActivationOrigin, Brand, Button, ButtonVariant, ChipAction,
    ChipBar, ChipBarState, ChipItem, ColorLevel, Constraints, ControlStatus, CopyPolicy, Cx, Empty,
    EmptyAction, Id, Invalidate, ItemKey, LayerStack, MeasureCx, Moment, Part, Props, PropsAction,
    PropsList, PropsRow, PropsState, PropsValue, Readiness, Rect, Revision, Runtime, Scene, Size,
    StylePatch, Theme, Ui, UpdateCause,
};

// =========================================================================
// 1. Button Probes: Construction, Measure, Update, Draw, Feedback, Zero-Area
// =========================================================================

#[test]
fn test_button_construction_and_builder_options() {
    let id = Id::new("test.btn");
    let btn = Button::new(id.clone(), "Save")
        .variant(ButtonVariant::Primary)
        .status(ControlStatus::Normal)
        .checked(Some(true))
        .icon(Some("💾"))
        .autofocus(true)
        .patch(StylePatch::empty())
        .patch_part(Part::LABEL, StylePatch::empty());

    assert_eq!(btn.id, id);
    assert_eq!(btn.label, "Save");
    assert_eq!(btn.variant, ButtonVariant::Primary);
    assert_eq!(btn.status, ControlStatus::Normal);
    assert_eq!(btn.checked, Some(true));
    assert_eq!(btn.icon, Some("💾"));
    assert!(btn.autofocus);
    assert!(btn.is_enabled());
    assert!(!btn.is_disabled());
    assert!(!btn.is_busy());

    let disabled_btn = btn.disabled(true);
    assert!(disabled_btn.is_disabled());
    assert!(!disabled_btn.is_enabled());

    let busy_btn = disabled_btn.busy(true);
    assert!(busy_btn.is_busy());
    assert!(!busy_btn.is_enabled());
}

#[test]
fn test_button_measure_display_width_and_unicode() {
    let id = Id::new("measure.btn");
    let theme = Theme::termrock();
    let cx = MeasureCx::new(Constraints::unbounded(), &theme, ColorLevel::TrueColor);

    // ASCII: "Ok" (2) + padding (2) = 4
    let btn_ascii = Button::new(id.clone(), "Ok");
    let size_ascii = btn_ascii.measure(&cx, Constraints::unbounded());
    assert_eq!(size_ascii, Size::new(4, 1));

    // Wide CJK: "确定" (4) + padding (2) = 6
    let btn_cjk = Button::new(id.clone(), "确定");
    let size_cjk = btn_cjk.measure(&cx, Constraints::unbounded());
    assert_eq!(size_cjk, Size::new(6, 1));

    // Toggle marker adds 2 cells: "Toggle" (6) + padding (2) + marker (2) = 10
    let btn_toggle = Button::new(id.clone(), "Toggle").checked(Some(false));
    let size_toggle = btn_toggle.measure(&cx, Constraints::unbounded());
    assert_eq!(size_toggle, Size::new(10, 1));

    // Busy spinner adds 2 cells: "Save" (4) + padding (2) + spinner (2) = 8
    let btn_busy = Button::new(id.clone(), "Save").busy(true);
    let size_busy = btn_busy.measure(&cx, Constraints::unbounded());
    assert_eq!(size_busy, Size::new(8, 1));
}

#[test]
fn test_button_zero_and_tiny_area_no_panic() {
    let id = Id::new("zero.btn");
    let btn = Button::new(id.clone(), "Action");
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();

    // 0x0 area
    let mut ui = Ui::new(&theme, Rect::zero(), &mut layers);
    let drawn = btn.draw(&mut ui, Rect::zero());
    assert_eq!(drawn, Rect::zero());
    assert!(!ui.hit_regions.contains_key(&id));

    // 1x1 area
    let tiny = Rect::new(0, 0, 1, 1);
    let mut ui_tiny = Ui::new(&theme, tiny, &mut layers);
    let drawn_tiny = btn.draw(&mut ui_tiny, tiny);
    assert_eq!(drawn_tiny.width, 1);
    assert_eq!(drawn_tiny.height, 1);
}

#[test]
fn test_button_disabled_and_busy_reject_activation() {
    let id = Id::new("disabled.btn");
    let disabled_btn = Button::new(id.clone(), "Disabled").disabled(true);
    let busy_btn = Button::new(id.clone(), "Busy").busy(true);

    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = std::collections::HashMap::from([(id.clone(), Rect::new(0, 0, 10, 1))]);

    let mut cx = Cx {
        cause: &cause_enter,
        moment: Moment::from_millis(100),
        intended_owner: Some(id.clone()),
        focus: Some(id.clone()),
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: Some(&geom),
    };

    // Disabled button must emit NO action
    let resp_dis = disabled_btn.update(&mut cx);
    assert_eq!(resp_dis.action, None);

    // Busy button must emit NO action
    let resp_busy = busy_btn.update(&mut cx);
    assert_eq!(resp_busy.action, None);

    // Mouse click rejection
    let cause_click = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Down,
            pos: ratatui::layout::Position::new(2, 0),
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_click;
    let resp_click = disabled_btn.update(&mut cx);
    assert_eq!(resp_click.action, None);
    assert_eq!(
        cx.new_capture, None,
        "Disabled button must not capture pointer"
    );
}

#[test]
fn test_button_pointer_down_capture_and_release_outside_cancels() {
    let id = Id::new("pointer.btn");
    let btn = Button::new(id.clone(), "ClickMe");
    let mut layers = LayerStack::new();
    let geom = std::collections::HashMap::from([(id.clone(), Rect::new(5, 5, 10, 1))]);

    // 1. Mouse Down inside: captures pointer, emits NO action
    let down_cause = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Down,
            pos: ratatui::layout::Position::new(7, 5),
        }),
        Moment::from_millis(100),
    );
    let mut cx = Cx {
        cause: &down_cause,
        moment: Moment::from_millis(100),
        intended_owner: Some(id.clone()),
        focus: None,
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: Some(&geom),
    };

    let resp_down = btn.update(&mut cx);
    assert_eq!(resp_down.action, None, "Down alone must never activate");
    assert_eq!(
        cx.new_capture,
        Some(Some(id.clone())),
        "Down inside must capture"
    );

    // 2. Mouse Up outside (pos 30, 20): releases capture, cancels activation
    let up_outside = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(30, 20),
        }),
        Moment::from_millis(150),
    );
    cx.cause = &up_outside;
    cx.pointer_capture = Some(id.clone());
    cx.new_capture = None;

    let resp_up = btn.update(&mut cx);
    assert_eq!(
        resp_up.action, None,
        "Release outside must cancel activation"
    );
    assert_eq!(
        cx.new_capture,
        Some(None),
        "Release outside must release capture"
    );

    // 3. Mouse Up inside (pos 7, 5): activates exactly once!
    let up_inside = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(7, 5),
        }),
        Moment::from_millis(200),
    );
    cx.cause = &up_inside;
    cx.pointer_capture = Some(id.clone());
    cx.new_capture = None;
    cx.feedback_requests.clear();

    let resp_activate = btn.update(&mut cx);
    assert_eq!(
        resp_activate.action,
        Some(Activated::new(ActivationOrigin::Pointer)),
        "Release inside captured button must activate"
    );
    assert_eq!(
        cx.new_capture,
        Some(None),
        "Release inside must release capture"
    );
    assert_eq!(cx.feedback_requests.len(), 1, "Must trigger 140ms feedback");
    assert_eq!(cx.feedback_requests[0].1, Duration::from_millis(140));
}

#[test]
fn test_button_keyboard_activation_enter_and_space() {
    let id = Id::new("kbd.btn");
    let btn = Button::new(id.clone(), "Keyboard");
    let mut layers = LayerStack::new();

    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(50),
    );
    let mut cx = Cx {
        cause: &cause_enter,
        moment: Moment::from_millis(50),
        intended_owner: Some(id.clone()),
        focus: Some(id.clone()),
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: None,
    };

    let resp_enter = btn.update(&mut cx);
    assert_eq!(
        resp_enter.action,
        Some(Activated::new(ActivationOrigin::Keyboard))
    );

    let cause_space = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char(' '),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(60),
    );
    cx.cause = &cause_space;
    let resp_space = btn.update(&mut cx);
    assert_eq!(
        resp_space.action,
        Some(Activated::new(ActivationOrigin::Keyboard))
    );
}

// Scene for testing 140ms feedback timing via Runtime
struct FeedbackProbeScene {
    btn_id: Id,
    activated: bool,
}

impl Scene for FeedbackProbeScene {
    fn update(&mut self, cx: &mut Cx<'_>, _cause: UpdateCause) {
        let btn = Button::new(self.btn_id.clone(), "Probe");
        let resp = btn.update(cx);
        if resp.action.is_some() {
            self.activated = true;
        }
    }

    fn draw(&self, ui: &mut Ui<'_>, area: Rect) {
        let btn = Button::new(self.btn_id.clone(), "Probe");
        btn.draw(ui, area);
    }
}

#[test]
fn test_button_feedback_duration_140ms_boundaries() {
    let btn_id = Id::new("feedback.btn");
    let scene = FeedbackProbeScene {
        btn_id: btn_id.clone(),
        activated: false,
    };
    let mut runtime = Runtime::new(scene, Theme::termrock());
    let area = Rect::new(0, 0, 40, 5);

    // Initial draw to publish geometry
    runtime.draw(area).unwrap();

    // Trigger activation at t = 1000 ms
    let click = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(2, 0),
        }),
        Moment::from_millis(1000),
    );
    runtime.handle(click, Moment::from_millis(1000)).unwrap();
    assert!(runtime.scene.activated);

    // Check feedback at exact boundaries:
    // At t = 1000 ms (elapsed 0 ms): active
    assert!(runtime.is_feedback(&btn_id, Moment::from_millis(1000)));

    // At t = 1139 ms (elapsed 139 ms): still active!
    assert!(runtime.is_feedback(&btn_id, Moment::from_millis(1139)));

    // At t = 1140 ms (elapsed 140 ms): expired!
    let tick_140 = UpdateCause::Tick(Moment::from_millis(1140));
    runtime.handle(tick_140, Moment::from_millis(1140)).unwrap();
    assert!(!runtime.is_feedback(&btn_id, Moment::from_millis(1140)));

    // At t = 1141 ms (elapsed 141 ms): expired!
    let tick_141 = UpdateCause::Tick(Moment::from_millis(1141));
    runtime.handle(tick_141, Moment::from_millis(1141)).unwrap();
    assert!(!runtime.is_feedback(&btn_id, Moment::from_millis(1141)));
}

#[test]
fn test_row_layout_and_row_layout_right() {
    let area = Rect::new(10, 5, 50, 2);
    let widths = vec![10, 12, 8];
    let gap = 2;

    let left = junie_tui::termrock::row_layout(area, &widths, gap);
    assert_eq!(left.len(), 3);
    assert_eq!(left[0], Rect::new(10, 5, 10, 1));
    assert_eq!(left[1], Rect::new(22, 5, 12, 1));
    assert_eq!(left[2], Rect::new(36, 5, 8, 1));

    let right = junie_tui::termrock::row_layout_right(area, &widths, gap);
    assert_eq!(right.len(), 3);
    // Total widths = 30 + 4 gap = 34. Slack = 50 - 34 = 16. Start = 10 + 16 = 26.
    assert_eq!(right[0], Rect::new(26, 5, 10, 1));
    assert_eq!(right[1], Rect::new(38, 5, 12, 1));
    assert_eq!(right[2], Rect::new(52, 5, 8, 1));
}

// =========================================================================
// 2. Brand Probes: Static vs Interactive, Metadata, Compact
// =========================================================================

#[test]
fn test_brand_static_mode_is_decorative_never_registers_focus_or_hit() {
    let id = Id::new("brand.static");
    let brand = Brand::new(id.clone(), "holla❯").meta("v1.2.0");
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();

    let mut ui = Ui::new(&theme, Rect::new(0, 0, 40, 1), &mut layers);
    let rect = brand.draw(&mut ui, Rect::new(0, 0, 40, 1));
    assert!(!rect.is_empty());
    assert!(
        !ui.hit_regions.contains_key(&id),
        "Static brand must never register hit"
    );
    assert!(
        !ui.focus_candidates.contains(&id),
        "Static brand must never register focus"
    );

    let cause = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Down,
            pos: ratatui::layout::Position::new(2, 0),
        }),
        Moment::from_millis(50),
    );
    let mut cx = Cx {
        cause: &cause,
        moment: Moment::from_millis(50),
        intended_owner: Some(id.clone()),
        focus: None,
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: None,
    };

    let resp = brand.update(&mut cx);
    assert_eq!(resp.action, None);
    assert_eq!(cx.new_capture, None);
}

#[test]
fn test_brand_interactive_mode_registers_and_activates() {
    let id = Id::new("brand.interactive");
    let brand = Brand::new(id.clone(), "jackin❯").interactive(true);
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();

    let mut ui = Ui::new(&theme, Rect::new(0, 0, 40, 1), &mut layers);
    brand.draw(&mut ui, Rect::new(0, 0, 40, 1));
    assert!(ui.hit_regions.contains_key(&id));
    assert!(ui.focus_candidates.contains(&id));

    let geom = std::collections::HashMap::from([(id.clone(), Rect::new(0, 0, 15, 1))]);
    let click = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(3, 0),
        }),
        Moment::from_millis(100),
    );
    let mut cx = Cx {
        cause: &click,
        moment: Moment::from_millis(100),
        intended_owner: Some(id.clone()),
        focus: None,
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: Some(&geom),
    };

    let resp = brand.update(&mut cx);
    assert_eq!(resp.action, Some(Activated::new(ActivationOrigin::Pointer)));
    assert_eq!(cx.feedback_requests.len(), 1);
}

// =========================================================================
// 3. ChipBar Probes: Key Navigation, Subpart Click Disjointness, Overflow
// =========================================================================

#[test]
fn test_chip_bar_keyboard_navigation_and_actions() {
    let id = Id::new("chipbar");
    let chips = vec![
        ChipItem::new(ItemKey::new(1), "status:active"),
        ChipItem::new(ItemKey::new(2), "role:admin").checked(Some(true)),
        ChipItem::new(ItemKey::new(3), "env:prod").closable(true),
    ];
    let bar = ChipBar::new(id.clone(), &chips, Revision::zero());
    let mut state = ChipBarState::new();
    let mut layers = LayerStack::new();

    // 1. Right arrow moves cursor from 0 to 1
    let right_key = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Right,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(10),
    );
    let mut cx = Cx {
        cause: &right_key,
        moment: Moment::from_millis(10),
        intended_owner: Some(id.clone()),
        focus: Some(id.clone()),
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: None,
    };
    let resp = bar.update(&mut cx, &mut state);
    assert_eq!(state.cursor, 1);
    assert_eq!(resp.action, None);

    // 2. Space toggles checked on chip 2
    let space_key = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char(' '),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(20),
    );
    cx.cause = &space_key;
    let resp_toggle = bar.update(&mut cx, &mut state);
    assert_eq!(
        resp_toggle.action,
        Some(ChipAction::SetChecked {
            key: ItemKey::new(2),
            checked: false,
            origin: ActivationOrigin::Keyboard,
        })
    );

    // 3. Move to chip 3 and delete it with 'x'
    state.set_cursor(2);
    let del_key = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('x'),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(30),
    );
    cx.cause = &del_key;
    let resp_del = bar.update(&mut cx, &mut state);
    assert_eq!(
        resp_del.action,
        Some(ChipAction::Close {
            key: ItemKey::new(3),
            origin: ActivationOrigin::Keyboard,
        })
    );
}

#[test]
fn test_chip_bar_close_click_never_activates_chip_body() {
    let id = Id::new("chipbar.clicks");
    let chips = vec![ChipItem::new(ItemKey::new(42), "target")];
    let bar = ChipBar::new(id.clone(), &chips, Revision::zero());
    let mut state = ChipBarState::new();
    let mut layers = LayerStack::new();

    let close_id = bar.close_id(ItemKey::new(42));
    let click_close = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(10, 0),
        }),
        Moment::from_millis(50),
    );
    let mut cx = Cx {
        cause: &click_close,
        moment: Moment::from_millis(50),
        intended_owner: Some(close_id),
        focus: None,
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: None,
    };

    let resp = bar.update(&mut cx, &mut state);
    // MUST emit Close, and MUST NOT emit Activate!
    assert_eq!(
        resp.action,
        Some(ChipAction::Close {
            key: ItemKey::new(42),
            origin: ActivationOrigin::Pointer,
        })
    );

    // Click on chip body emits Activate
    let chip_id = bar.chip_id(ItemKey::new(42));
    cx.intended_owner = Some(chip_id);
    let resp_body = bar.update(&mut cx, &mut state);
    assert_eq!(
        resp_body.action,
        Some(ChipAction::Activate {
            key: ItemKey::new(42),
            origin: ActivationOrigin::Pointer,
        })
    );
}

// =========================================================================
// 4. Props & PropsList Probes: Protected Copy Rejection & Static Invariants
// =========================================================================

#[test]
fn test_props_static_display_only_never_registers_focus_or_hit() {
    let id = Id::new("props.static");
    let rows = vec![
        PropsRow::new(ItemKey::new(1), "Host", PropsValue::Text("localhost")),
        PropsRow::new(ItemKey::new(2), "Port", PropsValue::Text("5432")),
    ];
    let props = Props::new(id.clone(), &rows);
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();

    let mut ui = Ui::new(&theme, Rect::new(0, 0, 40, 5), &mut layers);
    let rect = props.draw(&mut ui, Rect::new(0, 0, 40, 5));
    assert_eq!(rect.height, 2);
    assert!(
        ui.hit_regions.is_empty(),
        "Static Props must never register hit"
    );
    assert!(
        ui.focus_candidates.is_empty(),
        "Static Props must never register focus"
    );
}

#[test]
fn test_props_list_protected_value_copy_is_strictly_rejected() {
    let id = Id::new("props.list");
    let rows = vec![
        PropsRow::new(ItemKey::new(1), "Public", PropsValue::Text("open-value")).copyable(true),
        PropsRow::new(ItemKey::new(2), "Token", PropsValue::Protected("••••••••")).copyable(true),
    ];
    let list =
        PropsList::new(id.clone(), &rows, Revision::zero()).copy_policy(CopyPolicy::DenyProtected);

    let mut state = PropsState::new();
    let mut layers = LayerStack::new();

    // 1. Copy on safe public row (cursor = 0): succeeds
    state.set_cursor(0);
    let y_key = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('y'),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = Cx {
        cause: &y_key,
        moment: Moment::from_millis(100),
        intended_owner: Some(id.clone()),
        focus: Some(id.clone()),
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: None,
    };
    let resp_safe = list.update(&mut cx, &mut state);
    assert_eq!(
        resp_safe.action,
        Some(PropsAction::CopyRequested {
            key: ItemKey::new(1),
            value: "open-value".to_string(),
        })
    );

    // 2. Copy on protected row (cursor = 1): REJECTED! No action emitted!
    state.set_cursor(1);
    let resp_protected = list.update(&mut cx, &mut state);
    assert_eq!(
        resp_protected.action, None,
        "Protected row copy must be strictly rejected"
    );
}

// =========================================================================
// 5. Empty State Probes: Message-only vs Action Button
// =========================================================================

#[test]
fn test_empty_message_only_never_registers_focus_or_hit() {
    let id = Id::new("empty.msg");
    let empty = Empty::new(id.clone(), Readiness::Empty)
        .title("No documents found")
        .detail("Try adjusting your search criteria.");

    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let mut ui = Ui::new(&theme, Rect::new(0, 0, 50, 10), &mut layers);

    empty.draw(&mut ui, Rect::new(0, 0, 50, 10));
    assert!(
        !ui.hit_regions.contains_key(&id),
        "Message-only empty state must never register hit"
    );
    assert!(
        !ui.focus_candidates.contains(&id),
        "Message-only empty state must never register focus"
    );

    let cause = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(10),
    );
    let mut cx = Cx {
        cause: &cause,
        moment: Moment::from_millis(10),
        intended_owner: Some(id.clone()),
        focus: Some(id.clone()),
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: None,
    };
    let resp = empty.update(&mut cx);
    assert_eq!(resp.action, None);
}

#[test]
fn test_empty_with_action_button_draws_and_invokes() {
    let id = Id::new("empty.action");
    let empty = Empty::new(id.clone(), Readiness::Error("Connection lost"))
        .detail("The remote server closed the connection.")
        .action(Some(ActionMeta::new(ActionKey::new("retry"), "Retry Now")));

    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let mut ui = Ui::new(&theme, Rect::new(0, 0, 50, 10), &mut layers);

    empty.draw(&mut ui, Rect::new(0, 0, 50, 10));
    let child_btn_id = id.sub("action");
    assert!(
        ui.focus_candidates.contains(&child_btn_id),
        "Child retry button must be focus candidate"
    );
    assert!(
        ui.hit_regions.contains_key(&child_btn_id),
        "Child retry button must have hit region"
    );

    // Keyboard activate on child button
    let cause = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(10),
    );
    let mut cx = Cx {
        cause: &cause,
        moment: Moment::from_millis(10),
        intended_owner: Some(child_btn_id.clone()),
        focus: Some(child_btn_id),
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: &mut layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: None,
    };

    let resp = empty.update(&mut cx);
    assert_eq!(
        resp.action,
        Some(EmptyAction::Invoke {
            action: ActionKey::new("retry"),
            origin: ActivationOrigin::Keyboard,
        })
    );
}

// =========================================================================
// 6. Buffer Integration & Style Verification Probes
// =========================================================================

#[test]
fn test_button_buffer_rendering_and_styling() {
    let id = Id::new("render.btn");
    let btn = Button::new(id, "Submit").variant(ButtonVariant::Primary);
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 1).into());

    let mut ui = Ui::new(&theme, Rect::new(0, 0, 20, 1), &mut layers).with_buffer(&mut buf);
    btn.draw(&mut ui, Rect::new(0, 0, 20, 1));

    // Submit width: 6 chars + 2 padding = 8 cells.
    assert_eq!(buf[(0, 0)].symbol(), " ");
    assert_eq!(buf[(1, 0)].symbol(), "S");
    assert_eq!(buf[(6, 0)].symbol(), "t");
    assert_eq!(buf[(7, 0)].symbol(), " ");
    assert_eq!(buf[(1, 0)].bg, theme.tokens.accent);
    assert_eq!(buf[(1, 0)].fg, theme.tokens.text_on_accent);
}
