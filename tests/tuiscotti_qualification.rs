//! Tuiscotti API Qualification Probe.
//!
//! Validates:
//! 1. In-memory pure Ratatui widget render into Screen -> Frame -> CaptureBundle (all 6 formats:
//!    .ansi, .html, .png, .ascii, .txt, .frame.json) plus loss/fidelity sidecars.
//! 2. Companion format invariant validators (assert_seven_bit, assert_no_escapes,
//!    assert_normalized_sgr, assert_opaque_rgb, assert_static_offline, parse_canonical).
//! 3. PTY Session launch via Tui -> input/press -> wait_stable -> snapshot -> Frame -> capture_all.
//! 4. GroupedStore checking and scratch output handling.
//! 5. Negative comparison: cell mutation, pixel mutation, and missing approval detection.

use std::time::Duration;

use tuiscotti::Profile;
use tuiscotti::VENDORED_FACES;
use tuiscotti::formats::{
    assert_no_escapes, assert_normalized_sgr, assert_opaque_rgb, assert_seven_bit,
    assert_static_offline, capture_all, parse_canonical,
};
use tuiscotti::grouped::GroupedStore;
use tuiscotti::ratatui::{EdgePolicy, render_screen};
use tuiscotti::render::frame_from_screen;
use tuiscotti::snapshot::Status;
use tuiscotti::tui::Tui;

#[test]
fn qualify_pure_ratatui_capture_and_six_formats() {
    let cols = 80;
    let rows = 24;

    // Render pure Ratatui widget into Screen
    let capture = render_screen(cols, rows, |f| {
        f.render_widget("Pure Ratatui Render Invariant Check", f.area());
    }, EdgePolicy::ClipWithReplacement)
    .expect("render_screen succeeds");

    assert_eq!(capture.screen.cols(), cols);
    assert_eq!(capture.screen.rows(), rows);

    // Convert Screen to Frame
    let profile = Profile::default_profile();
    let frame = frame_from_screen(&capture.screen, &profile.name);
    assert_eq!(frame.cols, cols);
    assert_eq!(frame.rows, rows);

    // Render all 6 formats in one pass
    let mut renderer = profile
        .renderer(&VENDORED_FACES)
        .expect("vendored faces parse");
    let bundle = capture_all(&mut renderer, &frame, "pure-ratatui-qual")
        .expect("capture_all succeeds");

    // 1. .ascii validation
    assert_seven_bit(&bundle.ascii.text).expect("ascii projection is 7-bit");
    assert!(!bundle.ascii.text.is_empty());

    // 2. .txt validation
    assert_no_escapes(&bundle.txt).expect("txt projection has no escapes");
    assert!(bundle.txt.contains("Pure Ratatui Render Invariant Check"));

    // 3. .ansi validation
    assert_normalized_sgr(&bundle.ansi).expect("ansi projection is normalized SGR");
    assert!(bundle.ansi.contains("\x1b["));

    // 4. .png validation
    assert_opaque_rgb(&bundle.png).expect("png is opaque RGB");
    assert!(!bundle.png.is_empty());

    // 5. .html validation
    assert_static_offline(&bundle.html).expect("html is static and offline");
    assert!(bundle.html.contains("data:image/png;base64,"));

    // 6. .frame.json validation
    let parsed_frame = parse_canonical(&bundle.json).expect("frame.json is valid canonical JSON");
    assert_eq!(parsed_frame.cols, cols);
    assert_eq!(parsed_frame.rows, rows);
    assert_eq!(parsed_frame.digest(), frame.digest());

    // Check generation determinism
    assert_eq!(bundle.generation.frame_digest, frame.digest());
    assert_eq!(bundle.generation.profile, profile.name);
}

#[test]
fn qualify_pty_session_lifecycle_and_capture() {
    let showcase_bin = env!("CARGO_BIN_EXE_showcase");
    let cols = 80;
    let rows = 24;

    let session = Tui::new([showcase_bin, "--page", "overview", "--color", "truecolor"])
        .size(cols, rows)
        .env_remove("NO_COLOR")
        .env_remove("HOLLA_NO_MOTION")
        .spawn()
        .expect("session spawn succeeds");

    // Wait until boot needle or settled
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let cancel = tuiscotti::tui::CancelToken::new();
    let obs = session
        .wait_predicate(
            |obs| frame_from_screen(&obs.screen, "default").text().contains("Junie"),
            deadline,
            &cancel,
        )
        .expect("overview page loaded with needle");

    assert_eq!(obs.screen.cols(), cols);
    assert_eq!(obs.screen.rows(), rows);

    // Convert observed screen to Frame
    let profile = Profile::default_profile();
    let frame = frame_from_screen(&obs.screen, &profile.name);

    let mut renderer = profile
        .renderer(&VENDORED_FACES)
        .expect("vendored faces parse");
    let bundle = capture_all(&mut renderer, &frame, "pty-showcase-overview")
        .expect("capture_all on pty frame succeeds");

    assert_seven_bit(&bundle.ascii.text).expect("ascii 7-bit");
    assert_no_escapes(&bundle.txt).expect("txt clean");
    assert_normalized_sgr(&bundle.ansi).expect("ansi normalized");
    assert_opaque_rgb(&bundle.png).expect("png opaque");
    assert_static_offline(&bundle.html).expect("html static");
    let parsed = parse_canonical(&bundle.json).expect("valid frame json");
    assert_eq!(parsed.digest(), frame.digest());

    // Send key input to navigate
    session.press("Tab").expect("press Tab succeeds");
    let settle_deadline = std::time::Instant::now() + Duration::from_secs(2);
    session
        .wait_stable(settle_deadline, &cancel)
        .expect("session settled after Tab");

    // Clean exit
    session.press("q").expect("press q succeeds");
    let exit_deadline = std::time::Instant::now() + Duration::from_secs(3);
    let exit_wait = session
        .wait_exit(exit_deadline, &cancel)
        .expect("session exits cleanly");
    assert!(exit_wait.status.success(), "session exit was successful");
}

#[test]
fn qualify_grouped_store_and_negative_detection() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let approved_root = tmp.path().join("approved");
    let actual_root = tmp.path().join("actual");
    let diff_root = tmp.path().join("diff");
    let report_path = actual_root.join("report.html");

    std::fs::create_dir_all(&approved_root).expect("create approved root");

    let store = GroupedStore::new(&approved_root)
        .with_actual_root(&actual_root)
        .with_diff_root(&diff_root)
        .with_report_path(&report_path);

    let profile = Profile::default_profile();
    let mut renderer = profile
        .renderer(&VENDORED_FACES)
        .expect("vendored faces parse");

    // Create frame A
    let capture_a = render_screen(40, 10, |f| {
        f.render_widget("Hello World", f.area());
    }, EdgePolicy::ClipWithReplacement)
    .expect("render_screen");
    let frame_a = frame_from_screen(&capture_a.screen, &profile.name);

    let case_name = "test/qual/hello";

    // 1. Missing approval fails closed
    let outcome1 = store
        .check_with(&mut renderer, case_name, &frame_a, 1.0)
        .expect("check_with");
    assert_eq!(outcome1.status(), Status::MissingApproval);
    assert!(actual_root.join(format!("{case_name}.ansi")).exists());
    assert!(actual_root.join(format!("{case_name}.png")).exists());

    // 2. Bless/accept to approve
    store.accept(case_name).expect("accept succeeds");
    assert!(approved_root.join(format!("{case_name}.ansi")).exists());
    assert!(approved_root.join(format!("{case_name}.txt")).exists());
    assert!(approved_root.join(format!("{case_name}.html")).exists());
    assert!(approved_root.join(format!("{case_name}.png")).exists());

    // 3. Exact match passes
    let outcome2 = store
        .check_with(&mut renderer, case_name, &frame_a, 1.0)
        .expect("check_with");
    assert_eq!(outcome2.status(), Status::Matched);

    // 4. Negative test: mutate cell text -> CellsDiffer
    let capture_b = render_screen(40, 10, |f| {
        f.render_widget("Hello Earth", f.area());
    }, EdgePolicy::ClipWithReplacement)
    .expect("render_screen");
    let frame_b = frame_from_screen(&capture_b.screen, &profile.name);

    let outcome_mut_text = store
        .check_with(&mut renderer, case_name, &frame_b, 1.0)
        .expect("check_with");
    assert_eq!(outcome_mut_text.status(), Status::CellsDiffer);

    // 5. Negative test: pure color/style mutation with identical text -> CellsDiffer
    let mut frame_c = frame_a.clone();
    for cell in &mut frame_c.cells {
        if cell.symbol != " " {
            cell.fg = tuiscotti::Color::Rgb(tuiscotti::Rgb::new(0, 255, 255));
        }
    }
    assert_eq!(
        frame_a.text(),
        frame_c.text(),
        "text must be identical to isolate color mutation"
    );

    let outcome_mut_color = store
        .check_with(&mut renderer, case_name, &frame_c, 1.0)
        .expect("check_with");
    assert_eq!(outcome_mut_color.status(), Status::CellsDiffer);

    // 6. Negative test: mutate dimension -> DimensionMismatch
    let capture_d = render_screen(50, 10, |f| {
        f.render_widget("Hello World", f.area());
    }, EdgePolicy::ClipWithReplacement)
    .expect("render_screen");
    let frame_d = frame_from_screen(&capture_d.screen, &profile.name);

    let outcome_mut_dim = store
        .check_with(&mut renderer, case_name, &frame_d, 1.0)
        .expect("check_with");
    assert_eq!(outcome_mut_dim.status(), Status::DimensionMismatch);

    // 7. Negative test: mutate a decoded pixel in approved PNG (cells and text match) -> PixelsDiffer
    let approved_png_path = approved_root.join(format!("{case_name}.png"));
    let mut img = image::load_from_memory(&std::fs::read(&approved_png_path).unwrap())
        .expect("decode approved png")
        .to_rgb8();
    let pixel = img.get_pixel_mut(0, 0);
    pixel[0] = 255 - pixel[0];
    pixel[1] = 255 - pixel[1];
    pixel[2] = 255 - pixel[2];
    img.save(&approved_png_path).expect("save mutated png");

    let outcome_mut_pixel = store
        .check_with(&mut renderer, case_name, &frame_a, 1.0)
        .expect("check_with should succeed but report pixel difference");
    assert_eq!(outcome_mut_pixel.status(), Status::PixelsDiffer);
}

#[test]
fn qualify_corrupt_and_tampered_artifact_rejection() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let approved_root = tmp.path().join("approved");
    let actual_root = tmp.path().join("actual");
    let diff_root = tmp.path().join("diff");
    let report_path = actual_root.join("report.html");

    std::fs::create_dir_all(&approved_root).expect("create approved root");

    let store = GroupedStore::new(&approved_root)
        .with_actual_root(&actual_root)
        .with_diff_root(&diff_root)
        .with_report_path(&report_path);

    let profile = Profile::default_profile();
    let mut renderer = profile
        .renderer(&VENDORED_FACES)
        .expect("vendored faces parse");

    let capture = render_screen(40, 10, |f| {
        f.render_widget("Invariant Protection Gate", f.area());
    }, EdgePolicy::ClipWithReplacement)
    .expect("render_screen");
    let frame = frame_from_screen(&capture.screen, &profile.name);

    let case_name = "test/qual/tamper";

    // 1. Initial check fails closed as MissingApproval
    let outcome = store.check_with(&mut renderer, case_name, &frame, 1.0).unwrap();
    assert_eq!(outcome.status(), Status::MissingApproval);

    // 2. Accept into approved
    store.accept(case_name).expect("accept");
    let matched = store.check_with(&mut renderer, case_name, &frame, 1.0).unwrap();
    assert_eq!(matched.status(), Status::Matched);

    // Backup valid approved artifacts
    let approved_ansi = approved_root.join(format!("{case_name}.ansi"));
    let approved_txt = approved_root.join(format!("{case_name}.txt"));
    let approved_html = approved_root.join(format!("{case_name}.html"));
    let approved_png = approved_root.join(format!("{case_name}.png"));

    let orig_ansi = std::fs::read(&approved_ansi).unwrap();
    let orig_txt = std::fs::read(&approved_txt).unwrap();
    let orig_html = std::fs::read(&approved_html).unwrap();
    let orig_png = std::fs::read(&approved_png).unwrap();

    // 3. Test missing artifact rejection for each format -> MissingApproval
    // 3a. Missing .ansi
    std::fs::remove_file(&approved_ansi).unwrap();
    assert_eq!(
        store.check_with(&mut renderer, case_name, &frame, 1.0).unwrap().status(),
        Status::MissingApproval
    );
    std::fs::write(&approved_ansi, &orig_ansi).unwrap();

    // 3b. Missing .txt
    std::fs::remove_file(&approved_txt).unwrap();
    assert_eq!(
        store.check_with(&mut renderer, case_name, &frame, 1.0).unwrap().status(),
        Status::MissingApproval
    );
    std::fs::write(&approved_txt, &orig_txt).unwrap();

    // 3c. Missing .html
    std::fs::remove_file(&approved_html).unwrap();
    assert_eq!(
        store.check_with(&mut renderer, case_name, &frame, 1.0).unwrap().status(),
        Status::MissingApproval
    );
    std::fs::write(&approved_html, &orig_html).unwrap();

    // 3d. Missing .png
    std::fs::remove_file(&approved_png).unwrap();
    assert_eq!(
        store.check_with(&mut renderer, case_name, &frame, 1.0).unwrap().status(),
        Status::MissingApproval
    );
    std::fs::write(&approved_png, &orig_png).unwrap();

    // 4. Test corrupted artifact rejection for each format
    // 4a. Corrupted .ansi (bytes modified) -> CellsDiffer
    std::fs::write(&approved_ansi, b"corrupted ansi content").unwrap();
    assert_eq!(
        store.check_with(&mut renderer, case_name, &frame, 1.0).unwrap().status(),
        Status::CellsDiffer
    );
    std::fs::write(&approved_ansi, &orig_ansi).unwrap();

    // 4b. Corrupted .txt (bytes modified) -> CellsDiffer
    std::fs::write(&approved_txt, b"corrupted txt content").unwrap();
    assert_eq!(
        store.check_with(&mut renderer, case_name, &frame, 1.0).unwrap().status(),
        Status::CellsDiffer
    );
    std::fs::write(&approved_txt, &orig_txt).unwrap();

    // 4c. Corrupted .png (invalid PNG bytes) -> SnapshotError or non-Matched
    std::fs::write(&approved_png, b"NOT_A_VALID_PNG_CORRUPT_BYTES").unwrap();
    let corrupt_png_outcome = store.check_with(&mut renderer, case_name, &frame, 1.0);
    match corrupt_png_outcome {
        Ok(res) => assert_ne!(res.status(), Status::Matched),
        Err(_) => {} // Expected: error decoding invalid PNG bytes
    }
    std::fs::write(&approved_png, &orig_png).unwrap();

    // Verify restore works
    assert_eq!(
        store.check_with(&mut renderer, case_name, &frame, 1.0).unwrap().status(),
        Status::Matched
    );
}

