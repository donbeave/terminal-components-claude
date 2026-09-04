//! Shared real-App TestBackend harness for integration scenarios.

#![allow(
    dead_code,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    clippy::arithmetic_side_effects,
    clippy::doc_markdown,
    clippy::explicit_iter_loop,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::panic,
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used
)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Position;

use jackin_app::core::event::{Input, Key, Mouse, MouseKind, Outcome};
use jackin_app::{App, Motion, Scenario};

/// A deterministic terminal harness that drives the concrete Jackin shell.
pub struct H {
    pub app: App,
    pub term: Terminal<TestBackend>,
}

impl H {
    /// Construct and draw one fixture frame.
    pub fn new(scenario: Scenario, motion: Motion, frame: u64, w: u16, h: u16) -> Self {
        let app = App::for_scenario_with_theme(
            scenario,
            motion,
            frame,
            jackin_app::theme::Theme::junie(),
        );
        let term = Terminal::new(TestBackend::new(w, h)).expect("test backend");
        let mut harness = Self { app, term };
        harness.draw();
        harness
    }

    /// Render the current app state.
    pub fn draw(&mut self) {
        self.term
            .draw(|frame| self.app.render(frame))
            .expect("draw");
    }

    /// Send an ordinary key and redraw.
    pub fn key(&mut self, code: KeyCode) -> Outcome {
        let outcome = self.app.handle(Input::Key(Key {
            code,
            mods: KeyModifiers::NONE,
        }));
        self.draw();
        outcome
    }

    /// Send a control chord and redraw.
    pub fn ctrl(&mut self, c: char) -> Outcome {
        let outcome = self.app.handle(Input::Key(Key {
            code: KeyCode::Char(c),
            mods: KeyModifiers::CONTROL,
        }));
        self.draw();
        outcome
    }

    /// Type a complete string through the real key path.
    pub fn type_str(&mut self, value: &str) {
        for c in value.chars() {
            self.key(KeyCode::Char(c));
        }
    }

    /// Deliver deterministic virtual ticks.
    pub fn ticks(&mut self, count: usize) {
        for _ in 0..count {
            self.app.handle(Input::Tick);
        }
        self.draw();
    }

    /// Send one mouse event and redraw.
    pub fn mouse(&mut self, kind: MouseKind, x: u16, y: u16) -> Outcome {
        let outcome = self.app.handle(Input::Mouse(Mouse {
            kind,
            pos: Position::new(x, y),
        }));
        self.draw();
        outcome
    }

    /// Complete a mouse click.
    pub fn click(&mut self, x: u16, y: u16) {
        self.mouse(MouseKind::Down, x, y);
        self.mouse(MouseKind::Up, x, y);
    }

    /// Resize the real backend and redraw.
    pub fn resize(&mut self, w: u16, h: u16) {
        self.term.backend_mut().resize(w, h);
        self.app.handle(Input::Resize(w, h));
        self.draw();
    }

    /// Tab until a concrete widget owns focus.
    pub fn tab_to(&mut self, id: jackin_app::core::id::WidgetId) {
        for _ in 0..24 {
            if self.app.focus.current() == Some(id) {
                return;
            }
            self.key(KeyCode::Tab);
        }
        panic!(
            "focus never reached {id:?}: at {:?}",
            self.app.focus.current()
        );
    }

    /// Return the complete terminal text, including blank cells.
    pub fn text(&self) -> String {
        let buffer = self.term.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    /// Find a grapheme sequence in the current buffer.
    pub fn find(&self, needle: &str) -> Option<(u16, u16)> {
        let buffer = self.term.backend().buffer();
        let want: Vec<&str> =
            unicode_segmentation::UnicodeSegmentation::graphemes(needle, true).collect();
        for y in 0..buffer.area.height {
            let cells: Vec<&str> = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            for x in 0..cells.len().saturating_sub(want.len().saturating_sub(1)) {
                if cells[x..x + want.len()] == want[..] {
                    return Some((x as u16, y));
                }
            }
        }
        None
    }
}
