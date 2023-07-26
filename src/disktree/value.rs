use crate::{error::Result, Cell};
use std::io::Read;

/// The `ReadVal` trait defines the contract for reading concrete
/// types from a disk tree.
pub trait ReadVal<R> {
    /// The associated type `T` represents the result of
    /// deserialization, which may be the deserialized type or an
    /// error result, depending on fallibility.
    type T;

    /// Reads data from the provided reader and returns the
    /// deserialized result.
    ///
    /// # Arguments
    ///
    /// * `rdr` - A `Result` containing a `Cell` and a mutable
    /// reference to the reader.
    ///
    /// # Returns
    ///
    /// The deserialized result, which can be the deserialized type or
    /// an error.
    fn read(&self, rdr: Result<(Cell, &mut R)>) -> Self::T;
}

impl<R, T, F> ReadVal<R> for F
where
    R: Read,
    F: Fn(Result<(Cell, &mut R)>) -> T,
{
    type T = T;

    fn read(&self, rdr: Result<(Cell, &mut R)>) -> T {
        self(rdr)
    }
}
