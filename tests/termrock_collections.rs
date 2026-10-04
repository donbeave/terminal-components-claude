//! Comprehensive verification probes for Termrock P2 Keyed Collections & Reconciliation (TASK-006).
//!
//! Covers:
//! - AC-001: Stable key preservation, typed actions, shared runtime for List, FilterList,
//!   NavList, Tree, Tabs, Steps, and ScrollRegion.
//! - AC-002: State matrix: reorder, removal, deterministic fallback, duplicate labels,
//!   disabled rows, boundary navigation, and fail-closed duplicate key validation.
//! - AC-003: Bounded consumer-adoption verification across Showcase, TablePro, Jackin-Preview, and Holla.
//! - AC-004: Performance (100,000-row bounded draw), draw purity, and measurement constraints.

#![allow(unused_imports, unused_variables, dead_code, clippy::useless_vec)]

use std::collections::{BTreeSet, HashMap};
use std::time::Duration;

use junie_tui::core::event::{Input, Key, Mouse, MouseKind};
use junie_tui::termrock::{
    ActionKey, ActivationOrigin, AnimationSample, Axis, BranchActivation, CollectionError,
    Constraints, Cx, FadeDepth, FadePolicy, FilterList, FilterListAction, FilterListState,
    FlatNode, Flow, Id, Invalidate, ItemKey, Keyed, LayerStack, List, ListAction, ListState,
    MatchResult, MeasureCx, Moment, NavAction, NavItem, NavList, NavListState, NavMode, Part,
    Position, ProtectedRange, Readiness, Rect, Response, Revision, RowPainter, RowState, RowUi,
    Runtime, RuntimeError, Scene, ScrollAction, ScrollRegion, ScrollState, ScrollbarPolicy,
    SelectionMode, SelectionRequest, Size, StepItem, StepStatus, Steps, StepsAction, StepsMode,
    StepsState, StylePatch, TabItem, Tabs, TabsAction, TabsState, Theme, Tree, TreeAction,
    TreeNode, TreeSource, TreeState, Ui, UpdateCause, VisualState, child_item_key,
    reconcile_cursor, reconcile_cursor_with_fallback, reconcile_selection, validate_unique_keys,
};
use ratatui::crossterm::event::{KeyCode, KeyModifiers};

// =========================================================================
// Domain Test Fixtures
// =========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestItem {
    key: ItemKey,
    label: &'static str,
    disabled: bool,
}

impl TestItem {
    fn new(id: u64, label: &'static str) -> Self {
        Self {
            key: ItemKey::new(id),
            label,
            disabled: false,
        }
    }

    fn disabled(id: u64, label: &'static str) -> Self {
        Self {
            key: ItemKey::new(id),
            label,
            disabled: true,
        }
    }
}

impl Keyed for TestItem {
    fn key(&self) -> ItemKey {
        self.key
    }
}

// Tree test source
struct MockTreeSource {
    revision: Revision,
    roots: Vec<ItemKey>,
    nodes: HashMap<ItemKey, (TreeNode<'static>, Vec<ItemKey>)>,
}

impl MockTreeSource {
    fn new(revision: Revision) -> Self {
        Self {
            revision,
            roots: Vec::new(),
            nodes: HashMap::new(),
        }
    }

    fn add_node(
        &mut self,
        key: ItemKey,
        label: &'static str,
        children: Vec<ItemKey>,
        leaf: bool,
        is_root: bool,
    ) {
        if is_root {
            self.roots.push(key);
        }
        let tree_node = TreeNode::new(key, label, &[]).leaf(leaf);
        self.nodes.insert(key, (tree_node, children));
    }
}

impl TreeSource for MockTreeSource {
    fn revision(&self) -> Revision {
        self.revision
    }

    fn roots(&self) -> &[ItemKey] {
        &self.roots
    }

    fn node(&self, key: ItemKey) -> Option<TreeNode<'_>> {
        let (template, children) = self.nodes.get(&key)?;
        let mut n = template.clone();
        n.children = children;
        Some(n)
    }
}

// Helper to construct a standard Cx for testing
fn test_cx<'a>(
    cause: &'a UpdateCause,
    intended: Option<Id>,
    focus: Option<Id>,
    layers: &'a mut LayerStack,
) -> Cx<'a> {
    Cx {
        cause,
        moment: Moment::from_millis(100),
        intended_owner: intended,
        focus,
        pointer_capture: None,
        invalidate: Invalidate::None,
        layer_stack: layers,
        cursor_request: None,
        new_focus: None,
        new_capture: None,
        feedback_requests: Vec::new(),
        published_geometry: None,
    }
}

// =========================================================================
// AC-001: Preserves Stable Key & Emits Typed Navigation Actions
// =========================================================================

#[test]
fn test_list_single_selection_and_keyboard_activation() {
    let list_id = Id::new("test_list");
    let rows = vec![
        TestItem::new(1, "Alpha"),
        TestItem::new(2, "Beta"),
        TestItem::new(3, "Gamma"),
    ];

    let mut state = ListState::new().with_cursor(ItemKey::new(1));
    let mut layers = LayerStack::new();

    let list =
        List::new(list_id.clone(), &rows, Revision::zero()).selection_mode(SelectionMode::Single);

    // Press Down to move cursor to Beta
    let key_down = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Down,
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(10),
    );
    let mut cx = test_cx(&key_down, None, Some(list_id.clone()), &mut layers);
    let resp = list.update(&mut cx, &mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(2)));
    assert_eq!(resp.invalidate, Invalidate::Paint);

    // Press Enter to activate Beta
    let key_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(20),
    );
    let mut cx = test_cx(&key_enter, None, Some(list_id.clone()), &mut layers);
    let resp = list.update(&mut cx, &mut state);
    assert_eq!(
        resp.action,
        Some(ListAction::Activate {
            key: ItemKey::new(2),
            origin: ActivationOrigin::Keyboard,
        })
    );

    // Press Space to request selection
    let key_space = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char(' '),
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(30),
    );
    let mut cx = test_cx(&key_space, None, Some(list_id.clone()), &mut layers);
    let resp = list.update(&mut cx, &mut state);
    assert_eq!(
        resp.action,
        Some(ListAction::SelectionRequested(SelectionRequest::Select(
            ItemKey::new(2)
        )))
    );
}

#[test]
fn test_list_multi_selection_toggle_and_select_all() {
    let list_id = Id::new("multi_list");
    let rows = vec![
        TestItem::new(10, "First"),
        TestItem::new(20, "Second"),
        TestItem::new(30, "Third"),
    ];

    let mut state = ListState::new().with_cursor(ItemKey::new(20));
    let mut layers = LayerStack::new();

    let list =
        List::new(list_id.clone(), &rows, Revision::zero()).selection_mode(SelectionMode::Multiple);

    // Space on multi-mode toggles selection
    let key_space = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char(' '),
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(10),
    );
    let mut cx = test_cx(&key_space, None, Some(list_id.clone()), &mut layers);
    let resp = list.update(&mut cx, &mut state);
    assert_eq!(
        resp.action,
        Some(ListAction::SelectionRequested(SelectionRequest::Toggle(
            ItemKey::new(20)
        )))
    );

    // Ctrl+a requests select all
    let key_select_all = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('a'),
            mods: KeyModifiers::CONTROL,
        }),
        Moment::from_millis(20),
    );
    let mut cx = test_cx(&key_select_all, None, Some(list_id.clone()), &mut layers);
    let resp = list.update(&mut cx, &mut state);
    assert_eq!(
        resp.action,
        Some(ListAction::SelectionRequested(SelectionRequest::SelectAll))
    );
}

#[test]
fn test_filter_list_query_and_activation() {
    let id = Id::new("filter_view");
    let rows = vec![
        TestItem::new(1, "apple"),
        TestItem::new(2, "banana"),
        TestItem::new(3, "apricot"),
    ];

    let mut state = FilterListState::new();
    let mut layers = LayerStack::new();

    let flist = FilterList::new(id.clone(), &rows, Revision::zero()).filter(&|item, query| {
        if item.label.contains(query) {
            MatchResult {
                matched: true,
                score: 0,
                ranges: Vec::new(),
            }
        } else {
            MatchResult::none()
        }
    });

    // Type 'a' into query
    let key_a = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('a'),
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(10),
    );
    let mut cx = test_cx(&key_a, None, Some(id.clone()), &mut layers);
    let resp = flist.update(&mut cx, &mut state);
    assert_eq!(state.query, "a");
    assert_eq!(
        resp.action,
        Some(FilterListAction::FilterChanged("a".to_string()))
    );

    // Filtered rows for "a": apple, banana, apricot
    let matching = flist.filter_rows(&state.query);
    assert_eq!(matching.len(), 3);

    // Type 'p' -> "ap"
    let key_p = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('p'),
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(20),
    );
    let mut cx = test_cx(&key_p, None, Some(id.clone()), &mut layers);
    flist.update(&mut cx, &mut state);
    assert_eq!(state.query, "ap");

    // Filtered rows for "ap": apple (1), apricot (3)
    let matching_ap = flist.filter_rows(&state.query);
    assert_eq!(matching_ap.len(), 2);
    assert_eq!(matching_ap[0].key(), ItemKey::new(1));
    assert_eq!(matching_ap[1].key(), ItemKey::new(3));
}

#[test]
fn test_nav_list_skips_sections_and_disabled_entries() {
    let id = Id::new("nav");
    let items = vec![
        NavItem::section("WORKSPACE"),
        NavItem::new(ItemKey::new(1), "Projects"),
        NavItem::new(ItemKey::new(2), "Archive").disabled(true),
        NavItem::section("SETTINGS"),
        NavItem::new(ItemKey::new(3), "Preferences"),
    ];

    let mut state = NavListState::new();
    let mut layers = LayerStack::new();

    let nav = NavList::new(id.clone(), &items, Revision::zero());

    // Initial reconcile selects first selectable item: Projects (key 1)
    nav.reconcile(&mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(1)));

    // Press Down: skips Archive (disabled) and SETTINGS (section) -> lands on Preferences (key 3)!
    let key_down = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Down,
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(10),
    );
    let mut cx = test_cx(&key_down, None, Some(id.clone()), &mut layers);
    let resp = nav.update(&mut cx, &mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(3)));

    // Press Enter to navigate
    let key_enter = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Enter,
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(20),
    );
    let mut cx = test_cx(&key_enter, None, Some(id.clone()), &mut layers);
    let resp = nav.update(&mut cx, &mut state);
    assert_eq!(
        resp.action,
        Some(NavAction::Navigate {
            key: ItemKey::new(3),
            origin: ActivationOrigin::Keyboard,
        })
    );
}

#[test]
fn test_tree_expansion_and_disclosure_vs_body_activation() {
    let tree_id = Id::new("tree");
    let mut source = MockTreeSource::new(Revision::zero());

    // Root folder with two child files
    let root_k = ItemKey::new(100);
    let c1_k = ItemKey::new(101);
    let c2_k = ItemKey::new(102);

    source.add_node(root_k, "src", vec![c1_k, c2_k], false, true);
    source.add_node(c1_k, "main.rs", vec![], true, false);
    source.add_node(c2_k, "lib.rs", vec![], true, false);

    let mut state = TreeState::new();
    let mut layers = LayerStack::new();
    let tree = Tree::new(tree_id.clone(), &source);

    // Initial state: root is collapsed
    let flat_collapsed = tree.flatten(&state);
    assert_eq!(flat_collapsed.len(), 1);
    assert_eq!(flat_collapsed[0].key, root_k);

    // Expand via Right arrow
    state.cursor = Some(root_k);
    let key_right = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Right,
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(10),
    );
    let mut cx = test_cx(&key_right, None, Some(tree_id.clone()), &mut layers);
    let resp = tree.update(&mut cx, &mut state);
    assert!(state.is_expanded(root_k));
    assert_eq!(
        resp.action,
        Some(TreeAction::ExpansionChanged {
            key: root_k,
            expanded: true,
        })
    );

    // Flatten with expanded root: 3 visible nodes
    let flat_expanded = tree.flatten(&state);
    assert_eq!(flat_expanded.len(), 3);
    assert_eq!(flat_expanded[1].key, c1_k);
    assert_eq!(flat_expanded[2].key, c2_k);

    // Disclosure click toggles without activating
    let disc_id = tree_id.child(root_k).sub("disclosure");
    let click_disc = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Down,
            pos: ratatui::layout::Position::new(2, 0),
        }),
        Moment::from_millis(20),
    );
    let mut cx = test_cx(&click_disc, Some(disc_id), None, &mut layers);
    let resp = tree.update(&mut cx, &mut state);
    assert!(!state.is_expanded(root_k), "Disclosure click must collapse");
    assert_eq!(
        resp.action,
        Some(TreeAction::ExpansionChanged {
            key: root_k,
            expanded: false,
        })
    );
}

#[test]
fn test_tabs_navigation_selection_and_close() {
    let tabs_id = Id::new("editor_tabs");
    let tab_items = vec![
        TabItem::new(ItemKey::new(1), "tab1.rs").closable(true),
        TabItem::new(ItemKey::new(2), "tab2.rs").closable(false),
        TabItem::new(ItemKey::new(3), "tab3.rs").closable(true),
    ];

    let mut state = TabsState::new().with_active(ItemKey::new(1));
    let mut layers = LayerStack::new();
    let tabs = Tabs::new(tabs_id.clone(), &tab_items, Revision::zero());

    // Switch tab using digit '2'
    let key_2 = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('2'),
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(10),
    );
    let mut cx = test_cx(&key_2, None, Some(tabs_id.clone()), &mut layers);
    let resp = tabs.update(&mut cx, &mut state);
    assert_eq!(state.active, Some(ItemKey::new(2)));
    assert_eq!(
        resp.action,
        Some(TabsAction::Select {
            key: ItemKey::new(2),
            origin: ActivationOrigin::Keyboard,
        })
    );

    // Attempt close on non-closable tab2.rs ('x') -> produces no Close action!
    let key_x = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Char('x'),
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(20),
    );
    let mut cx = test_cx(&key_x, None, Some(tabs_id.clone()), &mut layers);
    let resp = tabs.update(&mut cx, &mut state);
    assert!(
        resp.action.is_none(),
        "Non-closable tab must not emit Close"
    );

    // Click close button on tab 3
    let close_btn_3 = tabs_id.child(ItemKey::new(3)).sub("close");
    let click_close = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::Down,
            pos: ratatui::layout::Position::new(25, 0),
        }),
        Moment::from_millis(30),
    );
    let mut cx = test_cx(&click_close, Some(close_btn_3), None, &mut layers);
    let resp = tabs.update(&mut cx, &mut state);
    assert_eq!(
        resp.action,
        Some(TabsAction::Close {
            key: ItemKey::new(3),
            origin: ActivationOrigin::Pointer,
        })
    );
}

#[test]
fn test_steps_display_vs_navigable_modes_and_animation() {
    let steps_id = Id::new("pipeline");
    let step_items = vec![
        StepItem::new(ItemKey::new(1), "Lint", StepStatus::Done),
        StepItem::new(ItemKey::new(2), "Build", StepStatus::Running),
        StepItem::new(ItemKey::new(3), "Test", StepStatus::Queued),
    ];

    let mut state = StepsState::new();
    let mut layers = LayerStack::new();

    // Display mode: ignores inputs, does not register focus
    let display_steps = Steps::new(steps_id.clone(), &step_items, Revision::zero())
        .mode(StepsMode::Display)
        .animation(AnimationSample::phase(3));

    let key_down = UpdateCause::Input(
        Input::Key(Key {
            code: KeyCode::Down,
            mods: KeyModifiers::empty(),
        }),
        Moment::from_millis(10),
    );
    let mut cx = test_cx(&key_down, None, Some(steps_id.clone()), &mut layers);
    let resp = display_steps.update(&mut cx, &mut state);
    assert!(resp.action.is_none());
    assert_eq!(state.cursor, None);

    // Navigable mode: allows keyboard inspection
    let nav_steps = Steps::new(steps_id.clone(), &step_items, Revision::zero())
        .mode(StepsMode::Navigable)
        .animation(AnimationSample::phase(3));

    nav_steps.reconcile(&mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(1)));

    let mut cx = test_cx(&key_down, None, Some(steps_id.clone()), &mut layers);
    let _resp = nav_steps.update(&mut cx, &mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(2)));

    // Spinner glyph reflects phase
    assert_eq!(StepStatus::Running.glyph(0), "⠋");
    assert_eq!(StepStatus::Running.glyph(3), "⠸");
    assert_eq!(StepStatus::Done.glyph(0), "✓");
    assert_eq!(StepStatus::Failed.glyph(0), "✗");
}

// =========================================================================
// AC-002: Reorder, Removal, Deterministic Fallback & State Matrix
// =========================================================================

#[test]
fn test_reconciliation_preserves_cursor_on_reorder() {
    let initial = vec![ItemKey::new(10), ItemKey::new(20), ItemKey::new(30)];
    let reordered = vec![ItemKey::new(30), ItemKey::new(10), ItemKey::new(20)];

    let cursor = Some(ItemKey::new(20));
    assert_eq!(
        reconcile_cursor(cursor, &reordered),
        Some(ItemKey::new(20)),
        "Cursor must follow stable key across source reorder"
    );
}

#[test]
fn test_reconciliation_fallback_to_successor_then_predecessor() {
    let prior = vec![
        ItemKey::new(1),
        ItemKey::new(2),
        ItemKey::new(3),
        ItemKey::new(4),
        ItemKey::new(5),
    ];

    // Remove item 3: successor 4 is present, fallback chooses 4
    let after_remove_3 = vec![
        ItemKey::new(1),
        ItemKey::new(2),
        ItemKey::new(4),
        ItemKey::new(5),
    ];
    let fallback = reconcile_cursor_with_fallback(Some(ItemKey::new(3)), &prior, &after_remove_3);
    assert_eq!(fallback, Some(ItemKey::new(4)));

    // Remove items 3, 4, 5: no successor left -> fallback chooses predecessor 2
    let after_remove_tail = vec![ItemKey::new(1), ItemKey::new(2)];
    let fallback_pred =
        reconcile_cursor_with_fallback(Some(ItemKey::new(3)), &prior, &after_remove_tail);
    assert_eq!(fallback_pred, Some(ItemKey::new(2)));

    // Empty list produces None
    assert_eq!(
        reconcile_cursor_with_fallback(Some(ItemKey::new(3)), &prior, &[]),
        None
    );
}

#[test]
fn test_reconcile_selection_drops_removed_keys_only() {
    let selected = vec![ItemKey::new(1), ItemKey::new(3), ItemKey::new(5)];
    let current_items = vec![ItemKey::new(1), ItemKey::new(2), ItemKey::new(5)];

    let reconciled = reconcile_selection(&selected, &current_items);
    assert_eq!(reconciled, vec![ItemKey::new(1), ItemKey::new(5)]);
}

#[test]
fn test_duplicate_labels_with_distinct_keys_remain_isolated() {
    let rows = vec![
        TestItem::new(100, "Duplicate Label"),
        TestItem::new(200, "Duplicate Label"),
    ];

    assert_eq!(rows[0].label, rows[1].label);
    assert_ne!(rows[0].key(), rows[1].key());

    // Unique key check passes
    assert!(validate_unique_keys(&rows).is_ok());

    let mut state = ListState::new().with_cursor(ItemKey::new(200));
    let list = List::new(Id::new("dup_test"), &rows, Revision::zero());
    list.reconcile(&mut state);

    assert_eq!(
        state.cursor,
        Some(ItemKey::new(200)),
        "Stable key distinguishes identical labels"
    );
}

#[test]
fn test_duplicate_keys_fail_closed() {
    let rows = vec![
        TestItem::new(42, "Item A"),
        TestItem::new(99, "Item B"),
        TestItem::new(42, "Item C with duplicate key 42"),
    ];

    let result = validate_unique_keys(&rows);
    assert_eq!(
        result,
        Err(CollectionError::DuplicateKey(ItemKey::new(42))),
        "Duplicate keys must fail validation closed"
    );
}

#[test]
fn test_scroll_region_bounds_and_wheel_edge_cases() {
    let mut state = ScrollState::new(0, 50, 10);
    let region = ScrollRegion::new(Id::new("scroll_edge"), Axis::Vertical, 50, 10);
    let mut layers = LayerStack::new();

    // Wheel at top boundary: offset remains 0, consumed
    let wheel_up = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::WheelUp,
            pos: ratatui::layout::Position::new(5, 5),
        }),
        Moment::from_millis(10),
    );
    let mut cx = test_cx(&wheel_up, Some(region.owner.clone()), None, &mut layers);
    let resp = region.update(&mut cx, &mut state);
    assert_eq!(state.offset, 0);

    // Scroll to bottom
    state.scroll_to(40);
    assert_eq!(state.offset, 40);

    // Wheel at bottom boundary: offset remains 40
    let wheel_down = UpdateCause::Input(
        Input::Mouse(Mouse {
            kind: MouseKind::WheelDown,
            pos: ratatui::layout::Position::new(5, 5),
        }),
        Moment::from_millis(20),
    );
    let mut cx = test_cx(&wheel_down, Some(region.owner.clone()), None, &mut layers);
    let resp = region.update(&mut cx, &mut state);
    assert_eq!(state.offset, 40);
}

// =========================================================================
// AC-003: Bounded Consumer-Adoption Scenarios
// =========================================================================

#[test]
fn test_showcase_consumer_adoption_list_and_tabs_scenario() {
    // Models Showcase Lists & Chrome settings pages
    let list_id = Id::new("showcase.lists.single");
    let langs = vec![
        TestItem::new(1, "Rust"),
        TestItem::new(2, "TypeScript"),
        TestItem::new(3, "Go"),
        TestItem::new(4, "Python"),
    ];

    let mut state = ListState::new().with_cursor(ItemKey::new(1));
    let list = List::new(list_id.clone(), &langs, Revision::zero());
    list.reconcile(&mut state);
    assert_eq!(state.cursor, Some(ItemKey::new(1)));

    // Showcase tab strip adoption
    let tab_strip_id = Id::new("showcase.chrome.tabs");
    let showcase_tabs = vec![
        TabItem::new(ItemKey::new(10), "Overview"),
        TabItem::new(ItemKey::new(20), "Metrics"),
        TabItem::new(ItemKey::new(30), "Settings"),
    ];
    let mut tab_state = TabsState::new().with_active(ItemKey::new(10));
    let tabs = Tabs::new(tab_strip_id, &showcase_tabs, Revision::zero());
    tabs.reconcile(&mut tab_state);
    assert_eq!(tab_state.active, Some(ItemKey::new(10)));
}

#[test]
fn test_tablepro_consumer_adoption_query_tabs_and_explorer_tree() {
    // Models TablePro workbench tabs & explorer hierarchy
    let qtabs_id = Id::new("tablepro.workbench.tabs");
    let query_tabs = vec![
        TabItem::new(ItemKey::new(1), "SELECT * FROM users").closable(true),
        TabItem::new(ItemKey::new(2), "Explain Plan").closable(true),
    ];
    let mut qstate = TabsState::new().with_active(ItemKey::new(1));
    let qtabs = Tabs::new(qtabs_id, &query_tabs, Revision::zero());
    qtabs.reconcile(&mut qstate);
    assert_eq!(qstate.active, Some(ItemKey::new(1)));

    // Explorer tree
    let mut tree_source = MockTreeSource::new(Revision::zero());
    let db_k = ItemKey::new(10);
    let table_k = ItemKey::new(11);
    tree_source.add_node(db_k, "production_db", vec![table_k], false, true);
    tree_source.add_node(table_k, "customers", vec![], true, false);

    let tree = Tree::new(Id::new("tablepro.explorer"), &tree_source);
    let mut tstate = TreeState::new();
    tstate.expand(db_k);
    let flat = tree.flatten(&tstate);
    assert_eq!(flat.len(), 2);
    assert_eq!(flat[1].node.label, "customers");
}

#[test]
fn test_jackin_preview_consumer_adoption_cockpit_steps_and_modals() {
    // Models Jackin Preview Cockpit steps rail & modal file picker
    let steps_id = Id::new("jackin.cockpit.steps");
    let steps = vec![
        StepItem::new(ItemKey::new(1), "Verify Workspace", StepStatus::Done),
        StepItem::new(ItemKey::new(2), "Spawn Agent", StepStatus::Running),
        StepItem::new(ItemKey::new(3), "Await Result", StepStatus::Queued),
    ];
    let rail = Steps::new(steps_id, &steps, Revision::zero()).mode(StepsMode::Display);
    let mut rstate = StepsState::new();
    rail.reconcile(&mut rstate);

    // Modal list
    let files = vec![
        TestItem::new(101, "src/"),
        TestItem::new(102, "Cargo.toml"),
        TestItem::new(103, "README.md"),
    ];
    let modal_list = List::new(Id::new("jackin.modal.list"), &files, Revision::zero());
    let mut mstate = ListState::new();
    modal_list.reconcile(&mut mstate);
    assert_eq!(mstate.cursor, Some(ItemKey::new(101)));
}

#[test]
fn test_holla_consumer_adoption_disk_tree_and_channel_tabs() {
    // Models Holla Disk screen & main tabs
    let holla_tabs_id = Id::new("holla.app.tabs");
    let holla_tabs = vec![
        TabItem::new(ItemKey::new(1), "#general"),
        TabItem::new(ItemKey::new(2), "#dev"),
        TabItem::new(ItemKey::new(3), "Direct Message"),
    ];
    let htabs = Tabs::new(holla_tabs_id, &holla_tabs, Revision::zero());
    let mut hstate = TabsState::new().with_active(ItemKey::new(1));
    htabs.reconcile(&mut hstate);
    assert_eq!(hstate.active, Some(ItemKey::new(1)));

    // Disk tree
    let mut disk_source = MockTreeSource::new(Revision::zero());
    let root_disk = ItemKey::new(50);
    let child_dir = ItemKey::new(51);
    disk_source.add_node(root_disk, "/", vec![child_dir], false, true);
    disk_source.add_node(child_dir, "home", vec![], true, false);

    let dtree = Tree::new(Id::new("holla.disk.tree"), &disk_source);
    let mut dtstate = TreeState::new();
    assert_eq!(dtree.flatten(&dtstate).len(), 1);
    dtstate.expand(root_disk);
    assert_eq!(dtree.flatten(&dtstate).len(), 2);
}

// =========================================================================
// AC-004: Performance (100,000-Row Bounded Draw), Purity & Measurements
// =========================================================================

#[test]
fn test_100_000_row_bounded_drawing() {
    let list_id = Id::new("huge_list");
    let total_rows = 100_000;
    let rows: Vec<TestItem> = (0..total_rows)
        .map(|i| TestItem::new(i as u64, "Row"))
        .collect();

    let mut state = ListState::new().with_cursor(ItemKey::new(50_000));
    state.scroll.total = total_rows;
    state.scroll.viewport = 20;
    state.scroll.scroll_to(50_000);

    let list = List::new(list_id.clone(), &rows, Revision::zero());

    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let area = Rect::new(0, 0, 80, 20);
    let mut ui = Ui::new(&theme, area, &mut layers);

    // Draw should execute in microseconds and visit strictly visible rows
    let start = std::time::Instant::now();
    list.draw(&mut ui, area, &state);
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(50),
        "100,000-row list draw took {:?}, must be strictly bounded",
        elapsed
    );

    // Hit regions should contain exactly the list container + 20 visible row items + scrollbar
    let row_hits = ui
        .hit_regions
        .keys()
        .filter(|k| k.as_str().contains("/c:"))
        .count();
    assert_eq!(
        row_hits, 20,
        "Exactly 20 visible rows must register hit regions"
    );
}

#[test]
fn test_draw_purity_does_not_mutate_state() {
    let list_id = Id::new("pure_list");
    let rows = vec![TestItem::new(1, "A"), TestItem::new(2, "B")];
    let state = ListState::new().with_cursor(ItemKey::new(1));

    let list = List::new(list_id, &rows, Revision::zero());
    let theme = Theme::termrock();
    let mut layers = LayerStack::new();
    let area = Rect::new(0, 0, 80, 20);

    let mut ui = Ui::new(&theme, area, &mut layers);
    list.draw(&mut ui, area, &state);

    assert_eq!(state.cursor, Some(ItemKey::new(1)));
    assert_eq!(state.scroll.offset, 0);
    assert_eq!(state.anchor, None);
}

#[test]
fn test_measure_pure_constraints() {
    let list_id = Id::new("measure_list");
    let rows = vec![
        TestItem::new(1, "A"),
        TestItem::new(2, "B"),
        TestItem::new(3, "C"),
    ];
    let list = List::new(list_id, &rows, Revision::zero());

    let theme = Theme::termrock();
    let cx = MeasureCx::new(
        Constraints::tight(Size::new(50, 10)),
        &theme,
        junie_tui::termrock::ColorLevel::TrueColor,
    );
    let size = list.measure(&cx, Constraints::new(Size::new(10, 1), Size::new(50, 10)));
    assert_eq!(size.height, 3);
}
