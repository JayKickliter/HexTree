[![CI](https://github.com/JayKickliter/HexTree/actions/workflows/rust.yml/badge.svg)](https://github.com/JayKickliter/HexTree/actions/workflows/rust.yml) [![Documentation](https://docs.rs/hextree/badge.svg)](https://docs.rs/hextree)

# HexTree

HexTree provides tree structures for efficiently representing geographic
regions using [H3 cell]s. It takes advantage of H3's hierarchical structure
to automatically compact large regions and provide fast spatial queries.

The primary structures are:

- [**HexTreeMap**]: an H3 cell-to-value map.
- [**HexTreeSet**]: an H3 cell set for spatial containment testing.

You can think of `HexTreeMap` vs. `HexTreeSet` as [`HashMap`] vs. [`HashSet`].

## How is this different from `HashMap<H3Cell, V>`?

HexTree leverages H3's hierarchical cell structure in two key ways:

**Hierarchical Queries**: When you query for a cell, the tree returns
a value even if only a parent cell was inserted. For instance, if you
insert a low-res cell but later query for a higher-res child cell, the
tree returns the value from the parent.

**Automatic Compaction**: With [compaction], the tree can automatically
coalesce 7 adjacent child cells into their parent cell, dramatically
reducing memory usage. For very large regions, compaction can continue
recursively to the lowest resolution cells (res-0), possibly removing
millions of redundant cells. For example, 4,795,661 res-7 cells
representing North America compact [into just 42,383 elements][us915].

The internal structure mirrors H3's hierarchy: the root contains 122
resolution-0 base cells, with each level below being a 7-ary tree
(matching H3's 7 possible child cells per parent). The tree supports
up to 15 levels of resolution, where the depth of a leaf node corresponds
to its H3 cell resolution.

## Features

* **`serde`**: support for serialization via [serde].
* **`disktree`**: on-disk memory-mapped storage for large trees (enables `serde`, `byteorder`, and `memmap`).

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE] or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license ([LICENSE-MIT] or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the
Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.


[`HashMap`]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
[`HashSet`]: https://doc.rust-lang.org/std/collections/struct.HashSet.html
[H3 cell]: https://h3geo.org/docs/core-library/h3Indexing
[serde]: https://docs.rs/serde/latest/serde
[compaction]: crate::compaction
[us915]: https://kepler.gl/demo?mapUrl=https://gist.githubusercontent.com/JayKickliter/8f91a8437b7dd89321b22cde50e71c3a/raw/4aafc62303d913edf58ac1bb7b3b656c8df188a1/us915.kepler.json
[**HexTreeMap**]: crate::HexTreeMap
[**HexTreeSet**]: crate::HexTreeSet
[LICENSE-APACHE]: https://github.com/JayKickliter/HexTree/blob/main/LICENSE-APACHE
[LICENSE-MIT]: https://github.com/JayKickliter/HexTree/blob/main/LICENSE-MIT
