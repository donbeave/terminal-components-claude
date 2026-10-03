//! Durable typed case registry and manifest definitions.
//!
//! Reconciles:
//! - 45 components (W01 to W45);
//! - 12 foundations (F01 to F12);
//! - 54 legacy family dispositions (C01 to C54);
//! - 45 capture plans;
//! - 222 component-specific cases;
//! - 302 legacy snapshot roots;
//! - Total 524 primary registered cases across the 4 applications.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const REFERENCE_APP_SHA: &str = "7bd6a331721737514a2477c894d922cb262ef07b";
pub const HISTORICAL_BASELINE_TAG: &str = "visual-baseline";
pub const HISTORICAL_BASELINE_COMMIT: &str = "4a79c0a2d40fca46fc406b77157ce3b3f12ec16b";
pub const TUISCOTTI_SHA: &str = "a47c9aaefb34e4c00026f99d8a8dd7ee5916b274";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AuthorityLane {
    ExistingOracle,
    ExtractedOracle,
    Extension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MotionPolicy {
    Full,
    Reduced,
    Paused,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CaptureMethod {
    PtySession,
    DirectRatatui,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryCase {
    pub id: String,
    pub owner: String,
    pub source_path: String,
    pub source_symbol: String,
    pub fixture: String,
    pub variant: String,
    pub state: String,
    pub authority_lane: AuthorityLane,
    pub event_program: Vec<serde_json::Value>,
    pub boot_needle: String,
    pub time_checkpoints_ms: Vec<u64>,
    pub dimensions: Vec<(u16, u16)>,
    pub capabilities: Vec<String>,
    pub motion_policy: MotionPolicy,
    pub expected_observations: Vec<String>,
    pub capture_method: CaptureMethod,
    pub provenance: String,
    pub approval_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentEntry {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub cases_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationEntry {
    pub id: String,
    pub slug: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamilyDispositionEntry {
    pub id: String,
    pub old_family: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredCasesManifest {
    pub schema: String,
    pub reference_app_sha: String,
    pub historical_baseline_tag: String,
    pub historical_baseline_commit: String,
    pub tuiscotti_sha: String,
    pub components_count: usize,
    pub foundations_count: usize,
    pub family_dispositions_count: usize,
    pub capture_plans_count: usize,
    pub component_cases_count: usize,
    pub legacy_roots_count: usize,
    pub total_cases_count: usize,
    pub components: Vec<ComponentEntry>,
    pub foundations: Vec<FoundationEntry>,
    pub family_dispositions: Vec<FamilyDispositionEntry>,
    pub cases: Vec<RegistryCase>,
}

impl RequiredCasesManifest {
    /// Load the canonical manifest from disk.
    pub fn load() -> Self {
        let manifest_bytes = include_bytes!("required_cases.json");
        serde_json::from_slice(manifest_bytes)
            .expect("deserializing required_cases.json must succeed")
    }

    /// Construct the typed registry cases map by case ID.
    pub fn cases_by_id(&self) -> BTreeMap<String, &RegistryCase> {
        let mut map = BTreeMap::new();
        for case in &self.cases {
            assert!(
                map.insert(case.id.clone(), case).is_none(),
                "duplicate case ID in registry: {}",
                case.id
            );
        }
        map
    }

    /// Get all case IDs.
    #[allow(dead_code)]
    pub fn case_ids(&self) -> BTreeSet<String> {
        self.cases.iter().map(|c| c.id.clone()).collect()
    }
}
