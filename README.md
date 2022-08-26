# HexSet

A HexSet is a data structure for efficient [point-in-polygon] testing
of geographic regions.

You, the user, create a set by inserting [H3 cells] into a
HexSet. Internally, HexSet decomposes cells into a tree of
resolution-0 cells at the root, branching through intermediate
resolutions until reaching the leaf cells you inserted.

HexSet automatically coalesces: on insert, any complete intermediate
cell in the tree is turned into a leaf cell. "Complete" is defined as
having all possibly child cells. Coalescing a cell allows for
optimized search, as any child cell of the coalesced cell is known to
be contained in the set.

[point-in-polygon]: https://en.wikipedia.org/wiki/Point_in_polygon
[H3 cells]: https://h3geo.org/docs/core-library/h3Indexing

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
