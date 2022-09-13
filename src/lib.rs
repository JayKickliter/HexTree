#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! hextree provides tree structures that represent geographic regions with H3 cells.
//!
//! The primary structures are:
//! - HexMap: an H3Cell-to-value map.
//! - HexSet: an H3Cell set for hit-testing.
//!
//! You can think of `HexMap` vs. `HexSet` as [`HashMap`] vs. [`HashSet`].
//!
//! For the rest of the documentation, we will use hextree to refer to
//! the general data structure.
//!
//! ## How is this different from `HashMap<H3Cell, V>`?
//!
//! The key feature of a hextree is that its keys (H3 cells) are
//! hierarchical. For instance, if you previously inserted an entry
//! for a low-res hex, but later query for a higher-res child hex, the
//! tree returns the value for the lower res hex. Additionally, with
//! [compaction], trees can automatically coalesce adjacent high-res
//! hexagons into their parent hex. For every large regions, the
//! compaction process _can_ continue to lowest resolution cells
//! (res-0), possibly removing millions of redundant cells from the
//! tree. For example, a set of 4,795,661 res-7 cells representing
//! North America coalesces [into a 42,383 element `HexSet`][us915].
//!
//! A hextree's internal structure exactly matches the semantics of an
//! [H3 cell]. The root of the tree has 122 resolution-0 nodes,
//! followed by 15 levels of 7-ary nodes. The level of an occupied
//! node, or leaf node, is the same as its corresponding H3 cell
//! resolution.
//!
//! ## Features
//!
//! * **`serde-support`**: support for serialization via [serde].
//!
//! [`HashMap`]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
//! [`HashSet`]: https://doc.rust-lang.org/std/collections/struct.HashSet.html
//! [H3 cell]: https://h3geo.org/docs/core-library/h3Indexing
//! [serde]: https://docs.rs/serde/latest/serde
//! [compaction]: crate::compaction
//! [us915]: https://www.google.com/maps/d/u/0/edit?mid=15wRzxmtmyzqf6fHU3yuW4hJAM9MoxLJs

pub mod compaction;
mod digits;
mod hexmap;
mod hexset;
mod node;

pub use crate::hexmap::HexMap;
pub use crate::hexset::HexSet;
pub use h3ron;
