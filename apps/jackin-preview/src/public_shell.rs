//! Public `tui-next` shell adapter.
//!
//! The legacy preview remains available to its compatibility harness while
//! this adapter owns the new two-phase runtime boundary. Route state is still
//! the product app's state; only input resolution and painting cross the
//! public facade here.

use std::sync::OnceLock;

use crate::public_tui::{self, App as PublicApp};
use crate::screens::{Jx, PublicRequest, Screen};
use crate::{App, MIN_HEIGHT, MIN_WIDTH, Route};

fn public_keymap() -> &'static public_tui::KeyMap {
    static MAP: OnceLock<public_tui::KeyMap> = OnceLock::new();
    MAP.get_or_init(|| {
        public_tui::KeyMap::new()
            .bind(
                public_tui::KeyPhase::Capture,
                public_tui::Chord::key(public_tui::KeyCode::Up),
                crate::screens::PUBLIC_MANAGER_UP,
            )
            .bind(
                public_tui::KeyPhase::Capture,
                public_tui::Chord::key(public_tui::KeyCode::Char('k')),
                crate::screens::PUBLIC_MANAGER_UP,
            )
            .bind(
                public_tui::KeyPhase::Capture,
                public_tui::Chord::key(public_tui::KeyCode::Down),
                crate::screens::PUBLIC_MANAGER_DOWN,
            )
            .bind(
                public_tui::KeyPhase::Capture,
                public_tui::Chord::key(public_tui::KeyCode::Char('j')),
                crate::screens::PUBLIC_MANAGER_DOWN,
            )
            .bind(
                public_tui::KeyPhase::Capture,
                public_tui::Chord::key(public_tui::KeyCode::Enter),
                crate::screens::PUBLIC_MANAGER_ACTIVATE,
            )
            .bind(
                public_tui::KeyPhase::Capture,
                public_tui::Chord::key(public_tui::KeyCode::Char('q')),
                crate::screens::PUBLIC_QUIT,
            )
    })
}

fn apply_request(app: &mut App, request: PublicRequest) {
    match request {
        PublicRequest::Status(status) => {
            app.status = Some((status, crate::theme::Tone::Normal, app.world.now_ms()));
        }
        PublicRequest::Quit => app.quit = true,
        PublicRequest::Go(go) => match go {
            crate::screens::Go::Manager => app.route = Route::Manager,
            crate::screens::Go::Settings => app.route = Route::Settings,
            crate::screens::Go::Accounts { .. } => app.route = Route::Accounts,
            crate::screens::Go::Usage { .. } => app.route = Route::Usage,
            crate::screens::Go::Editor { .. } => app.route = Route::Editor,
            crate::screens::Go::Prelude => app.route = Route::Prelude,
            crate::screens::Go::Launch { .. } => app.route = Route::Cockpit,
            crate::screens::Go::Attach { .. } | crate::screens::Go::NewSession { .. } => {
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

fn draw_generic(ui: &mut public_tui::Ui<'_>, area: public_tui::Rect, app: &App) {
    let title = route_title(app.route);
    let panel = public_tui::Panel::new(public_tui::Id::root("jackin.public.route"))
        .title(title)
        .focused(true);
    panel.draw(ui, area, |ui, inner| {
        let lines = [
            format!("Scenario: {:?}", app.scenario),
            format!("Frame: {}", app.world.clock.now_ms),
            format!("Workspaces: {}", app.world.workspaces.len()),
            "Press Esc to return to Workspaces".to_owned(),
        ];
        for (offset, line) in lines.iter().enumerate() {
            let y = inner.y.saturating_add(offset as u16);
            if y >= inner.bottom() {
                break;
            }
            ui.paint_str(
                public_tui::Rect {
                    x: inner.x,
                    y,
                    width: inner.width,
                    height: 1,
                },
                line,
                public_tui::Style::default(),
            );
        }
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
        match self.route {
            Route::Manager => self.screens.manager.draw(ui, area, &self.world),
            _ => draw_generic(ui, area, self),
        }
    }

    fn should_quit(&self) -> bool {
        self.quit
    }

    fn keymap(&self) -> &public_tui::KeyMap {
        public_keymap()
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
            self.route = Route::Manager;
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
}
