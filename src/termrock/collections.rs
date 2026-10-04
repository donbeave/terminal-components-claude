//! Keyed collection foundations, reconciliation algorithms, models, and traits.
//!
//! Provides stable semantic item keys, revision-aware reconciliation, selection
//! modes and requests, tree sources, and constrained row presenters.

use std::collections::HashSet;
use std::fmt;

use crate::termrock::author::RowUi;
pub use crate::termrock::empty::Readiness;
use crate::termrock::identity::{Id, ItemKey, Keyed, Revision};
use crate::termrock::layout::Rect;
use crate::termrock::response::VisualState;
pub use crate::termrock::text::MatchResult;

/// Implement [`Keyed`] for reference types.
impl<T: Keyed> Keyed for &T {
    fn key(&self) -> ItemKey {
        (**self).key()
    }
}

/// Selection mode supported by a keyed collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SelectionMode {
    /// Items cannot be selected; navigation cursor only.
    None,
    /// At most one item can be selected at a time.
    #[default]
    Single,
    /// Zero or more items can be selected simultaneously.
    Multiple,
}

/// Request to alter the collection's selection state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SelectionRequest {
    /// Select the specified item key.
    Select(ItemKey),
    /// Deselect the specified item key.
    Deselect(ItemKey),
    /// Toggle selection of the specified item key.
    Toggle(ItemKey),
    /// Select a contiguous range of keys between `from` and `to`.
    SelectRange { from: ItemKey, to: ItemKey },
    /// Select all selectable items in the collection.
    SelectAll,
    /// Clear all selections in the collection.
    Clear,
}

/// Dynamic presentation state of a collection row during painting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowState {
    pub visual: VisualState,
    pub selected: bool,
    pub cursor: bool,
    pub index: usize,
}

impl RowState {
    pub const fn new(visual: VisualState, selected: bool, cursor: bool, index: usize) -> Self {
        Self {
            visual,
            selected,
            cursor,
            index,
        }
    }
}

/// Constrained painter callback for a collection row.
pub type RowPainter<T> = dyn Fn(&mut RowUi<'_>, Rect, &T, RowState);

/// Errors arising from collection validation and reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionError {
    DuplicateKey(ItemKey),
    StaleRevision {
        source: Revision,
        expected: Revision,
    },
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey(key) => {
                write!(f, "Duplicate item key detected in collection: {key}")
            }
            Self::StaleRevision { source, expected } => {
                write!(
                    f,
                    "Collection revision stale: source {source} != expected {expected}"
                )
            }
        }
    }
}

impl std::error::Error for CollectionError {}

/// Validate that all item keys in the slice are unique, failing closed on duplicates.
pub fn validate_unique_keys<T: Keyed>(items: &[T]) -> Result<(), CollectionError> {
    let mut seen = HashSet::with_capacity(items.len());
    for item in items {
        let key = item.key();
        if !seen.insert(key) {
            return Err(CollectionError::DuplicateKey(key));
        }
    }
    Ok(())
}

/// Deterministically reconcile a cursor key against surviving items.
///
/// If `cursor` exists in `items`, it is retained.
/// Otherwise, falls back to the nearest successor or predecessor if known,
/// or the first item in the list.
pub fn reconcile_cursor(cursor: Option<ItemKey>, items: &[ItemKey]) -> Option<ItemKey> {
    if items.is_empty() {
        return None;
    }
    if let Some(c) = cursor.filter(|c| items.contains(c)) {
        return Some(c);
    }
    items.first().copied()
}

/// Reconcile a cursor key using prior item order for deterministic fallback:
/// chooses nearest successor, then predecessor, then first surviving item.
pub fn reconcile_cursor_with_fallback(
    cursor: Option<ItemKey>,
    prior_items: &[ItemKey],
    items: &[ItemKey],
) -> Option<ItemKey> {
    if items.is_empty() {
        return None;
    }
    let Some(c) = cursor else {
        return items.first().copied();
    };
    if items.contains(&c) {
        return Some(c);
    }

    if let Some(old_idx) = prior_items.iter().position(|&k| k == c) {
        // Nearest successor in prior items that survived
        for next in &prior_items[old_idx + 1..] {
            if items.contains(next) {
                return Some(*next);
            }
        }
        // Nearest predecessor in prior items that survived
        for prev in prior_items[..old_idx].iter().rev() {
            if items.contains(prev) {
                return Some(*prev);
            }
        }
        // Display position fallback
        if let Some(&k) = items.get(old_idx) {
            return Some(k);
        }
        if let Some(&k) = old_idx.checked_sub(1).and_then(|i| items.get(i)) {
            return Some(k);
        }
    }
    items.first().copied()
}

/// Reconcile selected keys against surviving items.
///
/// Drops keys that are no longer present in `items` while preserving order.
pub fn reconcile_selection(selected: &[ItemKey], items: &[ItemKey]) -> Vec<ItemKey> {
    selected
        .iter()
        .copied()
        .filter(|k| items.contains(k))
        .collect()
}

/// Extract child ItemKey from an Id if it matches parent's child pattern.
pub fn child_item_key(parent: &Id, candidate: &Id) -> Option<ItemKey> {
    let prefix = format!("{}/c:", parent.as_str());
    if candidate.as_str().starts_with(&prefix) {
        let rest = &candidate.as_str()[prefix.len()..];
        if let Some((_len_str, val_str)) = rest.split_once(':') {
            // Strip any further sub-paths (e.g. /s:...)
            let key_str = match val_str.split_once('/') {
                Some((k_str, _)) => k_str,
                None => val_str,
            };
            if let Ok(k) = key_str.parse::<u64>() {
                return Some(ItemKey::new(k));
            }
        }
    }
    None
}

/// Activation behavior for branch nodes in a hierarchical tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BranchActivation {
    /// Primary activation (e.g. Enter / Click) toggles expansion.
    #[default]
    Toggle,
    /// Primary activation emits an Activate action without expanding.
    Activate,
}

/// A borrowed hierarchical node in a `TreeSource`.
#[derive(Debug, Clone)]
pub struct TreeNode<'a> {
    pub key: ItemKey,
    pub label: &'a str,
    pub children: &'a [ItemKey],
    pub expanded: bool,
    pub leaf: bool,
    pub icon: Option<&'a str>,
    pub meta: Option<&'a str>,
    pub disabled: bool,
    pub busy: bool,
}

impl<'a> TreeNode<'a> {
    pub fn new(key: ItemKey, label: &'a str, children: &'a [ItemKey]) -> Self {
        Self {
            key,
            label,
            children,
            expanded: false,
            leaf: children.is_empty(),
            icon: None,
            meta: None,
            disabled: false,
            busy: false,
        }
    }

    pub fn leaf(mut self, leaf: bool) -> Self {
        self.leaf = leaf;
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn icon(mut self, icon: Option<&'a str>) -> Self {
        self.icon = icon;
        self
    }

    pub fn meta(mut self, meta: Option<&'a str>) -> Self {
        self.meta = meta;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }
}

impl<'a> Keyed for TreeNode<'a> {
    fn key(&self) -> ItemKey {
        self.key
    }
}

/// Read-only hierarchical source for Tree components.
pub trait TreeSource {
    fn revision(&self) -> Revision;
    fn roots(&self) -> &[ItemKey];
    fn node(&self, key: ItemKey) -> Option<TreeNode<'_>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct MockRow {
        key: ItemKey,
    }

    impl Keyed for MockRow {
        fn key(&self) -> ItemKey {
            self.key
        }
    }

    #[test]
    fn test_duplicate_keys_fail_validation() {
        let valid = vec![
            MockRow {
                key: ItemKey::new(1),
            },
            MockRow {
                key: ItemKey::new(2),
            },
        ];
        assert!(validate_unique_keys(&valid).is_ok());

        let invalid = vec![
            MockRow {
                key: ItemKey::new(1),
            },
            MockRow {
                key: ItemKey::new(2),
            },
            MockRow {
                key: ItemKey::new(1),
            },
        ];
        assert!(matches!(
            validate_unique_keys(&invalid),
            Err(CollectionError::DuplicateKey(k)) if k == ItemKey::new(1)
        ));
    }

    #[test]
    fn test_reconciliation_fallback_successor_then_predecessor() {
        let prior = vec![
            ItemKey::new(10),
            ItemKey::new(20),
            ItemKey::new(30),
            ItemKey::new(40),
        ];

        // Surviving key retained
        let surviving = vec![ItemKey::new(10), ItemKey::new(20), ItemKey::new(40)];
        assert_eq!(
            reconcile_cursor_with_fallback(Some(ItemKey::new(20)), &prior, &surviving),
            Some(ItemKey::new(20))
        );

        // Disappeared key falls back to nearest successor (30 disappeared -> 40)
        let removed_30 = vec![ItemKey::new(10), ItemKey::new(20), ItemKey::new(40)];
        assert_eq!(
            reconcile_cursor_with_fallback(Some(ItemKey::new(30)), &prior, &removed_30),
            Some(ItemKey::new(40))
        );

        // If successor also gone, fallback to predecessor (40 disappeared -> 30)
        let removed_40 = vec![ItemKey::new(10), ItemKey::new(20), ItemKey::new(30)];
        assert_eq!(
            reconcile_cursor_with_fallback(Some(ItemKey::new(40)), &prior, &removed_40),
            Some(ItemKey::new(30))
        );
    }

    #[test]
    fn test_child_item_key_extraction() {
        let parent = Id::new("my_list");
        let child = parent.child(ItemKey::new(12345));
        assert_eq!(child_item_key(&parent, &child), Some(ItemKey::new(12345)));

        let sub_child = child.sub("disclosure");
        assert_eq!(
            child_item_key(&parent, &sub_child),
            Some(ItemKey::new(12345))
        );

        let other = Id::new("other_list").child(ItemKey::new(999));
        assert_eq!(child_item_key(&parent, &other), None);
    }
}
