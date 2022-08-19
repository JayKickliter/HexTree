pub use h3ron;
use h3ron::{H3Cell, Index};
#[cfg(feature = "use-serde")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An HTree is a b(ish)-tree-like structure of hierarchical H3
/// hexagons, allowing for efficient region lookup.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "use-serde", derive(Serialize, Deserialize))]
pub struct HTree {
    /// All h3 0 base cell indices in the tree
    nodes: HashMap<u8, Node>,
}

// get all the Digits out of the cell
fn parse_h3cell(hex: H3Cell) -> Vec<usize> {
    let index = hex.h3index();
    let resolution = hex.resolution();

    if resolution == 0 {
        return Vec::new();
    }

    (0..resolution)
        .into_iter()
        .rev()
        .map(|r| {
            let offset = 0x2a - (3 * r);
            let digit = (index >> offset) & 0b111;
            assert!(digit < 7);
            digit as usize
        })
        .collect()
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "use-serde", derive(Serialize, Deserialize))]
struct Node {
    children: Box<[Option<Node>; 7]>,
}

impl Node {
    pub fn new() -> Self {
        Self {
            children: Box::new([None, None, None, None, None, None, None]),
        }
    }

    pub fn len(&self) -> usize {
        if self.is_full() {
            1
        } else {
            self.children
                .iter()
                .flatten()
                .map(|child| child.len())
                .sum()
        }
    }

    pub fn insert(&mut self, mut digits: Vec<usize>) {
        match digits.pop() {
            Some(digit) => {
                // TODO check if this node is "full"
                match self.children[digit].as_mut() {
                    Some(node) => node.insert(digits),
                    None => {
                        let mut node = Node::new();
                        node.insert(digits);
                        self.children[digit] = Some(node);
                    }
                }
            }
            None => (),
        }
    }

    pub fn is_full(&self) -> bool {
        self.children.iter().all(|c| c.is_none())
    }

    pub fn contains(&self, mut digits: Vec<usize>) -> bool {
        if self.is_full() {
            return true;
        }

        match digits.pop() {
            Some(digit) => {
                // TODO check if this node is "full"
                match &self.children[digit] {
                    Some(node) => node.contains(digits),
                    None => false,
                }
            }
            None => true,
        }
    }
}

impl HTree {
    /// Create a new HTree with given root resolution.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.values().map(|node| node.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, hex: H3Cell) {
        let base_cell = hex.base_cell_number();

        match self.nodes.get_mut(&base_cell) {
            Some(node) => node.insert(parse_h3cell(hex)),
            None => {
                let mut node = Node::new();
                node.insert(parse_h3cell(hex));
                self.nodes.insert(base_cell, node);
            }
        }
    }

    pub fn contains(&self, hex: H3Cell) -> bool {
        let base_cell = hex.base_cell_number();
        match self.nodes.get(&base_cell) {
            Some(node) => node.contains(parse_h3cell(hex)),
            None => false,
        }
    }
}

impl Default for HTree {
    fn default() -> Self {
        HTree::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{ReadBytesExt, LE};
    use easybench::bench;
    use geo_types::coord;
    use std::io::Cursor;

    static US915_SERIALIZED: &[u8] = include_bytes!("../test/US915.res7.h3idx");

    /// Perform a linear search of `region` for `target` cell.
    fn naive_contains(region: &[H3Cell], target: H3Cell) -> bool {
        let promotions = (0..16)
            .into_iter()
            .map(|res| {
                if res < target.resolution() {
                    target.get_parent(res).unwrap()
                } else {
                    target
                }
            })
            .collect::<Vec<H3Cell>>();
        for &cell in region {
            if cell == promotions[cell.resolution() as usize] {
                return true;
            }
        }
        false
    }

    #[test]
    fn all_up() {
        let mut hexagons: Vec<H3Cell> =
            Vec::with_capacity(US915_SERIALIZED.len() / std::mem::size_of::<H3Cell>());
        let mut csr = Cursor::new(US915_SERIALIZED);
        while let Ok(raw_index) = csr.read_u64::<LE>() {
            let cell = H3Cell::try_from(raw_index).unwrap();
            hexagons.push(cell);
        }
        assert!(!hexagons.is_empty());

        fn from_array(cells: &[H3Cell]) -> HTree {
            let mut tree = HTree::new();
            for cell in cells.into_iter() {
                tree.insert(*cell);
            }
            tree
        }

        let us915 = from_array(&hexagons);

        assert_eq!(us915.len(), hexagons.len());

        let tarpon_springs =
            H3Cell::from_coordinate(&coord! {x: -82.753822, y: 28.15215}, 12).unwrap();
        let gulf_of_mexico =
            H3Cell::from_coordinate(&coord! {x: -83.101920, y: 28.128096}, 12).unwrap();
        let paris = H3Cell::from_coordinate(&coord! {x: 2.340340, y: 48.868680}, 12).unwrap();

        assert!(us915.contains(tarpon_springs));
        assert!(naive_contains(&hexagons, tarpon_springs));

        assert!(!us915.contains(gulf_of_mexico));
        assert!(!naive_contains(&hexagons, gulf_of_mexico));

        assert!(!us915.contains(paris));
        assert!(!naive_contains(&hexagons, paris));

        println!("new from us915: {}", bench(|| from_array(&hexagons)));
        println!(
            "naive_contains(&hexagons, tarpon_springs): {}",
            bench(|| naive_contains(&hexagons, tarpon_springs))
        );
        println!(
            "us915.contains(tarpon_springs): {}",
            bench(|| us915.contains(tarpon_springs))
        );
        println!(
            "naive_contains(&hexagons, gulf_of_mexico): {}",
            bench(|| naive_contains(&hexagons, gulf_of_mexico))
        );
        println!(
            "us915.contains(gulf_of_mexico): {}",
            bench(|| us915.contains(tarpon_springs))
        );
        println!(
            "naive_contains(&hexagons, paris): {}",
            bench(|| naive_contains(&hexagons, paris))
        );
        println!("us915.contains(paris): {}", bench(|| us915.contains(paris)));
    }
}
