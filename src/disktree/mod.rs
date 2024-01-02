//! An on-disk hextree.

pub use tree::DiskTree;

mod dptr;
mod iter;
mod node;
mod tree;
mod writer;

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{LittleEndian as LE, ReadBytesExt};
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_roundtrip_monaco() {
        use crate::{compaction::EqCompactor, Cell, HexTreeMap};
        let idx_bytes = include_bytes!("../../assets/monaco.res12.h3idx");
        let rdr = &mut idx_bytes.as_slice();
        let mut cells = Vec::new();
        while let Ok(idx) = rdr.read_u64::<LE>() {
            cells.push(Cell::from_raw(idx).unwrap());
        }

        #[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
        enum Region {
            Monaco,
        }

        // Construct map with a compactor that automatically combines
        // cells with the same save value.
        let mut monaco = HexTreeMap::with_compactor(EqCompactor);

        // Now extend the map with cells and a region value.
        monaco.extend(cells.iter().copied().zip(std::iter::repeat(Region::Monaco)));

        // You can see in the map above that our set covers Point 1 (green
        // check) but not Point 2 (red x), let's test that.
        // Lat/lon 43.73631, 7.42418 @ res 12
        let point_1 = Cell::from_raw(0x8c3969a41da15ff).unwrap();
        // Lat/lon 43.73008, 7.42855 @ res 12
        let point_2 = Cell::from_raw(0x8c3969a415065ff).unwrap();

        let file = tempfile::NamedTempFile::new().unwrap();
        let (mut file, path) = file.keep().unwrap();
        println!("disktree path: {path:?}");
        monaco
            .to_disktree(&mut file, |wtr, val| bincode::serialize_into(wtr, val))
            .unwrap();
        let monaco_disktree = DiskTree::open(path).unwrap();

        assert_eq!(monaco.get(point_2).unzip().1, None);
        assert_eq!(monaco.get(point_1).unzip().1, Some(&Region::Monaco));

        for (ht_cell, &ht_val) in monaco.iter() {
            let now = std::time::Instant::now();
            let (dt_cell, val_buf) = monaco_disktree.get(ht_cell).unwrap().unwrap();
            let dt_val = bincode::deserialize_from(val_buf).unwrap();
            let lookup_duration = now.elapsed();
            println!("loookup of {dt_cell} took {lookup_duration:?}");
            assert_eq!(ht_val, dt_val);
            assert_eq!(ht_cell, dt_cell);
        }
    }

    #[test]
    fn test_iter() {
        use crate::{Cell, HexTreeMap};
        let idx_bytes = include_bytes!("../../assets/monaco.res12.h3idx");
        let rdr = &mut idx_bytes.as_slice();
        let mut cells = Vec::new();
        while let Ok(idx) = rdr.read_u64::<LE>() {
            cells.push(Cell::from_raw(idx).unwrap());
        }

        // Construct map with a compactor that automatically combines
        // cells with the same save value.
        let mut monaco = HexTreeMap::new();

        // Now extend the map with cells and a region value.
        monaco.extend(cells.iter().copied().zip(cells.iter().copied()));

        let file = tempfile::NamedTempFile::new().unwrap();
        let (mut file, path) = file.keep().unwrap();
        println!("disktree path: {path:?}");
        monaco
            .to_disktree(&mut file, |wtr, val| bincode::serialize_into(wtr, val))
            .unwrap();
        let monaco_disktree = DiskTree::open(path).unwrap();

        // Create the iterator with the user-defined deserialzer.
        let disktree_iter = monaco_disktree.iter().unwrap();
        let start = std::time::Instant::now();
        let mut disktree_collection = Vec::new();
        for res in disktree_iter {
            let (cell, val_buf) = res.unwrap();
            disktree_collection.push((cell, bincode::deserialize_from(val_buf).unwrap()));
        }
        let elapsed = start.elapsed();
        println!("{elapsed:?}");
        let start = std::time::Instant::now();
        let hextree_collection: Vec<_> = monaco.iter().map(|(k, v)| (k, *v)).collect();
        let elapsed = start.elapsed();
        println!("{elapsed:?}");

        assert_eq!(
            hextree_collection,
            disktree_collection,
            "iterating a disktree should yeild identically ordered elements as the hextree tree it was derived from"
        );
    }
}
