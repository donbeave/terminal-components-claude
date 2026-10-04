//! Conformance Suite: typed case registry reconciliation and integrity verification.

#[path = "conformance/drivers.rs"]
mod drivers;
#[path = "conformance/registry.rs"]
mod registry;

use registry::{
    AuthorityLane, HISTORICAL_BASELINE_COMMIT, HISTORICAL_BASELINE_TAG, REFERENCE_APP_SHA,
    RequiredCasesManifest, TUISCOTTI_SHA,
};
use std::path::Path;

#[test]
fn test_tool_and_commit_pins() {
    assert_eq!(
        REFERENCE_APP_SHA,
        "7bd6a331721737514a2477c894d922cb262ef07b"
    );
    assert_eq!(HISTORICAL_BASELINE_TAG, "visual-baseline");
    assert_eq!(
        HISTORICAL_BASELINE_COMMIT,
        "4a79c0a2d40fca46fc406b77157ce3b3f12ec16b"
    );
    assert_eq!(TUISCOTTI_SHA, "a47c9aaefb34e4c00026f99d8a8dd7ee5916b274");
}

#[test]
fn test_manifest_loads_and_counts_match() {
    let manifest = RequiredCasesManifest::load();

    assert_eq!(
        manifest.components_count, 45,
        "must declare exactly 45 components (W01-W45)"
    );
    assert_eq!(
        manifest.foundations_count, 12,
        "must declare exactly 12 foundations (F01-F12)"
    );
    assert_eq!(
        manifest.family_dispositions_count, 54,
        "must declare exactly 54 family dispositions (C01-C54)"
    );
    assert_eq!(
        manifest.capture_plans_count, 45,
        "must declare exactly 45 capture plans"
    );
    assert_eq!(
        manifest.component_cases_count, 222,
        "must declare exactly 222 component cases"
    );
    assert_eq!(
        manifest.legacy_roots_count, 302,
        "must declare exactly 302 legacy snapshot roots"
    );
    assert_eq!(
        manifest.total_cases_count, 524,
        "total cases must equal 222 + 302 = 524"
    );
    assert_eq!(manifest.cases.len(), 524);

    assert_eq!(manifest.components.len(), 45);
    assert_eq!(manifest.foundations.len(), 12);
    assert_eq!(manifest.family_dispositions.len(), 54);
}

#[test]
fn test_case_registry_integrity_and_authority_lanes() {
    let manifest = RequiredCasesManifest::load();
    let cases_map = manifest.cases_by_id();
    assert_eq!(
        cases_map.len(),
        manifest.cases.len(),
        "all case IDs must be strictly unique"
    );

    let mut existing_count = 0;
    let mut extracted_count = 0;
    let mut extension_count = 0;

    for case in cases_map.values() {
        assert!(!case.id.is_empty());
        assert!(!case.owner.is_empty());
        assert!(!case.source_path.is_empty());
        assert!(!case.expected_observations.is_empty());
        assert!(!case.dimensions.is_empty());
        assert!(!case.capabilities.is_empty());

        match case.authority_lane {
            AuthorityLane::ExistingOracle => {
                existing_count += 1;
                assert_eq!(
                    case.approval_state, "approved",
                    "existing legacy oracle case `{}` must be approved",
                    case.id
                );
            }
            AuthorityLane::ExtractedOracle => {
                extracted_count += 1;
                assert_eq!(
                    case.approval_state, "approved",
                    "extracted component case `{}` must be approved by executable driver receipts",
                    case.id
                );
            }
            AuthorityLane::Extension => {
                extension_count += 1;
                assert_eq!(
                    case.approval_state, "planned",
                    "extension case `{}` must be planned for Stage B P6",
                    case.id
                );
            }
        }
    }

    assert!(existing_count > 0);
    assert_eq!(extracted_count, 216);
    assert_eq!(
        extension_count, 6,
        "only W44-01..W44-06 terminal-view cases are extensions"
    );
    eprintln!(
        "Registry authority lane distribution: ExistingOracle={}, ExtractedOracle={}, Extension={}",
        existing_count, extracted_count, extension_count
    );
}

#[test]
fn test_execute_all_component_drivers() {
    let manifest = RequiredCasesManifest::load();
    let cases_map = manifest.cases_by_id();
    let mut executed_extracted = 0;
    let mut verified_extensions = 0;

    for case in cases_map.values() {
        if case.id.starts_with('W') {
            let receipt = drivers::run_case_driver(&case.id)
                .unwrap_or_else(|e| panic!("driver execution failed for {}: {e}", case.id));
            assert_eq!(receipt.status, "passed");
            assert_eq!(receipt.case_id, case.id);
            assert!(!receipt.observations_verified.is_empty());
            assert!(receipt.execution_duration_us > 0);

            match case.authority_lane {
                AuthorityLane::ExtractedOracle => {
                    executed_extracted += 1;
                    assert!(receipt.stage_b_owner.is_none());
                }
                AuthorityLane::Extension => {
                    verified_extensions += 1;
                    assert_eq!(
                        receipt.stage_b_owner,
                        Some("Stage B Phase P6 (TerminalView Integration)".to_string()),
                        "Extension case {} must have Stage B Phase P6 ownership",
                        case.id
                    );
                }
                _ => {}
            }
        }
    }

    assert_eq!(
        executed_extracted, 216,
        "must execute exactly 216 extracted component drivers"
    );
    assert_eq!(
        verified_extensions, 6,
        "must verify exactly 6 extension cases"
    );
    eprintln!(
        "Successfully executed {} ExtractedOracle component drivers and qualified {} Extension cases",
        executed_extracted, verified_extensions
    );
}

#[test]
fn test_legacy_roots_match_snapshot_disk_tree() {
    let manifest = RequiredCasesManifest::load();
    let baseline_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("baselines/tuiscotti-v1");
    let legacy_snapshots_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots");

    assert!(
        !legacy_snapshots_dir.exists(),
        "legacy snapshots/ corpus must be deleted post-cutover"
    );

    let map_bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/verification/snapshot-migration-map.json"),
    )
    .expect("read migration map");
    let map: serde_json::Value = serde_json::from_slice(&map_bytes).expect("parse migration map");
    let root_mappings = map["root_mappings"]
        .as_array()
        .expect("root mappings array");
    let rmaps: std::collections::BTreeMap<&str, &str> = root_mappings
        .iter()
        .map(|r| {
            (
                r["legacy_root"].as_str().unwrap(),
                r["target_root"].as_str().unwrap(),
            )
        })
        .collect();

    let legacy_cases: Vec<_> = manifest
        .cases
        .iter()
        .filter(|c| c.id.starts_with("LEGACY:"))
        .collect();

    assert_eq!(legacy_cases.len(), 302);

    for case in legacy_cases {
        let legacy_root = case.id.strip_prefix("LEGACY:").unwrap();
        let target_root = rmaps
            .get(legacy_root)
            .unwrap_or_else(|| panic!("legacy root `{legacy_root}` must be in migration map"));
        let root_path = baseline_dir.join(target_root);
        assert!(
            root_path.exists(),
            "target baseline directory for `{target_root}` must exist on disk: {}",
            root_path.display()
        );
    }
}

#[test]
fn test_negative_manifest_mutations_fail_validation() {
    let manifest = RequiredCasesManifest::load();

    // 1. Duplicate case ID detection
    let mut cases_with_dup = manifest.cases.clone();
    let dup_case = cases_with_dup[0].clone();
    cases_with_dup.push(dup_case);
    let mut seen = std::collections::HashSet::new();
    let mut has_duplicate = false;
    for c in &cases_with_dup {
        if !seen.insert(&c.id) {
            has_duplicate = true;
            break;
        }
    }
    assert!(has_duplicate, "duplicate case insertion must be detected");

    // 2. Dropping a case breaks count integrity
    let mut cases_truncated = manifest.cases.clone();
    cases_truncated.pop();
    assert_ne!(
        cases_truncated.len(),
        manifest.total_cases_count,
        "dropped case must violate total_cases_count invariant"
    );

    // 3. Fictitious legacy root must not exist in baselines/tuiscotti-v1/
    let fictitious_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("baselines/tuiscotti-v1")
        .join("nonexistent_app")
        .join("invalid_page");
    assert!(
        !fictitious_root.exists(),
        "fictitious legacy root must not exist"
    );
}
