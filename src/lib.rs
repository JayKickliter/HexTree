//! An `HTree` is a b(ish)-tree-like structure of hierarchical H3
//! hexagons, allowing for efficient region lookup.

pub use h3ron;
use h3ron::{H3Cell, Index};
use std::{iter::FromIterator, mem::size_of, ops::Deref, ops::DerefMut};

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "with-serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HexSet {
    /// All h3 0 base cell indices in the tree
    nodes: Box<[Option<Node>]>,
}

impl HexSet {
    /// Create an empty `HexSet`.
    pub fn new() -> Self {
        Self {
            nodes: vec![None; 122].into_boxed_slice(),
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.iter().flatten().map(|node| node.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn insert(&mut self, hex: H3Cell) {
        let base_cell = base(&hex);
        let digits = Digits::new(hex);
        match self.nodes[base_cell as usize].as_mut() {
            Some(node) => node.insert(digits),
            None => {
                let mut node = Node::new();
                node.insert(digits);
                self.nodes[base_cell as usize] = Some(node);
            }
        }
    }

    pub fn contains(&self, hex: &H3Cell) -> bool {
        let base_cell = base(hex);
        match self.nodes[base_cell as usize].as_ref() {
            Some(node) => {
                let digits = Digits::new(*hex);
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

impl FromIterator<H3Cell> for HexSet {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = H3Cell>,
    {
        let mut set = HexSet::new();
        for cell in iter {
            set.insert(cell);
        }
        set
    }
}

impl<'a> FromIterator<&'a H3Cell> for HexSet {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = &'a H3Cell>,
    {
        let mut set = HexSet::new();
        for cell in iter {
            set.insert(*cell);
        }
        set
    }
}

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(feature = "with-serde", derive(serde::Serialize, serde::Deserialize))]
struct Node(Box<[Option<Node>; 7]>);

impl Node {
    fn mem_size(&self) -> usize {
        size_of::<Self>() + self.iter().flatten().map(|n| n.mem_size()).sum::<usize>()
    }

    fn new() -> Self {
        Self(Box::new([None, None, None, None, None, None, None]))
    }

    fn len(&self) -> usize {
        if self.is_full() {
            1
        } else {
            self.iter().flatten().map(|child| child.len()).sum()
        }
    }

    fn insert(&mut self, mut digits: Digits) {
        match digits.next() {
            Some(digit) => match self[digit as usize].as_mut() {
                Some(node) => node.insert(digits),
                None => {
                    let mut node = Node::new();
                    node.insert(digits);
                    self[digit as usize] = Some(node);
                }
            },
            None => *self.0 = [None, None, None, None, None, None, None],
        };
        self.coalesce();
    }

    fn coalesce(&mut self) {
        if let [Some(n0), Some(n1), Some(n2), Some(n3), Some(n4), Some(n5), Some(n6)] = &*self.0 {
            if n0.is_full()
                && n1.is_full()
                && n2.is_full()
                && n3.is_full()
                && n4.is_full()
                && n5.is_full()
                && n6.is_full()
            {
                *self.0 = [None, None, None, None, None, None, None]
            }
        };
    }

    fn is_full(&self) -> bool {
        self.iter().all(|c| c.is_none())
    }

    fn contains(&self, mut digits: Digits) -> bool {
        if self.is_full() {
            return true;
        }

        match digits.next() {
            Some(digit) => {
                // TODO check if this node is "full"
                match &self[digit as usize] {
                    Some(node) => node.contains(digits),
                    None => false,
                }
            }
            // No digits left, but `self` isn't full, so this hex
            // can't fully contain the target.
            None => false,
        }
    }
}

impl Deref for Node {
    type Target = [Option<Node>];

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0[..]
    }
}

struct Digits {
    digits: u64,
    remaining: u8,
}

impl Digits {
    fn new(cell: H3Cell) -> Self {
        let res = cell.resolution();
        let mask = u128::MAX.wrapping_shl(64 - (3 * res as u32)) as u64;
        let digits: u64 = cell.h3index().wrapping_shl(19) & mask;
        Self {
            digits,
            remaining: res,
        }
    }
}

impl Iterator for Digits {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            let out = (self.digits & (0b111 << 61)) >> 61;
            self.digits <<= 3;
            debug_assert!(out < 7);
            self.remaining -= 1;
            Some(out as u8)
        }
    }
}

impl Default for HexSet {
    fn default() -> Self {
        HexSet::new()
    }
}

/// Returns a cell's base.
fn base(cell: &H3Cell) -> u8 {
    let index = cell.h3index();
    let base = (index >> 0x2D) & 0b111_1111;
    base as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use easybench::bench;
    use geo_types::coord;
    use h3_lorawan_regions as regions;
    use std::convert::TryFrom;

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

    fn from_indicies(indicies: &[u64]) -> (HexSet, Vec<H3Cell>) {
        let cells: Vec<H3Cell> = indicies
            .iter()
            .map(|&idx| H3Cell::try_from(idx).unwrap())
            .collect();
        let set: HexSet = cells.iter().collect();
        (set, cells)
    }

    #[test]
    fn all_up() {
        let (us915_tree, us915_cells) = from_indicies(regions::compact::US915);
        assert_eq!(us915_tree.len(), us915_cells.len());

        let tarpon_springs =
            H3Cell::from_coordinate(&coord! {x: -82.753822, y: 28.15215}, 12).unwrap();
        let gulf_of_mexico =
            H3Cell::from_coordinate(&coord! {x: -83.101920, y: 28.128096}, 0).unwrap();
        let paris = H3Cell::from_coordinate(&coord! {x: 2.340340, y: 48.868680}, 12).unwrap();

        assert!(us915_tree.contains(&tarpon_springs));
        assert!(naive_contains(&us915_cells, tarpon_springs));

        assert!(!us915_tree.contains(&gulf_of_mexico));
        assert!(!naive_contains(&us915_cells, gulf_of_mexico));

        assert!(!us915_tree.contains(&paris));
        assert!(!naive_contains(&us915_cells, paris));

        assert!(us915_cells.iter().all(|cell| us915_tree.contains(&*cell)));

        println!(
            "new from us915: {}",
            bench(|| us915_cells.iter().collect::<HexSet>())
        );
        println!(
            "naive_contains(&us915_cells, tarpon_springs): {}",
            bench(|| naive_contains(&us915_cells, tarpon_springs))
        );
        println!(
            "us915.contains(&tarpon_springs): {}",
            bench(|| us915_tree.contains(&tarpon_springs))
        );
        println!(
            "naive_contains(&us915_cells, gulf_of_mexico): {}",
            bench(|| naive_contains(&us915_cells, gulf_of_mexico))
        );
        println!(
            "us915.contains(&gulf_of_mexico): {}",
            bench(|| us915_tree.contains(&tarpon_springs))
        );
        println!(
            "naive_contains(&us915_cells, paris): {}",
            bench(|| naive_contains(&us915_cells, paris))
        );
        println!(
            "us915.contains(&paris): {}",
            bench(|| us915_tree.contains(&paris))
        );

        println!(
            "us915_cells.iter().all(|cell| us915.contains(&*cell)): {}",
            bench(|| us915_cells.iter().all(|cell| us915_tree.contains(&*cell)))
        );
    }

    #[test]
    fn all_regions() {
        let regions = &[
            ("AS923_1", from_indicies(regions::compact::AS923_1)),
            ("AS923_1B", from_indicies(regions::compact::AS923_1B)),
            ("AS923_2", from_indicies(regions::compact::AS923_2)),
            ("AS923_3", from_indicies(regions::compact::AS923_3)),
            ("AS923_4", from_indicies(regions::compact::AS923_4)),
            ("AU915", from_indicies(regions::compact::AU915)),
            ("CN470", from_indicies(regions::compact::CN470)),
            ("EU433", from_indicies(regions::compact::EU433)),
            ("EU868", from_indicies(regions::compact::EU868)),
            ("IN865", from_indicies(regions::compact::IN865)),
            ("KR920", from_indicies(regions::compact::KR920)),
            ("RU864", from_indicies(regions::compact::RU864)),
            ("US915", from_indicies(regions::compact::US915)),
        ];

        // Do membership tests across the cartesian product off all regions
        for (name_a, (tree_a, cells_a)) in regions.iter() {
            for (name_b, (_tree_b, cells_b)) in regions.iter() {
                if name_a == name_b {
                    assert_eq!(tree_a.len(), cells_a.len());
                    assert!(cells_a.iter().all(|cell| tree_a.contains(&*cell)));
                } else {
                    assert!(!cells_b.iter().any(|cell| tree_a.contains(&*cell)));
                }
            }
        }
    }

    #[test]
    fn test_digits() {
        let test_cases: &[(u64, &[u8])] = &[
            (577164439745200127, &[]),                    // res 0
            (585793956755800063, &[2, 0]),                // res 2
            (592638622797135871, &[6, 3, 2]),             // res 3
            (596251300178427903, &[3, 6, 6, 2]),          // res 4
            (599803672997658623, &[3, 4, 4, 1, 4]),       // res 5
            (604614882611953663, &[1, 4, 0, 4, 1, 0]),    // res 6
            (608557861265473535, &[2, 0, 2, 3, 2, 1, 1]), // res 7
        ];

        for (index, ref_digits) in test_cases {
            let cell = H3Cell::new(*index);
            let digits = Digits::new(cell).collect::<Vec<u8>>();
            assert_eq!(&&digits, ref_digits);
        }
    }

    #[test]
    fn test_compaction() {
        let (mut us915_tree, us915_cells) = from_indicies(regions::compact::US915);
        let (mut us915_nocompact_tree, us915_nocompact_cells) =
            from_indicies(regions::nocompact::US915);
        let gulf_of_mexico =
            H3Cell::from_coordinate(&coord! {x: -83.101920, y: 28.128096}, 0).unwrap();
        assert_eq!(us915_tree.len(), us915_nocompact_tree.len());
        assert!(us915_tree == us915_nocompact_tree);
        assert!(us915_nocompact_tree.len() < us915_nocompact_cells.len());
        assert!(us915_nocompact_cells
            .iter()
            .all(|&c| us915_nocompact_tree.contains(&c)));
        assert!(us915_cells
            .iter()
            .all(|&c| us915_nocompact_tree.contains(&c)));
        assert!(us915_nocompact_cells
            .iter()
            .all(|&c| us915_tree.contains(&c)));

        assert!(!us915_tree.contains(&gulf_of_mexico));
        assert!(!us915_nocompact_tree.contains(&gulf_of_mexico));
        us915_tree.insert(gulf_of_mexico);
        us915_nocompact_tree.insert(gulf_of_mexico);
        assert!(us915_tree.contains(&gulf_of_mexico));
        assert!(us915_nocompact_tree.contains(&gulf_of_mexico));
        assert_eq!(us915_tree.len(), us915_nocompact_tree.len());
    }

    #[test]
    fn test_mem_size() {
        // Sanity check that `Option<Node>` behaves the same as
        // `Option<Box<[Option<Node>; 7]>>` in that it uses `NULL` to
        // represent the `None` variant.
        assert_eq!(size_of::<Option<Node>>(), size_of::<*const ()>());
        assert_eq!(
            size_of::<Option<Node>>(),
            size_of::<Option<Box<[Option<Node>; 7]>>>()
        );
    }
}
