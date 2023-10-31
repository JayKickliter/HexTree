use crate::{cell::CellStack, node::Node, Cell};
use std::iter::{Enumerate, FlatMap};

type NodeStackIter<'a, V> = FlatMap<
    Enumerate<std::slice::Iter<'a, Option<Box<Node<V>>>>>,
    Option<(usize, &'a Node<V>)>,
    fn((usize, &'a Option<Box<Node<V>>>)) -> Option<(usize, &'a Node<V>)>,
>;

fn make_node_stack_iter<'a, V>(nodes: &'a [Option<Box<Node<V>>>]) -> NodeStackIter<'a, V> {
    fn map_fn<V>(item: (usize, &Option<Box<Node<V>>>)) -> Option<(usize, &Node<V>)> {
        if let (digit, Some(val)) = item {
            Some((digit, val))
        } else {
            None
        }
    }

    nodes
        .iter()
        .enumerate()
        .flat_map(map_fn as fn((_, &'a Option<Box<Node<V>>>)) -> Option<(_, &'a Node<V>)>)
}

pub(crate) struct Iter<'a, V> {
    stack: Vec<NodeStackIter<'a, V>>,
    curr: Option<(usize, &'a Node<V>)>,
    cell_stack: CellStack,
}

impl<'a, V> Iter<'a, V> {
    pub(crate) fn new(base: &'a [Option<Box<Node<V>>>], mut cell_stack: CellStack) -> Self {
        let mut iter = make_node_stack_iter(base);
        let curr = iter.next();
        let mut stack = Vec::with_capacity(16);
        stack.push(iter);
        if let Some((digit, _)) = curr {
            cell_stack.push(digit as u8)
        }
        Self {
            stack,
            curr,
            cell_stack,
        }
    }

    pub(crate) fn empty() -> Self {
        let stack = Vec::new();
        let curr = None;
        let cell_stack = CellStack::new();
        Self {
            stack,
            curr,
            cell_stack,
        }
    }
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (Cell, &'a V);

    fn next(&mut self) -> Option<(Cell, &'a V)> {
        while self.curr.is_none() {
            if let Some(mut iter) = self.stack.pop() {
                self.cell_stack.pop();
                if let Some(node) = iter.next() {
                    self.cell_stack.push(node.0 as u8);
                    self.curr = Some(node);
                    self.stack.push(iter);
                }
            } else {
                break;
            }
        }
        while let Some((digit, curr)) = self.curr {
            self.cell_stack.swap(digit as u8);
            match curr {
                Node::Parent(children) => {
                    let mut iter = make_node_stack_iter(children.as_ref());
                    self.curr = iter.next();
                    // This branch is not 100% necessary, but I prefer
                    // pushing an actual digit instead of 0 and
                    // relying on the swap the further up to replace
                    // it with the correct value.
                    if let Some((digit, _)) = self.curr {
                        self.cell_stack.push(digit as u8)
                    }
                    self.stack.push(iter);
                }
                Node::Leaf(value) => {
                    self.curr = None;
                    return Some((
                        *self.cell_stack.cell().expect("corrupted cell-stack"),
                        value,
                    ));
                }
            }
        }
        None
    }
}

type NodeStackIterMut<'a, V> = FlatMap<
    Enumerate<std::slice::IterMut<'a, Option<Box<Node<V>>>>>,
    Option<(usize, &'a mut Node<V>)>,
    fn((usize, &'a mut Option<Box<Node<V>>>)) -> Option<(usize, &'a mut Node<V>)>,
>;

fn make_node_stack_iter_mut<'a, V>(
    nodes: &'a mut [Option<Box<Node<V>>>],
) -> NodeStackIterMut<'a, V> {
    fn map_fn_mut<V>(item: (usize, &mut Option<Box<Node<V>>>)) -> Option<(usize, &mut Node<V>)> {
        if let (digit, Some(val)) = item {
            Some((digit, val))
        } else {
            None
        }
    }

    nodes.iter_mut().enumerate().flat_map(
        map_fn_mut as fn((_, &'a mut Option<Box<Node<V>>>)) -> Option<(_, &'a mut Node<V>)>,
    )
}

pub(crate) struct IterMut<'a, V> {
    stack: Vec<NodeStackIterMut<'a, V>>,
    curr: Option<(usize, &'a mut Node<V>)>,
    cell_stack: CellStack,
}

impl<'a, V> IterMut<'a, V> {
    pub(crate) fn new(base: &'a mut [Option<Box<Node<V>>>], mut cell_stack: CellStack) -> Self {
        let mut iter = make_node_stack_iter_mut(base);
        let curr = iter.next();
        let mut stack = Vec::with_capacity(16);
        stack.push(iter);
        if let Some((digit, _)) = curr {
            cell_stack.push(digit as u8)
        }
        Self {
            stack,
            curr,
            cell_stack,
        }
    }

    pub(crate) fn empty() -> Self {
        let stack = Vec::new();
        let curr = None;
        let cell_stack = CellStack::new();
        Self {
            stack,
            curr,
            cell_stack,
        }
    }
}

impl<'a, V> Iterator for IterMut<'a, V> {
    type Item = (Cell, &'a mut V);

    fn next(&mut self) -> Option<(Cell, &'a mut V)> {
        while self.curr.is_none() {
            if let Some(mut iter) = self.stack.pop() {
                self.cell_stack.pop();
                if let Some(node) = iter.next() {
                    self.cell_stack.push(node.0 as u8);
                    self.curr = Some(node);
                    self.stack.push(iter);
                }
            } else {
                break;
            }
        }
        while let Some((digit, curr)) = self.curr.take() {
            self.cell_stack.swap(digit as u8);
            match curr {
                Node::Parent(children) => {
                    let mut iter = make_node_stack_iter_mut(children.as_mut());
                    self.curr = iter.next();
                    // This branch is not 100% necessary, but I prefer
                    // pushing an actual digit instead of 0 and
                    // relying on the swap the further up to replace
                    // it with the correct value.
                    if let Some((digit, _)) = self.curr {
                        self.cell_stack.push(digit as u8)
                    }
                    self.stack.push(iter);
                }
                Node::Leaf(value) => {
                    self.curr = None;
                    return Some((
                        *self.cell_stack.cell().expect("corrupted cell-stack"),
                        value,
                    ));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{Cell, HexTreeMap};
    use byteorder::{LittleEndian as LE, ReadBytesExt};
    use geo::polygon;
    use h3_lorawan_regions::compact::US915 as COMPACT_US915_INDICES;
    use h3o::{
        geom::{ContainmentMode, PolyfillConfig, Polygon, ToCells},
        CellIndex, Resolution,
    };
    use std::convert::TryFrom;

    #[test]
    fn test_visit() {
        let parent = Cell::try_from(0x825997fffffffff).unwrap();
        let children = [
            Cell::try_from(0x835990fffffffff).unwrap(),
            Cell::try_from(0x835991fffffffff).unwrap(),
            Cell::try_from(0x835992fffffffff).unwrap(),
            Cell::try_from(0x835993fffffffff).unwrap(),
            Cell::try_from(0x835994fffffffff).unwrap(),
            Cell::try_from(0x835995fffffffff).unwrap(),
            Cell::try_from(0x835996fffffffff).unwrap(),
        ];

        let hexmap: HexTreeMap<Cell> = children.iter().map(|cell| (cell, cell)).collect();
        let visited = hexmap.subtree_iter(parent).collect::<Vec<_>>();

        for (expected, (actual_k, actual_v)) in children.iter().zip(visited.iter()) {
            assert_eq!(expected, *actual_v);
            assert_eq!(expected.res(), actual_k.res());
            assert_eq!(expected, actual_k);
        }
        assert_eq!(children.len(), visited.len());
    }

    #[test]
    fn test_kv_iter_derives_key_cells() {
        // Create a map where the key==value
        let hexmap = {
            let mut map = HexTreeMap::new();
            for cell in COMPACT_US915_INDICES
                .iter()
                .map(|&idx| Cell::try_from(idx).unwrap())
            {
                map.insert(cell, cell);
            }
            map
        };
        // Assert that the cell keys derived while iterating the tree,
        // and returned by `next()`, are the same as those we called
        // `insert` with.
        assert!(hexmap.iter().all(|(k, v)| k == *v));
    }

    #[test]
    fn test_kv_iter_mut_derives_key_cells() {
        // Create a map where the key==value
        let mut hexmap = {
            let mut map = HexTreeMap::new();
            for cell in COMPACT_US915_INDICES
                .iter()
                .map(|&idx| Cell::try_from(idx).unwrap())
            {
                map.insert(cell, cell);
            }
            map
        };
        // Assert that the cell keys derived while iterating the tree,
        // and returned by `next()`, are the same as those we called
        // `insert` with.
        assert!(hexmap.iter_mut().all(|(k, v)| k == *v));
    }

    #[test]
    fn test_kv_iter_mut() {
        let idx_bytes = include_bytes!("../assets/monaco.res12.h3idx");
        let rdr = &mut idx_bytes.as_slice();

        let cell_value_pairs = {
            let mut cell_value_pairs: Vec<(Cell, i32)> = Vec::new();
            let mut count = 0;
            while let Ok(idx) = rdr.read_u64::<LE>() {
                cell_value_pairs.push((Cell::try_from(idx).unwrap(), count));
                count += 1;
            }
            cell_value_pairs
        };

        let map = {
            let mut map = HexTreeMap::new();
            for (cell, value) in cell_value_pairs.iter() {
                map.insert(*cell, *value);
            }
            map
        };

        let map_plus_one = {
            let mut map = map;
            for (_, value) in map.iter_mut() {
                *value += 1;
            }
            map
        };

        assert!(cell_value_pairs
            .iter()
            .all(|(cell, value)| map_plus_one[cell] == value + 1));
    }

    #[test]
    fn test_subtree_iter_sum() {
        // {
        //   "type": "FeatureCollection",
        //   "features": [
        //     {
        //       "type": "Feature",
        //       "properties": {},
        //       "geometry": {
        //         "coordinates": [
        //           [
        //             2.2918576408729336,
        //             48.85772170856845
        //           ],
        //           [
        //             2.295281693366718,
        //             48.86007711794011
        //           ],
        //           [
        //             2.2968743826623665,
        //             48.859023236935656
        //           ],
        //           [
        //             2.293404431342765,
        //             48.85672213596601
        //           ],
        //           [
        //             2.2918484611075485,
        //             48.85772774822141
        //           ]
        //         ],
        //         "type": "LineString"
        //       }
        //     }
        //   ],
        //   "bbox": null
        // }
        let eiffel_tower_cells = {
            let eiffel_tower_poly: geo::Polygon<f64> = polygon![
                (x: 2.2918576408729336, y: 48.85772170856845),
                (x: 2.295281693366718,  y: 48.86007711794011),
                (x: 2.2968743826623665, y: 48.859023236935656),
                (x: 2.293404431342765,  y: 48.85672213596601),
                (x: 2.2918484611075485, y: 48.85772774822141),
                (x: 2.2918576408729336, y: 48.85772170856845),
            ];
            let eiffel_tower_poly = Polygon::from_degrees(eiffel_tower_poly).unwrap();
            let mut eiffel_tower_cells: Vec<CellIndex> = eiffel_tower_poly
                .to_cells(
                    PolyfillConfig::new(Resolution::Twelve)
                        .containment_mode(ContainmentMode::ContainsCentroid),
                )
                .collect();
            eiffel_tower_cells.sort();
            eiffel_tower_cells.dedup();
            eiffel_tower_cells
                .into_iter()
                .map(|cell| Cell::try_from(u64::from(cell)).unwrap())
                .collect::<Vec<Cell>>()
        };
        let mut hex_map: HexTreeMap<i32> = eiffel_tower_cells
            .iter()
            .enumerate()
            .map(|(i, &cell)| (cell, i as i32))
            .collect();
        let eiffel_tower_res1_parent = Cell::try_from(0x811fbffffffffff).unwrap();
        let value_sum: i32 = hex_map
            .subtree_iter(eiffel_tower_res1_parent)
            .map(|(_cell, val)| val)
            .sum();
        // Establish the sum of map values in the eiffel tower block.
        assert_eq!(value_sum, 22578);

        let west_mask_res9 = Cell::try_from(0x891fb46741bffff).unwrap();
        let east_mask_res9 = Cell::try_from(0x891fb467413ffff).unwrap();
        let west_value_sum: i32 = hex_map
            .subtree_iter(west_mask_res9)
            .map(|(_cell, val)| val)
            .sum();
        let east_value_sum: i32 = hex_map
            .subtree_iter(east_mask_res9)
            .map(|(_cell, val)| val)
            .sum();
        // Now we have the sum of two difference subtrees, both of
        // which should cover the entire eiffel tower block. Therefore
        // they their individual sums should be the same as the
        // overall sum.
        assert_eq!(value_sum, west_value_sum + east_value_sum);

        let expected_sum = hex_map.len() as i32 + value_sum;

        for (_cell, val) in hex_map.subtree_iter_mut(eiffel_tower_res1_parent) {
            *val += 1;
        }

        let value_sum: i32 = hex_map
            .subtree_iter(eiffel_tower_res1_parent)
            .map(|(_cell, val)| val)
            .sum();

        assert_eq!(value_sum, expected_sum);
    }
}
