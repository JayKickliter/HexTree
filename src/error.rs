/// Result type for this crate
pub type Result<T = ()> = std::result::Result<T, Error>;

/// Error type for this crate.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// An invalid raw source value was used for an H3 cell.
    Index(u64),

    /// An io error.
    #[cfg(feature = "disktree")]
    Io(std::io::Error),

    /// Not a disktree.
    #[cfg(feature = "disktree")]
    NotDisktree,

    /// Unsupported version.
    #[cfg(feature = "disktree")]
    Version(u8),

    /// Invalid value tag found in disktree.
    #[cfg(feature = "disktree")]
    InvalidTag(u8, u64),

    /// Invalid value size bytes found in disktree header.
    #[cfg(feature = "disktree")]
    Varint(u32),

    /// User-provided serializer failed.
    #[cfg(feature = "disktree")]
    Writer(Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(feature = "disktree")]
impl std::convert::From<std::io::Error> for Error {
    fn from(other: std::io::Error) -> Self {
        Error::Io(other)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Index(_) => None,

            #[cfg(feature = "disktree")]
            Error::Io(inner) => inner.source(),

            #[cfg(feature = "disktree")]
            Error::NotDisktree => None,

            #[cfg(feature = "disktree")]
            Error::Version(_) => None,

            #[cfg(feature = "disktree")]
            Error::InvalidTag(_, _) => None,

            #[cfg(feature = "disktree")]
            Error::Varint(_) => None,

            #[cfg(feature = "disktree")]
            Error::Writer(inner) => inner.source(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Index(bits) => write!(f, "raw u64 is not a valid H3 index: {bits}"),

            #[cfg(feature = "disktree")]
            Error::Io(io_error) => io_error.fmt(f),

            #[cfg(feature = "disktree")]
            Error::NotDisktree => {
                write!(f, "file missing magic header")
            }

            #[cfg(feature = "disktree")]
            Error::Version(version) => {
                write!(f, "unsupported version, got {version}")
            }

            #[cfg(feature = "disktree")]
            Error::InvalidTag(tag, pos) => {
                write!(f, "invalid tag, got {tag}, pos {pos}")
            }

            #[cfg(feature = "disktree")]
            Error::Varint(val) => {
                write!(f, "byte sequence is not a valid varint, got {val}")
            }

            #[cfg(feature = "disktree")]
            Error::Writer(writer_error) => {
                write!(f, "provided writer returned an error, got {writer_error}")
            }
        }
    }
}
