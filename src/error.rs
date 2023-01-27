use thiserror::Error;

/// Result type for this crate
pub type Result<T = ()> = std::result::Result<T, Error>;

/// Errors for this crate.
#[derive(Error, Debug)]
pub enum Error {
    /// An invalid raw source falue was used for an h3 cell
    #[error("invalid raw h3 value: {0}")]
    Invalid(u64),
}
