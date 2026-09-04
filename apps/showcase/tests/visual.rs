//! Visual digest matrix for the migrated showcase shell.
use junie_tui::{ColorLevel, Theme};
use junie_tui_testing::Harness;
use showcase_app::{App, PageId};

const BASELINE: junie_tui_testing::Baseline = junie_tui_testing::Baseline::new(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/baselines/showcase.txt"
));

const SIZES: [(u16, u16); 2] = [(80, 24), (120, 40)];
const COLORS: [ColorLevel; 2] = [ColorLevel::TrueColor, ColorLevel::Mono];
const EXPECTED_CASES: usize = 22 * SIZES.len() * 2 * COLORS.len();

/// Keep the legacy test name while covering every page in the full visual
/// matrix: the two legacy sizes, truecolour/mono, and both themes.
#[test]
fn showcase_visual_baseline() {
    let mut cases = 0;
    for page in PageId::ALL {
        for (width, height) in SIZES {
            for theme in [Theme::junie(), Theme::paper()] {
                for color in COLORS {
                    cases += 1;
                    let mut h = Harness::new(App::with_page(page), theme.clone(), width, height)
                        .with_color(color);
                    let _ = h.key(junie_tui::KeyCode::Tab);
                    h.snapshot().named(page.title()).assert_against(&BASELINE);
                }
            }
        }
    }
    assert_eq!(cases, EXPECTED_CASES);
}
