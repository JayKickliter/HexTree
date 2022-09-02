use easybench::bench;
use geo_types::coord;
use h3_lorawan_regions as regions;
use hexset::{
    h3ron::{H3Cell, Index},
    EqCompactor, HexMap, HexSet, SetCompactor,
};
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
fn time_creation() {
    let compacted_us915: Vec<H3Cell> = regions::compact::US915
        .iter()
        .map(|&idx| H3Cell::try_from(idx).unwrap())
        .collect();
    let plain_us915: Vec<H3Cell> = regions::nocompact::US915
        .iter()
        .map(|&idx| H3Cell::try_from(idx).unwrap())
        .collect();
    use std::time;
    let start = time::Instant::now();
    let us915_from_compacted_cells: HexSet = compacted_us915.iter().collect();
    let duration = time::Instant::now() - start;
    println!("US915 from precompacted cells {} ms", duration.as_millis());

    let start = time::Instant::now();
    let us915_from_plain_cells: HexSet = plain_us915.iter().collect();
    let duration = time::Instant::now() - start;
    println!("US915 from plain cells {} ms", duration.as_millis());
    assert!(us915_from_compacted_cells == us915_from_plain_cells);
}

#[test]
fn all_up() {
    let (us915_tree, us915_cells) = from_indicies(regions::compact::US915);
    assert_eq!(us915_tree.len(), us915_cells.len());

    let tarpon_springs = H3Cell::from_coordinate(coord! {x: -82.753822, y: 28.15215}, 12).unwrap();
    let gulf_of_mexico = H3Cell::from_coordinate(coord! {x: -83.101920, y: 28.128096}, 0).unwrap();
    let paris = H3Cell::from_coordinate(coord! {x: 2.340340, y: 48.868680}, 12).unwrap();

    assert!(us915_tree.contains(&tarpon_springs));
    assert!(naive_contains(&us915_cells, tarpon_springs));

    assert!(!us915_tree.contains(&gulf_of_mexico));
    assert!(!naive_contains(&us915_cells, gulf_of_mexico));

    assert!(!us915_tree.contains(&paris));
    assert!(!naive_contains(&us915_cells, paris));

    assert!(us915_cells.iter().all(|cell| us915_tree.contains(cell)));

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
        bench(|| us915_cells.iter().all(|cell| us915_tree.contains(cell)))
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
                assert!(cells_a.iter().all(|cell| tree_a.contains(cell)));
            } else {
                assert!(!cells_b.iter().any(|cell| tree_a.contains(cell)));
            }
        }
    }
}

#[test]
fn mono_hexmap() {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Region {
        AS923_1,
        AS923_1B,
        AS923_2,
        AS923_3,
        AS923_4,
        AU915,
        CN470,
        EU433,
        EU868,
        IN865,
        KR920,
        RU864,
        US915,
    }
    use Region::*;

    let regions = &[
        (AS923_1, regions::nocompact::AS923_1),
        (AS923_1B, regions::nocompact::AS923_1B),
        (AS923_2, regions::nocompact::AS923_2),
        (AS923_3, regions::nocompact::AS923_3),
        (AS923_4, regions::nocompact::AS923_4),
        (AU915, regions::nocompact::AU915),
        (CN470, regions::nocompact::CN470),
        (EU433, regions::nocompact::EU433),
        (EU868, regions::nocompact::EU868),
        (IN865, regions::nocompact::IN865),
        (KR920, regions::nocompact::KR920),
        (RU864, regions::nocompact::RU864),
        (US915, regions::nocompact::US915),
    ];

    let mut monomap = HexMap::new();
    let mut total_cells = 0;
    for (name, cells) in regions.iter() {
        total_cells += cells.len();
        for cell in cells.iter() {
            monomap.insert_and_compact(H3Cell::new(*cell), name, EqCompactor);
        }
    }
    println!(
        "total # of not-precompacted source cells for all {} regions: {}",
        regions.len(),
        total_cells
    );
    println!(
        "monomap.len() == {}: {}",
        monomap.len(),
        bench(|| monomap.len())
    );
    for (name, cells) in regions.iter() {
        println!(
            "monomap.get() returns correct region for all {} {:?} cells: {}",
            cells.len(),
            name,
            bench(|| cells
                .iter()
                .all(|c| monomap.get(&H3Cell::new(*c)) == Some(&name)))
        );

        assert!(cells
            .iter()
            .all(|c| monomap.get(&H3Cell::new(*c)) == Some(&name)));
    }
}

#[test]
fn test_compaction() {
    let (mut us915_tree, us915_cells) = from_indicies(regions::compact::US915);
    let (mut us915_nocompact_tree, us915_nocompact_cells) =
        from_indicies(regions::nocompact::US915);
    let gulf_of_mexico = H3Cell::from_coordinate(coord! {x: -83.101920, y: 28.128096}, 0).unwrap();
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
    us915_tree.insert_and_compact(gulf_of_mexico, (), SetCompactor);
    us915_nocompact_tree.insert_and_compact(gulf_of_mexico, (), SetCompactor);
    assert!(us915_tree.contains(&gulf_of_mexico));
    assert!(us915_nocompact_tree.contains(&gulf_of_mexico));
    assert_eq!(us915_tree.len(), us915_nocompact_tree.len());
}
