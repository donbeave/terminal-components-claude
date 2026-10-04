//! External Consumer Harness Test for Termrock P1 Public API.
//!
//! Proves that an external caller can implement a custom keyed component,
//! update caller-owned state, emit typed actions, measure, and draw
//! exclusively using public `junie_tui::termrock::*` exports.

#![allow(unused_imports, unused_variables, dead_code)]

use junie_tui::termrock::{
    ActionKey, ActivationOrigin, ColorLevel, Constraints, Cx, DismissReason, Flow, Id, Invalidate,
    ItemKey, Keyed, LayerKind, LayerSize, LayerSpec, LayerStack, MeasureCx, Moment, Part, Position,
    Rect, Response, Revision, Runtime, Scene, Size, Surface, TextAction, TextEditorCore, Theme, Ui,
    UpdateCause, ValueChanged, VisualState, author,
};

/// Caller-owned domain state.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DomainModel {
    counter: u32,
    selected_item: Option<ItemKey>,
    items: Vec<DomainItem>,
    revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DomainItem {
    key: ItemKey,
    label: String,
}

impl Keyed for DomainItem {
    fn key(&self) -> ItemKey {
        self.key
    }
}

/// Caller-owned durable component interaction state.
#[derive(Debug, Default)]
struct CounterControlState {
    focused: bool,
    hovered: bool,
}

/// Custom component using borrowed domain props and caller-owned state.
struct CounterControl<'a> {
    id: Id,
    label: &'a str,
    value: u32,
}

impl<'a> CounterControl<'a> {
    pub fn new(id: Id, label: &'a str, value: u32) -> Self {
        Self { id, label, value }
    }

    pub fn update(
        &self,
        cx: &mut Cx<'_>,
        state: &mut CounterControlState,
    ) -> Response<ValueChanged<u32>> {
        let is_target = cx.intended_owner() == Some(&self.id);
        state.focused = cx.has_focus(&self.id);

        if is_target {
            cx.request_invalidate(Invalidate::Paint);
            Response::action(
                self.id.clone(),
                ValueChanged {
                    value: self.value + 1,
                    origin: ActivationOrigin::Pointer,
                },
            )
            .with_flow(Flow::Consumed)
            .with_invalidate(Invalidate::Paint)
        } else {
            Response::bubble(self.id.clone())
        }
    }

    pub fn measure(&self, _cx: &MeasureCx<'_>, constraints: Constraints) -> Size {
        let needed = Size::new((self.label.len() as u16) + 6, 1);
        constraints.clamp(needed)
    }

    pub fn draw(&self, ui: &mut Ui<'_>, area: Rect, state: &CounterControlState) -> Rect {
        ui.register_focus(self.id.clone(), true);
        ui.register_hit(self.id.clone(), area);

        ui.part(self.id.clone(), Part::LABEL, area, |part_ui| {
            assert_eq!(part_ui.area, area);
        });

        area
    }
}

/// Generic consumer scene putting components together.
struct ConsumerScene {
    model: DomainModel,
    btn_state: CounterControlState,
    btn_id: Id,
    dialog_id: Id,
}

impl Scene for ConsumerScene {
    fn update(&mut self, cx: &mut Cx<'_>, _cause: UpdateCause) {
        let control = CounterControl::new(self.btn_id.clone(), "Click me", self.model.counter);
        let resp = control.update(cx, &mut self.btn_state);

        if let Some(action) = resp.action {
            self.model.counter = action.value;
            self.model.revision = self.model.revision.next().unwrap();
        }
    }

    fn draw(&self, ui: &mut Ui<'_>, area: Rect) {
        let control = CounterControl::new(self.btn_id.clone(), "Click me", self.model.counter);
        let btn_area = Rect::new(area.x + 1, area.y + 1, 20, 1);
        control.draw(ui, btn_area, &self.btn_state);
    }
}

#[test]
fn test_external_consumer_pure_public_api_usage() {
    let btn_id = Id::new("external.app").sub("button");
    let dialog_id = Id::new("external.app").sub("dialog");

    let items = vec![
        DomainItem {
            key: ItemKey::new(101),
            label: "First".into(),
        },
        DomainItem {
            key: ItemKey::new(102),
            label: "Second".into(),
        },
    ];

    let model = DomainModel {
        counter: 0,
        selected_item: Some(ItemKey::new(101)),
        items,
        revision: Revision::zero(),
    };

    let scene = ConsumerScene {
        model,
        btn_state: CounterControlState::default(),
        btn_id: btn_id.clone(),
        dialog_id,
    };

    let theme = Theme::termrock();
    let mut runtime = Runtime::new(scene, theme);
    let area = Rect::new(0, 0, 80, 24);

    // Initial draw publishes geometry
    let frame1 = runtime.draw(area).unwrap();
    assert_eq!(frame1.published_regions, 1);

    // Simulate clicking the button
    let click_cause = UpdateCause::Input(
        junie_tui::core::event::Input::Mouse(junie_tui::core::event::Mouse {
            kind: junie_tui::core::event::MouseKind::Down,
            pos: ratatui::layout::Position::new(2, 1),
        }),
        Moment::from_millis(50),
    );

    let report = runtime
        .handle(click_cause, Moment::from_millis(50))
        .unwrap();
    assert!(report.handled);
    assert_eq!(runtime.scene.model.counter, 1);
    assert_eq!(runtime.scene.model.revision, Revision::new(1));

    // Second draw with updated model
    let frame2 = runtime.draw(area).unwrap();
    assert_eq!(frame2.published_regions, 1);
}
