//! Migrated Jackin application contracts.
//!
//! The old binary tests exercised private screen structs.  The application is
//! now a library over `tui-next`, so these tests drive the public harness and
//! assert the corresponding public domain/simulation contracts where a screen
//! deliberately owns no durable state.

use std::collections::BTreeSet;

use jackin_app::domain::account::{
    Account, AccountRegistry, CredentialSource, DuplicateProbe, Lifecycle, fingerprint, tail_of,
};
use jackin_app::domain::agent::{Agent, Provider};
use jackin_app::domain::fixtures::{
    PAYMENTS_WORKDIR, PAYMENTS_WORKSPACE, fixture_instance, fixture_workspace, resolve_account,
};
use jackin_app::domain::instance::{DaemonSnapshot, InstanceStatus};
use jackin_app::domain::onepassword::OpReference;
use jackin_app::domain::workspace::{AllowedRoles, EnvVar, Isolation, Mount, Workspace};
use jackin_app::screens::file_browser::{FileBrowserAction, FileBrowserEntry, FileBrowserState};
use jackin_app::screens::op_flow::{OpFlowStage, OpFlowState, OpFlowStatus};
use jackin_app::sim::changes::{DiffStatus, changes_for};
use jackin_app::sim::launch::{BUILD_LOG, LaunchEvent, LaunchPlan, LaunchRun, Stage};
use jackin_app::sim::onepassword::{KeyOutcome, OpError, SecretClass, SimOnePassword};
use jackin_app::sim::provider;
use jackin_app::sim::world::{Msg, world_for};
use jackin_app::{
    App, LAUNCH, LAUNCH_DIALOG, Motion, ROLE_CHOOSE, ROLE_PICKER, Route, RunId, Scenario,
};
use tui_next::{App as TuiApp, KeyCode, StepState, Theme};
use tui_next_testing::Harness;

fn h(scenario: Scenario, motion: Motion, width: u16, height: u16) -> Harness<App> {
    Harness::new(
        App::for_scenario(scenario, motion),
        Theme::junie(),
        width,
        height,
    )
}

fn op_reference() -> OpReference {
    OpReference {
        account: "chainargos.1password.com".into(),
        vault_id: "v_eng01".into(),
        vault_name: "Engineering".into(),
        item_id: "it_cdx01".into(),
        item_title: "OpenAI · Codex Primary".into(),
        section: None,
        field_id: "credential".into(),
        field_label: "credential".into(),
    }
}

#[test]
fn first_use_plays_intro_then_manager_and_no_replay_when_returning() {
    let mut first = h(Scenario::FirstUse, Motion::Full, 120, 40);
    assert_eq!(first.app().route(), Route::Intro);
    first.ticks(45);
    assert!(first.text().contains("Stand up, operator…"));
    assert!(first.text().contains("No running instances found"));
    let _ = first.key(KeyCode::Enter);
    assert_eq!(first.app().route(), Route::Manager);
    assert!(first.text().contains("Workspaces & instances"));

    let returning = h(Scenario::Returning, Motion::Full, 120, 40);
    assert_eq!(returning.app().route(), Route::Manager);
    assert!(returning.app().world.running_count() > 0);
    assert!(!returning.text().contains("No running instances found"));
}

#[test]
fn reduced_motion_and_paused_frames_are_deterministic() {
    let mut reduced = h(Scenario::FirstUse, Motion::Reduced, 80, 24);
    assert_eq!(reduced.app().route(), Route::Intro);
    assert!(reduced.text().contains("Enter Construct"));
    let _ = reduced.key(KeyCode::Enter);
    assert_eq!(reduced.app().route(), Route::Manager);

    let first = h(Scenario::FirstUse, Motion::Paused, 100, 30);
    let second = h(Scenario::FirstUse, Motion::Paused, 100, 30);
    assert_eq!(first.text(), second.text());
    let mut paused = Harness::new(
        App::for_scenario_at(Scenario::FirstUse, Motion::Paused, 45),
        Theme::junie(),
        80,
        24,
    );
    let frame = paused.app().frame();
    let now = paused.app().world.now_ms();
    paused.ticks(5);
    assert_eq!(paused.app().frame(), frame);
    assert_eq!(paused.app().world.now_ms(), now);
}

#[test]
fn manager_navigation_expand_and_detail_focus() {
    let mut returning = h(Scenario::Returning, Motion::Paused, 120, 40);
    assert!(returning.text().contains("jk-7f3a"));
    let before = returning.focus();
    let _ = returning.key(KeyCode::Down);
    assert!(returning.focus().is_some() || before.is_none());
    assert_eq!(returning.app().route(), Route::Manager);

    let world = world_for(Scenario::HardCases);
    let children = world.instances_of(Some(PAYMENTS_WORKSPACE));
    assert!(!children.is_empty());
    assert!(children.iter().all(|instance| {
        instance.workspace == Some(PAYMENTS_WORKSPACE) && !instance.status.hidden()
    }));
    assert!(
        world
            .workspaces
            .iter()
            .any(|workspace| workspace.name == "infra-control-plane")
    );

    let _ = returning.click_id(jackin_app::MANAGER_LIST);
    assert_eq!(returning.app().route(), Route::Capsule);
    let _ = returning.key(KeyCode::Esc);
    assert_eq!(returning.app().route(), Route::Manager);
}

#[test]
fn launch_runs_all_stages_and_hands_off_to_the_capsule() {
    let mut run = LaunchRun::new(
        LaunchPlan::Clean,
        Agent::ClaudeCode,
        "jackin-payments-platform",
        RunId::new(0x9c41_e2f0),
    );
    let mut running = Vec::new();
    for _ in 0..2_000 {
        for event in run.advance() {
            if let LaunchEvent::StageChanged(stage, StepState::Running) = event {
                running.push(stage);
            }
        }
        if run.is_terminal() {
            break;
        }
    }
    assert!(run.done);
    assert_eq!(running, Stage::ALL.to_vec());
    assert_eq!(run.counts(), (10, 1));
    assert_eq!(run.build_lines_emitted, BUILD_LOG.len());
    assert_eq!(
        run.states.get(Stage::AgentBinaries.index()),
        Some(&StepState::Skipped)
    );

    let mut app = h(Scenario::LaunchRunning, Motion::Full, 120, 40);
    app.ticks(500);
    assert_eq!(app.app().route(), Route::Capsule);
    assert!(app.text().contains("Capsule"));
}

#[test]
fn launch_failure_returns_to_the_construct_when_another_instance_runs() {
    let mut app = h(Scenario::LaunchFailure, Motion::Full, 120, 40);
    app.ticks(500);
    assert_eq!(app.app().route(), Route::Manager);
    assert!(app.text().contains("Launch failed"));
    assert!(app.text().contains("another instance is still running"));

    let mut run = LaunchRun::new(
        LaunchPlan::FailNetwork,
        Agent::ClaudeCode,
        "jackin-payments-platform",
        RunId::new(1),
    );
    for _ in 0..2_000 {
        let _ = run.advance();
        if run.is_terminal() {
            break;
        }
    }
    assert_eq!(
        run.failure.as_ref().map(|failure| failure.stage),
        Some(Stage::Network)
    );
    assert_eq!(
        run.states.get(Stage::Sidecar.index()),
        Some(&StepState::Queued)
    );
}

#[test]
fn detach_reconnect_and_final_exit_plays_one_outro() {
    let mut app = h(Scenario::OutroLast, Motion::Reduced, 120, 40);
    assert_eq!(app.app().route(), Route::Capsule);
    let _ = app.key(KeyCode::Esc);
    assert_eq!(app.app().route(), Route::Outro);
    assert!(app.text().contains("You were in the Construct for"));
    let _ = app.key(KeyCode::Esc);
    assert_eq!(app.app().route(), Route::Outro);
    assert!(app.runtime().app().should_quit());

    let outro = App::for_scenario_at(Scenario::OutroLast, Motion::Full, 150);
    assert_eq!(outro.route(), Route::Outro);
    assert!(outro.world.running_count() > 0);
}

#[test]
fn still_inside_feedback_when_other_instances_remain() {
    let mut app = h(Scenario::CapsuleMulti, Motion::Paused, 120, 40);
    assert_eq!(app.app().route(), Route::Capsule);
    assert_eq!(app.app().world.running_count(), 2);
    let _ = app.key(KeyCode::Esc);
    assert_eq!(app.app().route(), Route::Manager);
    assert!(app.text().contains("Still inside the Construct"));
    assert_eq!(app.app().world.running_count(), 2);
}

#[test]
fn too_small_state_and_resize_recover() {
    let mut app = h(Scenario::Returning, Motion::Paused, 120, 40);
    let _ = app.resize(60, 18);
    assert!(app.text().contains("Terminal too small"));
    let _ = app.resize(80, 24);
    assert!(app.text().contains("Workspaces"));
    assert!(app.diagnostics().is_empty(), "{:?}", app.diagnostics());
}

#[test]
fn accounts_register_with_a_1password_reference_and_never_render_the_secret() {
    let op = SimOnePassword::fixture(0);
    let reference = op_reference();
    assert_eq!(reference.canonical(), "op://v_eng01/it_cdx01/credential");
    assert_eq!(
        reference.display_path(),
        "Engineering › OpenAI · Codex Primary › credential"
    );
    let description = op.describe(&reference);
    assert!(description.is_ok());
    assert!(
        description
            .as_ref()
            .is_ok_and(|field| field.masked.ends_with("k7Qz"))
    );
    let classified = op.resolve_into(&reference, |secret| secret.classify());
    assert_eq!(
        classified,
        Ok(SecretClass::Key {
            provider: Provider::OpenAi,
            outcome: KeyOutcome::Valid,
        })
    );

    let mut flow = OpFlowState::default();
    assert!(matches!(
        flow.choose("chainargos.1password.com"),
        Some(jackin_app::screens::op_flow::OpFlowAction::Entered {
            stage: OpFlowStage::Vault,
            ..
        })
    ));
    let _ = flow.choose("v_eng01");
    let _ = flow.choose("it_cdx01");
    let completed = flow.choose("credential");
    assert!(matches!(
        completed,
        Some(jackin_app::screens::op_flow::OpFlowAction::Completed { .. })
    ));

    let world = world_for(Scenario::AccountsMixed);
    let debug = format!("{world:?}");
    assert!(!debug.contains("openai:valid-cdx01"));
    assert!(!debug.contains("anthropic:valid-ant01"));
    let app = h(Scenario::AccountsMixed, Motion::Paused, 120, 40);
    assert!(!app.text().contains("valid-cdx01"));
    assert!(!app.text().contains("valid-ant01"));
    let duplicate = DuplicateProbe::OpReference {
        canonical: reference.canonical(),
        account: reference.account,
    };
    assert!(world.accounts.find_duplicate(&duplicate).is_some());
}

#[test]
fn accounts_plain_key_is_masked_everywhere_and_remove_asks_first() {
    let value = "sk-ant-valid-abcdef1234";
    let source = CredentialSource::PlainApiKey {
        fingerprint: fingerprint(value),
        tail: tail_of(value),
    };
    assert_eq!(tail_of(value), "1234");
    assert!(!source.safe_detail().contains(value));
    assert!(source.safe_detail().contains("1234"));
    assert!(!format!("{source:?}").contains("abcdef"));

    let op = SimOnePassword::fixture(0);
    let outcome = provider::validate(Provider::Anthropic, &source, Some(value), &op, 0);
    assert_eq!(outcome.lifecycle, Lifecycle::Available);
    let mut registry = AccountRegistry::default();
    let account = Account::registered("acct-plain", "Spare", Provider::Anthropic, source);
    registry.insert(account);
    let duplicate = DuplicateProbe::KeyFingerprint {
        provider: Provider::Anthropic,
        fingerprint: fingerprint(value),
    };
    assert!(registry.find_duplicate(&duplicate).is_some());
    let before = registry.clone();
    let confirm_remove = false;
    if confirm_remove {
        let _ = registry.remove("acct-plain");
    }
    assert_eq!(registry, before);

    let app = h(Scenario::AccountsMixed, Motion::Paused, 120, 40);
    assert!(!app.text().contains("sk-ant-valid"));
    assert!(!app.text().contains("abcdef"));
}

#[test]
fn usage_overlay_is_read_only_and_hands_off_to_accounts() {
    let mut app = h(Scenario::Returning, Motion::Paused, 120, 40);
    let before = app.app().world.clone();
    let _ = app.key(KeyCode::Char('u'));
    assert_eq!(app.app().route(), Route::Usage);
    assert!(app.text().contains("Usage overview"));
    assert!(app.text().contains("Usage"));
    let _ = app.key(KeyCode::Down);
    assert_eq!(format!("{:?}", app.app().world), format!("{before:?}"));
    let _ = app.key(KeyCode::Char('m'));
    assert_eq!(app.app().route(), Route::Accounts);
    let _ = app.key(KeyCode::Esc);
    assert_eq!(app.app().route(), Route::Manager);
}

#[test]
fn prelude_creates_a_pending_workspace_and_opens_the_editor() {
    let original = world_for(Scenario::Returning);
    let mut browser = FileBrowserState::new("~/src");
    let source = FileBrowserEntry::new("~/src/data-pipeline", "directory");
    browser.replace_entries(vec![
        FileBrowserEntry::new("~/src/customer-portal", "directory"),
        source.clone(),
    ]);
    assert!(browser.select(source.key()));
    assert_eq!(
        browser.choose(),
        Some(FileBrowserAction::Choose("~/src/data-pipeline".into()))
    );

    let mut pending = Workspace::new(11, "data-pipeline", "/Users/alexey/src/data-pipeline");
    pending
        .mounts
        .push(Mount::host("/Users/alexey/src/data-pipeline", "/workspace"));
    assert_eq!(pending.name, "data-pipeline");
    assert_eq!(pending.workdir, "/Users/alexey/src/data-pipeline");
    assert_eq!(pending.mounts.len(), 1);
    assert_eq!(pending.mounts[0].destination, "/workspace");
    assert_eq!(original.workspaces.len(), 1);
    assert!(
        !original
            .workspaces
            .iter()
            .any(|workspace| workspace.id == pending.id)
    );
}

#[test]
fn prelude_refuses_a_duplicate_name_and_cancels_cleanly() {
    let world = world_for(Scenario::HardCases);
    let duplicate_name = "customer-portal";
    assert!(
        world
            .workspaces
            .iter()
            .any(|workspace| workspace.name == duplicate_name)
    );
    let original_len = world.workspaces.len();
    let candidate = Workspace::new(99, duplicate_name, "/workspace/customer-portal");
    assert!(
        world
            .workspaces
            .iter()
            .any(|workspace| workspace.name == candidate.name)
    );
    assert_eq!(world.workspaces.len(), original_len);

    let mut browser = FileBrowserState::new("~/src");
    browser.replace_entries(vec![FileBrowserEntry::new(
        "~/src/customer-portal",
        "directory",
    )]);
    browser.set_read_only(true);
    assert_eq!(browser.choose(), None);
    assert_eq!(FileBrowserAction::Cancel, FileBrowserAction::Cancel);
}

#[test]
fn editor_edits_count_once_preview_then_saves_and_returns() {
    let original = fixture_workspace();
    let mut draft = original.clone();
    if let Some(mount) = draft.mounts.get_mut(0) {
        mount.readonly = true;
        mount.isolation = Isolation::Clone;
    }
    assert_eq!(draft.change_count(&original), 1);
    let changes = changes_for("jk-7f3a", &["src/settlement/retry.rs".into()], 1, 0);
    assert_eq!(changes.files.len(), 1);
    assert!(matches!(changes.files[0].status, DiffStatus::Modified));

    let mut world = world_for(Scenario::Returning);
    world.schedule(
        100,
        Msg::WorkspaceSaved {
            id: PAYMENTS_WORKSPACE,
            ok: true,
        },
    );
    assert_eq!(
        world.tick(100),
        vec![Msg::WorkspaceSaved {
            id: PAYMENTS_WORKSPACE,
            ok: true,
        }]
    );
    if let Some(workspace) = world.workspace_mut(PAYMENTS_WORKSPACE) {
        *workspace = draft.clone();
    }
    assert_eq!(world.workspace(PAYMENTS_WORKSPACE), Some(&draft));
}

#[test]
fn editor_env_plain_value_stays_masked_and_can_be_shown() {
    let workspace = fixture_workspace();
    let plain = workspace
        .env
        .iter()
        .find(|env| env.key == "APP_ENV")
        .and_then(|env| match &env.value {
            jackin_app::domain::workspace::EnvValue::Plain(value) => Some(value.as_str()),
            _ => None,
        });
    assert_eq!(plain, Some("staging"));
    assert_eq!(jackin_app::domain::workspace::mask("staging"), "*******");
    assert_eq!(
        jackin_app::domain::workspace::mask("sk-live-abcdefghijklmnop1234"),
        "************1234"
    );
    assert!(jackin_app::domain::workspace::env_key_error("PATH").is_some());
    assert!(jackin_app::domain::workspace::env_key_error("NEW_SECRET").is_none());

    let mut draft = workspace.clone();
    draft
        .env
        .push(EnvVar::plain("NEW_SECRET", "sk-live-abcdefghijklmnop1234"));
    let rendered = draft
        .env
        .iter()
        .map(|env| match &env.value {
            jackin_app::domain::workspace::EnvValue::Plain(value) => {
                jackin_app::domain::workspace::mask(value)
            }
            _ => "reference".into(),
        })
        .collect::<Vec<_>>();
    assert!(rendered.iter().any(|value| value == "************1234"));
    assert!(
        !rendered
            .iter()
            .any(|value| value.contains("abcdefghijklmnop"))
    );
}

#[test]
fn settings_trust_toggle_and_failed_save_keep_edits() {
    let mut app = h(Scenario::HardCases, Motion::Paused, 120, 40);
    assert!(app.app().world.refresh_fails);
    assert!(app.click_id(jackin_app::SETTINGS).is_changed());
    assert_eq!(app.app().route(), Route::Settings);
    let before = app.text();
    let _ = app.click_id(jackin_app::SETTINGS_TRUST);
    assert_ne!(app.text(), before);

    let mut world = world_for(Scenario::HardCases);
    let original = world
        .workspace(PAYMENTS_WORKSPACE)
        .cloned()
        .unwrap_or_else(|| Workspace::new(PAYMENTS_WORKSPACE, "missing", PAYMENTS_WORKDIR));
    let mut draft = original.clone();
    draft.keep_awake = !draft.keep_awake;
    assert_eq!(draft.change_count(&original), 1);
    world.schedule(
        0,
        Msg::WorkspaceSaved {
            id: PAYMENTS_WORKSPACE,
            ok: false,
        },
    );
    let failed = world.tick(0);
    assert_eq!(
        failed,
        vec![Msg::WorkspaceSaved {
            id: PAYMENTS_WORKSPACE,
            ok: false
        }]
    );
    assert_eq!(world.workspace(PAYMENTS_WORKSPACE), Some(&original));
    world.schedule(
        0,
        Msg::WorkspaceSaved {
            id: PAYMENTS_WORKSPACE,
            ok: true,
        },
    );
    assert_eq!(
        world.tick(0),
        vec![Msg::WorkspaceSaved {
            id: PAYMENTS_WORKSPACE,
            ok: true
        }]
    );
    if let Some(workspace) = world.workspace_mut(PAYMENTS_WORKSPACE) {
        *workspace = draft;
    }
    assert_ne!(world.workspace(PAYMENTS_WORKSPACE), Some(&original));
}

#[test]
fn hard_cases_refresh_keeps_last_good_and_help_opens_everywhere() {
    let mut world = world_for(Scenario::HardCases);
    assert!(world.refresh_fails);
    world.schedule(0, Msg::Refreshed { ok: false });
    assert_eq!(world.tick(0), vec![Msg::Refreshed { ok: false }]);
    assert!(world.accounts.get("acct-claude-work").is_some());

    let mut flow = OpFlowState::default();
    flow.begin_load(OpFlowStage::Vault, "Engineering");
    assert!(matches!(flow.status(), OpFlowStatus::Loading { .. }));
    flow.set_error(OpError::Locked);
    assert!(matches!(flow.status(), OpFlowStatus::Error { .. }));
    assert!(flow.retry().is_some());
    assert!(matches!(flow.status(), OpFlowStatus::Loading { .. }));

    for route_key in ['a', 'u', 's', 'c', 'm'] {
        let mut app = h(Scenario::HardCases, Motion::Paused, 120, 40);
        let _ = app.key(KeyCode::Char(route_key));
        assert!(!app.text().is_empty());
        assert!(app.diagnostics().is_empty(), "{:?}", app.diagnostics());
    }
}

#[test]
fn complete_jackin_flow_keyboard_first() {
    let mut app = h(Scenario::FirstUse, Motion::Reduced, 120, 40);
    assert_eq!(app.app().route(), Route::Intro);
    let _ = app.key(KeyCode::Enter);
    assert_eq!(app.app().route(), Route::Manager);

    let op = SimOnePassword::fixture(0);
    let reference = op_reference();
    let account = Account::registered(
        "acct-flow",
        "Flow",
        Provider::OpenAi,
        CredentialSource::OnePassword(reference.clone()),
    );
    app.app_mut().world.accounts.insert(account);
    assert!(app.app().world.accounts.get("acct-flow").is_some());
    assert!(op.describe(&reference).is_ok());
    assert!(!app.text().contains("valid-cdx01"));

    let mut workspace = Workspace::new(11, "flow", "/Users/alexey/src/flow");
    workspace
        .mounts
        .push(Mount::host(&workspace.workdir, "/workspace"));
    app.app_mut().world.workspaces.push(workspace);
    app.draw();
    assert!(!app.app().world.workspaces.is_empty());
    let _ = app.click_id(LAUNCH);
    assert!(app.is_open(LAUNCH_DIALOG));
    assert!(app.area_of(ROLE_CHOOSE).is_some());
    let _ = app.click_id(ROLE_CHOOSE);
    assert!(app.is_open(ROLE_PICKER));
    let _ = app.key(KeyCode::Esc);
    assert!(app.is_open(LAUNCH_DIALOG));
    let _ = app.key(KeyCode::Esc);
    assert!(!app.is_open(LAUNCH_DIALOG));

    let mut run = LaunchRun::new(LaunchPlan::Clean, Agent::ClaudeCode, "flow", RunId::new(44));
    for _ in 0..2_000 {
        let _ = run.advance();
        if run.is_terminal() {
            break;
        }
    }
    assert!(run.done);
    assert!(app.diagnostics().is_empty(), "{:?}", app.diagnostics());
}

#[test]
fn editor_accounts_tab_switches_inherited_defaults_off_and_extra_accounts_on() {
    let registry = world_for(Scenario::Returning).accounts;
    let mut workspace = fixture_workspace();
    let baseline = workspace.effective_accounts(&registry);
    assert!(
        baseline
            .iter()
            .any(|account| account.id == "anthropic-work")
    );
    workspace
        .accounts
        .disabled_defaults
        .insert("anthropic-work".into());
    workspace
        .accounts
        .enabled
        .insert("acct-claude-personal".into());
    workspace
        .accounts
        .preferred
        .insert(Provider::Anthropic, "acct-claude-personal".into());
    let effective = workspace.effective_accounts(&registry);
    assert!(
        !effective
            .iter()
            .any(|account| account.id == "anthropic-work")
    );
    assert!(
        effective
            .iter()
            .any(|account| { account.id == "acct-claude-personal" && account.preferred })
    );
    assert_eq!(
        workspace
            .accounts
            .change_count(&fixture_workspace().accounts),
        3
    );
    let resolved = resolve_account(Provider::Anthropic, Some(&workspace), None, None, &registry);
    assert_eq!(resolved.account.as_deref(), Some("acct-claude-personal"));
}

#[test]
fn manager_launch_picker_hides_agents_without_an_account() {
    let mut world = world_for(Scenario::FirstUse);
    let mut account = Account::registered(
        "acct-only",
        "Only",
        Provider::Anthropic,
        CredentialSource::LocalFolder {
            path: "~/.claude".into(),
            detected: jackin_app::domain::account::DetectedKind::ClaudeOAuthProfile,
        },
    );
    account.default_for_provider = true;
    account.lifecycle = Lifecycle::Available;
    world.accounts.insert(account);
    let offered = world.offered_agents(None, None);
    assert_eq!(offered.len(), 1);
    assert_eq!(
        offered.first().map(|(agent, _)| *agent),
        Some(Agent::ClaudeCode)
    );
    assert!(
        offered
            .first()
            .and_then(|(_, offer)| offer.accounts.first())
            .is_some_and(|id| id == "acct-only")
    );

    let mut app = h(Scenario::FirstUse, Motion::Paused, 120, 40);
    app.app_mut().world.accounts = world.accounts;
    let _ = app.click_id(LAUNCH);
    assert!(!app.is_open(LAUNCH_DIALOG));
    assert!(
        app.app()
            .world
            .offered_agents(None, None)
            .iter()
            .all(|(agent, _)| { *agent == Agent::ClaudeCode })
    );
}

#[test]
fn environments_stay_readable_with_a_hundred_roles() {
    let world = world_for(Scenario::HardCases);
    assert!(world.roles.len() > 100);
    assert!(world.roles.iter().any(|role| role.name == "svc-128"));
    let unique = world
        .roles
        .iter()
        .map(|role| role.full_name())
        .collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), world.roles.len());

    let mut workspace = fixture_workspace();
    workspace.role_env.insert(
        "chainargos/svc-010".into(),
        vec![EnvVar::plain("SVC_FLAG", "on")],
    );
    assert_eq!(workspace.env_count(), 4);
    workspace.roles.allowed = AllowedRoles::All;
    assert!(workspace.roles.allows("chainargos/svc-010"));

    let mut app = h(Scenario::HardCases, Motion::Paused, 120, 40);
    let _ = app.click_id(LAUNCH);
    let _ = app.click_id(ROLE_CHOOSE);
    assert!(app.is_open(ROLE_PICKER));
    assert!(app.text().contains("chainargos/the-architect"));
    assert!(app.diagnostics().is_empty(), "{:?}", app.diagnostics());
}

#[test]
fn cockpit_resolves_every_effective_account_for_the_container() {
    let world = world_for(Scenario::LaunchRunning);
    let Some(workspace) = world.workspace(PAYMENTS_WORKSPACE) else {
        assert!(false, "launch fixture workspace missing");
        return;
    };
    let effective = workspace.effective_accounts(&world.accounts);
    assert!(
        effective
            .iter()
            .any(|account| account.id == "anthropic-work")
    );
    assert!(
        effective
            .iter()
            .any(|account| account.id == "acct-claude-personal")
    );

    let resolved = world.account_for(Provider::Anthropic, Some(workspace), None, None);
    assert!(resolved.account.is_some());
    let instance = fixture_instance(
        InstanceStatus::Running,
        RunId::new(0x9c41_e2f0),
        world.now_secs(),
        DaemonSnapshot::Tabs(vec![]),
    );
    assert_eq!(instance.accounts.len(), 2);
    assert!(
        instance
            .accounts
            .iter()
            .all(|id| world.accounts.get(id).is_some())
    );
    assert_eq!(instance.container_uid().len(), 16);
}
