//! Disposable evidence dump for the Showcase visual review.
//!
//! This file exists only in the scratch worktree used to review a candidate.
//! It intentionally does not compare or write the repository baseline.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use showcase_app::{App, PageId};
use tui_next::{ColorLevel, KeyCode, Theme};
use tui_next_testing::Harness;

const SIZES: [(u16, u16); 2] = [(80, 24), (120, 40)];
const COLORS: [ColorLevel; 2] = [ColorLevel::TrueColor, ColorLevel::Mono];

fn output_dir() -> PathBuf {
    std::env::var_os("SHOWCASE_EVIDENCE_DIR")
        .map(PathBuf::from)
        .expect("SHOWCASE_EVIDENCE_DIR")
}

fn filename(index: usize, key: &str) -> String {
    let safe = key
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    format!("{index:03}_{safe}.txt")
}

#[test]
fn dump_showcase_visual_frames() {
    let dir = output_dir();
    fs::create_dir_all(&dir).expect("create evidence directory");
    let mut manifest = String::new();
    let mut index = 0;

    for page in PageId::ALL {
        for (width, height) in SIZES {
            for theme in [Theme::junie(), Theme::paper()] {
                for color in COLORS {
                    let mut harness =
                        Harness::new(App::with_page(page), theme.clone(), width, height)
                            .with_color(color);
                    // This is the legacy capture setup and is part of the
                    // digest contract: focus the first page control first.
                    let _ = harness.key(KeyCode::Tab);
                    let scene = harness.snapshot().named(page.title());
                    let key = scene.key();
                    let digest = scene.digest();
                    let name = filename(index, &key);
                    let frame_path = dir.join(&name);
                    let mut frame = String::new();
                    writeln!(frame, "key={key}").expect("write frame header");
                    writeln!(frame, "digest={digest:016x}").expect("write frame digest");
                    frame.push('\n');
                    frame.push_str(&scene.text());
                    if !frame.ends_with('\n') {
                        frame.push('\n');
                    }
                    fs::write(&frame_path, frame).expect("write frame text");
                    writeln!(manifest, "{index:03}\t{key}\t{digest:016x}\t{name}")
                        .expect("write evidence manifest");
                    index += 1;
                }
            }
        }
    }

    assert_eq!(index, 176, "the review matrix must contain 176 frames");
    fs::write(dir.join("manifest.tsv"), manifest).expect("write evidence manifest");
}
