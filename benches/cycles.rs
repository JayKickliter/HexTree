use geo_types::coord;
use h3_lorawan_regions::{
    compact::US915 as COMPACT_US915_INDICES, nocompact::US915 as PLAIN_US915_INDICES,
};
use hextree::{compaction::EqCompactor, h3ron::H3Cell, HexMap, HexSet};
use iai::{black_box, main};
use std::convert::TryFrom;

fn main() {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    #[allow(dead_code)]
    enum Region {
        EU868,
        US915,
    }

    let mut us915_hexmap = HexMap::with_compactor(EqCompactor);
    us915_hexmap.extend(
        PLAIN_US915_INDICES
            .iter()
            .map(|&idx| H3Cell::try_from(idx).unwrap())
            .zip(std::iter::repeat(Region::US915)),
    );

    let tarpon_springs = coord! {x: -82.753822, y: 28.15215};
    let gulf_of_mexico = coord! {x: -83.101920, y: 28.128096};
    let paris = coord! {x: 2.340340, y: 48.868680};

    // [0, 4, 8, 12, 15]
    let tarpon_springs_15 = H3Cell::from_coordinate(tarpon_springs, 15).unwrap();
    let gulf_of_mexico_15 = H3Cell::from_coordinate(gulf_of_mexico, 15).unwrap();
    let paris_15 = H3Cell::from_coordinate(paris, 15).unwrap();

    let benchmarks: &[&(&'static str, fn())] = &[&("us915/get/res15/tarpon-springs", || {
        let _ =
            black_box(us915_hexmap.contains(
                H3Cell::from_coordinate(coord! {x: -82.753822, y: 28.15215}, 15).unwrap(),
            ));
    })];
    iai::runner(benchmarks);
}
