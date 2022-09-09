#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! TODO.
//!
//! ```
//! todo!("document crate");
//! ```

pub mod compaction;
mod digits;
mod hexmap;
mod hexset;
mod node;

pub use crate::hexmap::HexMap;
pub use crate::hexset::HexSet;
pub use h3ron;
