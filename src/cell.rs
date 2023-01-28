#![allow(missing_docs)]
use crate::{Error, Result};
use std::{convert::TryFrom, fmt};

bitfield::bitfield! {
    /// An H3 index.
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[cfg_attr(
        feature = "serde-support",
        derive(serde::Serialize, serde::Deserialize),
        serde(transparent)
    )]
    pub struct Index(u64);
    pub reserved,       _              : 63;
    u8; pub mode,       _              : 62, 59;
    u8; pub mode_dep,   set_mode_dep   : 58, 56;
    u8; pub resolution, set_resolution : 55, 52;
    u8; pub base_cell,  set_base_cell  : 51, 45;
    u8; pub res1digit,  set_res1digit  : 44, 42;
    u8; pub res2digit,  set_res2digit  : 41, 39;
    u8; pub res3digit,  set_res3digit  : 38, 36;
    u8; pub res4digit,  set_res4digit  : 35, 33;
    u8; pub res5digit,  set_res5digit  : 32, 30;
    u8; pub res6digit,  set_res6digit  : 29, 27;
    u8; pub res7digit,  set_res7digit  : 26, 24;
    u8; pub res8digit,  set_res8digit  : 23, 21;
    u8; pub res9digit,  set_res9digit  : 20, 18;
    u8; pub res10digit, set_res10digit : 17, 15;
    u8; pub res11digit, set_res11digit : 14, 12;
    u8; pub res12digit, set_res12digit : 11,  9;
    u8; pub res13digit, set_res13digit :  8,  6;
    u8; pub res14digit, set_res14digit :  5,  3;
    u8; pub res15digit, set_res15digit :  2,  0;
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde-support",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct Cell(pub(crate) u64);

impl Cell {
    pub fn from_raw(raw: u64) -> Result<Self> {
        let idx = Index(raw);
        if
        // reserved must be 0
        !idx.reserved() &&
        // we only care about mode 1 (cell) indicies
        idx.mode() == 1 &&
        // there are only 122 base cells
        idx.base_cell() < 122
        {
            Ok(Cell(idx.0))
        } else {
            Err(Error::Invalid(raw))
        }
    }

    pub fn into_raw(self) -> u64 {
        self.0
    }

    pub fn to_parent(&self, res: u8) -> Option<Self> {
        match self.res() {
            v if v < res => None,
            v if v == res => Some(*self),
            _ => {
                let mut idx = Index(self.0);
                idx.set_resolution(res);
                let lower_bits = u64::MAX >> (64 - (15 - res) * 3);
                let raw = idx.0 | lower_bits;
                Some(Cell(raw))
            }
        }
    }

    pub fn base(&self) -> u8 {
        let base = Index(self.0).base_cell();
        debug_assert!(base < 122, "valid base indices are [0,122]");
        base
    }

    pub fn res(&self) -> u8 {
        Index(self.0).resolution()
    }
}

impl TryFrom<u64> for Cell {
    type Error = Error;

    fn try_from(raw: u64) -> Result<Cell> {
        Cell::from_raw(raw)
    }
}

impl fmt::Debug for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::result::Result<(), fmt::Error> {
        write!(f, "{:0x}", self.0)
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
        assert_eq!(idx.resolution(), 5);
        assert_eq!(idx.base_cell(), 20);
        assert_eq!(idx.res1digit(), 0);
        assert_eq!(idx.res2digit(), 6);
        assert_eq!(idx.res3digit(), 4);
        assert_eq!(idx.res4digit(), 3);
        assert_eq!(idx.res5digit(), 4);
        assert_eq!(idx.res6digit(), 7);
        assert_eq!(idx.res7digit(), 7);
        assert_eq!(idx.res8digit(), 7);
        assert_eq!(idx.res9digit(), 7);
        assert_eq!(idx.res10digit(), 7);
        assert_eq!(idx.res11digit(), 7);
        assert_eq!(idx.res12digit(), 7);
        assert_eq!(idx.res13digit(), 7);
        assert_eq!(idx.res14digit(), 7);
        assert_eq!(idx.res15digit(), 7);
    }

    #[test]
    fn test_cell_to_parent() {
        let cell = Cell::try_from(0x85283473fffffff).unwrap();
        let parent = cell.to_parent(cell.res()).unwrap();
        assert_eq!(cell, parent);
        let parent = cell.to_parent(4).unwrap();
        let parent_idx = Index(parent.0);
        assert_eq!(parent.res(), 4);
        assert_eq!(parent_idx.res5digit(), 7);
        assert_eq!(parent_idx.res4digit(), 3);
        let parent = cell.to_parent(0).unwrap();
        let parent_idx = Index(parent.0);
        assert_eq!(parent_idx.res4digit(), 7);
        assert_eq!(parent_idx.res3digit(), 7);
        assert_eq!(parent_idx.res2digit(), 7);
        assert_eq!(parent_idx.res1digit(), 7);
        assert_eq!(parent_idx.base_cell(), 20);
    }
}
