//! User definable compaction.

/// A user provided compactor.
pub trait Compactor<V> {
    /// Compact the thing.
    fn compact(&mut self, res: u8, children: [Option<&V>; 7]) -> Option<V>;
}

/// Compacts when all children are complete.
pub struct SetCompactor;

impl Compactor<()> for SetCompactor {
    fn compact(&mut self, _res: u8, children: [Option<&()>; 7]) -> Option<()> {
        if children.iter().all(Option::is_some) {
            Some(())
        } else {
            None
        }
    }
}

/// Compacts when all children are complete and have the same value.
pub struct EqCompactor;

impl<V: PartialEq + Clone> Compactor<V> for EqCompactor {
    fn compact(&mut self, _res: u8, children: [Option<&V>; 7]) -> Option<V> {
        if let [Some(v0), Some(v1), Some(v2), Some(v3), Some(v4), Some(v5), Some(v6)] = children {
            if v0 == v1 && v1 == v2 && v2 == v3 && v3 == v4 && v4 == v5 && v5 == v6 {
                return Some(v0.clone());
            }
        };
        None
    }
}

impl<V, F> Compactor<V> for F
where
    F: FnMut(u8, [Option<&V>; 7]) -> Option<V>,
{
    fn compact(&mut self, res: u8, children: [Option<&V>; 7]) -> Option<V> {
        self(res, children)
    }
}
