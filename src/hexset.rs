use crate::{compaction::SetCompactor, h3ron::H3Cell, HexMap};
use std::iter::FromIterator;

/// A HexSex is a structure for representing geographical regions and
/// efficiently testing performing hit-tests on that region. Or, in
/// other words: I have a region defined; does it contain this
/// point on earth?
///
///
/// # Usage
///
/// <iframe src="https://www.google.com/maps/d/u/0/embed?mid=1Ty1LhqAipSTl6lsXH7YAjE6kdNmEvCw&ehbc=2E312F" width="100%" height="480"></iframe>
///
/// ----
///
/// Let's create a HexSet for Monaco as visualized in the map
///
/// ```
/// # use hextree::h3ron::Error;
/// #
/// # fn main() -> Result<(), Error> {
/// use geo_types::coord;
/// use hextree::{h3ron::H3Cell, HexSet};
/// #
/// #    use byteorder::{LittleEndian as LE, ReadBytesExt};
/// #    use hextree::h3ron::FromH3Index;
/// #    let idx_bytes = include_bytes!("../assets//monaco.res12.h3idx");
/// #    let rdr = &mut idx_bytes.as_slice();
/// #    let mut cells = Vec::new();
/// #    while let Ok(idx) = rdr.read_u64::<LE>() {
/// #        cells.push(H3Cell::from_h3index(idx));
/// #    }
///
/// // `cells` is a slice of `H3Cell`s
/// let monaco: HexSet = cells.iter().collect();
///
/// // You can see in the map above that our set covers Point 1 (green
/// // check) but not Point 2 (red x), let's test that.
/// let point_1 = H3Cell::from_coordinate(coord! {x: 7.42418, y: 43.73631}, 12)?;
/// let point_2 = H3Cell::from_coordinate(coord! {x: 7.42855, y: 43.73008}, 12)?;
///
/// assert!(monaco.contains(point_1));
/// assert!(!monaco.contains(point_2));
///
/// #     Ok(())
/// # }
/// ```
pub type HexSet = HexMap<(), SetCompactor>;

impl FromIterator<H3Cell> for HexSet {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = H3Cell>,
    {
        let mut set = HexMap::with_compactor(SetCompactor);
        for cell in iter {
            set.insert(cell, ());
        }
        set
    }
}

impl<'a> FromIterator<&'a H3Cell> for HexSet {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a H3Cell>,
    {
        let mut set = HexMap::with_compactor(SetCompactor);
        for cell in iter {
            set.insert(*cell, ());
        }
        set
    }
}
