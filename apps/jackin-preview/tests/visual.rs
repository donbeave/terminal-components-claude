//! Scratch visual determinism coverage.
//!
//! This intentionally does not read or write a golden baseline.  It provides
//! repeatable digests for independent review while the migrated frames are
//! still being classified; blessing remains an explicit coordinator action.

use jackin_app::{App, Motion, Scenario};
use tui_next::Theme;
use tui_next_testing::Harness;

#[test]
fn jackin_visual_baseline() {
    for scenario in Scenario::ALL {
        for (width, height) in [(80, 24), (100, 30), (120, 40), (160, 50)] {
            let first = Harness::new(
                App::for_scenario_at(scenario, Motion::Paused, 0),
                Theme::junie(),
                width,
                height,
            );
            let second = Harness::new(
                App::for_scenario_at(scenario, Motion::Paused, 0),
                Theme::junie(),
                width,
                height,
            );
            assert_eq!(first.snapshot().digest(), second.snapshot().digest());
            assert_eq!(
                first.text(),
                second.text(),
                "{} {width}x{height}",
                scenario.name()
            );
            assert!(first.diagnostics().is_empty(), "{}", scenario.name());
        }
    }
}
