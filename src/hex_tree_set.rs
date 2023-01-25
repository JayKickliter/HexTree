use crate::{compaction::SetCompactor, index::Index, HexTreeMap};
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
/// # use h3ron::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use geo_types::coord;
/// use hextree::{Index, HexTreeSet};
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
/// // `cells` is a slice of `Index`s
/// let monaco: HexTreeSet = cells.iter().collect();
///
/// // You can see in the map above that our set covers Point 1 (green
/// // check) but not Point 2 (red x), let's test that.
/// let point_1 = H3Cell::from_coordinate(coord! {x: 7.42418, y: 43.73631}, 12)?;
/// let point_2 = H3Cell::from_coordinate(coord! {x: 7.42855, y: 43.73008}, 12)?;
///
/// assert!(monaco.contains(Index::from_raw(*point_1).unwrap()));
/// assert!(!monaco.contains(Index::from_raw(*point_2).unwrap()));
///
/// #     Ok(())
/// # }
/// ```
pub type HexTreeSet = HexTreeMap<(), SetCompactor>;

impl FromIterator<Index> for HexTreeSet {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Index>,
    {
        let mut set = HexTreeMap::with_compactor(SetCompactor);
        for cell in iter {
            set.insert(cell, ());
        }
        set
    }
}

impl<'a> FromIterator<&'a Index> for HexTreeSet {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a Index>,
    {
        let mut set = HexTreeMap::with_compactor(SetCompactor);
        for cell in iter {
            set.insert(*cell, ());
        }
        set
    }
}
