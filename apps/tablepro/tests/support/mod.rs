//! Shared legacy TestBackend harness for TablePro integration tests.

#![allow(
    dead_code,
    unreachable_pub,
    clippy::all,
    reason = "the visual helper mirrors the unchanged legacy test harness"
)]

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Position;

use tablepro_app::legacy_facade::core::event::{Input, Key, Mouse, MouseKind, Outcome};
use tablepro_app::legacy_facade::core::id::WidgetId;
use tablepro_app::legacy_facade::theme::Theme;

use tablepro_app::app::App;
use tablepro_app::workbench::WorkTab;

pub struct H {
    pub app: App,
    pub term: Terminal<TestBackend>,
}

impl H {
    pub fn new(w: u16, h: u16) -> Self {
        let app = App::new(Theme::junie());
        let term = Terminal::new(TestBackend::new(w, h)).unwrap();
        let mut hh = Self { app, term };
        hh.draw();
        hh
    }

    pub fn connected(w: u16, h: u16) -> Self {
        let mut hh = Self::new(w, h);
        let i = hh
            .app
            .connections
            .connections
            .iter()
            .position(|c| c.name == "Production")
            .unwrap();
        hh.app.connect(i);
        hh.draw();
        hh
    }

    pub fn draw(&mut self) {
        self.term.draw(|f| self.app.render(f)).unwrap();
    }

    pub fn key(&mut self, code: KeyCode) -> Outcome {
        let o = self.app.handle(Input::Key(Key {
            code,
            mods: KeyModifiers::NONE,
        }));
        self.draw();
        o
    }

    pub fn ctrl(&mut self, c: char) -> Outcome {
        let o = self.app.handle(Input::Key(Key {
            code: KeyCode::Char(c),
            mods: KeyModifiers::CONTROL,
        }));
        self.draw();
        o
    }

    pub fn alt(&mut self, c: char) -> Outcome {
        let o = self.app.handle(Input::Key(Key {
            code: KeyCode::Char(c),
            mods: KeyModifiers::ALT,
        }));
        self.draw();
        o
    }

    pub fn type_str(&mut self, s: &str) {
        for c in s.chars() {
            self.key(KeyCode::Char(c));
        }
    }

    pub fn ticks(&mut self, n: usize) {
        for _ in 0..n {
            self.app.handle(Input::Tick);
        }
        self.draw();
    }

    pub fn mouse(&mut self, kind: MouseKind, x: u16, y: u16) -> Outcome {
        let o = self.app.handle(Input::Mouse(Mouse {
            kind,
            pos: Position::new(x, y),
        }));
        self.draw();
        o
    }

    pub fn click(&mut self, x: u16, y: u16) {
        self.mouse(MouseKind::Down, x, y);
        self.mouse(MouseKind::Up, x, y);
    }

    pub fn text(&self) -> String {
        let buf = self.term.backend().buffer();
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    pub fn find(&self, needle: &str) -> Option<(u16, u16)> {
        let buf = self.term.backend().buffer();
        let want: Vec<&str> =
            unicode_segmentation::UnicodeSegmentation::graphemes(needle, true).collect();
        for y in 0..buf.area.height {
            let cells: Vec<&str> = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
            for x in 0..cells.len().saturating_sub(want.len() - 1) {
                if cells[x..x + want.len()] == want[..] {
                    return Some((x as u16, y));
                }
            }
        }
        None
    }

    pub fn focus(&self) -> Option<WidgetId> {
        self.app.focus.current()
    }

    pub fn wb(&self) -> &tablepro_app::workbench::Workbench {
        self.app.workbench.as_ref().unwrap()
    }

    pub fn wb_query(&self) -> &tablepro_app::tabs::QueryTab {
        match self.wb().active_tab() {
            Some(WorkTab::Query(q)) => q,
            _ => panic!("active tab is not a query"),
        }
    }
}
