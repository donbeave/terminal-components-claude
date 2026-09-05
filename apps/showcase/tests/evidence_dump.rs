//! Scratch-only frame capture for migration review evidence.
//!
//! This test is intentionally untracked and writes only under `EVIDENCE_DIR`.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use showcase_app::{App, PageId};
use tui_next::{ColorLevel, KeyCode, Theme};
use tui_next_testing::Harness;

const SIZES: [(u16, u16); 2] = [(80, 24), (120, 40)];
const COLORS: [ColorLevel; 2] = [ColorLevel::TrueColor, ColorLevel::Mono];

#[test]
fn dump_all_visual_frames() {
    let root = PathBuf::from(std::env::var("EVIDENCE_DIR").expect("EVIDENCE_DIR"));
    fs::create_dir_all(&root).expect("create evidence directory");
    let frames = root.join("frames");
    fs::create_dir_all(&frames).expect("create frame directory");
    let mut manifest = String::from("index\tkey\tdigest\tframe\n");
    let mut cases = 0usize;

    for page in PageId::ALL {
        for (width, height) in SIZES {
            for theme in [Theme::junie(), Theme::paper()] {
                for color in COLORS {
                    let mut harness =
                        Harness::new(App::with_page(page), theme.clone(), width, height)
                            .with_color(color);
                    // Preserve the legacy capture setup exactly.
                    let _ = harness.key(KeyCode::Tab);
                    let scene = harness.snapshot().named(page.title());
                    let key = scene.key();
                    let digest = format!("{:016x}", scene.digest());
                    let index = cases;
                    let file = format!("{index:03}.txt");
                    let mut frame = String::new();
                    writeln!(frame, "# key: {key}").expect("frame key");
                    writeln!(frame, "# digest: {digest}").expect("frame digest");
                    frame.push_str(&scene.text());
                    frame.push('\n');
                    fs::write(frames.join(&file), frame).expect("write frame");
                    writeln!(manifest, "{index:03}\t{key}\t{digest}\tframes/{file}")
                        .expect("manifest row");
                    cases = cases.saturating_add(1);
                }
            }
        }
    }

    assert_eq!(cases, 176);
    fs::write(root.join("manifest.tsv"), manifest).expect("write manifest");
}
