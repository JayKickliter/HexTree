/// Result type for this crate
pub type Result<T = ()> = std::result::Result<T, Error>;

/// Error type for this crate.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An invalid raw source value was used for an H3 cell.
    Index(u64),

    /// An io error.
    #[cfg(feature = "hexdb")]
    Io(std::io::Error),

    /// Not a hexdb.
    #[cfg(feature = "hexdb")]
    NotHexDb,

    /// Unsupported version.
    #[cfg(feature = "hexdb")]
    Version(u8),

    /// Invalid value tag found in hexdb.
    #[cfg(feature = "hexdb")]
    InvalidTag(u8, u64),

    /// Invalid value size bytes found in hexdb header.
    #[cfg(feature = "hexdb")]
    Varint(u32),

    /// User-provided serializer failed.
    #[cfg(feature = "hexdb")]
    Writer(Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(feature = "hexdb")]
impl std::convert::From<std::io::Error> for Error {
    fn from(other: std::io::Error) -> Self {
        Error::Io(other)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Index(_) => None,

            #[cfg(feature = "hexdb")]
            Error::Io(inner) => inner.source(),

            #[cfg(feature = "hexdb")]
            Error::NotHexDb => None,

            #[cfg(feature = "hexdb")]
            Error::Version(_) => None,

            #[cfg(feature = "hexdb")]
            Error::InvalidTag(_, _) => None,

            #[cfg(feature = "hexdb")]
            Error::Varint(_) => None,

            #[cfg(feature = "hexdb")]
            Error::Writer(inner) => inner.source(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Index(bits) => write!(f, "raw u64 is not a valid H3 index: {bits}"),

            #[cfg(feature = "hexdb")]
            Error::Io(io_error) => io_error.fmt(f),

            #[cfg(feature = "hexdb")]
            Error::NotHexDb => {
                write!(f, "file missing magic header")
            }

            #[cfg(feature = "hexdb")]
            Error::Version(version) => {
                write!(f, "unsupported version, got {version}")
            }

            #[cfg(feature = "hexdb")]
            Error::InvalidTag(tag, pos) => {
                write!(f, "invalid tag, got {tag}, pos {pos}")
            }

            #[cfg(feature = "hexdb")]
            Error::Varint(val) => {
                write!(f, "byte sequence is not a valid varint, got {val}")
            }

            #[cfg(feature = "hexdb")]
            Error::Writer(writer_error) => {
                write!(f, "provided writer returned an error, got {writer_error}")
            }
        }
    }
}
