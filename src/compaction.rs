//! User-pluggable compaction strategies.
//!
//! Compaction allows the tree to automatically coalesce child cells into
//! their parent when certain conditions are met, reducing memory usage
//! and improving query performance.

use crate::Cell;

/// A user-provided compactor.
///
/// The compactor trait allows you to customize compaction behavior after
/// calling `insert` on a tree.
pub trait Compactor<V> {
    /// Called after every insert into a non-leaf node.
    ///
    /// Given an intermediate (non-leaf) node's cell and up to 7
    /// children, you can choose to leave the node alone by returning
    /// `None`, or turn it into a leaf node by returning `Some(value)`.
    fn compact(&mut self, cell: Cell, children: [Option<&V>; 7]) -> Option<V>;
}

/// A compactor that performs no compaction.
///
/// This is the default compactor and leaves all inserted cells as-is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NullCompactor;

impl<V> Compactor<V> for NullCompactor {
    fn compact(&mut self, _cell: Cell, _children: [Option<&V>; 7]) -> Option<V> {
        None
    }
}

/// A compactor that coalesces nodes when all 7 children are present.
///
/// This is typically used with `HexTreeSet` (where values are `()`).
/// When all 7 children of a node are complete, they are replaced with
/// a single parent cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetCompactor;

impl Compactor<()> for SetCompactor {
    fn compact(&mut self, _cell: Cell, children: [Option<&()>; 7]) -> Option<()> {
        if children.iter().all(Option::is_some) {
            Some(())
        } else {
            None
        }
    }
}

/// A compactor that coalesces nodes when all 7 children have equal values.
///
/// When all 7 children of a node are present and have the same value,
/// they are replaced with a single parent cell containing that value.
/// This is useful for maps where large contiguous regions share the same value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EqCompactor;

impl<V: PartialEq + Clone> Compactor<V> for EqCompactor {
    fn compact(&mut self, _cell: Cell, children: [Option<&V>; 7]) -> Option<V> {
        if let [Some(v0), Some(v1), Some(v2), Some(v3), Some(v4), Some(v5), Some(v6)] = children {
            if v0 == v1 && v1 == v2 && v2 == v3 && v3 == v4 && v4 == v5 && v5 == v6 {
                return Some(v0.clone());
            }
        };
        None
    }
}
