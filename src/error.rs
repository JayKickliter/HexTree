use thiserror::Error;

/// Result type for this crate
pub type Result<T = ()> = std::result::Result<T, Error>;

/// Error type for this crate.
#[derive(Error, Debug)]
pub enum Error {
    /// An invalid raw source value was used for an H3 cell.
    #[error("raw u64 does not a valid H3 index: {0}")]
    Index(u64),
}
