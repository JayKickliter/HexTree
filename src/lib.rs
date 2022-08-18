pub use h3ron;
use h3ron::{H3Cell, HasH3Resolution, Index};
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
        let promoted = hex.get_parent(self.resolution() + 1).unwrap();
        if self.hex == hex {
            // We're inserting a hex that covers all possible
            // children, therefore we can coalesce.
            self.children = None
        } else if let Some(children) = self.children.as_mut() {
            match children.binary_search_by_key(&promoted, |node| node.hex) {
                Ok(pos) => children[pos].insert(hex),
                Err(pos) => {
                    let mut node = Node::new(promoted);
                    node.insert(hex);
                    children.insert(pos, node)
                }
            }
        } else {
            let mut node = Node::new(promoted);
            node.insert(hex);
            self.children = Some(vec![node])
        }
    }

    pub fn resolution(&self) -> u8 {
        self.hex.resolution()
    }

    pub fn contains(&self, hex: H3Cell) -> bool {
        assert!(!(hex == self.hex && !self.children.is_none()));
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
        assert!(hex.h3_resolution() >= self.root_res);
        let promoted = hex.get_parent(self.root_res).unwrap();
        match self.nodes.binary_search_by_key(&promoted, |node| node.hex) {
            Ok(pos) => {
                self.nodes[pos].insert(hex);
            }
            Err(pos) => {
                let mut node = Node::new(promoted);
                node.insert(hex);
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
