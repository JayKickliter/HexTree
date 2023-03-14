[![CI](https://github.com/JayKickliter/HexTree/actions/workflows/rust.yml/badge.svg)](https://github.com/JayKickliter/HexTree/actions/workflows/rust.yml) [![Documentation](https://docs.rs/hextree/badge.svg)](https://docs.rs/hextree)

# HexTree

hextree provides tree structures that represent geographic regions
with [H3 cell]s.

The primary structures are:

- **HexTreeMap**: an H3 cell-to-value map.
- **HexTreeSet**: an H3 cell set for hit-testing.

You can think of `HexTreeMap` vs. `HexTreeSet` as [`HashMap`] vs. [`HashSet`].

## How is this different from `HashMap<H3Cell, V>`?

The key feature of a hextree is that its keys (H3 cells) are
hierarchical. For instance, if you previously inserted an entry for a
low-res cell, but later query for a higher-res child cell, the tree
returns the value for the lower res cell. Additionally, with
[compaction], trees can automatically coalesce adjacent high-res cells
into their parent cell. For very large regions, the compaction process
_can_ continue to lowest resolution cells (res-0), possibly removing
millions of redundant cells from the tree. For example, a set of
4,795,661 res-7 cells representing North America coalesces [into a
42,383 element `HexTreeSet`][us915].

A hextree's internal structure exactly matches the semantics of an [H3
cell]. The root of the tree has 122 resolution-0 nodes, followed by 15
levels of 7-ary nodes. The level of an occupied node, or leaf node, is
the same as its corresponding H3 cell resolution.

## Features

* **`serde-support`**: support for serialization via [serde].

[`HashMap`]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
[`HashSet`]: https://doc.rust-lang.org/std/collections/struct.HashSet.html
[H3 cell]: https://h3geo.org/docs/core-library/h3Indexing
[serde]: https://docs.rs/serde/latest/serde
[compaction]: crate::compaction
[us915]: https://www.google.com/maps/d/u/0/edit?mid=15wRzxmtmyzqf6fHU3yuW4hJAM9MoxLJs

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
