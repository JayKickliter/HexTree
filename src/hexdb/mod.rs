//! An on-disk hextree.

#[cfg(not(target_pointer_width = "64"))]
compile_warning!("hexdb may silently fail on non-64bit systems");

pub use tree::HexDb;

mod dbseek;
mod dptr;
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
        println!("hexdb path: {path:?}");
        monaco
            .to_hexdb(&mut file, |wtr, val| bincode::serialize_into(wtr, val))
            .unwrap();
        let monaco_hexdb = HexDb::open(path).unwrap();

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
            monaco_hexdb.get_raw(point_1_res8).unwrap(),
            Some((cell, crate::hexdb::node::Node::Parent(_))) if cell == point_1_res8
        ));

        for (ht_cell, &ht_val) in monaco.iter() {
            let now = std::time::Instant::now();
            let (dt_cell, val_buf) = monaco_hexdb.get(ht_cell).unwrap().unwrap();
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

        let monaco_hexdb: HexDb = {
            let file = tempfile::NamedTempFile::new().unwrap();
            let (mut file, path) = file.keep().unwrap();
            monaco_hextree
                .to_hexdb(&mut file, |wtr, val| wtr.write_all(val))
                .unwrap();
            let _ = file;
            HexDb::open(path).unwrap()
        };

        // Assert neither hashmap nor hexdb contain reserved cells.
        for cell in test_cells {
            assert!(!monaco_hashmap.contains_key(&cell));
            assert!(!monaco_hexdb.contains(cell).unwrap());
        }

        // Assert hexdb contains all the same values as the
        // hashmap.
        for (cell, val) in monaco_hashmap
            .iter()
            .map(|(cell, vec)| (**cell, vec.as_slice()))
        {
            assert_eq!((cell, val), monaco_hexdb.get(cell).unwrap().unwrap())
        }

        // Assert hashmap contains all the same values as the
        // hexdb.
        for (cell, val) in monaco_hexdb.iter().unwrap().map(|entry| entry.unwrap()) {
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
        // cells with the same save value.
        let mut monaco = HexTreeMap::new();

        // Now extend the map with cells and a region value.
        monaco.extend(cells.iter().copied().zip(cells.iter().copied()));

        let file = tempfile::NamedTempFile::new().unwrap();
        let (mut file, path) = file.keep().unwrap();
        println!("hexdb path: {path:?}");
        monaco
            .to_hexdb(&mut file, |wtr, val| bincode::serialize_into(wtr, val))
            .unwrap();
        let monaco_hexdb = HexDb::open(path).unwrap();

        // Create the iterator with the user-defined deserialzer.
        let hexdb_iter = monaco_hexdb.iter().unwrap();
        let start = std::time::Instant::now();
        let mut hexdb_collection = Vec::new();
        for res in hexdb_iter {
            let (cell, val_buf) = res.unwrap();
            hexdb_collection.push((cell, bincode::deserialize_from(val_buf).unwrap()));
        }
        let elapsed = start.elapsed();
        println!("{elapsed:?}");
        let start = std::time::Instant::now();
        let hextree_collection: Vec<_> = monaco.iter().map(|(k, v)| (k, *v)).collect();
        let elapsed = start.elapsed();
        println!("{elapsed:?}");

        assert_eq!(
            hextree_collection,
            hexdb_collection,
            "iterating a hexdb should yield identically ordered elements as the hextree tree it was derived from"
        );
    }

    #[test]
    fn test_empty_hexdb() {
        use crate::HexTreeMap;
        use std::io::Cursor;
        let mut wtr = vec![];
        HexTreeMap::<&[u8]>::new()
            .to_hexdb(Cursor::new(&mut wtr), |wtr, val| wtr.write_all(val))
            .unwrap();
        let hexdb = HexDb::with_buf(wtr).unwrap();
        assert_eq!(0, hexdb.iter().unwrap().count());
    }
}
