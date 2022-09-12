#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! `hextree` provides structures that can represent geographic regions as H3 cells.
//!
//! The primary structures are:
//! - HexMap: a region to value map.
//! - HexSet: a region without values.
//!
//! You can think of `HexMap` vs. `HexSet` as [`HashMap`] vs. [`HashSet`].
//!
//! [`HashMap`]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
//! [`HashSet`]: https://doc.rust-lang.org/std/collections/struct.HashSet.html
//!
//! # Features
//!
//! * **`serde-support`**: support for serialization via [serde].
//!
//! [serde]: https://docs.rs/serde/latest/serde

pub mod compaction;
mod digits;
mod hexmap;
mod hexset;
mod node;

pub use crate::hexmap::HexMap;
pub use crate::hexset::HexSet;
pub use h3ron;
