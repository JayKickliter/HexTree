use crate::{
    compaction::Compactor,
    digits::Digits,
    h3ron::{H3Cell, Index},
};

// TODO: storing indices in nodes is not necessary, since the index
// can always be derived by the path through the tree to get to the
// node. That said, storing the index doesn't impose much lookup
// overhead.
//
// The benefit of storing indices is vastly simpler Hex+Value
// iteration of a tree.
#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde-support",
    derive(serde::Serialize, serde::Deserialize)
)]
#[repr(align(64))]
pub(crate) enum Node<V> {
    Parent(H3Cell, [Option<Box<Node<V>>>; 7]),
    Leaf(H3Cell, V),
}

impl<V> Node<V> {
    pub(crate) fn new(hex: H3Cell) -> Self {
        Self::Parent(hex, [None, None, None, None, None, None, None])
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Leaf(_, _) => 1,
            Self::Parent(_, children) => children.iter().flatten().map(|child| child.len()).sum(),
        }
    }

    pub(crate) fn insert<C>(
        &mut self,
        hex: H3Cell,
        res: u8,
        mut digits: Digits,
        value: V,
        compactor: &mut C,
    ) where
        C: Compactor<V>,
    {
        match digits.next() {
            None => {
                debug_assert_eq!(res, hex.resolution());
                *self = Self::Leaf(hex, value)
            }
            Some(digit) => match self {
                Self::Leaf(leaf_hex, _) => {
                    debug_assert_eq!(*leaf_hex, hex.get_parent(res).unwrap());
                    return;
                }
                Self::Parent(parent_hex, children) => {
                    debug_assert_eq!(parent_hex.resolution(), res);
                    match children[digit as usize].as_mut() {
                        Some(node) => node.insert(hex, res + 1, digits, value, compactor),
                        None => {
                            let mut node = Node::new(
                                hex.get_parent(res + 1)
                                    .expect("Digits returned Some, promotion should work"),
                            );
                            node.insert(hex, res + 1, digits, value, compactor);
                            children[digit as usize] = Some(Box::new(node));
                        }
                    };
                }
            },
        };
        self.coalesce(res, compactor);
    }

    pub(crate) fn coalesce<C>(&mut self, res: u8, compactor: &mut C)
    where
        C: Compactor<V>,
    {
        if let Self::Parent(hex, children) = self {
            debug_assert_eq!(hex.resolution(), res);
            if children
                .iter()
                .any(|n| matches!(n.as_ref().map(|n| n.as_ref()), Some(Self::Parent(_, _))))
            {
                return;
            }
            let mut arr: [Option<&V>; 7] = [None, None, None, None, None, None, None];
            for (v, n) in arr.iter_mut().zip(children.iter()) {
                *v = n.as_ref().map(|n| n.as_ref()).and_then(Node::value);
            }
            if let Some(value) = compactor.compact(res, arr) {
                *self = Self::Leaf(*hex, value)
            }
        };
    }

    pub(crate) fn value(&self) -> Option<&V> {
        match self {
            Self::Leaf(_, value) => Some(value),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn contains(&self, mut digits: Digits) -> bool {
        match (digits.next(), self) {
            (_, Self::Leaf(_, _)) => true,
            (Some(digit), Self::Parent(_, children)) => {
                // TODO check if this node is "full"
                match &children.as_slice()[digit as usize] {
                    Some(node) => node.contains(digits),
                    None => false,
                }
            }
            // No digits left, but `self` isn't full, so this hex
            // can't fully contain the target.
            (None, Self::Parent(_, _)) => false,
        }
    }

    pub(crate) fn get(&self, mut digits: Digits) -> Option<&V> {
        if let Self::Leaf(_, val) = self {
            return Some(val);
        }

        match (digits.next(), self) {
            (_, Self::Leaf(_, _)) => unreachable!(),
            (Some(digit), Self::Parent(_, children)) => {
                match &children.as_slice()[digit as usize] {
                    Some(node) => node.get(digits),
                    None => None,
                }
            }
            // No digits left, but `self` isn't full, so this hex
            // can't fully contain the target.
            (None, Self::Parent(_, _)) => None,
        }
    }

    pub(crate) fn get_mut(&mut self, mut digits: Digits) -> Option<&mut V> {
        if let Self::Leaf(_, val) = self {
            return Some(val);
        }
        match (digits.next(), self) {
            (_, Self::Leaf(_, _)) => unreachable!(),
            (Some(digit), Self::Parent(_, children)) => {
                match &mut children.as_mut_slice()[digit as usize] {
                    Some(node) => node.get_mut(digits),
                    None => None,
                }
            }
            // No digits left, but `self` isn't full, so this hex
            // can't fully contain the target.
            (None, Self::Parent(_, _)) => None,
        }
    }
}
