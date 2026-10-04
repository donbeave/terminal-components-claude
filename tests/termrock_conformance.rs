//! Termrock Conformance Suite and Protected Path Invariants.

#![allow(unused_imports, unused_variables, dead_code)]

#[path = "conformance/drivers.rs"]
mod drivers;
#[path = "conformance/registry.rs"]
mod registry;

use std::path::Path;

#[test]
fn protected_paths() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    // snapshots/ must NOT exist (cut over to baselines/tuiscotti-v1)
    assert!(
        !manifest_dir.join("snapshots").exists(),
        "legacy snapshots/ must remain deleted"
    );

    // Protected paths must remain intact
    assert!(manifest_dir.join("src/bin").exists(), "src/bin must exist");
    assert!(
        manifest_dir.join("tests/visual_baseline").exists(),
        "tests/visual_baseline must exist"
    );
    assert!(
        manifest_dir.join("Cargo.toml").exists(),
        "Cargo.toml must exist"
    );
    assert!(
        manifest_dir.join("Cargo.lock").exists(),
        "Cargo.lock must exist"
    );
}

#[test]
fn test_termrock_conformance_pins_and_manifest() {
    assert_eq!(registry::HISTORICAL_BASELINE_TAG, "visual-baseline");
    assert_eq!(
        registry::HISTORICAL_BASELINE_COMMIT,
        "4a79c0a2d40fca46fc406b77157ce3b3f12ec16b"
    );

    let manifest = registry::RequiredCasesManifest::load();
    assert_eq!(manifest.components_count, 45);
    assert_eq!(manifest.foundations_count, 12);
    assert_eq!(manifest.family_dispositions_count, 54);
    assert_eq!(manifest.total_cases_count, 524);
}
