//! On-disk memory-mapped storage for HexTree.
//!
//! DiskTree provides a serialized, memory-mapped representation of a HexTreeMap,
//! allowing you to store and query very large trees without loading them entirely
//! into memory.

#[cfg(not(target_pointer_width = "64"))]
compile_warning!("disktree may silently fail on non-64bit systems");

pub use tree::DiskTreeMap;

mod dptr;
mod dtseek;
mod iter;
mod node;
mod tree;
mod varint;
mod writer;

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{LittleEndian as LE, ReadBytesExt};
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

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
        // cells with the same value.
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
        let monaco_disktree = DiskTreeMap::open(path).unwrap();

        assert_eq!(monaco.get(point_2), None);
        assert_eq!(
            monaco.get(point_1),
            Some((point_1.to_parent(9).unwrap(), &Region::Monaco))
        );

        let point_1_res8 = point_1.to_parent(8).unwrap();
        assert!(matches!(
            monaco.get_raw(point_1_res8),
            Some((cell, crate::node::Node::Parent(_))) if cell == point_1_res8
        ));

        assert!(matches!(
            monaco_disktree.get_raw(point_1_res8).unwrap(),
            Some((cell, _, crate::disktree::node::Node::Parent(_))) if cell == point_1_res8
        ));

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
    fn test_variable_sized_vals() {
        use crate::{Cell, HexTreeMap};

        let (keeper_cells, test_cells): (Vec<Cell>, Vec<Cell>) = {
            let idx_bytes = include_bytes!("../../assets/monaco.res12.h3idx");
            let rdr = &mut idx_bytes.as_slice();
            let mut cells = Vec::new();
            while let Ok(idx) = rdr.read_u64::<LE>() {
                cells.push(Cell::from_raw(idx).unwrap());
            }
            let (l, r) = cells.split_at(625);
            (l.to_vec(), r.to_vec())
        };

        assert_eq!(keeper_cells.len(), 625);
        assert_eq!(test_cells.len(), 200);

        fn cell_to_value(cell: &Cell) -> Vec<u8> {
            use std::hash::{Hash, Hasher};
            let mut s = std::collections::hash_map::DefaultHasher::new();
            cell.hash(&mut s);
            // Generate length between 0..=0xFFFF;
            let len = match s.finish() & 0xFFFF {
                len if len.trailing_ones() == 8 => 0,
                len => len,
            };
            // assert_ne!(len, 0);
            (0..len).map(|idx| idx as u8).collect::<Vec<u8>>()
        }

        let mut zero_len_val_cnt = 0;

        let monaco_hashmap: HashMap<&Cell, Vec<u8>> = {
            let mut map = HashMap::new();
            for cell in &keeper_cells {
                let val = cell_to_value(cell);
                if val.is_empty() {
                    zero_len_val_cnt += 1;
                }
                map.insert(cell, val);
            }
            map
        };

        // Ensure we get at least one 0-length value.
        assert_ne!(zero_len_val_cnt, 0);

        let monaco_hextree: HexTreeMap<&[u8]> = {
            let mut map = HexTreeMap::new();
            for (cell, val) in &monaco_hashmap {
                map.insert(**cell, val.as_slice())
            }
            map
        };

        let monaco_disktree: DiskTreeMap = {
            let file = tempfile::NamedTempFile::new().unwrap();
            let (mut file, path) = file.keep().unwrap();
            monaco_hextree
                .to_disktree(&mut file, |wtr, val| wtr.write_all(val))
                .unwrap();
            let _ = file;
            DiskTreeMap::open(path).unwrap()
        };

        // Assert neither hashmap nor disktree contain reserved cells.
        for cell in test_cells {
            assert!(!monaco_hashmap.contains_key(&cell));
            assert!(!monaco_disktree.contains(cell).unwrap());
        }

        // Assert disktree contains all the same values as the
        // hashmap.
        for (cell, val) in monaco_hashmap
            .iter()
            .map(|(cell, vec)| (**cell, vec.as_slice()))
        {
            assert_eq!((cell, val), monaco_disktree.get(cell).unwrap().unwrap())
        }

        // Assert hashmap contains all the same values as the
        // disktree.
        for (cell, val) in monaco_disktree.iter().unwrap().map(|entry| entry.unwrap()) {
            assert_eq!(
                (cell, val),
                (
                    cell,
                    monaco_hashmap.get(&cell).map(|vec| vec.as_slice()).unwrap()
                )
            )
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
        // cells with the same value.
        let mut monaco = HexTreeMap::new();

        // Now extend the map with cells and a region value.
        monaco.extend(cells.iter().copied().zip(cells.iter().copied()));

        let file = tempfile::NamedTempFile::new().unwrap();
        let (mut file, path) = file.keep().unwrap();
        println!("disktree path: {path:?}");
        monaco
            .to_disktree(&mut file, |wtr, val| bincode::serialize_into(wtr, val))
            .unwrap();
        let monaco_disktree = DiskTreeMap::open(path).unwrap();

        // Create the iterator with the user-defined deserializer.
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
            "iterating a disktree should yield identically ordered elements as the hextree tree it was derived from"
        );
    }

    #[test]
    fn test_empty_disktree() {
        use crate::HexTreeMap;
        use std::io::Cursor;
        let mut wtr = vec![];
        HexTreeMap::<&[u8]>::new()
            .to_disktree(Cursor::new(&mut wtr), |wtr, val| wtr.write_all(val))
            .unwrap();
        let disktree = DiskTreeMap::with_buf(wtr).unwrap();
        assert_eq!(0, disktree.iter().unwrap().count());
    }

    #[test]
    fn test_descendants() {
        use crate::{compaction::NullCompactor, Cell, HexTreeMap};
        use h3o::{CellIndex, Resolution};
        use std::{convert::TryFrom, io::Cursor};

        // https://wolf-h3-viewer.glitch.me/?h3=863969a47ffffff
        let monaco_res6_ci = CellIndex::try_from(0x863969a47ffffff).unwrap();
        let monaco_res6_cell = Cell::try_from(u64::from(monaco_res6_ci)).unwrap();
        // https://wolf-h3-viewer.glitch.me/?h3=863969a6fffffff
        let not_monaco_res6_ci = CellIndex::try_from(0x863969a6fffffff).unwrap();

        let monaco_res10_cells = monaco_res6_ci
            .children(Resolution::Ten)
            .map(|ci| Cell::try_from(u64::from(ci)).unwrap())
            .collect::<Vec<_>>();

        let not_monaco_res10_cells = not_monaco_res6_ci
            .children(Resolution::Ten)
            .map(|ci| Cell::try_from(u64::from(ci)).unwrap())
            .collect::<Vec<_>>();

        let monaco_hextree: HexTreeMap<(), NullCompactor> = monaco_res10_cells
            .iter()
            .copied()
            .map(|cell| (cell, ()))
            .collect();

        let combined_hextree: HexTreeMap<(), NullCompactor> = monaco_res10_cells
            .iter()
            .chain(not_monaco_res10_cells.iter())
            .copied()
            .map(|cell| (cell, ()))
            .collect();

        let combined_disktree = {
            let mut combined_disktree_buf = vec![];
            combined_hextree
                .to_disktree(Cursor::new(&mut combined_disktree_buf), |wtr, ()| {
                    wtr.write_all(&[])
                })
                .unwrap();
            DiskTreeMap::with_buf(combined_disktree_buf).unwrap()
        };

        // Ensure calling `descendants` on leaf returns only a single
        // item iterator of the leaf.
        for (hextree_leaf, _) in combined_hextree.iter() {
            let leaf_vec = combined_disktree
                .descendants(hextree_leaf)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(
                leaf_vec.len(),
                1,
                "Iterator must have exactly one element for a leaf"
            );
            assert_eq!(hextree_leaf, leaf_vec[0].0);
        }

        assert_eq!(
            combined_hextree.len(),
            combined_disktree.iter().unwrap().count()
        );

        let combined_collect = combined_disktree
            .descendants(monaco_res6_cell)
            .unwrap()
            .map(|item| item.unwrap().0)
            .collect::<Vec<_>>();
        let monaco_hextree_collect = monaco_hextree.iter().map(|item| item.0).collect::<Vec<_>>();
        assert_eq!(combined_collect, monaco_hextree_collect);
    }
}
