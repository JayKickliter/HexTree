use crate::{compactor::Compactor, digits::Digits};
use std::mem;

#[derive(Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde-support",
    derive(serde::Serialize, serde::Deserialize)
)]
pub(crate) enum Node<V> {
    Parent([Option<Box<Node<V>>>; 7]),
    Leaf(V),
}

impl<V> Node<V> {
    pub(crate) fn mem_size(&self) -> usize {
        mem::size_of::<Self>()
            + match self {
                Self::Leaf(_) => 0,
                Self::Parent(children) => children
                    .iter()
                    .flatten()
                    .map(|n| n.mem_size())
                    .sum::<usize>(),
            }
    }

    pub(crate) fn new() -> Self {
        Self::Parent([None, None, None, None, None, None, None])
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Leaf(_) => 1,
            Self::Parent(children) => children.iter().flatten().map(|child| child.len()).sum(),
        }
    }

    pub(crate) fn insert(&mut self, mut digits: Digits, value: V) {
        match digits.next() {
            None => *self = Self::Leaf(value),
            Some(digit) => match self {
                Self::Leaf(_) => (),
                Self::Parent(children) => {
                    match children[digit as usize].as_mut() {
                        Some(node) => node.insert(digits, value),
                        None => {
                            let mut node = Node::new();
                            node.insert(digits, value);
                            children[digit as usize] = Some(Box::new(node));
                        }
                    };
                }
            },
        };
    }

    pub(crate) fn insert_and_compact<C>(
        &mut self,
        res: u8,
        mut digits: Digits,
        value: V,
        compactor: &mut C,
    ) where
        C: Compactor<V>,
    {
        match digits.next() {
            None => *self = Self::Leaf(value),
            Some(digit) => match self {
                Self::Leaf(_) => return,
                Self::Parent(children) => {
                    match children[digit as usize].as_mut() {
                        Some(node) => node.insert_and_compact(res + 1, digits, value, compactor),
                        None => {
                            let mut node = Node::new();
                            node.insert_and_compact(res + 1, digits, value, compactor);
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
        if let Self::Parent(children) = self {
            if children
                .iter()
                .any(|n| matches!(n.as_ref().map(|n| n.as_ref()), Some(Self::Parent(_))))
            {
                return;
            }
            let mut arr: [Option<&V>; 7] = [None, None, None, None, None, None, None];
            for (v, n) in arr.iter_mut().zip(children.iter()) {
                *v = n.as_ref().map(|n| n.as_ref()).and_then(Node::value);
            }
            if let Some(value) = compactor.compact(res, arr) {
                *self = Self::Leaf(value)
            }
        };
    }

    pub(crate) fn value(&self) -> Option<&V> {
        match self {
            Self::Leaf(value) => Some(value),
            _ => None,
        }
    }

    #[inline]
    pub(crate) fn contains(&self, mut digits: Digits) -> bool {
        match (digits.next(), self) {
            (_, Self::Leaf(_)) => true,
            (Some(digit), Self::Parent(children)) => {
                // TODO check if this node is "full"
                match &children.as_slice()[digit as usize] {
                    Some(node) => node.contains(digits),
                    None => false,
                }
            }
            // No digits left, but `self` isn't full, so this hex
            // can't fully contain the target.
            (None, Self::Parent(_)) => false,
        }
    }

    pub(crate) fn get(&self, mut digits: Digits) -> Option<&V> {
        if let Self::Leaf(val) = self {
            return Some(val);
        }

        match (digits.next(), self) {
            (_, Self::Leaf(_)) => unreachable!(),
            (Some(digit), Self::Parent(children)) => {
                // TODO check if this node is "full"
                match &children.as_slice()[digit as usize] {
                    Some(node) => node.get(digits),
                    None => None,
                }
            }
            // No digits left, but `self` isn't full, so this hex
            // can't fully contain the target.
            (None, Self::Parent(_)) => None,
        }
    }
}
