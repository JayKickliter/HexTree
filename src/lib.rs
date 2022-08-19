use h3ron::{H3Cell, Index};
#[cfg(feature = "use-serde")]
use serde::{Deserialize, Serialize};
use std::mem::size_of;

/// An `HTree` is a b(ish)-tree-like structure of hierarchical H3
/// hexagons, allowing for efficient region lookup.
#[derive(Clone)]
#[cfg_attr(feature = "use-serde", derive(Serialize, Deserialize))]
pub struct HexSet {
    /// All h3 0 base cell indices in the tree
    nodes: Box<[Option<Node>]>,
}

// get all the Digits out of the cell
fn parse_h3cell(hex: H3Cell, out: &mut [u8]) {
    let index = hex.h3index();
    let resolution = hex.resolution();

    for (r, o) in (0..resolution).into_iter().zip(out) {
        let offset = 0x2a - (3 * r);
        let digit = (index >> offset) & 0b111;
        assert!(digit < 7);
        *o = digit as u8;
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "use-serde", derive(Serialize, Deserialize))]
struct Node {
    children: Box<[Option<Node>; 7]>,
}

impl Node {
    pub fn mem_size(&self) -> usize {
        size_of::<Self>()
            + self
                .children
                .iter()
                .flatten()
                .map(|n| n.mem_size())
                .sum::<usize>()
    }

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

    pub fn insert(&mut self, digits: &[u8]) {
        match digits.split_first() {
            Some((&digit, rest)) => match self.children[digit as usize].as_mut() {
                Some(node) => node.insert(rest),
                None => {
                    let mut node = Node::new();
                    node.insert(rest);
                    self.children[digit as usize] = Some(node);
                }
            },
            None => (),
        };
    }

    pub fn is_full(&self) -> bool {
        self.children.iter().all(|c| c.is_none())
    }

    pub fn contains(&self, digits: &[u8]) -> bool {
        if self.is_full() {
            return true;
        }

        match digits.split_first() {
            Some((&digit, rest)) => {
                // TODO check if this node is "full"
                match &self.children[digit as usize] {
                    Some(node) => node.contains(rest),
                    None => false,
                }
            }
            None => true,
        }
    }
}

impl HexSet {
    /// Create a new `HTree` with given root resolution.
    pub fn new() -> Self {
        Self {
            nodes: vec![None; 128].into_boxed_slice(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.iter().flatten().map(|node| node.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, hex: H3Cell) {
        let base_cell = hex.base_cell_number();
        let mut digit_buf = [0; 16];
        parse_h3cell(hex, &mut digit_buf[..hex.resolution() as usize]);
        let digits = &digit_buf[..hex.resolution() as usize];
        match self.nodes[base_cell as usize].as_mut() {
            Some(node) => node.insert(digits),
            None => {
                let mut node = Node::new();
                node.insert(digits);
                self.nodes[base_cell as usize] = Some(node);
            }
        }
    }

    pub fn contains(&self, hex: H3Cell) -> bool {
        let base_cell = hex.base_cell_number();
        match self.nodes[base_cell as usize].as_ref() {
            Some(node) => {
                let mut digit_buf = [0; 16];
                parse_h3cell(hex, &mut digit_buf[..hex.resolution() as usize]);
                let digits = &digit_buf[..hex.resolution() as usize];
                node.contains(digits)
            }
            None => false,
        }
    }

    /// Returns the current memory use of this `HexSet`.
    ///
    /// Note: due to memory alignment, the actual total may be higher
    ///       than reported.
    pub fn mem_size(&self) -> usize {
        size_of::<Self>()
            + self
                .nodes
                .iter()
                .flatten()
                .map(|n| n.mem_size())
                .sum::<usize>()
    }
}

impl Default for HexSet {
    fn default() -> Self {
        HexSet::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{ReadBytesExt, LE};
    use easybench::bench;
    use geo_types::coord;
    use std::io::Cursor;

    static AS923_1_SERIALIZED: &[u8] = include_bytes!("../test/AS923-1.res7.h3idx");
    static AS923_1B_SERIALIZED: &[u8] = include_bytes!("../test/AS923-1B.res7.h3idx");
    static AS923_2_SERIALIZED: &[u8] = include_bytes!("../test/AS923-2.res7.h3idx");
    static AS923_3_SERIALIZED: &[u8] = include_bytes!("../test/AS923-3.res7.h3idx");
    static AS923_4_SERIALIZED: &[u8] = include_bytes!("../test/AS923-4.res7.h3idx");
    static AU915_SERIALIZED: &[u8] = include_bytes!("../test/AU915.res7.h3idx");
    static CN470_SERIALIZED: &[u8] = include_bytes!("../test/CN470.res7.h3idx");
    static EU433_SERIALIZED: &[u8] = include_bytes!("../test/EU433.res7.h3idx");
    static EU868_SERIALIZED: &[u8] = include_bytes!("../test/EU868.res7.h3idx");
    static IN865_SERIALIZED: &[u8] = include_bytes!("../test/IN865.res7.h3idx");
    static KR920_SERIALIZED: &[u8] = include_bytes!("../test/KR920.res7.h3idx");
    static RU864_SERIALIZED: &[u8] = include_bytes!("../test/RU864.res7.h3idx");
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

    fn from_array(cells: &[H3Cell]) -> HexSet {
        let mut tree = HexSet::new();
        for cell in cells.iter() {
            tree.insert(*cell);
        }
        tree
    }

    fn from_serialized(serialized: &[u8]) -> (HexSet, Vec<H3Cell>) {
        let mut hexagons: Vec<H3Cell> =
            Vec::with_capacity(serialized.len() / std::mem::size_of::<H3Cell>());
        let mut csr = Cursor::new(serialized);
        while let Ok(raw_index) = csr.read_u64::<LE>() {
            let cell = H3Cell::try_from(raw_index).unwrap();
            hexagons.push(cell);
        }
        assert!(!hexagons.is_empty());
        let tree = from_array(&hexagons);
        (tree, hexagons)
    }

    #[test]
    fn all_up() {
        let (us915_tree, us915_cells) = from_serialized(US915_SERIALIZED);

        assert_eq!(us915_tree.len(), us915_cells.len());

        let tarpon_springs =
            H3Cell::from_coordinate(&coord! {x: -82.753822, y: 28.15215}, 12).unwrap();
        let gulf_of_mexico =
            H3Cell::from_coordinate(&coord! {x: -83.101920, y: 28.128096}, 12).unwrap();
        let paris = H3Cell::from_coordinate(&coord! {x: 2.340340, y: 48.868680}, 12).unwrap();

        assert!(us915_tree.contains(tarpon_springs));
        assert!(naive_contains(&us915_cells, tarpon_springs));

        assert!(!us915_tree.contains(gulf_of_mexico));
        assert!(!naive_contains(&us915_cells, gulf_of_mexico));

        assert!(!us915_tree.contains(paris));
        assert!(!naive_contains(&us915_cells, paris));

        assert!(us915_cells.iter().all(|cell| us915_tree.contains(*cell)));

        println!("new from us915: {}", bench(|| from_array(&us915_cells)));
        println!(
            "naive_contains(&us915_cells, tarpon_springs): {}",
            bench(|| naive_contains(&us915_cells, tarpon_springs))
        );
        println!(
            "us915.contains(tarpon_springs): {}",
            bench(|| us915_tree.contains(tarpon_springs))
        );
        println!(
            "naive_contains(&us915_cells, gulf_of_mexico): {}",
            bench(|| naive_contains(&us915_cells, gulf_of_mexico))
        );
        println!(
            "us915.contains(gulf_of_mexico): {}",
            bench(|| us915_tree.contains(tarpon_springs))
        );
        println!(
            "naive_contains(&us915_cells, paris): {}",
            bench(|| naive_contains(&us915_cells, paris))
        );
        println!(
            "us915.contains(paris): {}",
            bench(|| us915_tree.contains(paris))
        );

        println!(
            "us915_cells.iter().all(|cell| us915.contains(*cell)): {}",
            bench(|| us915_cells.iter().all(|cell| us915_tree.contains(*cell)))
        );
    }

    #[test]
    fn all_regions() {
        let regions = &[
            ("AS923_1", from_serialized(AS923_1_SERIALIZED)),
            ("AS923_1B", from_serialized(AS923_1B_SERIALIZED)),
            ("AS923_2", from_serialized(AS923_2_SERIALIZED)),
            ("AS923_3", from_serialized(AS923_3_SERIALIZED)),
            ("AS923_4", from_serialized(AS923_4_SERIALIZED)),
            ("AU915", from_serialized(AU915_SERIALIZED)),
            ("CN470", from_serialized(CN470_SERIALIZED)),
            ("EU433", from_serialized(EU433_SERIALIZED)),
            ("EU868", from_serialized(EU868_SERIALIZED)),
            ("IN865", from_serialized(IN865_SERIALIZED)),
            ("KR920", from_serialized(KR920_SERIALIZED)),
            ("RU864", from_serialized(RU864_SERIALIZED)),
            ("US915", from_serialized(US915_SERIALIZED)),
        ];

        // Do membership tests across the cartesian product off all regions
        for (name_a, (tree_a, cells_a)) in regions.iter() {
            for (name_b, (_tree_b, cells_b)) in regions.iter() {
                if name_a == name_b {
                    assert_eq!(tree_a.len(), cells_a.len());
                    assert!(cells_a.iter().all(|cell| tree_a.contains(*cell)));
                } else {
                    assert!(!cells_b.iter().any(|cell| tree_a.contains(*cell)));
                }
            }
        }
    }
}
