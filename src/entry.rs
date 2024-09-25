//! `HexTreeMap`'s Entry API.

use crate::{compaction::Compactor, Cell, HexTreeMap};

/// A view into a single entry in a map, which may either be vacant or
/// occupied.
///
/// This enum is constructed from the [entry][HexTreeMap::entry]
/// method on [HexTreeMap][HexTreeMap].
pub enum Entry<'a, V, C> {
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, V>),
    /// A vacant entry.
    Vacant(VacantEntry<'a, V, C>),
}

/// A view into an occupied entry in a `HexTreeMap`. It is part of the
/// [`Entry`] enum.
pub struct OccupiedEntry<'a, V> {
    pub(crate) target_cell: Cell,
    pub(crate) cell_value: (Cell, &'a mut V),
}

/// A view into a vacant entry in a `HexTreeMap`. It is part of the
/// [`Entry`] enum.
pub struct VacantEntry<'a, V, C> {
    pub(crate) target_cell: Cell,
    pub(crate) map: &'a mut HexTreeMap<V, C>,
}

impl<'a, V, C> Entry<'a, V, C>
where
    C: Compactor<V>,
{
    /// Provides in-place mutable access to an occupied entry before
    /// any potential inserts into the map.
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
    /// map.entry(eiffel_tower_res12)
    ///    .and_modify(|_actual_cell, v| *v = "Paris")
    ///    .or_insert("France");
    /// assert_eq!(map[eiffel_tower_res12], "France");
    ///
    /// map.entry(eiffel_tower_res12)
    ///     .and_modify(|_actual_cell, v| *v = "Paris")
    ///     .or_insert("France");
    /// assert_eq!(map[eiffel_tower_res12], "Paris");
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(Cell, &mut V),
    {
        match self {
            Entry::Occupied(OccupiedEntry {
                target_cell,
                cell_value: (cell, value),
            }) => {
                f(cell, value);
                Entry::Occupied(OccupiedEntry {
                    target_cell,
                    cell_value: (cell, value),
                })
            }
            Entry::Vacant(_) => self,
        }
    }

    /// Ensures a value is in the entry by inserting the default if
    /// empty, and returns a mutable reference to the value in the
    /// entry.
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
    /// map.entry(eiffel_tower_res12)
    ///    .or_insert("Paris");
    /// assert_eq!(map[eiffel_tower_res12], "Paris");
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_insert(self, default: V) -> (Cell, &'a mut V) {
        match self {
            Entry::Occupied(OccupiedEntry {
                target_cell: _,
                cell_value,
            }) => cell_value,
            Entry::Vacant(VacantEntry { target_cell, map }) => {
                map.insert(target_cell, default);
                // We just inserted; unwrap is fine.
                map.get_mut(target_cell).unwrap()
            }
        }
    }

    /// Ensures a value is in the entry by inserting the result of the
    /// default function if empty, and returns a mutable reference to
    /// the value in the entry.
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
    /// map.entry(eiffel_tower_res12)
    ///    .or_insert_with(|| "Paris");
    /// assert_eq!(map[eiffel_tower_res12], "Paris");
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_insert_with<F>(self, default: F) -> (Cell, &'a mut V)
    where
        F: FnOnce() -> V,
    {
        match self {
            Entry::Occupied(OccupiedEntry {
                target_cell: _,
                cell_value,
            }) => cell_value,
            Entry::Vacant(VacantEntry { target_cell, map }) => {
                map.insert(target_cell, default());
                // We just inserted; unwrap is fine.
                map.get_mut(target_cell).unwrap()
            }
        }
    }
}

impl<'a, V, C> Entry<'a, V, C>
where
    V: Default,
    C: Compactor<V>,
{
    /// Ensures a value is in the entry by inserting the default value
    /// if empty, and returns a mutable reference to the value in the
    /// entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> hextree::Result<()> {
    /// use hextree::{Cell, HexTreeMap};
    ///
    /// let mut map: HexTreeMap<Option<&str>> = HexTreeMap::new();
    /// let eiffel_tower_res12 = Cell::from_raw(0x8c1fb46741ae9ff)?;
    ///
    /// map.entry(eiffel_tower_res12).or_default();
    /// assert_eq!(map[eiffel_tower_res12], None);
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_default(self) -> (Cell, &'a mut V) {
        match self {
            Entry::Occupied(OccupiedEntry {
                target_cell: _,
                cell_value,
            }) => cell_value,
            Entry::Vacant(VacantEntry { target_cell, map }) => {
                map.insert(target_cell, Default::default());
                map.get_mut(target_cell).expect("we just inserted")
            }
        }
    }
}
