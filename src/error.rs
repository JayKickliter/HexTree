/// Result type for this crate
pub type Result<T = ()> = std::result::Result<T, Error>;

/// Error type for this crate.
#[derive(Debug)]
pub enum Error {
    /// An invalid raw source value was used for an H3 cell.
    Index(u64),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Index(bits) => write!(f, "raw u64 is not a valid H3 index: {bits}"),
        }
    }
}
