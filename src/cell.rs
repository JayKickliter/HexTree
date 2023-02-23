use crate::{Error, Result};
use std::{convert::TryFrom, fmt};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde-support",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct Index(u64);

#[allow(dead_code)]
impl Index {
    pub const fn reserved(self) -> bool {
        self.0 >> 0x3F == 1
    }

    pub const fn mode(self) -> u8 {
        (self.0 >> 0x3B) as u8 & 0b1111
    }

    pub const fn mode_dep(self) -> u8 {
        (self.0 >> 0x38) as u8 & 0b111
    }

    pub const fn res(self) -> u8 {
        let res = (self.0 >> 0x34) as u8 & 0b1111;
        debug_assert!(res < 16);
        res
    }

    #[must_use]
    pub const fn set_res(self, res: u8) -> Self {
        debug_assert!(res < 16);
        let mask = 0b1111 << 0x34;
        let masked_index = self.0 & !mask;
        let shifted_res = ((res & 0b1111) as u64) << 0x34;
        Self(masked_index | shifted_res)
    }

    pub const fn base(self) -> u8 {
        let base = (self.0 >> 0x2D) as u8 & 0b111_1111;
        debug_assert!(base < 122);
        base
    }

    pub const fn digit(self, res: u8) -> Option<u8> {
        debug_assert!(res < 16);
        if res == 0 || res > 15 {
            None
        } else {
            Some(((self.0 >> ((15 - res) * 3)) as u8) & 0b111)
        }
    }
}

/// [HexTreeMap][crate::HexTreeMap]'s key type.
#[derive(Copy, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde-support",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct Cell(pub(crate) u64);

impl Cell {
    /// Constructs a new Cell from a raw [u64] H3 index.
    ///
    /// # Errors
    ///
    /// Returns an error if u64 is not a valid [bit-representation] of
    /// an H3 cell (mode 1 H3 index).
    ///
    /// [bit-representation]: https://h3geo.org/docs/core-library/h3Indexing/
    pub const fn from_raw(raw: u64) -> Result<Self> {
        let idx = Index(raw);
        if
        // reserved must be 0
        !idx.reserved() &&
        // we only care about mode 1 (cell) indicies
        idx.mode() == 1 &&
        // there are only 122 base cells
        idx.base() < 122
        {
            Ok(Cell(idx.0))
        } else {
            Err(Error::Index(raw))
        }
    }

    /// Returns the raw [u64] H3 index for this cell.
    pub const fn into_raw(self) -> u64 {
        self.0
    }

    /// Returns this cell's parent at the specified resolution.
    ///
    /// Returns Some if `res` is less-than or equal-to this cell's
    /// resolution, otherwise returns None.
    pub const fn to_parent(&self, res: u8) -> Option<Self> {
        match self.res() {
            v if v < res => None,
            v if v == res => Some(*self),
            _ => {
                let idx = Index(self.0);
                let idx = idx.set_res(res);
                let lower_bits = u64::MAX >> (64 - (15 - res) * 3);
                let raw = idx.0 | lower_bits;
                Some(Cell(raw))
            }
        }
    }

    /// Returns this cell's base (res-0 parent).
    pub const fn base(&self) -> u8 {
        let base = Index(self.0).base();
        debug_assert!(base < 122, "valid base indices are [0,122]");
        base
    }

    /// Returns this cell's resolution.
    pub const fn res(&self) -> u8 {
        Index(self.0).res()
    }
}

impl TryFrom<u64> for Cell {
    type Error = Error;

    fn try_from(raw: u64) -> Result<Cell> {
        Cell::from_raw(raw)
    }
}

impl fmt::Debug for Cell {
    /// [H3 Index](https://h3geo.org/docs/core-library/h3Indexing/):
    /// > The canonical string representation of an H3Index is the
    /// > hexadecimal representation of the integer, using lowercase
    /// > letters. The string representation is variable length (no zero
    /// > padding) and is not prefixed or suffixed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        write!(f, "{:0x}", self.0)
    }
}

impl fmt::Display for Cell {
    /// [H3 Index](https://h3geo.org/docs/core-library/h3Indexing/):
    /// > The canonical string representation of an H3Index is the
    /// > hexadecimal representation of the integer, using lowercase
    /// > letters. The string representation is variable length (no zero
    /// > padding) and is not prefixed or suffixed.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        write!(f, "{:x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_bitfields() {
        let idx = Index(0x85283473fffffff);
        assert!(!idx.reserved());
        assert_eq!(idx.mode(), 1);
        assert_eq!(idx.mode_dep(), 0);
        assert_eq!(idx.res(), 5);
        assert_eq!(idx.base(), 20);
        assert_eq!(idx.digit(1), Some(0));
        assert_eq!(idx.digit(2), Some(6));
        assert_eq!(idx.digit(3), Some(4));
        assert_eq!(idx.digit(4), Some(3));
        assert_eq!(idx.digit(5), Some(4));
        assert_eq!(idx.digit(6), Some(7));
        assert_eq!(idx.digit(7), Some(7));
        assert_eq!(idx.digit(8), Some(7));
        assert_eq!(idx.digit(9), Some(7));
        assert_eq!(idx.digit(10), Some(7));
        assert_eq!(idx.digit(11), Some(7));
        assert_eq!(idx.digit(12), Some(7));
        assert_eq!(idx.digit(13), Some(7));
        assert_eq!(idx.digit(14), Some(7));
        assert_eq!(idx.digit(15), Some(7));
    }

    #[test]
    fn test_cell_to_parent() {
        let cell = Cell::try_from(0x85283473fffffff).unwrap();
        let parent = cell.to_parent(cell.res()).unwrap();
        assert_eq!(cell, parent);
        let parent = cell.to_parent(4).unwrap();
        let parent_idx = Index(parent.0);
        assert_eq!(parent.res(), 4);
        assert_eq!(parent_idx.digit(5), Some(7));
        assert_eq!(parent_idx.digit(4), Some(3));
        let parent = cell.to_parent(0).unwrap();
        let parent_idx = Index(parent.0);
        assert_eq!(parent_idx.digit(4), Some(7));
        assert_eq!(parent_idx.digit(3), Some(7));
        assert_eq!(parent_idx.digit(2), Some(7));
        assert_eq!(parent_idx.digit(1), Some(7));
        assert_eq!(parent_idx.base(), 20);
    }
}
