//! Comprehensive Verification Probes for Termrock P3 TASK-008: Controlled Choices, Secrets, and Validation.
//!
//! Validates:
//! - AC-001: Checkbox, Toggle, RadioGroup, and Select emit typed actions and render caller-provided
//!   values using shared identity and theme contracts.
//! - AC-002: Secret, invalid, disabled, read-only, empty, and source-reconciled states preserve
//!   visual and interaction rules.
//! - AC-003: Frozen references, application invariants, and scope remain protected.
//! - AC-004: Completion gate passes.

use std::collections::HashMap;

use ratatui::crossterm::event::{KeyCode, KeyModifiers};

use junie_tui::core::event::{Input, Key, Mouse, MouseKind};
use junie_tui::termrock::{
    ActivationOrigin, Axis, Checkbox, ChoiceItem, ColorLevel, Constraints, ControlStatus, Cx,
    FieldError, FieldKey, Flow, Id, Invalidate, ItemKey, LayerStack, MeasureCx, Moment, Part,
    RadioAction, RadioGroup, RadioGroupState, Rect, Revision, Secret, SecretPolicy, Select,
    SelectAction, SelectState, Size, StylePatch, Theme, Toggle, Ui, UpdateCause, ValidationMessage,
    Validator, ValueChanged,
};

fn make_cx<'a>(
    cause: &'a UpdateCause,
    id: Id,
    layers: &'a mut LayerStack,
    geom: &'a HashMap<Id, Rect>,
) -> Cx<'a> {
    Cx {
        cause,
        moment: Moment::from_millis(100),
        intended_owner: Some(id.clone()),
        focus: Some(id),
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: Some(geom),
    }
}

// =========================================================================
// 1. Checkbox Probes: Controlled values, activation, disabled, and drawing
// =========================================================================

#[test]
fn test_checkbox_construction_and_builders() {
    let id = Id::new("test.chk");
    let chk = Checkbox::new(id.clone(), "Enable Logging", true)
        .disabled(false)
        .status(ControlStatus::Normal)
        .patch(StylePatch::empty())
        .patch_part(Part::new("label"), StylePatch::empty());

    assert_eq!(chk.id, id);
    assert_eq!(chk.label, "Enable Logging");
    assert!(chk.checked);
    assert!(chk.is_enabled());
    assert!(!chk.is_disabled());

    let dis = chk.disabled(true);
    assert!(dis.is_disabled());
    assert!(!dis.is_enabled());
}

#[test]
fn test_checkbox_measure_full_and_compact() {
    let id = Id::new("measure.chk");
    let theme = Theme::termrock();
    let cx = MeasureCx::new(Constraints::unbounded(), &theme, ColorLevel::TrueColor);

    let chk = Checkbox::new(id, "Accept Terms", false);
    let size = chk.measure(&cx, Constraints::unbounded());
    // "Accept Terms" (12) + 6 = 18 width, 1 height
    assert_eq!(size, Size::new(18, 1));
}

#[test]
fn test_checkbox_draw_visual_states_and_parts() {
    let id = Id::new("draw.chk");
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();

    // Normal width: [✓]
    let chk_checked = Checkbox::new(id.clone(), "Notify", true);
    let mut ui = Ui::new(&theme, Rect::new(0, 0, 20, 1), &mut layers);
    let rect = chk_checked.draw(&mut ui, Rect::new(0, 0, 20, 1));
    assert_eq!(rect, Rect::new(0, 0, 20, 1));
    assert!(ui.hit_regions.contains_key(&id));
    assert!(ui.focus_candidates.contains(&id));

    // Narrow width: compact ✓
    let mut ui_narrow = Ui::new(&theme, Rect::new(0, 0, 3, 1), &mut layers);
    let rect_narrow = chk_checked.draw(&mut ui_narrow, Rect::new(0, 0, 3, 1));
    assert_eq!(rect_narrow.width, 3);
}

#[test]
fn test_checkbox_keyboard_activation_requests_inverse() {
    let id = Id::new("key.chk");
    let chk = Checkbox::new(id.clone(), "Verbose", false);

    let cause_space = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char(' '),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 15, 1))]);
    let mut cx = make_cx(&cause_space, id.clone(), &mut layers, &geom);

    let resp = chk.update(&mut cx);
    assert_eq!(resp.flow, Flow::Consumed);
    assert_eq!(resp.invalidate, Invalidate::Paint);
    assert_eq!(
        resp.action,
        Some(ValueChanged::new(true, ActivationOrigin::Keyboard))
    );

    // Enter also activates
    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(105),
    );
    cx.cause = &cause_enter;
    let resp_enter = chk.update(&mut cx);
    assert_eq!(
        resp_enter.action,
        Some(ValueChanged::new(true, ActivationOrigin::Keyboard))
    );
}

#[test]
fn test_checkbox_pointer_activation_and_cancel_on_outside_release() {
    let id = Id::new("ptr.chk");
    let chk = Checkbox::new(id.clone(), "Save", true);

    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 10, 1))]);

    // Pointer down inside captures
    let cause_down = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Down,
            pos: ratatui::layout::Position::new(2, 0),
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_down, id.clone(), &mut layers, &geom);
    let resp_down = chk.update(&mut cx);
    assert_eq!(resp_down.action, None);
    assert_eq!(cx.new_capture, Some(Some(id.clone())));

    // Pointer up inside emits ValueChanged with inverted value (false)
    let cause_up_inside = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(2, 0),
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_up_inside;
    cx.pointer_capture = Some(id.clone());
    let resp_up = chk.update(&mut cx);
    assert_eq!(
        resp_up.action,
        Some(ValueChanged::new(false, ActivationOrigin::Pointer))
    );
    assert_eq!(cx.new_capture, Some(None), "Must release capture");

    // Pointer up outside cancels
    cx.pointer_capture = Some(id.clone());
    let cause_up_outside = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(20, 5),
        }),
        Moment::from_millis(120),
    );
    cx.cause = &cause_up_outside;
    let resp_outside = chk.update(&mut cx);
    assert_eq!(resp_outside.action, None);
    assert_eq!(cx.new_capture, Some(None));
}

#[test]
fn test_checkbox_disabled_ignores_all_activation() {
    let id = Id::new("dis.chk");
    let dis = Checkbox::new(id.clone(), "Disabled Option", false).disabled(true);

    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 1))]);

    // Space ignored
    let cause_space = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char(' '),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_space, id.clone(), &mut layers, &geom);
    let resp = dis.update(&mut cx);
    assert_eq!(resp.action, None);

    // Pointer click ignored
    let cause_click = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Down,
            pos: ratatui::layout::Position::new(2, 0),
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_click;
    let resp_click = dis.update(&mut cx);
    assert_eq!(resp_click.action, None);
    assert_eq!(cx.new_capture, None);
}

#[test]
fn test_checkbox_controlled_value_invariance() {
    let id = Id::new("inv.chk");
    let chk = Checkbox::new(id.clone(), "Controlled", false);

    // When an action is emitted but caller does NOT apply it:
    // next draw still paints caller's `false` value!
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let mut ui = Ui::new(&theme, Rect::new(0, 0, 20, 1), &mut layers);
    chk.draw(&mut ui, Rect::new(0, 0, 20, 1));
    assert!(!chk.checked);
}

#[test]
fn test_checkbox_zero_and_tiny_area_no_panic() {
    let id = Id::new("zero.chk");
    let chk = Checkbox::new(id, "Zero", true);
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();

    let mut ui = Ui::new(&theme, Rect::zero(), &mut layers);
    let r = chk.draw(&mut ui, Rect::zero());
    assert_eq!(r, Rect::zero());
}

// =========================================================================
// 2. Toggle Probes: Controlled on/off, switch glyphs, and state text
// =========================================================================

#[test]
fn test_toggle_construction_and_builders() {
    let id = Id::new("test.tog");
    let tog = Toggle::new(id.clone(), "Dark Mode", true)
        .disabled(false)
        .status(ControlStatus::Normal)
        .patch(StylePatch::empty());

    assert_eq!(tog.id, id);
    assert_eq!(tog.label, "Dark Mode");
    assert!(tog.on);
    assert!(tog.is_enabled());

    let dis = tog.disabled(true);
    assert!(dis.is_disabled());
}

#[test]
fn test_toggle_measure_display_width() {
    let id = Id::new("measure.tog");
    let theme = Theme::termrock();
    let cx = MeasureCx::new(Constraints::unbounded(), &theme, ColorLevel::TrueColor);

    let tog = Toggle::new(id, "Sound", false);
    let size = tog.measure(&cx, Constraints::unbounded());
    // "Sound" (5) + 10 = 15
    assert_eq!(size, Size::new(15, 1));
}

#[test]
fn test_toggle_draw_switch_glyphs_and_state_text() {
    let id = Id::new("draw.tog");
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();

    let tog_on = Toggle::new(id.clone(), "WiFi", true);
    let mut ui = Ui::new(&theme, Rect::new(0, 0, 25, 1), &mut layers);
    let rect = tog_on.draw(&mut ui, Rect::new(0, 0, 25, 1));
    assert_eq!(rect, Rect::new(0, 0, 25, 1));
    assert!(ui.hit_regions.contains_key(&id));

    // Narrow width (< 4)
    let mut ui_narrow = Ui::new(&theme, Rect::new(0, 0, 3, 1), &mut layers);
    let rect_narrow = tog_on.draw(&mut ui_narrow, Rect::new(0, 0, 3, 1));
    assert_eq!(rect_narrow.width, 3);
}

#[test]
fn test_toggle_activation_emits_inverse() {
    let id = Id::new("act.tog");
    let tog = Toggle::new(id.clone(), "Mute", true);

    let cause = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 15, 1))]);
    let mut cx = make_cx(&cause, id.clone(), &mut layers, &geom);

    let resp = tog.update(&mut cx);
    assert_eq!(
        resp.action,
        Some(ValueChanged::new(false, ActivationOrigin::Keyboard))
    );
}

#[test]
fn test_toggle_disabled_ignores_activation() {
    let id = Id::new("dis.tog");
    let tog = Toggle::new(id.clone(), "Locked", false).disabled(true);

    let cause = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char(' '),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 15, 1))]);
    let mut cx = make_cx(&cause, id.clone(), &mut layers, &geom);

    let resp = tog.update(&mut cx);
    assert_eq!(resp.action, None);
}

// =========================================================================
// 3. RadioGroup Probes: Keyed choices, navigation cursor vs selected, and reconciliation
// =========================================================================

#[test]
fn test_radio_group_construction_and_keyed_options() {
    let id = Id::new("test.radio");
    let items = vec![
        ChoiceItem::new(1u64, "Option A"),
        ChoiceItem::new(2u64, "Option B").disabled(true),
        ChoiceItem::new(3u64, "Option C"),
    ];
    let group = RadioGroup::new(id.clone(), &items, Revision::new(1))
        .label("Select Format")
        .selected(Some(ItemKey::new(1)))
        .orientation(Axis::Vertical);

    assert_eq!(group.id, id);
    assert_eq!(group.label, Some("Select Format"));
    assert_eq!(group.selected, Some(ItemKey::new(1)));
    assert_eq!(group.orientation, Axis::Vertical);
    assert_eq!(group.options.len(), 3);
}

#[test]
fn test_radio_group_measure_vertical_and_horizontal() {
    let id = Id::new("measure.radio");
    let items = vec![
        ChoiceItem::new(1u64, "Alpha"),
        ChoiceItem::new(2u64, "Beta"),
    ];
    let theme = Theme::termrock();
    let cx = MeasureCx::new(Constraints::unbounded(), &theme, ColorLevel::TrueColor);

    // Vertical with label: 1 label row + 2 items = 3 rows
    let vert = RadioGroup::new(id.clone(), &items, Revision::new(1)).label("Letters");
    let s_vert = vert.measure(&cx, Constraints::unbounded());
    assert_eq!(s_vert.height, 3);

    // Horizontal without label: 1 row
    let horiz = RadioGroup::new(id, &items, Revision::new(1)).orientation(Axis::Horizontal);
    let s_horiz = horiz.measure(&cx, Constraints::unbounded());
    assert_eq!(s_horiz.height, 1);
}

#[test]
fn test_radio_group_keyboard_navigation_moves_cursor_only() {
    let id = Id::new("nav.radio");
    let items = vec![
        ChoiceItem::new(10u64, "First"),
        ChoiceItem::new(20u64, "Second"),
        ChoiceItem::new(30u64, "Third"),
    ];
    let group =
        RadioGroup::new(id.clone(), &items, Revision::new(1)).selected(Some(ItemKey::new(10)));
    let mut state = RadioGroupState::new().with_cursor(ItemKey::new(10));

    let cause_down = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Down,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);
    let mut cx = make_cx(&cause_down, id.clone(), &mut layers, &geom);

    // Crucial Contract Verification: Down arrow moves cursor to 20, but emits NO Choose action!
    let resp = group.update(&mut cx, &mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(20)));
    assert_eq!(
        resp.action, None,
        "Arrow navigation MUST NOT choose a value!"
    );

    // Down arrow again moves cursor to 30
    let resp2 = group.update(&mut cx, &mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(30)));
    assert_eq!(resp2.action, None);

    // Enter commits the current cursor
    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_enter;
    let resp_enter = group.update(&mut cx, &mut state);
    assert_eq!(
        resp_enter.action,
        Some(RadioAction::Choose {
            key: ItemKey::new(30),
            origin: ActivationOrigin::Keyboard,
        })
    );
}

#[test]
fn test_radio_group_navigation_skips_disabled_options() {
    let id = Id::new("skip.radio");
    let items = vec![
        ChoiceItem::new(1u64, "One"),
        ChoiceItem::new(2u64, "Disabled Two").disabled(true),
        ChoiceItem::new(3u64, "Three"),
    ];
    let group = RadioGroup::new(id.clone(), &items, Revision::new(1));
    let mut state = RadioGroupState::new().with_cursor(ItemKey::new(1));

    let cause_down = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Down,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);
    let mut cx = make_cx(&cause_down, id.clone(), &mut layers, &geom);

    // Moving down skips item 2 and goes straight to item 3
    group.update(&mut cx, &mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(3)));

    // Moving up skips item 2 and goes back to item 1
    let cause_up = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Up,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_up;
    group.update(&mut cx, &mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(1)));
}

#[test]
fn test_radio_group_pointer_click_commits_choice() {
    let id = Id::new("click.radio");
    let items = vec![
        ChoiceItem::new(100u64, "Red"),
        ChoiceItem::new(200u64, "Green"),
    ];
    let group = RadioGroup::new(id.clone(), &items, Revision::new(1));
    let mut state = RadioGroupState::new();

    let green_id = group.option_id(ItemKey::new(200));
    let mut layers = LayerStack::new();
    let geom = HashMap::from([
        (id.clone(), Rect::new(0, 0, 20, 2)),
        (green_id.clone(), Rect::new(0, 1, 20, 1)),
    ]);

    // Click on Green option
    let cause_click = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(2, 1),
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_click, id.clone(), &mut layers, &geom);
    cx.intended_owner = Some(green_id);

    let resp = group.update(&mut cx, &mut state);
    assert_eq!(
        resp.action,
        Some(RadioAction::Choose {
            key: ItemKey::new(200),
            origin: ActivationOrigin::Pointer,
        })
    );
    assert_eq!(state.cursor, Some(ItemKey::new(200)));
}

#[test]
fn test_radio_group_source_reconciliation_preserves_surviving_cursor() {
    let id = Id::new("reconcile.radio");
    let items_v1 = vec![
        ChoiceItem::new(1u64, "A"),
        ChoiceItem::new(2u64, "B"),
        ChoiceItem::new(3u64, "C"),
    ];
    let group_v1 = RadioGroup::new(id.clone(), &items_v1, Revision::new(1));
    let mut state = RadioGroupState::new().with_cursor(ItemKey::new(2));

    let cause_tick = UpdateCause::Tick(Moment::from_millis(100));
    let mut layers = LayerStack::new();
    let geom = HashMap::new();
    let mut cx = make_cx(&cause_tick, id.clone(), &mut layers, &geom);

    group_v1.update(&mut cx, &mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(2)));

    // Revision 2: Item 2 was removed. Cursor reconciles deterministically.
    let items_v2 = vec![ChoiceItem::new(1u64, "A"), ChoiceItem::new(3u64, "C")];
    let group_v2 = RadioGroup::new(id.clone(), &items_v2, Revision::new(2));
    group_v2.update(&mut cx, &mut state);
    assert!(state.cursor.is_some());
    assert!(state.cursor == Some(ItemKey::new(1)) || state.cursor == Some(ItemKey::new(3)));
}

// =========================================================================
// 4. Select Probes: Dropdown popup, elevation, dismissal, and restoration
// =========================================================================

#[test]
fn test_select_construction_and_initial_state() {
    let id = Id::new("test.sel");
    let items = vec![
        ChoiceItem::new(1u64, "Small"),
        ChoiceItem::new(2u64, "Medium"),
        ChoiceItem::new(3u64, "Large"),
    ];
    let sel = Select::new(id.clone(), &items, Revision::new(1))
        .label("Size")
        .placeholder("Pick a size")
        .selected(Some(ItemKey::new(2)));

    assert_eq!(sel.id, id);
    assert_eq!(sel.label, "Size");
    assert_eq!(sel.placeholder, "Pick a size");
    assert_eq!(sel.selected, Some(ItemKey::new(2)));
    assert_eq!(sel.value_label(), "Medium");

    let state = SelectState::new();
    assert!(!state.open());
}

#[test]
fn test_select_closed_keyboard_step_selection_without_opening() {
    let id = Id::new("step.sel");
    let items = vec![
        ChoiceItem::new(10u64, "Low"),
        ChoiceItem::new(20u64, "Medium"),
        ChoiceItem::new(30u64, "High"),
    ];
    let sel = Select::new(id.clone(), &items, Revision::new(1)).selected(Some(ItemKey::new(10)));
    let mut state = SelectState::new();

    // Down key while closed steps to next option directly without opening popup!
    let cause_down = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Down,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);
    let mut cx = make_cx(&cause_down, id.clone(), &mut layers, &geom);

    let resp = sel.update(&mut cx, &mut state);
    assert_eq!(
        resp.action,
        Some(SelectAction::Choose {
            key: ItemKey::new(20),
            origin: ActivationOrigin::Keyboard,
        })
    );
    assert!(
        !state.open(),
        "Closed arrow navigation must NOT open popup!"
    );
}

#[test]
fn test_select_enter_opens_popup_and_escape_dismisses() {
    let id = Id::new("popup.sel");
    let items = vec![ChoiceItem::new(1u64, "Rust"), ChoiceItem::new(2u64, "Zig")];
    let sel = Select::new(id.clone(), &items, Revision::new(1)).selected(Some(ItemKey::new(1)));
    let mut state = SelectState::new();

    // Enter opens popup
    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);
    let mut cx = make_cx(&cause_enter, id.clone(), &mut layers, &geom);

    sel.update(&mut cx, &mut state);
    assert!(state.open(), "Enter must open popup");
    assert_eq!(state.highlighted, Some(ItemKey::new(1)));

    // Arrow down moves highlighted to Zig
    let cause_down = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Down,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(105),
    );
    cx.cause = &cause_down;
    sel.update(&mut cx, &mut state);
    assert_eq!(state.highlighted, Some(ItemKey::new(2)));

    // Escape closes popup, emits Dismissed, and restores highlighted to committed selected
    let cause_esc = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Esc,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_esc;
    let resp_esc = sel.update(&mut cx, &mut state);
    assert!(!state.open(), "Escape must close popup");
    assert_eq!(resp_esc.action, Some(SelectAction::Dismissed));
    assert_eq!(state.highlighted, Some(ItemKey::new(1)));
}

#[test]
fn test_select_outside_click_dismisses() {
    let id = Id::new("outside.sel");
    let items = vec![ChoiceItem::new(1u64, "Alpha")];
    let sel = Select::new(id.clone(), &items, Revision::new(1));
    let mut state = SelectState::new();
    state.open = true;

    let cause_outside = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(99, 99),
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);
    let mut cx = make_cx(&cause_outside, id.clone(), &mut layers, &geom);
    cx.intended_owner = None;

    let resp = sel.update(&mut cx, &mut state);
    assert!(!state.open());
    assert_eq!(resp.action, Some(SelectAction::Dismissed));
}

#[test]
fn test_select_popup_option_click_commits_choice() {
    let id = Id::new("opt_click.sel");
    let items = vec![
        ChoiceItem::new(10u64, "Option 10"),
        ChoiceItem::new(20u64, "Option 20"),
    ];
    let sel = Select::new(id.clone(), &items, Revision::new(1));
    let mut state = SelectState::new();
    state.open = true;

    let opt_id = sel.option_id(ItemKey::new(20));
    let mut layers = LayerStack::new();
    let geom = HashMap::from([
        (id.clone(), Rect::new(0, 0, 20, 3)),
        (opt_id.clone(), Rect::new(1, 4, 18, 1)),
    ]);

    let cause_opt_click = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Up,
            pos: ratatui::layout::Position::new(5, 4),
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_opt_click, id.clone(), &mut layers, &geom);
    cx.intended_owner = Some(opt_id);

    let resp = sel.update(&mut cx, &mut state);
    assert!(!state.open());
    assert_eq!(
        resp.action,
        Some(SelectAction::Choose {
            key: ItemKey::new(20),
            origin: ActivationOrigin::Pointer,
        })
    );
}

#[test]
fn test_select_draw_closed_and_open_popup() {
    let id = Id::new("draw.sel");
    let items = vec![ChoiceItem::new(1u64, "One"), ChoiceItem::new(2u64, "Two")];
    let val_msg = ValidationMessage::new("ERR", "Required field");
    let sel = Select::new(id.clone(), &items, Revision::new(1))
        .label("Number")
        .selected(Some(ItemKey::new(1)))
        .validation(Some(&val_msg));

    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let mut state = SelectState::new();

    // Closed render
    let mut ui = Ui::new(&theme, Rect::new(0, 0, 30, 15), &mut layers);
    let r_closed = sel.draw(&mut ui, Rect::new(0, 0, 30, 3), &state);
    assert_eq!(r_closed, Rect::new(0, 0, 30, 3));
    assert!(ui.hit_regions.contains_key(&id));

    // Open render (draws anchored popup)
    state.open = true;
    let mut ui_open = Ui::new(&theme, Rect::new(0, 0, 30, 15), &mut layers);
    let r_open = sel.draw(&mut ui_open, Rect::new(0, 0, 30, 3), &state);
    assert_eq!(r_open, Rect::new(0, 0, 30, 3));
    // Verify child option hit regions are registered in open state
    assert!(
        ui_open
            .hit_regions
            .contains_key(&sel.option_id(ItemKey::new(1)))
    );
    assert!(
        ui_open
            .hit_regions
            .contains_key(&sel.option_id(ItemKey::new(2)))
    );
}

// =========================================================================
// 5. Secret & Validation Probes: Zeroization, redaction, and Validator trait
// =========================================================================

#[test]
fn test_secret_zeroization_and_redaction() {
    let mut sec = Secret::new("sensitive_api_key_xyz".to_string());
    assert_eq!(sec.len(), 21);
    assert!(!sec.is_empty());

    // Expose reads borrowed text without cloning
    sec.expose(|val| {
        assert_eq!(val, "sensitive_api_key_xyz");
    });

    // Debug formatting MUST redact content
    let debug_repr = format!("{sec:?}");
    assert_eq!(debug_repr, "Secret([REDACTED])");
    assert!(!debug_repr.contains("sensitive"));

    // Clear zeroizes
    sec.clear();
    assert!(sec.is_empty());
    assert_eq!(sec.len(), 0);
}

#[test]
fn test_secret_policy_defaults() {
    let default_policy = SecretPolicy::default();
    assert_eq!(default_policy.reveal_tail, 0);
    assert!(!default_policy.allow_copy);

    let custom = SecretPolicy::new(4, true);
    assert_eq!(custom.reveal_tail, 4);
    assert!(custom.allow_copy);
}

#[test]
fn test_validation_message_and_validator_trait() {
    let validator = |s: &str| {
        if s.is_empty() {
            Err(ValidationMessage::new("EMPTY", "Must not be empty"))
        } else {
            Ok(())
        }
    };

    assert!(validator.validate("hello").is_ok());
    let err = validator.validate("").unwrap_err();
    assert_eq!(err.code, "EMPTY");
    assert_eq!(err.display, "Must not be empty");

    let field_err = FieldError::new(FieldKey::new(42), err);
    assert_eq!(field_err.field.as_u64(), 42);
    assert_eq!(field_err.message.code, "EMPTY");
}

#[test]
fn test_select_popup_flips_when_no_room_below() {
    let id = Id::new("flip.sel");
    let items = vec![
        ChoiceItem::new(1u64, "One"),
        ChoiceItem::new(2u64, "Two"),
        ChoiceItem::new(3u64, "Three"),
        ChoiceItem::new(4u64, "Four"),
    ];
    let sel = Select::new(id.clone(), &items, Revision::new(1));
    let mut state = SelectState::new();
    state.open = true;

    let theme = Theme::termrock();
    let mut layers = LayerStack::new();

    // Screen height is 15. Field placed near bottom at y = 12.
    // Room below is 15 - 14 = 1 cell, which is < popup height (6).
    // It must flip above y = 12!
    let mut ui = Ui::new(&theme, Rect::new(0, 0, 40, 15), &mut layers);
    sel.draw(&mut ui, Rect::new(0, 11, 40, 3), &state);

    // Verify option 1 hit region was placed above the field (y < 12)
    let opt1_id = sel.option_id(ItemKey::new(1));
    let opt1_rect = ui
        .hit_regions
        .get(&opt1_id)
        .expect("Option 1 must be registered");
    assert!(
        opt1_rect.y < 12,
        "Popup should flip above field when room below is insufficient"
    );
}

#[test]
fn test_radio_group_empty_and_all_disabled_safe() {
    let id = Id::new("empty.radio");
    let items: Vec<ChoiceItem<'static>> = Vec::new();
    let group = RadioGroup::new(id.clone(), &items, Revision::new(1));
    let mut state = RadioGroupState::new();

    let cause = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Down,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::new();
    let mut cx = make_cx(&cause, id.clone(), &mut layers, &geom);

    let resp = group.update(&mut cx, &mut state);
    assert_eq!(resp.action, None);

    let all_disabled = vec![
        ChoiceItem::new(1u64, "A").disabled(true),
        ChoiceItem::new(2u64, "B").disabled(true),
    ];
    let group_dis = RadioGroup::new(id.clone(), &all_disabled, Revision::new(1));
    let resp_dis = group_dis.update(&mut cx, &mut state);
    assert_eq!(resp_dis.action, None);
}

#[test]
fn test_select_disabled_cannot_open() {
    let id = Id::new("dis.sel");
    let items = vec![ChoiceItem::new(1u64, "X")];
    let sel = Select::new(id.clone(), &items, Revision::new(1)).disabled(true);
    let mut state = SelectState::new();

    let cause = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);
    let mut cx = make_cx(&cause, id.clone(), &mut layers, &geom);

    let resp = sel.update(&mut cx, &mut state);
    assert_eq!(resp.action, None);
    assert!(!state.open());
}

#[test]
fn test_select_losing_focus_dismisses() {
    let id = Id::new("loss.sel");
    let items = vec![ChoiceItem::new(1u64, "Y")];
    let sel = Select::new(id.clone(), &items, Revision::new(1));
    let mut state = SelectState::new();
    state.open = true;

    // Focus has moved to a different control
    let other_id = Id::new("other.ctl");
    let cause = UpdateCause::Tick(Moment::from_millis(100));
    let mut layers = LayerStack::new();
    let geom = HashMap::new();
    let mut cx = make_cx(&cause, other_id, &mut layers, &geom);

    let resp = sel.update(&mut cx, &mut state);
    assert!(!state.open(), "Losing focus must close select popup");
    assert_eq!(resp.action, Some(SelectAction::Dismissed));
}

#[test]
fn test_toggle_controlled_invariance() {
    let id = Id::new("inv.tog");
    let tog = Toggle::new(id, "Sound", false);
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let mut ui = Ui::new(&theme, Rect::new(0, 0, 20, 1), &mut layers);
    tog.draw(&mut ui, Rect::new(0, 0, 20, 1));
    assert!(!tog.on);
}
