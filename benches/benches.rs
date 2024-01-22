use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use geo::{coord, polygon};
use h3_lorawan_regions::{
    compact::US915 as COMPACT_US915_INDICES, nocompact::US915 as PLAIN_US915_INDICES,
};
use h3o::{
    geom::{ContainmentMode, PolyfillConfig, Polygon, ToCells},
    CellIndex, Resolution,
};
use h3ron::H3Cell;
use hextree::{compaction::EqCompactor, Cell, HexTreeMap, HexTreeSet};
use std::convert::TryFrom;

fn set_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("US915 HexTreeSet lookup");

    let us915_set: HexTreeSet = PLAIN_US915_INDICES
        .iter()
        .map(|&idx| Cell::try_from(idx).unwrap())
        .collect();

    let tarpon_springs = coord! {x: -82.753822, y: 28.15215};
    let gulf_of_mexico = coord! {x: -83.101920, y: 28.128096};
    let paris = coord! {x: 2.340340, y: 48.868680};

    for resolution in [0, 4, 8, 12, 15] {
        let tarpon_springs =
            Cell::try_from(*H3Cell::from_coordinate(tarpon_springs, resolution).unwrap()).unwrap();
        let gulf_of_mexico =
            Cell::try_from(*H3Cell::from_coordinate(gulf_of_mexico, resolution).unwrap()).unwrap();
        let paris = Cell::try_from(*H3Cell::from_coordinate(paris, resolution).unwrap()).unwrap();

        group.bench_with_input(
            BenchmarkId::new("Tarpon Spring", resolution),
            &tarpon_springs,
            |b, &cell| b.iter(|| us915_set.contains(cell)),
        );

        group.bench_with_input(
            BenchmarkId::new("Gulf of Mexico", resolution),
            &gulf_of_mexico,
            |b, &cell| b.iter(|| us915_set.contains(cell)),
        );

        group.bench_with_input(BenchmarkId::new("Paris", resolution), &paris, |b, &cell| {
            b.iter(|| us915_set.contains(cell))
        });
    }
}

#[cfg(not(feature = "disktree"))]
fn disk_set_lookup(_c: &mut Criterion) {}

#[cfg(feature = "disktree")]
fn disk_set_lookup(c: &mut Criterion) {
    use hextree::disktree::DiskTreeMap;
    let mut group = c.benchmark_group("US915 DiskTreeSet lookup");

    let us915_disk_set = {
        let us915_set: HexTreeSet = PLAIN_US915_INDICES
            .iter()
            .map(|&idx| Cell::try_from(idx).unwrap())
            .collect();
        let mut file = tempfile::tempfile().unwrap();
        us915_set
            .to_disktree(&mut file, |_, _| Ok::<(), std::io::Error>(()))
            .unwrap();
        DiskTreeMap::memmap(&file).unwrap()
    };

    let tarpon_springs = coord! {x: -82.753822, y: 28.15215};
    let gulf_of_mexico = coord! {x: -83.101920, y: 28.128096};
    let paris = coord! {x: 2.340340, y: 48.868680};

    for resolution in [0, 4, 8, 12, 15] {
        let tarpon_springs =
            Cell::try_from(*H3Cell::from_coordinate(tarpon_springs, resolution).unwrap()).unwrap();
        let gulf_of_mexico =
            Cell::try_from(*H3Cell::from_coordinate(gulf_of_mexico, resolution).unwrap()).unwrap();
        let paris = Cell::try_from(*H3Cell::from_coordinate(paris, resolution).unwrap()).unwrap();

        group.bench_with_input(
            BenchmarkId::new("Tarpon Spring", resolution),
            &tarpon_springs,
            |b, &cell| b.iter(|| us915_disk_set.contains(cell)),
        );

        group.bench_with_input(
            BenchmarkId::new("Gulf of Mexico", resolution),
            &gulf_of_mexico,
            |b, &cell| b.iter(|| us915_disk_set.contains(cell)),
        );

        group.bench_with_input(BenchmarkId::new("Paris", resolution), &paris, |b, &cell| {
            b.iter(|| us915_disk_set.contains(cell))
        });
    }
}

fn set_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("US915 HexTreeSet construction");

    let precompacted_us915_cells: Vec<Cell> = COMPACT_US915_INDICES
        .iter()
        .map(|&idx| Cell::try_from(idx).unwrap())
        .collect();
    let plain_us915_cells: Vec<Cell> = PLAIN_US915_INDICES
        .iter()
        .map(|&idx| Cell::try_from(idx).unwrap())
        .collect();

    group.bench_function("pre-compacted", |b| {
        b.iter(|| precompacted_us915_cells.iter().collect::<HexTreeSet>())
    });

    group.bench_function("plain", |b| {
        b.iter(|| plain_us915_cells.iter().collect::<HexTreeSet>())
    });
}

fn map_lookup(c: &mut Criterion) {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[allow(dead_code)]
    enum Region {
        EU868,
        US915,
    }

    let mut group = c.benchmark_group("US915 HexTreeMap lookup");

    let mut us915_map = HexTreeMap::with_compactor(EqCompactor);
    us915_map.extend(
        PLAIN_US915_INDICES
            .iter()
            .map(|&idx| Cell::try_from(idx).unwrap())
            .zip(std::iter::repeat(Region::US915)),
    );

    let tarpon_springs = coord! {x: -82.753822, y: 28.15215};
    let gulf_of_mexico = coord! {x: -83.101920, y: 28.128096};
    let paris = coord! {x: 2.340340, y: 48.868680};

    for resolution in [0, 4, 8, 12, 15] {
        let tarpon_springs =
            Cell::try_from(*H3Cell::from_coordinate(tarpon_springs, resolution).unwrap()).unwrap();
        let gulf_of_mexico =
            Cell::try_from(*H3Cell::from_coordinate(gulf_of_mexico, resolution).unwrap()).unwrap();
        let paris = Cell::try_from(*H3Cell::from_coordinate(paris, resolution).unwrap()).unwrap();

        group.bench_with_input(
            BenchmarkId::new("Tarpon Spring", resolution),
            &tarpon_springs,
            |b, &cell| b.iter(|| us915_map.get(cell)),
        );

        group.bench_with_input(
            BenchmarkId::new("Gulf of Mexico", resolution),
            &gulf_of_mexico,
            |b, &cell| b.iter(|| us915_map.get(cell)),
        );

        group.bench_with_input(BenchmarkId::new("Paris", resolution), &paris, |b, &cell| {
            b.iter(|| us915_map.get(cell))
        });
    }
}

fn map_construction(c: &mut Criterion) {
    // The value type for the map
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[allow(dead_code)]
    enum Region {
        EU868,
        US915,
    }

    let mut group = c.benchmark_group("US915 HexTreeMap construction");

    let precompacted_us915_cells: Vec<Cell> = COMPACT_US915_INDICES
        .iter()
        .map(|&idx| Cell::try_from(idx).unwrap())
        .collect();
    let plain_us915_cells: Vec<Cell> = PLAIN_US915_INDICES
        .iter()
        .map(|&idx| Cell::try_from(idx).unwrap())
        .collect();

    group.bench_function("pre-compacted", |b| {
        b.iter(|| {
            let mut map = HexTreeMap::with_compactor(EqCompactor);
            map.extend(
                precompacted_us915_cells
                    .iter()
                    .zip(std::iter::repeat(&black_box(Region::US915)))
                    .map(|(c, v)| (*c, *v)),
            );
            map
        })
    });

    group.bench_function("plain", |b| {
        b.iter(|| {
            let mut map = HexTreeMap::with_compactor(EqCompactor);
            map.extend(
                plain_us915_cells
                    .iter()
                    .zip(std::iter::repeat(&black_box(Region::US915)))
                    .map(|(c, v)| (*c, *v)),
            );
            map
        })
    });
}

fn map_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("US915 pre-compacted HexTreeMap iteration");

    let map_precompacted: HexTreeMap<u32> = COMPACT_US915_INDICES
        .iter()
        .map(|&idx| Cell::try_from(idx).unwrap())
        .zip(0..)
        .collect();

    group.bench_function("collect to vec", |b| {
        b.iter(|| {
            let out: Vec<(Cell, &u32)> = map_precompacted.iter().collect();
            out
        })
    });

    group.bench_function("count", |b| b.iter(|| map_precompacted.iter().count()));
}

fn set_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("US915 pre-compacted HexTreeSet iteration");

    let set_precompacted: HexTreeSet = COMPACT_US915_INDICES
        .iter()
        .map(|&idx| Cell::try_from(idx).unwrap())
        .collect();

    group.bench_function("collect to vec", |b| {
        b.iter(|| {
            let out: Vec<Cell> = set_precompacted.iter().map(|cv| cv.0).collect();
            out
        })
    });

    group.bench_function("count", |b| b.iter(|| set_precompacted.iter().count()));
}

fn subtree_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("Subtree iteration");

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
                    .containment_mode(ContainmentMode::IntersectsBoundary),
            )
            .collect();
        eiffel_tower_cells.sort();
        eiffel_tower_cells.dedup();
        eiffel_tower_cells
            .into_iter()
            .map(|cell| Cell::try_from(u64::from(cell)).unwrap())
            .collect::<Vec<Cell>>()
    };
    let hex_map: HexTreeMap<i32> = eiffel_tower_cells
        .iter()
        .enumerate()
        .map(|(i, &cell)| (cell, i as i32))
        .collect();

    let eiffel_tower_res1_parent = eiffel_tower_cells[0].to_parent(1).unwrap();
    group.bench_function("Eiffel Tower Sum - Res1", |b| {
        b.iter(|| {
            hex_map
                .subtree_iter(eiffel_tower_res1_parent)
                .map(|(_cell, val)| val)
                .sum::<i32>()
        })
    });

    let eiffel_tower_res6_parent = eiffel_tower_cells[0].to_parent(7).unwrap();
    group.bench_function("Eiffel Tower Sum - Res7", |b| {
        b.iter(|| {
            hex_map
                .subtree_iter(eiffel_tower_res6_parent)
                .map(|(_cell, val)| val)
                .sum::<i32>()
        })
    });
}

criterion_group!(
    benches,
    set_lookup,
    disk_set_lookup,
    subtree_iter,
    map_lookup,
    set_iteration,
    map_iteration,
    set_construction,
    map_construction,
);
criterion_main!(benches);
