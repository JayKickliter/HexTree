use geo_types::coord;
use h3_lorawan_regions as regions;
use h3ron::H3Cell;
use hextree::{compaction::EqCompactor, HexTreeMap, HexTreeSet, Index};
use std::convert::TryFrom;

/// Perform a linear search of `region` for `target` cell.
fn naive_contains(region: &[Index], target: Index) -> bool {
    let promotions = (0..16)
        .into_iter()
        .map(|res| {
            if res < target.resolution() {
                target.parent(res).unwrap()
            } else {
                target
            }
        })
        .collect::<Vec<Index>>();
    for &cell in region {
        if cell == promotions[cell.resolution() as usize] {
            return true;
        }
    }
    false
}

fn from_indicies(indicies: &[u64]) -> (HexTreeSet, Vec<Index>) {
    let cells: Vec<Index> = indicies
        .iter()
        .map(|&idx| Index::try_from(idx).unwrap())
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
    let tarpon_springs = Index::try_from(*tarpon_springs).unwrap();
    let gulf_of_mexico = Index::try_from(*gulf_of_mexico).unwrap();
    let paris = Index::try_from(*paris).unwrap();

    assert!(us915_tree.contains(tarpon_springs));
    assert!(naive_contains(&us915_cells, tarpon_springs));

    assert!(!us915_tree.contains(gulf_of_mexico));
    assert!(!naive_contains(&us915_cells, gulf_of_mexico));

    assert!(!us915_tree.contains(paris));
    assert!(!naive_contains(&us915_cells, paris));

    assert!(us915_cells.iter().all(|&cell| us915_tree.contains(cell)));
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
            monomap.insert(Index::try_from(*cell).unwrap(), name);
        }
    }

    for (name, cells) in regions.iter() {
        assert!(cells
            .iter()
            .all(|c| monomap.get(Index::try_from(*c).unwrap()) == Some(&name)));
    }
}

#[test]
fn test_compaction() {
    let (mut us915_tree, us915_cells) = from_indicies(regions::compact::US915);
    let (mut us915_nocompact_tree, us915_nocompact_cells) =
        from_indicies(regions::nocompact::US915);
    let gulf_of_mexico = H3Cell::from_coordinate(coord! {x: -83.101920, y: 28.128096}, 0).unwrap();
    let gulf_of_mexico = Index::try_from(*gulf_of_mexico).unwrap();
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
