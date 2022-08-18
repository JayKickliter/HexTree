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

    let mut children = Vec::new();

    for r in 0..(resolution - 1) {
        let offset = 0x2a + ( 3 * r);
        children.push(((index >> offset) & 0b111) as usize);
    }
    children.reverse();
    return children;
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "use-serde", derive(Serialize, Deserialize))]
struct Node {
    children: Box<[Option<Node>; 7]>,
}

impl Node {
    pub fn new() -> Self {
        Self {
            children: Box::new([None, None, None, None, None, None, None])
        }
    }

    pub fn insert(&mut self, mut digits: Vec<usize>) {
        match digits.pop() {
            Some(digit) => {
                // TODO check if this node is "full"
                match &self.children[digit] {
                    Some(mut node) =>
                        node.insert(digits),
                    None => {
                        let mut node = Node::new();
                        node.insert(digits);
                        self.children[digit] = Some(node);
                    }
                }
            },
            None =>
                return
        }
    }

    pub fn contains(&self, mut digits: Vec<usize>) -> bool {
        match digits.pop() {
            Some(digit) => {
                // TODO check if this node is "full"
                match &self.children[digit] {
                    Some(node) =>
                        node.contains(digits),
                    None => {
                        false
                    }
                }
            },
            None =>
                true
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

    pub fn insert(&mut self, hex: H3Cell) {
        let base_cell = hex.base_cell_number();

        match &self.nodes.get(&base_cell) {
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
            None => false
        }
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

        let tarpon_springs =
            H3Cell::from_coordinate(&coord! {x: -82.753822, y: 28.15215}, 12).unwrap();
        let gulf_of_mexico =
            H3Cell::from_coordinate(&coord! {x: -83.101920, y: 28.128096}, 12).unwrap();
        let paris = H3Cell::from_coordinate(&coord! {x: 2.340340, y: 48.868680}, 12).unwrap();

        assert!(us915.contains(tarpon_springs));
        assert!(!us915.contains(gulf_of_mexico));
        assert!(!us915.contains(paris));

        println!(
            "new from us915: {}",
            bench(|| from_array(&hexagons))
        );
        println!(
            "us915.contains(tarpon_springs): {}",
            bench(|| us915.contains(tarpon_springs))
        );
        println!(
            "us915.contains(gulf_of_mexico): {}",
            bench(|| us915.contains(tarpon_springs))
        );
        println!("us915.contains(paris): {}", bench(|| us915.contains(paris)));
    }
}
