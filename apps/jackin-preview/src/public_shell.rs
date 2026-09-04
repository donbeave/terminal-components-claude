//! Public `tui-next` shell adapter.
//!
//! The legacy preview remains available to its compatibility harness while
//! this adapter owns the new two-phase runtime boundary. Route state is still
//! the product app's state; only input resolution and painting cross the
//! public facade here.

use crate::public_tui::{self, App as PublicApp};
use crate::screens::{Jx, PublicRequest, Screen};
use crate::{App, MIN_HEIGHT, MIN_WIDTH, Route};

pub(crate) fn public_keymap() -> public_tui::KeyMap {
    public_tui::KeyMap::new()
        .bind(
            public_tui::KeyPhase::Capture,
            public_tui::Chord::key(public_tui::KeyCode::Up),
            crate::screens::PUBLIC_NAV_UP,
        )
        .bind(
            public_tui::KeyPhase::Capture,
            public_tui::Chord::key(public_tui::KeyCode::Char('k')),
            crate::screens::PUBLIC_NAV_UP,
        )
        .bind(
            public_tui::KeyPhase::Capture,
            public_tui::Chord::key(public_tui::KeyCode::Down),
            crate::screens::PUBLIC_NAV_DOWN,
        )
        .bind(
            public_tui::KeyPhase::Capture,
            public_tui::Chord::key(public_tui::KeyCode::Char('j')),
            crate::screens::PUBLIC_NAV_DOWN,
        )
        .bind(
            public_tui::KeyPhase::Capture,
            public_tui::Chord::key(public_tui::KeyCode::Enter),
            crate::screens::PUBLIC_ACTIVATE,
        )
        .bind(
            public_tui::KeyPhase::Capture,
            public_tui::Chord::key(public_tui::KeyCode::Char('q')),
            crate::screens::PUBLIC_QUIT,
        )
}

fn apply_request(app: &mut App, request: PublicRequest) {
    match request {
        PublicRequest::Status(status) => {
            app.status = Some((status, crate::theme::Tone::Normal, app.world.now_ms()));
        }
        PublicRequest::Quit => app.quit = true,
        PublicRequest::Go(go) => match go {
            crate::screens::Go::Manager => app.route = Route::Manager,
            crate::screens::Go::Settings => {
                if app.screens.settings.is_none() {
                    app.screens.settings =
                        Some(crate::screens::settings::SettingsScreen::new(&app.world));
                }
                app.route = Route::Settings;
            }
            crate::screens::Go::Accounts { select } => {
                if let Some(id) = select {
                    app.screens.accounts.select(Some(id));
                }
                app.route = Route::Accounts;
            }
            crate::screens::Go::Usage { select } => {
                app.screens.usage.select(select);
                app.route = Route::Usage;
            }
            crate::screens::Go::Editor { workspace, pending } => {
                app.screens.editor = Some(crate::screens::editor::EditorScreen::new(
                    &app.world,
                    workspace,
                    pending.map(|workspace| *workspace),
                ));
                app.route = Route::Editor;
            }
            crate::screens::Go::Prelude => {
                if app.screens.prelude.is_none() {
                    app.screens.prelude =
                        Some(crate::screens::prelude::PreludeScreen::new(&app.world));
                }
                app.route = Route::Prelude;
            }
            crate::screens::Go::Launch {
                workspace,
                role,
                agent,
                account,
                plan,
            } => {
                app.screens.cockpit = Some(crate::screens::cockpit::CockpitScreen::new(
                    &app.world, workspace, role, agent, account, plan, app.motion,
                ));
                app.route = Route::Cockpit;
            }
            crate::screens::Go::Attach { instance, pane } => {
                app.screens.capsule = Some(crate::screens::capsule::CapsuleScreen::new(
                    &instance, &app.world, pane,
                ));
                app.route = Route::Capsule;
            }
            crate::screens::Go::NewSession { instance, .. } => {
                app.screens.capsule = Some(crate::screens::capsule::CapsuleScreen::new(
                    &instance, &app.world, None,
                ));
                app.route = Route::Capsule;
            }
            crate::screens::Go::Detach
            | crate::screens::Go::InstanceEnded { .. }
            | crate::screens::Go::LaunchFailedAck { .. } => app.route = Route::Manager,
            crate::screens::Go::Quit => app.quit = true,
        },
    }
}

fn route_title(route: Route) -> &'static str {
    match route {
        Route::Intro => "Prelude",
        Route::Manager => "Workspaces",
        Route::Prelude => "Prelude",
        Route::Editor => "Workspace editor",
        Route::Settings => "Settings",
        Route::Accounts => "Accounts",
        Route::Usage => "Usage",
        Route::Cockpit => "Launch cockpit",
        Route::Handoff => "Handoff",
        Route::Capsule => "Capsule",
        Route::Outro => "Outro",
    }
}

fn draw_chrome(ui: &mut public_tui::Ui<'_>, area: public_tui::Rect, app: &App) -> public_tui::Rect {
    let head = public_tui::Rect { height: 1, ..area };
    public_tui::Brand::new(
        public_tui::Id::root("jackin.public.brand"),
        crate::app::BRAND_MARK,
    )
    .compact(true)
    .tagline(route_title(app.route))
    .draw(ui, head);
    if area.height < 3 {
        return public_tui::Rect { height: 0, ..area };
    }
    let status_area = public_tui::Rect {
        x: area.x,
        y: area.bottom().saturating_sub(1),
        width: area.width,
        height: 1,
    };
    let status = app
        .status
        .as_ref()
        .map(|(message, _, _)| message.as_str())
        .unwrap_or("↑↓ navigate · Enter select · Esc back · q quit");
    ui.paint_str(status_area, status, ui.surface_style());
    public_tui::Rect {
        x: area.x,
        y: area.y.saturating_add(1),
        width: area.width,
        height: area.height.saturating_sub(2),
    }
}

fn draw_transition(ui: &mut public_tui::Ui<'_>, area: public_tui::Rect, app: &App) {
    let (title, line) = match app.route {
        Route::Intro => (
            "Prelude",
            app.intro
                .as_ref()
                .map(|intro| format!("Intro · {:?} · frame {}", intro.phase(), intro.tick))
                .unwrap_or_else(|| "Intro ready".into()),
        ),
        Route::Handoff => (
            "Handoff",
            app.handoff
                .map(|tick| {
                    format!(
                        "Handoff · {:?} · frame {tick}",
                        crate::rain::handoff_stage(tick)
                    )
                })
                .unwrap_or_else(|| "Preparing Capsule".into()),
        ),
        Route::Outro => (
            "Outro",
            app.outro
                .as_ref()
                .and_then(crate::rain::OutroState::caption)
                .unwrap_or_else(|| "Leaving the Construct".into()),
        ),
        _ => (route_title(app.route), "Transition".into()),
    };
    public_tui::Panel::new(public_tui::Id::root("jackin.public.transition"))
        .title(title)
        .focused(true)
        .draw(ui, area, |ui, inner| {
            ui.paint_str(inner, &line, ui.surface_style());
        });
}

impl PublicApp for App {
    fn update(&mut self, cx: &mut public_tui::Cx<'_>) -> public_tui::Response<()> {
        if cx.command() == Some(crate::screens::PUBLIC_QUIT) {
            self.quit = true;
            cx.quit();
            return public_tui::Response::changed();
        }

        let mut requests = Vec::new();
        let response = {
            let mut jx = Jx::new(&mut requests);
            match self.route {
                Route::Manager => self.screens.manager.update(cx, &mut jx, &mut self.world),
                Route::Accounts => self.screens.accounts.update(cx, &mut jx, &mut self.world),
                Route::Usage => self.screens.usage.update(cx, &mut jx, &mut self.world),
                Route::Settings => self
                    .screens
                    .settings
                    .as_mut()
                    .map(|screen| screen.update(cx, &mut jx, &mut self.world))
                    .unwrap_or_else(public_tui::Response::ignored),
                Route::Prelude => self
                    .screens
                    .prelude
                    .as_mut()
                    .map(|screen| screen.update(cx, &mut jx, &mut self.world))
                    .unwrap_or_else(public_tui::Response::ignored),
                Route::Editor => self
                    .screens
                    .editor
                    .as_mut()
                    .map(|screen| screen.update(cx, &mut jx, &mut self.world))
                    .unwrap_or_else(public_tui::Response::ignored),
                Route::Cockpit => self
                    .screens
                    .cockpit
                    .as_mut()
                    .map(|screen| screen.update(cx, &mut jx, &mut self.world))
                    .unwrap_or_else(public_tui::Response::ignored),
                Route::Capsule => self
                    .screens
                    .capsule
                    .as_mut()
                    .map(|screen| screen.update(cx, &mut jx, &mut self.world))
                    .unwrap_or_else(public_tui::Response::ignored),
                _ => public_tui::Response::ignored(),
            }
        };
        for request in requests {
            apply_request(self, request);
        }
        response
    }

    fn draw(&self, ui: &mut public_tui::Ui<'_>) {
        let area = ui.full();
        if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
            public_tui::TooSmall::new(
                public_tui::Id::root("jackin.public.too-small"),
                "Jackin Preview",
            )
            .minimum(MIN_WIDTH, MIN_HEIGHT)
            .draw(ui, area);
            return;
        }
        let body = draw_chrome(ui, area, self);
        match self.route {
            Route::Intro | Route::Handoff | Route::Outro => draw_transition(ui, body, self),
            Route::Manager => self.screens.manager.draw(ui, body, &self.world),
            Route::Accounts => self.screens.accounts.draw(ui, body, &self.world),
            Route::Usage => self.screens.usage.draw(ui, body, &self.world),
            Route::Settings => {
                if let Some(screen) = self.screens.settings.as_ref() {
                    screen.draw(ui, body, &self.world);
                }
            }
            Route::Prelude => {
                if let Some(screen) = self.screens.prelude.as_ref() {
                    screen.draw(ui, body, &self.world);
                }
            }
            Route::Editor => {
                if let Some(screen) = self.screens.editor.as_ref() {
                    screen.draw(ui, body, &self.world);
                }
            }
            Route::Cockpit => {
                if let Some(screen) = self.screens.cockpit.as_ref() {
                    screen.draw(ui, body, &self.world);
                }
            }
            Route::Capsule => {
                if let Some(screen) = self.screens.capsule.as_ref() {
                    screen.draw(ui, body, &self.world);
                }
            }
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn keymap(&self) -> &public_tui::KeyMap {
        &self.public_keymap
    }

    fn min_size(&self) -> public_tui::Size {
        public_tui::Size {
            min: (MIN_WIDTH, MIN_HEIGHT),
            preferred: (120, 40),
        }
    }

    fn on_esc(&mut self, cx: &mut public_tui::Cx<'_>) -> public_tui::Response<()> {
        if self.route == Route::Manager {
            self.quit = true;
            cx.quit();
        } else {
            let mut requests = Vec::new();
            let response = {
                let mut jx = Jx::new(&mut requests);
                match self.route {
                    Route::Accounts => {
                        self.screens
                            .accounts
                            .on_esc_top(cx, &mut jx, &mut self.world)
                    }
                    Route::Usage => self.screens.usage.on_esc_top(cx, &mut jx, &mut self.world),
                    Route::Settings => self
                        .screens
                        .settings
                        .as_mut()
                        .map(|screen| screen.on_esc_top(cx, &mut jx, &mut self.world))
                        .unwrap_or_else(public_tui::Response::ignored),
                    Route::Prelude => self
                        .screens
                        .prelude
                        .as_mut()
                        .map(|screen| screen.on_esc_top(cx, &mut jx, &mut self.world))
                        .unwrap_or_else(public_tui::Response::ignored),
                    Route::Editor => self
                        .screens
                        .editor
                        .as_mut()
                        .map(|screen| screen.on_esc_top(cx, &mut jx, &mut self.world))
                        .unwrap_or_else(public_tui::Response::ignored),
                    Route::Cockpit => self
                        .screens
                        .cockpit
                        .as_mut()
                        .map(|screen| screen.on_esc_top(cx, &mut jx, &mut self.world))
                        .unwrap_or_else(public_tui::Response::ignored),
                    Route::Capsule => self
                        .screens
                        .capsule
                        .as_mut()
                        .map(|screen| screen.on_esc_top(cx, &mut jx, &mut self.world))
                        .unwrap_or_else(public_tui::Response::ignored),
                    _ => {
                        self.route = Route::Manager;
                        public_tui::Response::changed()
                    }
                }
            };
            for request in requests {
                apply_request(self, request);
            }
            return response;
        }
        public_tui::Response::changed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::{Motion, Scenario};

    #[test]
    fn public_navigation_requests_update_the_product_route() {
        let mut app = App::for_scenario(Scenario::FirstUse, Motion::Reduced);
        apply_request(&mut app, PublicRequest::Go(crate::screens::Go::Settings));
        assert_eq!(app.route, Route::Settings);
        apply_request(&mut app, PublicRequest::Go(crate::screens::Go::Manager));
        assert_eq!(app.route, Route::Manager);
    }

    #[test]
    fn public_shell_minimum_size_matches_legacy_fixture_contract() {
        assert_eq!((MIN_WIDTH, MIN_HEIGHT), (72, 20));
        assert_eq!(route_title(Route::Manager), "Workspaces");
    }

    #[test]
    fn public_routes_materialize_product_screen_state() {
        let mut app = App::for_scenario(Scenario::FirstUse, Motion::Reduced);
        assert!(app.screens.settings.is_none());
        assert!(app.screens.prelude.is_none());
        apply_request(&mut app, PublicRequest::Go(crate::screens::Go::Settings));
        assert!(app.screens.settings.is_some());
        apply_request(&mut app, PublicRequest::Go(crate::screens::Go::Prelude));
        let prelude = app.screens.prelude.as_ref().expect("public prelude");
        assert_eq!(
            prelude.stepper_line(),
            "Source · Destination · Edit · Working dir · Name"
        );
    }

    #[test]
    fn public_accounts_selection_reconciles_against_fixture_rows() {
        let mut app = App::for_scenario(Scenario::FirstUse, Motion::Reduced);
        app.screens.accounts.select(Some("missing-account".into()));
        app.screens.accounts.reconcile_public(&app.world);
        assert_eq!(
            app.screens.accounts.selected,
            crate::screens::accounts::Sel::Overview
        );
    }

    #[test]
    fn public_route_requests_construct_remaining_screen_adapters() {
        let mut app = App::for_scenario(Scenario::FirstUse, Motion::Reduced);
        apply_request(
            &mut app,
            PublicRequest::Go(crate::screens::Go::Usage { select: None }),
        );
        assert_eq!(app.route, Route::Usage);
        apply_request(
            &mut app,
            PublicRequest::Go(crate::screens::Go::Editor {
                workspace: None,
                pending: None,
            }),
        );
        assert!(app.screens.editor.is_some());
        apply_request(
            &mut app,
            PublicRequest::Go(crate::screens::Go::Launch {
                workspace: None,
                role: "the-architect".into(),
                agent: crate::domain::agent::Agent::ClaudeCode,
                account: None,
                plan: crate::sim::launch::LaunchPlan::Clean,
            }),
        );
        assert!(app.screens.cockpit.is_some());
    }
}
