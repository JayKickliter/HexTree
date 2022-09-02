#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

//! A HexMap is a structure for representing geographical regions and
//! efficiently testing performing hit-tests on that region. Or, in
//! other words: I have a region defined; does it contain this
//! point on earth?
//!
//! # Features
//!
//! * **`serde-support`**: support for \[de\]serializing a HexMap via [serde].
//!
//! [serde]: https://docs.rs/serde/latest/serde/
//!
//! # Usage
//!
//! <iframe src="https://www.google.com/maps/d/u/0/embed?mid=1Ty1LhqAipSTl6lsXH7YAjE6kdNmEvCw&ehbc=2E312F" width="640" height="480"></iframe>
//!
//! ----
//!
//! Let's create a HexMap for Monaco as visualized in the map
//!
//! ```
//! # use hexset::h3ron::Error;
//! #
//! # fn main() -> Result<(), Error> {
//! use geo_types::coord;
//! use hexset::{h3ron::H3Cell, HexSet};
//! #
//! #    use byteorder::{LittleEndian as LE, ReadBytesExt};
//! #    use hexset::h3ron::FromH3Index;
//! #    let idx_bytes = include_bytes!("../assets//monaco.res12.h3idx");
//! #    let rdr = &mut idx_bytes.as_slice();
//! #    let mut cells = Vec::new();
//! #    while let Ok(idx) = rdr.read_u64::<LE>() {
//! #        cells.push(H3Cell::from_h3index(idx));
//! #    }
//!
//! // `cells` is a slice of `H3Cell`s
//! let monaco: HexSet = cells.iter().collect();
//!
//! // You can see in the map above that our set covers Point 1 (green
//! // check) but not Point 2 (red x), let's test that.
//! let point_1 = H3Cell::from_coordinate(coord! {x: 7.42418, y: 43.73631}, 12)?;
//! let point_2 = H3Cell::from_coordinate(coord! {x: 7.42855, y: 43.73008}, 12)?;
//!
//! assert!(monaco.contains(&point_1));
//! assert!(!monaco.contains(&point_2));
//!
//! #     Ok(())
//! # }
//! ```

pub use h3ron;
use h3ron::{H3Cell, Index};
use std::{cmp::PartialEq, iter::FromIterator, mem};

/// An efficient way to represent any portion(s) of Earth as a set of
/// `H3` hexagons.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde-support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct HexMap<V> {
    /// All h3 0 base cell indices in the tree
    nodes: Box<[Option<Node<V>>]>,
}

/// A HexSet is HexMap with no value.
pub type HexSet = HexMap<()>;

impl<V: Clone + PartialEq> HexMap<V> {
    /// Constructs a new, empty `HexMap`.
    ///
    /// Incurs a single heap allocation to store all 122 resolution-0
    /// H3 cells.
    pub fn new() -> Self {
        Self {
            nodes: vec![None; 122].into_boxed_slice(),
        }
    }

    /// Returns the number of H3 cells in the set.
    ///
    /// This method only considers complete, or leaf, hexagons in the
    /// set. Due to automatic compaction, this number may be
    /// significantly smaller than the number of source cells used to
    /// create the set.
    pub fn len(&self) -> usize {
        self.nodes.iter().flatten().map(|node| node.len()).sum()
    }

    /// Returns `true` if the set contains no cells.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Adds a hexagon to the set.
    pub fn insert(&mut self, hex: H3Cell, value: V) {
        let base_cell = base(&hex);
        let digits = Digits::new(hex);
        match self.nodes[base_cell as usize].as_mut() {
            Some(node) => node.insert(digits, value),
            None => {
                let mut node = Node::new();
                node.insert(digits, value);
                self.nodes[base_cell as usize] = Some(node);
            }
        }
    }

    /// Returns `true` if the set fully contains `hex`.
    ///
    /// This method will return `true` if any of the following are
    /// true:
    ///
    /// 1. There was an earlier [insert][Self::insert] call with
    ///    precisely this target hex.
    /// 2. Several previously inserted hexagons coalesced into
    ///    precisely this target hex.
    /// 3. The set contains a complete (leaf) parent of this target
    ///    hex due to 1 or 2.
    pub fn contains(&self, hex: &H3Cell) -> bool {
        let base_cell = base(hex);
        match self.nodes[base_cell as usize].as_ref() {
            Some(node) => {
                let digits = Digits::new(*hex);
                node.contains(digits)
            }
            None => false,
        }
    }

    /// Returns a reference to the value corresponding to the given hex.
    pub fn get(&self, hex: &H3Cell) -> Option<&V> {
        let base_cell = base(hex);
        match self.nodes[base_cell as usize].as_ref() {
            Some(node) => {
                let digits = Digits::new(*hex);
                node.get(digits)
            }
            None => None,
        }
    }

    /// Returns the current memory use of this set.
    ///
    /// Note: The actual total may be higher than reported due to
    ///       memory alignment.
    pub fn mem_size(&self) -> usize {
        mem::size_of::<Self>()
            + self
                .nodes
                .iter()
                .flatten()
                .map(|n| n.mem_size())
                .sum::<usize>()
    }
}

impl FromIterator<H3Cell> for HexMap<()> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = H3Cell>,
    {
        let mut set = HexMap::new();
        for cell in iter {
            set.insert(cell, ());
        }
        set
    }
}

impl<'a> FromIterator<&'a H3Cell> for HexMap<()> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a H3Cell>,
    {
        let mut set = HexMap::new();
        for cell in iter {
            set.insert(*cell, ());
        }
        set
    }
}

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde-support",
    derive(serde::Serialize, serde::Deserialize)
)]
enum Node<V> {
    Parent([Option<Box<Node<V>>>; 7]),
    Leaf(V),
}

impl<V: Clone + PartialEq> Node<V> {
    fn mem_size(&self) -> usize {
        mem::size_of::<Self>()
            + match self {
                Self::Leaf(_) => 0,
                Self::Parent(children) => children
                    .iter()
                    .flatten()
                    .map(|n| n.mem_size())
                    .sum::<usize>(),
            }
    }

    fn new() -> Self {
        Self::Parent([None, None, None, None, None, None, None])
    }

    fn len(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Parent(children) => children.iter().flatten().map(|child| child.len()).sum(),
        }
    }

    fn insert(&mut self, mut digits: Digits, value: V) {
        match digits.next() {
            None => *self = Self::Leaf(value),
            Some(digit) => match self {
                Self::Leaf(_) => return,
                Self::Parent(children) => {
                    match children[digit as usize].as_mut() {
                        Some(node) => node.insert(digits, value),
                        None => {
                            let mut node = Node::new();
                            node.insert(digits, value);
                            children[digit as usize] = Some(Box::new(node));
                        }
                    };
                }
            },
        };
        self.coalesce();
    }

    fn coalesce(&mut self) {
        if let Self::Parent(
            [Some(n0), Some(n1), Some(n2), Some(n3), Some(n4), Some(n5), Some(n6)],
        ) = self
        {
            match (
                n0.value(),
                n1.value(),
                n2.value(),
                n3.value(),
                n4.value(),
                n5.value(),
                n6.value(),
            ) {
                (Some(v0), Some(v1), Some(v2), Some(v3), Some(v4), Some(v5), Some(v6))
                    if v0 == v1 && v1 == v2 && v2 == v3 && v3 == v4 && v4 == v5 && v5 == v6 =>
                {
                    *self = Self::Leaf(v0.clone())
                }
                _ => (),
            }
        };
    }

    fn is_full(&self) -> bool {
        matches!(self, Self::Leaf(_))
    }

    fn value(&self) -> Option<&V> {
        match self {
            Self::Leaf(value) => Some(value),
            _ => None,
        }
    }

    #[inline]
    fn contains(&self, mut digits: Digits) -> bool {
        if self.is_full() {
            return true;
        }

        match (digits.next(), self) {
            (_, Self::Leaf(_)) => true,
            (Some(digit), Self::Parent(children)) => {
                // TODO check if this node is "full"
                match &children.as_slice()[digit as usize] {
                    Some(node) => node.contains(digits),
                    None => false,
                }
            }
            // No digits left, but `self` isn't full, so this hex
            // can't fully contain the target.
            (None, Self::Parent(_)) => false,
        }
    }

    fn get(&self, mut digits: Digits) -> Option<&V> {
        if let Self::Leaf(val) = self {
            return Some(val);
        }

        match (digits.next(), self) {
            (_, Self::Leaf(_)) => unreachable!(),
            (Some(digit), Self::Parent(children)) => {
                // TODO check if this node is "full"
                match &children.as_slice()[digit as usize] {
                    Some(node) => node.get(digits),
                    None => None,
                }
            }
            // No digits left, but `self` isn't full, so this hex
            // can't fully contain the target.
            (None, Self::Parent(_)) => None,
        }
    }
}

struct Digits {
    digits: u64,
    remaining: u8,
}

impl Digits {
    fn new(cell: H3Cell) -> Self {
        let res = cell.resolution();
        let mask = u128::MAX.wrapping_shl(64 - (3 * res as u32)) as u64;
        let digits: u64 = cell.h3index().wrapping_shl(19) & mask;
        Self {
            digits,
            remaining: res,
        }
    }
}

impl Iterator for Digits {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            let out = (self.digits & (0b111 << 61)) >> 61;
            self.digits <<= 3;
            debug_assert!(out < 7);
            self.remaining -= 1;
            Some(out as u8)
        }
    }
}

impl<V: Clone + PartialEq> Default for HexMap<V> {
    fn default() -> Self {
        HexMap::new()
    }
}

/// Returns a cell's base.
fn base(cell: &H3Cell) -> u8 {
    let index = cell.h3index();
    let base = (index >> 0x2D) & 0b111_1111;
    base as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digits() {
        let test_cases: &[(u64, &[u8])] = &[
            (577164439745200127, &[]),                    // res 0
            (585793956755800063, &[2, 0]),                // res 2
            (592638622797135871, &[6, 3, 2]),             // res 3
            (596251300178427903, &[3, 6, 6, 2]),          // res 4
            (599803672997658623, &[3, 4, 4, 1, 4]),       // res 5
            (604614882611953663, &[1, 4, 0, 4, 1, 0]),    // res 6
            (608557861265473535, &[2, 0, 2, 3, 2, 1, 1]), // res 7
        ];
        for (index, ref_digits) in test_cases {
            let cell = H3Cell::new(*index);
            let digits = Digits::new(cell).collect::<Vec<u8>>();
            assert_eq!(&&digits, ref_digits);
        }
    }
}
