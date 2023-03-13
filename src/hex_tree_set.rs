use crate::{compaction::SetCompactor, Cell, HexTreeMap};
use std::iter::FromIterator;

/// A HexTreeSet is a structure for representing geographical regions
/// and efficiently testing performing hit-tests on that region. Or,
/// in other words: I have a region defined; does it contain this
/// point on earth?
///
///
/// # Usage
///
/// <iframe src="https://www.google.com/maps/d/u/0/embed?mid=1Ty1LhqAipSTl6lsXH7YAjE6kdNmEvCw&ehbc=2E312F" width="100%" height="480"></iframe>
///
/// ----
///
/// Let's create a HexTreeSet for Monaco as visualized in the map
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use geo_types::coord;
/// use hextree::{Cell, HexTreeSet};
/// #
/// #    use byteorder::{LittleEndian as LE, ReadBytesExt};
/// #    let idx_bytes = include_bytes!("../assets//monaco.res12.h3idx");
/// #    let rdr = &mut idx_bytes.as_slice();
/// #    let mut cells = Vec::new();
/// #    while let Ok(idx) = rdr.read_u64::<LE>() {
/// #        cells.push(Cell::from_raw(idx)?);
/// #    }
///
/// // `cells` is a slice of `Index`s
/// let monaco: HexTreeSet = cells.iter().collect();
///
/// // You can see in the map above that our set covers Point 1 (green
/// // check) but not Point 2 (red x), let's test that.
/// // Lat/lon 43.73631, 7.42418 @ res 12
/// let point_1 = Cell::from_raw(0x8c3969a41da15ff)?;
/// // Lat/lon 43.73008, 7.42855 @ res 12
/// let point_2 = Cell::from_raw(0x8c3969a415065ff)?;
///
/// assert!(monaco.contains(point_1));
/// assert!(!monaco.contains(point_2));
///
/// #     Ok(())
/// # }
/// ```
pub type HexTreeSet = HexTreeMap<(), SetCompactor>;

impl FromIterator<Cell> for HexTreeSet {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Cell>,
    {
        let mut set = HexTreeMap::with_compactor(SetCompactor);
        for cell in iter {
            set.insert(cell, ());
        }
        set
    }
}

impl<'a> FromIterator<&'a Cell> for HexTreeSet {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a Cell>,
    {
        let mut set = HexTreeMap::with_compactor(SetCompactor);
        for cell in iter {
            set.insert(*cell, ());
        }
        set
    }
}
