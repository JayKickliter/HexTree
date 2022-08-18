pub use h3ron;
use h3ron::{H3Cell, Index};
#[cfg(feature = "use-serde")]
use serde::{Deserialize, Serialize};

/// An HTree is a b(ish)-tree-like structure of hierarchical H3
/// hexagons, allowing for efficient region lookup.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "use-serde", derive(Serialize, Deserialize))]
pub struct HTree {
    /// First level, and coarsest, H3 resolution of the tree.
    root_res: u8,
    nodes: Vec<Node>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "use-serde", derive(Serialize, Deserialize))]
struct Node {
    hex: H3Cell,
    children: Option<Vec<Node>>,
}

impl Node {
    pub fn new(hex: H3Cell) -> Self {
        Self {
            hex,
            children: None,
        }
    }

    pub fn insert(&mut self, hex: H3Cell) {
        assert!(hex.resolution() > self.resolution() || hex == self.hex);
        // hex reinterpreted at the same resolution of self.children
        let promoted = if hex.resolution() == self.resolution() + 1 {
            hex
        } else {
            hex.get_parent(self.resolution() + 1).unwrap()
        };

        if self.hex == hex {
            // We're inserting a hex that covers all possible
            // children, therefore we can coalesce.
            self.children = None
        } else if let Some(children) = self.children.as_mut() {
            match children.binary_search_by_key(&promoted, |node| node.hex) {
                Ok(pos) => children[pos].insert(hex),
                Err(pos) => {
                    let mut node = Node::new(promoted);
                    if promoted != hex {
                        node.insert(hex);
                    }
                    children.insert(pos, node)
                }
            }
        } else {
            let mut node = Node::new(promoted);
            if promoted != hex {
                node.insert(hex);
            }
            self.children = Some(vec![node])
        }
    }

    pub fn resolution(&self) -> u8 {
        self.hex.resolution()
    }

    pub fn contains(&self, hex: H3Cell) -> bool {
        assert!(hex != self.hex || self.children.is_none());
        assert!(hex.resolution() >= self.hex.resolution());

        if !self.hex.is_parent_of(&hex) {
            // Simplest case: hex is outside of self
            return false;
        }

        if self.children.is_none() {
            // self is a leaf node, and we already know self is a
            // parent, therefore hex is a member
            return true;
        }

        // hex reinterpreted at the same resolution of self.children
        let promoted = hex.get_parent(self.resolution() + 1).unwrap();
        if let Ok(pos) = self
            .children
            .as_ref()
            .expect("already checked !is_none()")
            .binary_search_by_key(&promoted, |node| node.hex)
        {
            self.children.as_ref().expect("already checked !is_none()")[pos].contains(hex)
        } else {
            false
        }
    }
}

impl HTree {
    /// Create a new HTree with given root resolution.
    pub fn new(root_res: u8) -> Self {
        Self {
            root_res,
            nodes: Vec::new(),
        }
    }

    pub fn insert(&mut self, hex: H3Cell) {
        assert!(hex.resolution() >= self.root_res);
        let promoted = if hex.resolution() == self.root_res {
            hex
        } else {
            hex.get_parent(self.root_res).unwrap()
        };
        match self.nodes.binary_search_by_key(&promoted, |node| node.hex) {
            Ok(pos) => {
                self.nodes[pos].insert(hex);
            }
            Err(pos) => {
                let mut node = Node::new(promoted);
                if hex.resolution() > self.root_res {
                    node.insert(hex);
                }
                self.nodes.insert(pos, node);
            }
        }
    }

    pub fn contains(&self, hex: H3Cell) -> bool {
        assert!(hex.resolution() >= self.root_res);
        let promoted = hex.get_parent(self.root_res).unwrap();
        if let Ok(pos) = self.nodes.binary_search_by_key(&promoted, |node| node.hex) {
            self.nodes[pos].contains(hex)
        } else {
            false
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
        let mut base_res = u8::MAX;
        while let Ok(raw_index) = csr.read_u64::<LE>() {
            let cell = H3Cell::try_from(raw_index).unwrap();
            base_res = std::cmp::min(base_res, cell.resolution());
            hexagons.push(cell);
        }
        assert!(!hexagons.is_empty());

        let tree = {
            let mut tree = HTree::new(base_res);
            for hex in hexagons.into_iter() {
                tree.insert(hex);
            }
            tree
        };

        let tarpon_springs =
            H3Cell::from_coordinate(&coord! {x: -82.753822, y: 28.15215}, 12).unwrap();
        let gulf_of_mexico =
            H3Cell::from_coordinate(&coord! {x: -83.101920, y: 28.128096}, 12).unwrap();
        let paris = H3Cell::from_coordinate(&coord! {x: 2.340340, y: 48.868680}, 12).unwrap();
        assert!(tree.contains(tarpon_springs));
        assert!(!tree.contains(gulf_of_mexico));
        assert!(!tree.contains(paris));
        println!(
            "tree.contains(tarpon_springs): {}",
            bench(|| tree.contains(tarpon_springs))
        );
        println!(
            "tree.contains(gulf_of_mexico): {}",
            bench(|| tree.contains(tarpon_springs))
        );
        println!(
            "tree.contains(gulf_of_mexico): {}",
            bench(|| tree.contains(paris))
        );
    }
}
