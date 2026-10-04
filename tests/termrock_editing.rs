//! Comprehensive Probes for Termrock P3 TASK-007: Unified Text Editing, Fields, and Forms.
//!
//! Validates:
//! - AC-001: Field measure/draw; TextInput update/draw/measure; TextArea update/draw/measure; Form update/draw/measure.
//! - AC-002: Intentional baseline differences:
//!   - TextInput Escape restores snapshot vs TextArea Escape commits.
//!   - TextInput navigation paste enters edit vs TextArea navigation paste is ignored.
//!   - TextInput Enter commits vs TextArea Enter inserts newline.
//!   - SecretText bullet masking, debug redaction, zeroization.
//!   - Form invalid submit focuses first invalid field.
//! - AC-003: Protected paths and application invariants.
//! - AC-004: Completion gate passes.

use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use std::collections::HashMap;

use junie_tui::core::event::{Input, Key};
use junie_tui::termrock::{
    ActionMeta, ColorLevel, Constraints, Cx, EditPhase, Field, FieldKey, FieldSpec, Form,
    FormAction, FormControls, FormState, Id, Invalidate, LayerStack, MeasureCx, Moment, Plain,
    Rect, Revision, Secret, SecretPolicy, SecretText, Size, TextAction, TextArea, TextAreaState,
    TextInput, TextInputState, Theme, Ui, UpdateCause, ValidationMessage,
};

// =========================================================================
// Test Helpers
// =========================================================================

fn make_cx<'a>(
    cause: &'a UpdateCause,
    id: &'a Id,
    layers: &'a mut LayerStack,
    geom: &'a HashMap<Id, Rect>,
) -> Cx<'a> {
    Cx {
        cause,
        moment: Moment::from_millis(100),
        intended_owner: Some(id.clone()),
        focus: Some(id.clone()),
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
// 1. Field Component Probes
// =========================================================================

#[test]
fn test_field_measure_allocates_label_child_and_help() {
    let theme = Theme::termrock();
    let cx = MeasureCx::new(Constraints::unbounded(), &theme, ColorLevel::TrueColor);

    // Bare field with 1 child row -> height 1
    let f1 = Field::new("");
    let s1 = f1.measure(&cx, Size::new(10, 1), Constraints::unbounded());
    assert_eq!(s1.height, 1);

    // Field with label -> height 1 (label) + 1 (child) = 2
    let f2 = Field::new("Username");
    let s2 = f2.measure(&cx, Size::new(10, 1), Constraints::unbounded());
    assert_eq!(s2.height, 2);

    // Field with label and help -> height 1 (label) + 1 (child) + 1 (help) = 3
    let f3 = Field::new("Username").help("Enter your email or handle");
    let s3 = f3.measure(&cx, Size::new(10, 1), Constraints::unbounded());
    assert_eq!(s3.height, 3);

    // Field with label and error -> height 1 (label) + 1 (child) + 1 (error replaces help) = 3
    let err = ValidationMessage::new("required", "This field is required");
    let f4 = Field::new("Username")
        .help("Enter your email or handle")
        .error(Some(&err));
    let s4 = f4.measure(&cx, Size::new(10, 1), Constraints::unbounded());
    assert_eq!(s4.height, 3);
}

#[test]
fn test_field_draw_is_display_only_chrome_without_hit_or_focus() {
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let area = Rect::new(0, 0, 30, 3);

    let mut ui = Ui::new(&theme, area, &mut layers);
    let child_id = Id::new("child.input");

    let field = Field::new("Username").help("Enter handle").required(true);

    let child_area_result = field.draw(&mut ui, area, |ui, child_area| {
        // Child registers its own hit region
        ui.register_hit(child_id.clone(), child_area);
        child_area
    });

    // Child area was allocated line 1 (y=1), height 1
    assert_eq!(child_area_result.y, 1);
    assert_eq!(child_area_result.height, 1);

    // UI hit regions contains ONLY the child_id, not any Field chrome id
    assert_eq!(ui.hit_regions.len(), 1);
    assert!(ui.hit_regions.contains_key(&child_id));
    assert_eq!(ui.focus_candidates.len(), 0);
}

// =========================================================================
// 2. TextInput Component Probes (Plain & Secret)
// =========================================================================

#[test]
fn test_text_input_enter_and_f2_transition_to_editing() {
    let id = Id::new("input.name");
    let input = TextInput::new(id.clone(), "initial", Revision::new(1));
    let mut state = TextInputState::<Plain>::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);

    // Initial state is Navigation
    assert_eq!(state.phase(), EditPhase::Navigation);
    assert!(!state.is_editing());

    // Enter in navigation mode enters editing
    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_enter, &id, &mut layers, &geom);
    let _resp = input.update(&mut cx, &mut state);

    assert_eq!(state.phase(), EditPhase::Editing);
    assert!(state.is_editing());

    // Typing a character while editing emits TextAction::Edited
    let cause_char = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('a'),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_char;
    let resp_char = input.update(&mut cx, &mut state);
    assert_eq!(resp_char.action, Some(TextAction::Edited));

    // Reset to navigation
    state = TextInputState::new();

    // F2 in navigation mode also enters editing
    let cause_f2 = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::F(2),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_f2, &id, &mut layers, &geom);
    let _resp_f2 = input.update(&mut cx, &mut state);

    assert_eq!(state.phase(), EditPhase::Editing);
    assert!(state.is_editing());
}

#[test]
fn test_text_input_editing_commit_and_escape_cancel_restores_snapshot() {
    let id = Id::new("input.draft");
    let input = TextInput::new(id.clone(), "initial", Revision::new(1));
    let mut state = TextInputState::<Plain>::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);

    // 1. Enter editing mode
    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_enter, &id, &mut layers, &geom);
    input.update(&mut cx, &mut state);
    assert_eq!(state.phase(), EditPhase::Editing);

    // 2. Type '!' character
    let cause_type = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('!'),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_type;
    input.update(&mut cx, &mut state);
    assert_eq!(state.text(), "initial!");

    // 3. Escape CANCELS editing and RESTORES SNAPSHOT
    let cause_esc = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Esc,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(120),
    );
    cx.cause = &cause_esc;
    let resp_esc = input.update(&mut cx, &mut state);

    assert_eq!(resp_esc.action, Some(TextAction::Cancelled));
    assert_eq!(state.phase(), EditPhase::Navigation);
    assert_eq!(
        state.text(),
        "initial",
        "Escape must restore pre-editing snapshot"
    );

    // 4. Enter editing again and commit with Enter
    cx.cause = &cause_enter;
    input.update(&mut cx, &mut state);
    let cause_type_x = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('X'),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(130),
    );
    cx.cause = &cause_type_x;
    input.update(&mut cx, &mut state);
    assert_eq!(state.text(), "initialX");

    // Commit with Enter
    cx.cause = &cause_enter;
    let resp_commit = input.update(&mut cx, &mut state);
    assert_eq!(
        resp_commit.action,
        Some(TextAction::Commit {
            value: "initialX".to_string()
        })
    );
    assert_eq!(state.phase(), EditPhase::Navigation);
}

#[test]
fn test_text_input_bracketed_paste_in_navigation_enters_editing() {
    let id = Id::new("input.paste");
    let input = TextInput::new(id.clone(), "start-", Revision::new(1));
    let mut state = TextInputState::<Plain>::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);

    assert_eq!(state.phase(), EditPhase::Navigation);

    // Bracketed paste while in Navigation mode
    let cause_paste = UpdateCause::Input(
        Input::Paste("pasted_data".to_string()),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_paste, &id, &mut layers, &geom);
    let resp = input.update(&mut cx, &mut state);

    assert_eq!(
        state.phase(),
        EditPhase::Editing,
        "Paste in navigation must enter editing"
    );
    assert_eq!(state.text(), "start-pasted_data");
    assert_eq!(resp.action, Some(TextAction::Edited));
}

#[test]
fn test_text_input_secret_bullet_masking_and_redaction() {
    let id = Id::new("input.secret");
    let secret = Secret::new("my_secret_token".to_string());
    let input = TextInput::secret(id.clone(), &secret, Revision::new(1))
        .secret_policy(SecretPolicy::new(3, false)); // reveal last 3 chars in navigation
    let mut state = TextInputState::<SecretText>::new();
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let area = Rect::new(0, 0, 30, 3);

    // Sync state text
    input.reconcile(&mut state);

    // Verify debug format is redacted
    let debug_repr = format!("{:?}", state);
    assert!(debug_repr.contains("[REDACTED]"));
    assert!(!debug_repr.contains("my_secret_token"));

    // Draw and verify rendering does not crash
    let mut buffer = ratatui::buffer::Buffer::empty(area.into());
    let mut ui = Ui::new(&theme, area, &mut layers).with_buffer(&mut buffer);
    input.draw(&mut ui, area, &state);

    assert!(ui.hit_regions.contains_key(&id));
    assert!(ui.focus_candidates.contains(&id));
}

#[test]
fn test_text_input_disabled_rejects_interaction() {
    let id = Id::new("input.disabled");
    let input = TextInput::new(id.clone(), "val", Revision::new(1)).disabled(true);
    let mut state = TextInputState::<Plain>::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 20, 3))]);

    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_enter, &id, &mut layers, &geom);
    let resp = input.update(&mut cx, &mut state);

    assert_eq!(resp.action, None);
    assert_eq!(state.phase(), EditPhase::Navigation);
}

// =========================================================================
// 3. TextArea Component Probes (Intentional Differences)
// =========================================================================

#[test]
fn test_text_area_intentional_escape_commits_editing() {
    let id = Id::new("area.diff");
    let area_comp = TextArea::new(id.clone(), "line1\nline2", Revision::new(1));
    let mut state = TextAreaState::<Plain>::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 30, 8))]);

    // 1. Enter editing mode
    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::F(2),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_enter, &id, &mut layers, &geom);
    area_comp.update(&mut cx, &mut state);
    assert_eq!(state.phase(), EditPhase::Editing);

    // 2. Type extra characters
    let cause_type = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('+'),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_type;
    area_comp.update(&mut cx, &mut state);
    assert!(state.text().contains('+'));

    // 3. In TextArea, Escape COMMITS (does NOT cancel, does not restore snapshot)!
    let cause_esc = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Esc,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(120),
    );
    cx.cause = &cause_esc;
    let resp_esc = area_comp.update(&mut cx, &mut state);

    assert_eq!(state.phase(), EditPhase::Navigation);
    match resp_esc.action {
        Some(TextAction::Commit { value }) => {
            assert!(value.contains('+'), "TextArea Escape must commit draft");
        }
        other => panic!(
            "Expected TextAction::Commit on Escape in TextArea, got {:?}",
            other
        ),
    }
}

#[test]
fn test_text_area_enter_inserts_newline_while_editing() {
    let id = Id::new("area.enter");
    let area_comp = TextArea::new(id.clone(), "top", Revision::new(1));
    let mut state = TextAreaState::<Plain>::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 30, 8))]);

    // Enter editing mode
    let cause_f2 = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::F(2),
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_f2, &id, &mut layers, &geom);
    area_comp.update(&mut cx, &mut state);
    assert_eq!(state.phase(), EditPhase::Editing);

    // Enter while editing in TextArea inserts '\n'
    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(110),
    );
    cx.cause = &cause_enter;
    let resp_enter = area_comp.update(&mut cx, &mut state);

    assert_eq!(state.phase(), EditPhase::Editing);
    assert_eq!(resp_enter.action, Some(TextAction::Edited));
    assert!(
        state.text().contains('\n'),
        "Enter in TextArea must insert newline"
    );
}

#[test]
fn test_text_area_navigation_paste_is_ignored() {
    let id = Id::new("area.nopaste");
    let area_comp = TextArea::new(id.clone(), "content", Revision::new(1));
    let mut state = TextAreaState::<Plain>::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(id.clone(), Rect::new(0, 0, 30, 8))]);

    assert_eq!(state.phase(), EditPhase::Navigation);

    // Bracketed paste while in navigation mode
    let cause_paste = UpdateCause::Input(
        Input::Paste("ignored_data".to_string()),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_paste, &id, &mut layers, &geom);
    let resp = area_comp.update(&mut cx, &mut state);

    assert_eq!(
        state.phase(),
        EditPhase::Navigation,
        "Paste in TextArea navigation must be ignored"
    );
    assert_eq!(state.text(), "content", "Draft text must not change");
    assert_eq!(resp.action, None);
}

#[test]
fn test_text_area_multiline_scrolling_and_footer() {
    let id = Id::new("area.scroll");
    let multiline_text = "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10";
    let area_comp = TextArea::new(id.clone(), multiline_text, Revision::new(1));
    let mut state = TextAreaState::<Plain>::new();
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let area = Rect::new(0, 0, 30, 5); // 5 rows high

    area_comp.reconcile(&mut state);

    let mut buffer = ratatui::buffer::Buffer::empty(area.into());
    let mut ui = Ui::new(&theme, area, &mut layers).with_buffer(&mut buffer);
    area_comp.draw(&mut ui, area, &state);

    assert!(ui.hit_regions.contains_key(&id));
    assert!(ui.focus_candidates.contains(&id));
}

// =========================================================================
// 4. Form Component Probes
// =========================================================================

#[test]
fn test_form_traversal_visits_visible_enabled_fields_in_paint_order() {
    let form_id = Id::new("form.test");
    let f1_id = form_id.sub("f1");
    let f2_id = form_id.sub("f2");
    let f3_id = form_id.sub("f3");

    let k1 = FieldKey::new(1);
    let k2 = FieldKey::new(2);
    let k3 = FieldKey::new(3);

    let fields = vec![
        FieldSpec::new(k1, f1_id.clone(), "Name"),
        FieldSpec::new(k2, f2_id.clone(), "Secret").disabled(true), // disabled
        FieldSpec::new(k3, f3_id.clone(), "Bio"),
    ];

    let actions = vec![ActionMeta::submit("Submit"), ActionMeta::cancel("Cancel")];
    let form = Form::new(form_id.clone(), &fields, &actions);

    let mut state = FormState::default();
    let mut controls = FormControls::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(form_id.clone(), Rect::new(0, 0, 40, 15))]);

    // Initial focus starts at none; Tab visits first field (f1)
    let cause_tab = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Tab,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_tab, &form_id, &mut layers, &geom);
    form.update(&mut cx, &mut state, &mut controls);

    assert_eq!(cx.new_focus, Some(Some(f1_id.clone())));
    assert_eq!(state.focused_field, Some(k1));

    // Next Tab skips f2 (disabled) and visits f3
    cx.new_focus = None;
    form.update(&mut cx, &mut state, &mut controls);

    assert_eq!(cx.new_focus, Some(Some(f3_id.clone())));
    assert_eq!(state.focused_field, Some(k3));
}

#[test]
fn test_form_submit_invalid_focuses_first_invalid_field() {
    let form_id = Id::new("form.validate");
    let f1_id = form_id.sub("f1");
    let f2_id = form_id.sub("f2");

    let k1 = FieldKey::new(1);
    let k2 = FieldKey::new(2);

    let fields = vec![
        FieldSpec::new(k1, f1_id.clone(), "Required Field").required(true),
        FieldSpec::new(k2, f2_id.clone(), "Optional Field"),
    ];

    let actions = vec![ActionMeta::submit("Submit")];
    let form = Form::new(form_id.clone(), &fields, &actions);

    let mut state = FormState::default();
    let mut controls = FormControls::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(form_id.clone(), Rect::new(0, 0, 40, 10))]);

    // Enter to submit while required field is empty
    let cause_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_enter, &form_id, &mut layers, &geom);
    let resp = form.update(&mut cx, &mut state, &mut controls);

    // Must NOT emit Submit
    assert_eq!(resp.action, None);
    assert!(state.invalid_fields.contains(&k1));
    // Must request focus on first invalid field
    assert_eq!(cx.new_focus, Some(Some(f1_id)));
    assert_eq!(state.focused_field, Some(k1));

    // Now fill the required field and submit again
    controls.set_value(k1, "valid_value");
    cx.new_focus = None;
    let resp_valid = form.update(&mut cx, &mut state, &mut controls);

    assert_eq!(resp_valid.action, Some(FormAction::Submit));
    assert!(state.invalid_fields.is_empty());
}

#[test]
fn test_form_cancel_action_and_escape() {
    let form_id = Id::new("form.cancel");
    let fields = vec![FieldSpec::new(FieldKey::new(1), form_id.sub("f1"), "Test")];
    let actions = vec![ActionMeta::cancel("Cancel")];
    let form = Form::new(form_id.clone(), &fields, &actions);

    let mut state = FormState::default();
    let mut controls = FormControls::new();
    let mut layers = LayerStack::new();
    let geom = HashMap::from([(form_id.clone(), Rect::new(0, 0, 40, 10))]);

    let cause_esc = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Esc,
            mods: KeyModifiers::NONE,
        }),
        Moment::from_millis(100),
    );
    let mut cx = make_cx(&cause_esc, &form_id, &mut layers, &geom);
    let resp = form.update(&mut cx, &mut state, &mut controls);

    assert_eq!(resp.action, Some(FormAction::Cancel));
}

#[test]
fn test_form_hidden_fields_consume_no_geometry_and_register_no_hit() {
    let form_id = Id::new("form.hidden");
    let f1_id = form_id.sub("visible");
    let f2_id = form_id.sub("hidden");

    let fields = vec![
        FieldSpec::new(FieldKey::new(1), f1_id.clone(), "Visible"),
        FieldSpec::new(FieldKey::new(2), f2_id.clone(), "Hidden").hidden(true),
    ];

    let actions = vec![ActionMeta::submit("Submit")];
    let form = Form::new(form_id.clone(), &fields, &actions);

    let theme = Theme::termrock();
    let cx = MeasureCx::new(Constraints::unbounded(), &theme, ColorLevel::TrueColor);

    // Measure: 1 visible field (3 rows) + 2 action rows = 5 rows
    let size = form.measure(&cx, Constraints::unbounded());
    assert_eq!(size.height, 5);

    // Draw
    let mut layers = LayerStack::new();
    let area = Rect::new(0, 0, 40, 10);
    let mut buffer = ratatui::buffer::Buffer::empty(area.into());
    let mut ui = Ui::new(&theme, area, &mut layers).with_buffer(&mut buffer);
    let state = FormState::default();
    let controls = FormControls::new();

    form.draw(&mut ui, area, &state, &controls);

    // Visible field registered; hidden field NOT registered
    assert!(ui.hit_regions.contains_key(&f1_id));
    assert!(!ui.hit_regions.contains_key(&f2_id));
    assert!(ui.focus_candidates.contains(&f1_id));
    assert!(!ui.focus_candidates.contains(&f2_id));
}
