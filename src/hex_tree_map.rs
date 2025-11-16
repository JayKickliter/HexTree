//! A HexTreeMap is a structure for mapping geographical regions to values.

pub use crate::entry::{Entry, OccupiedEntry, VacantEntry};
use crate::{
    cell::CellStack,
    compaction::{Compactor, NullCompactor},
    digits::Digits,
    node::Node,
    Cell,
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
/// <iframe src="https://kepler.gl/demo?mapUrl=https://gist.githubusercontent.com/JayKickliter/8f91a8437b7dd89321b22cde50e71c3a/raw/a60c83cb15e75aba660fb6535d8e0061fa504205/monaco.kepler.json" width="100%" height="600"></iframe>
///
/// ----
///
/// Let's create a HexTreeMap for Monaco as visualized in the map
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use hextree::{Cell, compaction::EqCompactor, HexTreeMap};
/// #
/// #    use byteorder::{LittleEndian as LE, ReadBytesExt};
/// #    let idx_bytes = include_bytes!("../assets//monaco.res12.h3idx");
/// #    let rdr = &mut idx_bytes.as_slice();
/// #    let mut cells = Vec::new();
/// #    while let Ok(idx) = rdr.read_u64::<LE>() {
/// #        cells.push(Cell::from_raw(idx)?);
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
/// // Lat/lon 43.73631, 7.42418 @ res 12
/// let point_1 = Cell::from_raw(0x8c3969a41da15ff)?;
/// // Lat/lon 43.73008, 7.42855 @ res 12
/// let point_2 = Cell::from_raw(0x8c3969a415065ff)?;
///
/// assert_eq!(monaco.get(point_1).unzip().1, Some(&Region::Monaco));
/// assert_eq!(monaco.get(point_2).unzip().1, None);
///
/// #     Ok(())
/// # }
/// ```
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HexTreeMap<V, C = NullCompactor> {
    /// All h3 0 base cell indices in the tree
    pub(crate) nodes: Box<[Option<Box<Node<V>>>]>,
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
    /// Adds a cell/value pair to the set.
    pub fn insert(&mut self, cell: Cell, value: V) {
        let base_cell = cell.base();
        let digits = Digits::new(cell);
        match self.nodes[base_cell as usize].as_mut() {
            Some(node) => node.insert(cell, 0_u8, digits, value, &mut self.compactor),
            None => {
                let mut node = Box::new(Node::new());
                node.insert(cell, 0_u8, digits, value, &mut self.compactor);
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
    /// This method only considers complete, or leaf, cells in the
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

    /// Returns `true` if the set fully contains `cell`.
    ///
    /// This method will return `true` if any of the following are
    /// true:
    ///
    /// 1. There was an earlier [insert][Self::insert] call with
    ///    precisely this target cell.
    /// 2. Several previously inserted cells coalesced into
    ///    precisely this target cell.
    /// 3. The set contains a complete (leaf) parent of this target
    ///    cell due to 1 or 2.
    pub fn contains(&self, cell: Cell) -> bool {
        let base_cell = cell.base();
        match self.nodes[base_cell as usize].as_ref() {
            Some(node) => {
                let digits = Digits::new(cell);
                node.contains(digits)
            }
            None => false,
        }
    }

    /// Returns a reference to the value corresponding to the given
    /// target cell or one of its parents.
    ///
    /// Note that this method also returns a Cell, which may be a
    /// parent of the target cell provided.
    #[inline]
    pub fn get(&self, cell: Cell) -> Option<(Cell, &V)> {
        match self.get_raw(cell) {
            Some((cell, Node::Leaf(val))) => Some((cell, val)),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn get_raw(&self, cell: Cell) -> Option<(Cell, &Node<V>)> {
        let base_cell = cell.base();
        match self.nodes[base_cell as usize].as_ref() {
            Some(node) => {
                let digits = Digits::new(cell);
                node.get(0, cell, digits)
            }
            None => None,
        }
    }

    /// Returns a mutable reference to the value corresponding to the
    /// given target cell or one of its parents.
    ///
    /// Note that this method also returns a Cell, which may be a
    /// parent of the target cell provided.
    #[inline]
    pub fn get_mut(&mut self, cell: Cell) -> Option<(Cell, &mut V)> {
        match self.get_raw_mut(cell) {
            Some((cell, &mut Node::Leaf(ref mut val))) => Some((cell, val)),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn get_raw_mut(&mut self, cell: Cell) -> Option<(Cell, &mut Node<V>)> {
        let base_cell = cell.base();
        match self.nodes[base_cell as usize].as_mut() {
            Some(node) => {
                let digits = Digits::new(cell);
                node.get_mut(0, cell, digits)
            }
            None => None,
        }
    }

    /// Gets the entry in the map for the corresponding cell.
    pub fn entry(&'_ mut self, cell: Cell) -> Entry<'_, V, C> {
        if self.get(cell).is_none() {
            return Entry::Vacant(VacantEntry {
                target_cell: cell,
                map: self,
            });
        }
        Entry::Occupied(OccupiedEntry {
            target_cell: cell,
            cell_value: self.get_mut(cell).unwrap(),
        })
    }

    /// An iterator visiting all cell-value pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (Cell, &V)> {
        crate::iteration::Iter::new(&self.nodes, CellStack::new())
    }

    /// An iterator visiting all cell-value pairs in arbitrary order
    /// with mutable references to the values.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Cell, &mut V)> {
        crate::iteration::IterMut::new(&mut self.nodes, CellStack::new())
    }

    /// An iterator visiting the specified cell or its children
    /// references to the values.
    pub fn descendants(&self, cell: Cell) -> impl Iterator<Item = (Cell, &V)> {
        let base_cell = cell.base();
        match self.nodes[base_cell as usize].as_ref() {
            Some(node) => {
                let digits = Digits::new(cell);
                match node.get(0, cell, digits) {
                    Some((cell, Node::Leaf(val))) => Some((cell, val))
                        .into_iter()
                        .chain(crate::iteration::Iter::empty()),
                    Some((cell, Node::Parent(children))) => None
                        .into_iter()
                        .chain(crate::iteration::Iter::new(children, CellStack::from(cell))),
                    None => None.into_iter().chain(crate::iteration::Iter::empty()),
                }
            }
            None => None.into_iter().chain(crate::iteration::Iter::empty()),
        }
    }

    /// An iterator visiting the specified cell or its children with
    /// mutable references to the values.
    pub fn descendants_mut(&mut self, cell: Cell) -> impl Iterator<Item = (Cell, &mut V)> {
        let base_cell = cell.base();
        match self.nodes[base_cell as usize].as_mut() {
            Some(node) => {
                let digits = Digits::new(cell);
                match node.get_mut(0, cell, digits) {
                    Some((cell, Node::Leaf(val))) => Some((cell, val))
                        .into_iter()
                        .chain(crate::iteration::IterMut::empty()),
                    Some((cell, Node::Parent(children))) => None.into_iter().chain(
                        crate::iteration::IterMut::new(children, CellStack::from(cell)),
                    ),
                    None => None.into_iter().chain(crate::iteration::IterMut::empty()),
                }
            }
            None => None.into_iter().chain(crate::iteration::IterMut::empty()),
        }
    }
}

impl<V: PartialEq> Default for HexTreeMap<V, NullCompactor> {
    fn default() -> Self {
        HexTreeMap::new()
    }
}

impl<V> FromIterator<(Cell, V)> for HexTreeMap<V, NullCompactor> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (Cell, V)>,
    {
        let mut map = HexTreeMap::new();
        for (cell, value) in iter {
            map.insert(cell, value);
        }
        map
    }
}

impl<'a, V: Copy + 'a> FromIterator<(&'a Cell, &'a V)> for HexTreeMap<V, NullCompactor> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (&'a Cell, &'a V)>,
    {
        let mut map = HexTreeMap::new();
        for (cell, value) in iter {
            map.insert(*cell, *value);
        }
        map
    }
}

impl<V, C: Compactor<V>> Extend<(Cell, V)> for HexTreeMap<V, C> {
    fn extend<I: IntoIterator<Item = (Cell, V)>>(&mut self, iter: I) {
        for (cell, val) in iter {
            self.insert(cell, val)
        }
    }
}

impl<'a, V: Copy + 'a, C: Compactor<V>> Extend<(&'a Cell, &'a V)> for HexTreeMap<V, C> {
    fn extend<I: IntoIterator<Item = (&'a Cell, &'a V)>>(&mut self, iter: I) {
        for (cell, val) in iter {
            self.insert(*cell, *val)
        }
    }
}

impl<V, C> std::ops::Index<Cell> for HexTreeMap<V, C> {
    type Output = V;

    /// Returns a reference to the value corresponding to the supplied
    /// key.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> hextree::Result<()> {;
    /// use hextree::{Cell, HexTreeMap};
    ///
    /// let mut map = HexTreeMap::new();
    /// let eiffel_tower_res12 = Cell::from_raw(0x8c1fb46741ae9ff)?;
    ///
    /// map.insert(eiffel_tower_res12, "France");
    /// assert_eq!(map[eiffel_tower_res12], "France");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the cell is not present in the `HexTreeMap`.
    fn index(&self, cell: Cell) -> &V {
        self.get(cell).expect("no entry found for cell").1
    }
}

impl<V, C> std::ops::Index<&Cell> for HexTreeMap<V, C> {
    type Output = V;

    /// Returns a reference to the value corresponding to the supplied
    /// key.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> hextree::Result<()> {;
    /// use hextree::{Cell, HexTreeMap};
    ///
    /// let mut map = HexTreeMap::new();
    /// let eiffel_tower_res12 = Cell::from_raw(0x8c1fb46741ae9ff)?;
    ///
    /// map.insert(eiffel_tower_res12, "France");
    /// assert_eq!(map[&eiffel_tower_res12], "France");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the cell is not present in the `HexTreeMap`.
    fn index(&self, cell: &Cell) -> &V {
        self.get(*cell).expect("no entry found for cell").1
    }
}

impl<V, C> std::ops::IndexMut<Cell> for HexTreeMap<V, C> {
    /// Returns a reference to the value corresponding to the supplied
    /// key.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> hextree::Result<()> {;
    /// use hextree::{Cell, HexTreeMap};
    ///
    /// let mut map = HexTreeMap::new();
    /// let eiffel_tower_res12 = Cell::from_raw(0x8c1fb46741ae9ff)?;
    ///
    /// map.insert(eiffel_tower_res12, "France");
    /// assert_eq!(map[eiffel_tower_res12], "France");
    ///
    /// map[eiffel_tower_res12] = "Paris";
    /// assert_eq!(map[eiffel_tower_res12], "Paris");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the cell is not present in the `HexTreeMap`.
    fn index_mut(&mut self, cell: Cell) -> &mut V {
        self.get_mut(cell).expect("no entry found for cell").1
    }
}

impl<V, C> std::ops::IndexMut<&Cell> for HexTreeMap<V, C> {
    /// Returns a reference to the value corresponding to the supplied
    /// key.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> hextree::Result<()> {;
    /// use hextree::{Cell, HexTreeMap};
    ///
    /// let mut map = HexTreeMap::new();
    /// let eiffel_tower_res12 = Cell::from_raw(0x8c1fb46741ae9ff)?;
    ///
    /// map.insert(eiffel_tower_res12, "France");
    /// assert_eq!(map[&eiffel_tower_res12], "France");
    ///
    /// map[&eiffel_tower_res12] = "Paris";
    /// assert_eq!(map[&eiffel_tower_res12], "Paris");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the cell is not present in the `HexTreeMap`.
    fn index_mut(&mut self, cell: &Cell) -> &mut V {
        self.get_mut(*cell).expect("no entry found for cell").1
    }
}

impl<V: std::fmt::Debug, C> std::fmt::Debug for HexTreeMap<V, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;
        let mut iter = self.iter();
        if let Some((cell, val)) = iter.next() {
            write!(f, "{cell:?}: {val:?}")?
        }
        for (cell, val) in iter {
            write!(f, ", {cell:?}: {val:?}")?
        }
        f.write_str("}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<HexTreeMap<i32>>();
    }

    #[test]
    fn map_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<HexTreeMap<i32>>();
    }
}
