//! `HexMap`'s Entry API.

use crate::{compaction::Compactor, h3ron::H3Cell, HexMap};

/// A view into a single entry in a map, which may either be vacant or
/// occupied.
///
/// This enum is constructed from the [entry][HexMap::entry] method on
/// [HexMap][HexMap].
pub enum Entry<'a, V, C> {
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, V>),
    /// A vacant entry.
    Vacant(VacantEntry<'a, V, C>),
}

/// A view into an occupied entry in a `HexMap`. It is part of the
/// [`Entry`] enum.
pub struct OccupiedEntry<'a, V> {
    pub(crate) hex: H3Cell,
    pub(crate) value: &'a mut V,
}

/// A view into a vacant entry in a `HexMap`. It is part of the
/// [`Entry`] enum.
pub struct VacantEntry<'a, V, C> {
    pub(crate) hex: H3Cell,
    pub(crate) map: &'a mut HexMap<V, C>,
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
    /// use hextree::{h3ron::{H3Cell, Index}, HexMap};
    ///
    /// let mut map = HexMap::new();
    /// let eiffel_tower_res12 = H3Cell::new(0x8c1fb46741ae9ff);
    ///
    /// map.entry(eiffel_tower_res12)
    ///    .and_modify(|v| *v = "Paris")
    ///    .or_insert("France");
    /// assert_eq!(map[eiffel_tower_res12], "France");
    ///
    /// map.entry(eiffel_tower_res12)
    ///     .and_modify(|v| *v = "Paris")
    ///     .or_insert("France");
    /// assert_eq!(map[eiffel_tower_res12], "Paris");
    /// ```
    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut V),
    {
        match self {
            Entry::Occupied(OccupiedEntry { hex, value }) => {
                f(value);
                Entry::Occupied(OccupiedEntry { hex, value })
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
    /// use hextree::{h3ron::{H3Cell, Index}, HexMap};
    ///
    /// let mut map = HexMap::new();
    /// let eiffel_tower_res12 = H3Cell::new(0x8c1fb46741ae9ff);
    ///
    /// map.entry(eiffel_tower_res12)
    ///    .or_insert("Paris");
    /// assert_eq!(map[eiffel_tower_res12], "Paris");
    /// ```
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(OccupiedEntry { hex: _hex, value }) => value,
            Entry::Vacant(VacantEntry { hex, map }) => {
                map.insert(hex, default);
                // We just inserted; unwrap is fine.
                map.get_mut(hex).unwrap()
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
    /// use hextree::{h3ron::{H3Cell, Index}, HexMap};
    ///
    /// let mut map = HexMap::new();
    /// let eiffel_tower_res12 = H3Cell::new(0x8c1fb46741ae9ff);
    ///
    /// map.entry(eiffel_tower_res12)
    ///    .or_insert_with(|| "Paris");
    /// assert_eq!(map[eiffel_tower_res12], "Paris");
    /// ```
    pub fn or_insert_with<F>(self, default: F) -> &'a mut V
    where
        F: FnOnce() -> V,
    {
        match self {
            Entry::Occupied(OccupiedEntry { hex: _hex, value }) => value,
            Entry::Vacant(VacantEntry { hex, map }) => {
                map.insert(hex, default());
                // We just inserted; unwrap is fine.
                map.get_mut(hex).unwrap()
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
    /// use hextree::{h3ron::{H3Cell, Index}, HexMap};
    ///
    /// let mut map: HexMap<Option<&str>> = HexMap::new();
    /// let eiffel_tower_res12 = H3Cell::new(0x8c1fb46741ae9ff);
    ///
    /// map.entry(eiffel_tower_res12).or_default();
    /// assert_eq!(map[eiffel_tower_res12], None);
    /// ```
    pub fn or_default(self) -> &'a mut V {
        match self {
            Entry::Occupied(OccupiedEntry { hex: _hex, value }) => value,
            Entry::Vacant(VacantEntry { hex, map }) => {
                map.insert(hex, Default::default());
                // We just inserted; unwrap is fine.
                map.get_mut(hex).unwrap()
            }
        }
    }
}
