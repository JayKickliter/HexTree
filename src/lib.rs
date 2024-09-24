#![deny(unsafe_code, missing_docs, rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(doctest), doc = include_str!("../README.md"))]

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
