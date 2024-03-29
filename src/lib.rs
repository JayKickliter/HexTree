#![deny(unsafe_code, missing_docs, rustdoc::broken_intra_doc_links)]

//! hextree provides tree structures that represent geographic regions
//! with H3 cells.
//!
//! The primary structures are:
//! - [`HexTreeMap`]: an Cell-to-value map.
//! - [`HexTreeSet`]: a Cell set for hit-testing.
//!
//! You can think of `HexTreeMap` vs. `HexTreeSet` as [`HashMap`] vs. [`HashSet`].
//!
//! For the rest of the documentation, we will use hextree to refer to
//! the general data structure.
//!
//! ## How is this different from `HashMap<Cell, V>`?
//!
//! The key feature of a hextree is that its keys (H3 cells) are
//! hierarchical. For instance, if you previously inserted an entry
//! for a low-res cell, but later query for a higher-res child cell,
//! the tree returns the value for the lower res cell. Additionally,
//! with [compaction], trees can automatically coalesce adjacent
//! high-res cells into their parent cell. For very large regions, the
//! compaction process _can_ continue to lowest resolution cells
//! (res-0), possibly removing millions of redundant cells from the
//! tree. For example, a set of 4,795,661 res-7 cells representing
//! North America coalesces [into a 42,383 element
//! `HexTreeSet`][us915].
//!
//! A hextree's internal structure exactly matches the semantics of an
//! [H3 cell]. The root of the tree has 122 resolution-0 nodes,
//! followed by 15 levels of 7-ary nodes. The level of an occupied
//! node, or leaf node, is the same as its corresponding H3 cell
//! resolution.
//!
//! ## Features
//!
//! * **`serde`**: support for serialization via [serde].
//!
//! [`HashMap`]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
//! [`HashSet`]: https://doc.rust-lang.org/std/collections/struct.HashSet.html
//! [H3 cell]: https://h3geo.org/docs/core-library/h3Indexing
//! [serde]: https://docs.rs/serde/latest/serde
//! [compaction]: crate::compaction
//! [us915]: https://www.google.com/maps/d/u/0/edit?mid=15wRzxmtmyzqf6fHU3yuW4hJAM9MoxLJs

mod cell;
pub mod compaction;
mod digits;
#[cfg(feature = "disktree")]
pub mod disktree;
mod entry;
mod error;
pub mod hex_tree_map;
mod hex_tree_set;
mod iteration;
mod node;

pub use crate::cell::Cell;
pub use crate::hex_tree_map::HexTreeMap;
pub use crate::hex_tree_set::HexTreeSet;
pub use error::{Error, Result};
#[cfg(feature = "disktree")]
pub use memmap;
