//! A HexTreeMap is a structure for mapping geographical regions to values.

pub use crate::entry::{Entry, OccupiedEntry, VacantEntry};
use crate::{
    compaction::{Compactor, NullCompactor},
    digits::Digits,
    index::Index,
    node::Node,
};
use std::{cmp::PartialEq, iter::FromIterator};

/// A HexTreeMap is a structure for mapping geographical regions to
/// values.
///
///
/// [serde]: https://docs.rs/serde/latest/serde/
///
/// # Usage
///
/// <iframe src="https://www.google.com/maps/d/u/0/embed?mid=1Ty1LhqAipSTl6lsXH7YAjE6kdNmEvCw&ehbc=2E312F" width="100%" height="480"></iframe>
///
/// ----
///
/// Let's create a HexTreeMap for Monaco as visualized in the map
///
/// ```
/// # use h3ron::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use geo_types::coord;
/// use hextree::{Index, compaction::EqCompactor, HexTreeMap};
/// use h3ron::H3Cell;
/// #
/// #    use byteorder::{LittleEndian as LE, ReadBytesExt};
/// #    use h3ron::{Index as H3Index, FromH3Index};
/// #    let idx_bytes = include_bytes!("../assets//monaco.res12.h3idx");
/// #    let rdr = &mut idx_bytes.as_slice();
/// #    let mut cells = Vec::new();
/// #    while let Ok(idx) = rdr.read_u64::<LE>() {
/// #        cells.push(Index::from_raw(idx).unwrap());
/// #    }
///
/// #[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// enum Region {
///     Monaco,
/// }
///
/// // Construct map with a compactor that automatically combines
/// // cells with the same save value.
/// let mut monaco = HexTreeMap::with_compactor(EqCompactor);
///
/// // Now extend the map with cells and a region value.
/// monaco.extend(cells.iter().copied().zip(std::iter::repeat(Region::Monaco)));
///
/// // You can see in the map above that our set covers Point 1 (green
/// // check) but not Point 2 (red x), let's test that.
/// let point_1 = H3Cell::from_coordinate(coord! {x: 7.42418, y: 43.73631}, 12)?;
/// let point_2 = H3Cell::from_coordinate(coord! {x: 7.42855, y: 43.73008}, 12)?;
///
/// assert_eq!(monaco.get(Index::from_raw(*point_1).unwrap()), Some(&Region::Monaco));
/// assert_eq!(monaco.get(Index::from_raw(*point_2).unwrap()), None);
///
/// #     Ok(())
/// # }
/// ```
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde-support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct HexTreeMap<V, C = NullCompactor> {
    /// All h3 0 base cell indices in the tree
    nodes: Box<[Option<Box<Node<V>>>]>,
    /// User-provided compator. Defaults to the null compactor.
    compactor: C,
}

impl<V> HexTreeMap<V, NullCompactor> {
    /// Constructs a new, empty `HexTreeMap` with the no-op
    /// `NullCompactor`.
    ///
    /// Incurs a single heap allocation to store all 122 resolution-0
    /// H3 cells.
    pub fn new() -> Self {
        Self {
            nodes: std::iter::repeat_with(|| None)
                .take(122)
                .collect::<Box<[Option<Box<Node<V>>>]>>(),
            compactor: NullCompactor,
        }
    }
}

impl<V, C: Compactor<V>> HexTreeMap<V, C> {
    /// Adds a hexagon/value pair to the set.
    pub fn insert(&mut self, hex: Index, value: V) {
        let base_cell = hex.base_cell();
        let digits = Digits::new(hex);
        match self.nodes[base_cell as usize].as_mut() {
            Some(node) => node.insert(hex, 0_u8, digits, value, &mut self.compactor),
            None => {
                let mut node = Box::new(Node::new(
                    hex.parent(0).expect("any hex can be promoted to res 0"),
                ));
                node.insert(hex, 0_u8, digits, value, &mut self.compactor);
                self.nodes[base_cell as usize] = Some(node);
            }
        }
    }
}

impl<V, C> HexTreeMap<V, C> {
    /// Constructs a new, empty `HexTreeMap` with the provided
    /// [compactor][crate::compaction].
    ///
    /// Incurs a single heap allocation to store all 122 resolution-0
    /// H3 cells.
    pub fn with_compactor(compactor: C) -> Self {
        Self {
            nodes: std::iter::repeat_with(|| None)
                .take(122)
                .collect::<Box<[Option<Box<Node<V>>>]>>(),
            compactor,
        }
    }

    /// Replace the current compactor with the new one, consuming
    /// `self`.
    ///
    /// This method is useful if you want to use one compaction
    /// strategy for creating an initial, then another one for updates
    /// later.
    pub fn replace_compactor<NewC>(self, new_compactor: NewC) -> HexTreeMap<V, NewC> {
        HexTreeMap {
            nodes: self.nodes,
            compactor: new_compactor,
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
    pub fn contains(&self, hex: Index) -> bool {
        let base_cell = hex.base_cell();
        match self.nodes[base_cell as usize].as_ref() {
            Some(node) => {
                let digits = Digits::new(hex);
                node.contains(digits)
            }
            None => false,
        }
    }

    /// Returns a reference to the value corresponding to the given
    /// hex or one of its parents.
    pub fn get(&self, hex: Index) -> Option<&V> {
        let base_cell = hex.base_cell();
        match self.nodes[base_cell as usize].as_ref() {
            Some(node) => {
                let digits = Digits::new(hex);
                node.get(digits)
            }
            None => None,
        }
    }

    /// Returns a reference to the value corresponding to the given
    /// hex or one of its parents.
    pub fn get_mut(&mut self, hex: Index) -> Option<&mut V> {
        let base_cell = hex.base_cell();
        match self.nodes[base_cell as usize].as_mut() {
            Some(node) => {
                let digits = Digits::new(hex);
                node.get_mut(digits)
            }
            None => None,
        }
    }

    /// Gets the entry in the map for the corresponding cell.
    pub fn entry(&'_ mut self, hex: Index) -> Entry<'_, V, C> {
        if self.get(hex).is_none() {
            return Entry::Vacant(VacantEntry { hex, map: self });
        }
        Entry::Occupied(OccupiedEntry {
            hex,
            value: self.get_mut(hex).unwrap(),
        })
    }

    /// An iterator visiting all cell-value pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&Index, &V)> {
        crate::iteration::Iter::new(&self.nodes)
    }

    /// An iterator visiting all cell-value pairs in arbitrary order
    /// with mutable references to the values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Index, &mut V)> {
        crate::iteration::IterMut::new(&mut self.nodes)
    }
}

impl<V: PartialEq> Default for HexTreeMap<V, NullCompactor> {
    fn default() -> Self {
        HexTreeMap::new()
    }
}

impl<V> FromIterator<(Index, V)> for HexTreeMap<V, NullCompactor> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (Index, V)>,
    {
        let mut map = HexTreeMap::new();
        for (cell, value) in iter {
            map.insert(cell, value);
        }
        map
    }
}

impl<'a, V: Copy + 'a> FromIterator<(&'a Index, &'a V)> for HexTreeMap<V, NullCompactor> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (&'a Index, &'a V)>,
    {
        let mut map = HexTreeMap::new();
        for (cell, value) in iter {
            map.insert(*cell, *value);
        }
        map
    }
}

impl<V, C: Compactor<V>> Extend<(Index, V)> for HexTreeMap<V, C> {
    fn extend<I: IntoIterator<Item = (Index, V)>>(&mut self, iter: I) {
        for (cell, val) in iter {
            self.insert(cell, val)
        }
    }
}

impl<'a, V: Copy + 'a, C: Compactor<V>> Extend<(&'a Index, &'a V)> for HexTreeMap<V, C> {
    fn extend<I: IntoIterator<Item = (&'a Index, &'a V)>>(&mut self, iter: I) {
        for (cell, val) in iter {
            self.insert(*cell, *val)
        }
    }
}

impl<V, C> std::ops::Index<Index> for HexTreeMap<V, C> {
    type Output = V;

    /// Returns a reference to the value corresponding to the supplied
    /// key.
    ///
    /// # Examples
    ///
    /// ```
    /// use hextree::{Index, HexTreeMap};
    ///
    /// let mut map = HexTreeMap::new();
    /// let eiffel_tower_res12 = Index::from_raw(0x8c1fb46741ae9ff).unwrap();
    ///
    /// map.insert(eiffel_tower_res12, "France");
    /// assert_eq!(map[eiffel_tower_res12], "France");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the cell is not present in the `HexTreeMap`.
    fn index(&self, cell: Index) -> &V {
        self.get(cell).expect("no entry found for cell")
    }
}

impl<V, C> std::ops::Index<&Index> for HexTreeMap<V, C> {
    type Output = V;

    /// Returns a reference to the value corresponding to the supplied
    /// key.
    ///
    /// # Examples
    ///
    /// ```
    /// use hextree::{Index, HexTreeMap};
    ///
    /// let mut map = HexTreeMap::new();
    /// let eiffel_tower_res12 = Index::from_raw(0x8c1fb46741ae9ff).unwrap();
    ///
    /// map.insert(eiffel_tower_res12, "France");
    /// assert_eq!(map[&eiffel_tower_res12], "France");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the cell is not present in the `HexTreeMap`.
    fn index(&self, cell: &Index) -> &V {
        self.get(*cell).expect("no entry found for cell")
    }
}

impl<V, C> std::ops::IndexMut<Index> for HexTreeMap<V, C> {
    /// Returns a reference to the value corresponding to the supplied
    /// key.
    ///
    /// # Examples
    ///
    /// ```
    /// use hextree::{Index, HexTreeMap};
    ///
    /// let mut map = HexTreeMap::new();
    /// let eiffel_tower_res12 = Index::from_raw(0x8c1fb46741ae9ff).unwrap();
    ///
    /// map.insert(eiffel_tower_res12, "France");
    /// assert_eq!(map[eiffel_tower_res12], "France");
    ///
    /// map[eiffel_tower_res12] = "Paris";
    /// assert_eq!(map[eiffel_tower_res12], "Paris");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the cell is not present in the `HexTreeMap`.
    fn index_mut(&mut self, cell: Index) -> &mut V {
        self.get_mut(cell).expect("no entry found for cell")
    }
}

impl<V, C> std::ops::IndexMut<&Index> for HexTreeMap<V, C> {
    /// Returns a reference to the value corresponding to the supplied
    /// key.
    ///
    /// # Examples
    ///
    /// ```
    /// use hextree::{Index, HexTreeMap};
    ///
    /// let mut map = HexTreeMap::new();
    /// let eiffel_tower_res12 = Index::from_raw(0x8c1fb46741ae9ff).unwrap();
    ///
    /// map.insert(eiffel_tower_res12, "France");
    /// assert_eq!(map[&eiffel_tower_res12], "France");
    ///
    /// map[&eiffel_tower_res12] = "Paris";
    /// assert_eq!(map[&eiffel_tower_res12], "Paris");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the cell is not present in the `HexTreeMap`.
    fn index_mut(&mut self, cell: &Index) -> &mut V {
        self.get_mut(*cell).expect("no entry found for cell")
    }
}

impl<V: std::fmt::Debug, C> std::fmt::Debug for HexTreeMap<V, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;
        let mut iter = self.iter();
        if let Some((cell, val)) = iter.next() {
            write!(f, "{:x}: {:?}", cell.0, val)?
        }
        for (cell, val) in iter {
            write!(f, ", {:x}: {:?}", cell.0, val)?
        }
        f.write_str("}")
    }
}
