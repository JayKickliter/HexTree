use crate::{
    compaction::Compactor,
    digits::{base, Digits},
    h3ron::H3Cell,
    node::Node,
};
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

impl<V> HexMap<V> {
    /// Constructs a new, empty `HexMap`.
    ///
    /// Incurs a single heap allocation to store all 122 resolution-0
    /// H3 cells.
    pub fn new() -> Self {
        Self {
            nodes: std::iter::repeat_with(|| None)
                .take(122)
                .collect::<Vec<Option<Node<V>>>>()
                .into_boxed_slice(),
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

    /// Adds a hexagon to the set and run provided compaction routine.
    pub fn insert_and_compact<C>(&mut self, hex: H3Cell, value: V, mut compactor: C)
    where
        C: Compactor<V>,
    {
        let base_cell = base(&hex);
        let digits = Digits::new(hex);
        match self.nodes[base_cell as usize].as_mut() {
            Some(node) => node.insert_and_compact(0, digits, value, &mut compactor),
            None => {
                let mut node = Node::new();
                node.insert_and_compact(0, digits, value, &mut compactor);
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

impl<V: PartialEq> Default for HexMap<V> {
    fn default() -> Self {
        HexMap::new()
    }
}

impl<V> FromIterator<(H3Cell, V)> for HexMap<V> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (H3Cell, V)>,
    {
        let mut map = HexMap::new();
        for (cell, value) in iter {
            map.insert(cell, value);
        }
        map
    }
}

impl<'a, V: Copy + 'a> FromIterator<(&'a H3Cell, &'a V)> for HexMap<V> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (&'a H3Cell, &'a V)>,
    {
        let mut map = HexMap::new();
        for (cell, value) in iter {
            map.insert(*cell, *value);
        }
        map
    }
}

impl<V, C: Compactor<V>> FromIterator<(H3Cell, V, C)> for HexMap<V> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (H3Cell, V, C)>,
    {
        let mut map = HexMap::new();
        for (cell, value, compactor) in iter {
            map.insert_and_compact(cell, value, compactor);
        }
        map
    }
}

impl<'a, V: Copy + 'a, C: Compactor<V> + Copy + 'a> FromIterator<(&'a H3Cell, &'a V, &'a C)>
    for HexMap<V>
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (&'a H3Cell, &'a V, &'a C)>,
    {
        let mut map = HexMap::new();
        for (cell, value, compactor) in iter {
            map.insert_and_compact(*cell, *value, *compactor);
        }
        map
    }
}
