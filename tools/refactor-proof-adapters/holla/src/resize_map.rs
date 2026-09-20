//! Native-to-resized assertion/coordinate map.

use crate::observe::{ORACLE_SIZES, status_row};
use crate::tsv::Table;

const SITES_TSV: &str = include_str!("../data/holla-route-sites.tsv");

/// Mapping category from holla-route-expansion.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapCategory {
    /// Replay the original domain assertion unchanged.
    DimensionIndependent,
    /// Replace only the coordinate with oracle owner geometry per size.
    SourceRelativeGeometry,
    /// Native-only layout guard; resized capture keeps the full frame.
    NativeLayoutSpecific,
    /// Resolve pointer owner then freeze per-size oracle coordinates.
    PointerTargetRemap,
}

/// One native site expanded across the four oracle sizes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapRow {
    /// Site identity (`file:Lline`).
    pub site_id: String,
    /// Source file.
    pub source_file: String,
    /// Source blob hash.
    pub source_sha256: String,
    /// Test or helper symbol.
    pub source_symbol: String,
    /// Source line.
    pub line: u32,
    /// Planner site kinds.
    pub site_kinds: String,
    /// Original source line text.
    pub source_line: String,
    /// Mapping category.
    pub category: MapCategory,
    /// Per-size mapped records.
    pub sizes: Vec<SizeMapping>,
}

/// Geometry mapping at one size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeMapping {
    /// Width.
    pub width: u16,
    /// Height.
    pub height: u16,
    /// Mapped status row when applicable.
    pub status_row: u16,
    /// Mapped pointer cell when the native site names one.
    pub pointer: Option<(u16, u16)>,
}

/// Build the complete native-resize-map from the frozen site inventory.
#[must_use]
pub fn native_resize_map() -> Vec<MapRow> {
    let Some(table) = Table::parse(SITES_TSV) else {
        return Vec::new();
    };
    let mut rows = Vec::with_capacity(table.len());
    let mut index = 0;
    while index < table.len() {
        let site_id = table.get(index, "site_id").unwrap_or("").to_owned();
        let source_file = table.get(index, "source_file").unwrap_or("").to_owned();
        let source_sha256 = table.get(index, "source_sha256").unwrap_or("").to_owned();
        let source_symbol = table.get(index, "source_symbol").unwrap_or("").to_owned();
        let line = table
            .get(index, "line")
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let site_kinds = table.get(index, "site_kinds").unwrap_or("").to_owned();
        let source_line = table.get(index, "source_line").unwrap_or("").to_owned();
        let category = classify(&site_kinds, &source_line);
        let pointer_native = parse_pointer(&source_line);
        let sizes = ORACLE_SIZES
            .iter()
            .map(|&(width, height)| SizeMapping {
                width,
                height,
                status_row: status_row(height),
                pointer: pointer_native.map(|(x, y)| remap_pointer(x, y, height)),
            })
            .collect();
        rows.push(MapRow {
            site_id,
            source_file,
            source_sha256,
            source_symbol,
            line,
            site_kinds,
            source_line,
            category,
            sizes,
        });
        index = index.saturating_add(1);
    }
    rows
}

fn classify(kinds: &str, line: &str) -> MapCategory {
    let lower = line.to_ascii_lowercase();
    if lower.contains("mouse")
        || lower.contains("click(")
        || lower.contains(".click")
        || lower.contains("click_id")
    {
        MapCategory::PointerTargetRemap
    } else if kinds.contains("spatial-or-input")
        || lower.contains("row(")
        || lower.contains("row 3")
        || lower.contains("bottom()")
    {
        MapCategory::SourceRelativeGeometry
    } else if lower.contains("108") || lower.contains("split") || lower.contains("width") {
        MapCategory::NativeLayoutSpecific
    } else {
        MapCategory::DimensionIndependent
    }
}

fn parse_pointer(line: &str) -> Option<(u16, u16)> {
    // Native examples: `(3,38)` or `click(x, y)`.
    let start = line.find('(')?;
    let end = line.get(start..)?.find(')')?;
    let inner = line.get(start.saturating_add(1)..start.saturating_add(end))?;
    let (xs, ys) = inner.split_once(',')?;
    let x = xs.trim().parse().ok()?;
    let y = ys.trim().parse().ok()?;
    Some((x, y))
}

fn remap_pointer(x: u16, y: u16, height: u16) -> (u16, u16) {
    // Native status clicks at height 40 use row 38 = height - 2.
    if y >= 22 {
        (x, status_row(height))
    } else {
        (x, y)
    }
}

/// Number of mapped source sites.
#[must_use]
pub fn site_count() -> usize {
    native_resize_map().len()
}
