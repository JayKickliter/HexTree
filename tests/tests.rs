use geo::coord;
use h3_lorawan_regions as regions;
use h3ron::H3Cell;
use hextree::{compaction::EqCompactor, Cell, HexTreeMap, HexTreeSet};

/// Perform a linear search of `region` for `target` cell.
fn naive_contains(region: &[Cell], target: Cell) -> bool {
    let promotions = (0..16)
        .map(|res| {
            if res < target.res() {
                target.to_parent(res).unwrap()
            } else {
                target
            }
        })
        .collect::<Vec<Cell>>();
    for &cell in region {
        if cell == promotions[cell.res() as usize] {
            return true;
        }
    }
    false
}

fn from_indicies(indicies: &[u64]) -> (HexTreeSet, Vec<Cell>) {
    let cells: Vec<Cell> = indicies
        .iter()
        .map(|&idx| Cell::from_raw(idx).unwrap())
        .collect();
    let set: HexTreeSet = cells.iter().collect();
    (set, cells)
}

#[test]
fn all_up() {
    let (us915_tree, us915_cells) = from_indicies(regions::compact::US915);
    assert_eq!(us915_tree.len(), us915_cells.len());

    let tarpon_springs = H3Cell::from_coordinate(coord! {x: -82.753822, y: 28.15215}, 12).unwrap();
    let gulf_of_mexico = H3Cell::from_coordinate(coord! {x: -83.101920, y: 28.128096}, 0).unwrap();
    let paris = H3Cell::from_coordinate(coord! {x: 2.340340, y: 48.868680}, 12).unwrap();
    let tarpon_springs = Cell::from_raw(*tarpon_springs).unwrap();
    let gulf_of_mexico = Cell::from_raw(*gulf_of_mexico).unwrap();
    let paris = Cell::from_raw(*paris).unwrap();

    assert!(us915_tree.contains(tarpon_springs));
    assert!(naive_contains(&us915_cells, tarpon_springs));

    assert!(!us915_tree.contains(gulf_of_mexico));
    assert!(!naive_contains(&us915_cells, gulf_of_mexico));

    assert!(!us915_tree.contains(paris));
    assert!(!naive_contains(&us915_cells, paris));

    assert!(us915_cells
        .iter()
        .all(|&cell| { us915_tree.get(cell).unwrap().0 == cell }));

    for expected in us915_cells.iter().filter(|cell| cell.res() > 0) {
        let parent_to_expected = expected.to_parent(expected.res() - 1).unwrap();
        let subtree = us915_tree
            .subtree_iter(parent_to_expected)
            .map(|(cell, _)| cell);
        let subcells = subtree.collect::<Vec<Cell>>();
        assert_ne!(subcells.len(), 0);
        for subcell in subcells {
            assert_eq!(
                subcell.to_parent(parent_to_expected.res()).unwrap(),
                parent_to_expected
            );
        }
    }

    // https://wolf-h3-viewer.glitch.me/?h3=812a3ffffffffff
    let northeast_res1 = Cell::from_raw(0x812a3ffffffffff).unwrap();

    // Lets get rid of all raw cells not under northeast_res1.
    let expected_north_cells = {
        let mut expected_north_cells = us915_cells
            .iter()
            .filter(|&&cell| cell.res() > 1 && cell.is_related_to(&northeast_res1))
            .copied()
            .collect::<Vec<Cell>>();
        expected_north_cells.sort_by_key(|cell| cell.into_raw());
        expected_north_cells
    };

    let subtree_cells = {
        let mut subtree_cells = us915_tree
            .subtree_iter(northeast_res1)
            .map(|(cell, _)| cell)
            .collect::<Vec<Cell>>();
        subtree_cells.sort_by_key(|cell| cell.into_raw());
        subtree_cells
    };

    assert_eq!(expected_north_cells, subtree_cells);
}

#[test]
fn mono_map() {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Region {
        EU868,
        US915,
    }
    use Region::*;

    let regions = &[
        (EU868, regions::nocompact::EU868),
        (US915, regions::nocompact::US915),
    ];

    let mut monomap = HexTreeMap::with_compactor(EqCompactor);

    for (name, cells) in regions.iter() {
        for cell in cells.iter() {
            monomap.insert(Cell::from_raw(*cell).unwrap(), name);
        }
    }

    for (name, cells) in regions.iter() {
        assert!(cells.iter().map(|c| Cell::from_raw(*c).unwrap()).all(|c| {
            if let Some((cell, val)) = monomap.get(c) {
                c.to_parent(cell.res()) == Some(cell) && val == &name
            } else {
                false
            }
        }));
    }
}

#[test]
fn test_compaction() {
    let (mut us915_tree, us915_cells) = from_indicies(regions::compact::US915);
    let (mut us915_nocompact_tree, us915_nocompact_cells) =
        from_indicies(regions::nocompact::US915);
    let gulf_of_mexico = H3Cell::from_coordinate(coord! {x: -83.101920, y: 28.128096}, 0).unwrap();
    let gulf_of_mexico = Cell::from_raw(*gulf_of_mexico).unwrap();
    assert_eq!(us915_tree.len(), us915_nocompact_tree.len());
    assert!(us915_tree == us915_nocompact_tree);
    assert!(us915_nocompact_tree.len() < us915_nocompact_cells.len());
    assert!(us915_nocompact_cells
        .iter()
        .all(|&c| us915_nocompact_tree.contains(c)));
    assert!(us915_cells
        .iter()
        .all(|&c| us915_nocompact_tree.contains(c)));
    assert!(us915_nocompact_cells
        .iter()
        .all(|&c| us915_tree.contains(c)));

    assert!(!us915_tree.contains(gulf_of_mexico));
    assert!(!us915_nocompact_tree.contains(gulf_of_mexico));
    us915_tree.insert(gulf_of_mexico, ());
    us915_nocompact_tree.insert(gulf_of_mexico, ());
    assert!(us915_tree.contains(gulf_of_mexico));
    assert!(us915_nocompact_tree.contains(gulf_of_mexico));
    assert_eq!(us915_tree.len(), us915_nocompact_tree.len());
}
