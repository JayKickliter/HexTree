use crate::{cell::CellStack, node::Node, Cell};
use std::iter::{Enumerate, FlatMap};

type NodeStackIter<'a, V> = FlatMap<
    Enumerate<std::slice::Iter<'a, Option<Box<Node<V>>>>>,
    Option<(usize, &'a Node<V>)>,
    fn((usize, &'a Option<Box<Node<V>>>)) -> Option<(usize, &'a Node<V>)>,
>;

fn make_node_stack_iter<'a, V>(nodes: &'a [Option<Box<Node<V>>>]) -> NodeStackIter<'a, V> {
    fn map_fn<V>(item: (usize, &Option<Box<Node<V>>>)) -> Option<(usize, &Node<V>)> {
        if let (digit, Some(val)) = item {
            Some((digit, val))
        } else {
            None
        }
    }

    nodes
        .iter()
        .enumerate()
        .flat_map(map_fn as fn((_, &'a Option<Box<Node<V>>>)) -> Option<(_, &'a Node<V>)>)
}

pub(crate) struct Iter<'a, V> {
    stack: Vec<NodeStackIter<'a, V>>,
    curr: Option<(usize, &'a Node<V>)>,
    cell_stack: CellStack,
}

impl<'a, V> Iter<'a, V> {
    pub(crate) fn new(base: &'a [Option<Box<Node<V>>>]) -> Self {
        let mut iter = make_node_stack_iter(base);
        let curr = iter.next();
        let mut stack = Vec::with_capacity(16);
        stack.push(iter);
        let mut cell_stack = CellStack::new();
        if let Some((digit, _)) = curr {
            cell_stack.push(digit as u8)
        }
        Self {
            stack,
            curr,
            cell_stack,
        }
    }
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (Cell, &'a V);

    fn next(&mut self) -> Option<(Cell, &'a V)> {
        while self.curr.is_none() {
            if let Some(mut iter) = self.stack.pop() {
                self.cell_stack.pop();
                if let Some(node) = iter.next() {
                    self.cell_stack.push(node.0 as u8);
                    self.curr = Some(node);
                    self.stack.push(iter);
                }
            } else {
                break;
            }
        }
        while let Some((digit, curr)) = self.curr {
            self.cell_stack.swap(digit as u8);
            match curr {
                Node::Parent(children) => {
                    let mut iter = make_node_stack_iter(children.as_ref());
                    self.curr = iter.next();
                    // This branch is not 100% necessary, but I prefer
                    // pushing an actual digit instead of 0 and
                    // relying on the swap the further up to replace
                    // it with the correct value.
                    if let Some((digit, _)) = self.curr {
                        self.cell_stack.push(digit as u8)
                    }
                    self.stack.push(iter);
                }
                Node::Leaf(value) => {
                    self.curr = None;
                    return Some((
                        *self.cell_stack.cell().expect("corrupted cell-stack"),
                        value,
                    ));
                }
            }
        }
        None
    }
}

type NodeStackIterMut<'a, V> = FlatMap<
    Enumerate<std::slice::IterMut<'a, Option<Box<Node<V>>>>>,
    Option<(usize, &'a mut Node<V>)>,
    fn((usize, &'a mut Option<Box<Node<V>>>)) -> Option<(usize, &'a mut Node<V>)>,
>;

fn make_node_stack_iter_mut<'a, V>(
    nodes: &'a mut [Option<Box<Node<V>>>],
) -> NodeStackIterMut<'a, V> {
    fn map_fn_mut<V>(item: (usize, &mut Option<Box<Node<V>>>)) -> Option<(usize, &mut Node<V>)> {
        if let (digit, Some(val)) = item {
            Some((digit, val))
        } else {
            None
        }
    }

    nodes.iter_mut().enumerate().flat_map(
        map_fn_mut as fn((_, &'a mut Option<Box<Node<V>>>)) -> Option<(_, &'a mut Node<V>)>,
    )
}

pub(crate) struct IterMut<'a, V> {
    stack: Vec<NodeStackIterMut<'a, V>>,
    curr: Option<(usize, &'a mut Node<V>)>,
    cell_stack: CellStack,
}

impl<'a, V> IterMut<'a, V> {
    pub(crate) fn new(base: &'a mut [Option<Box<Node<V>>>]) -> Self {
        let mut iter = make_node_stack_iter_mut(base);
        let curr = iter.next();
        let mut stack = Vec::with_capacity(16);
        stack.push(iter);
        let mut cell_stack = CellStack::new();
        if let Some((digit, _)) = curr {
            cell_stack.push(digit as u8)
        }
        Self {
            stack,
            curr,
            cell_stack,
        }
    }
}

impl<'a, V> Iterator for IterMut<'a, V> {
    type Item = (Cell, &'a mut V);

    fn next(&mut self) -> Option<(Cell, &'a mut V)> {
        while self.curr.is_none() {
            if let Some(mut iter) = self.stack.pop() {
                self.cell_stack.pop();
                if let Some(node) = iter.next() {
                    self.cell_stack.push(node.0 as u8);
                    self.curr = Some(node);
                    self.stack.push(iter);
                }
            } else {
                break;
            }
        }
        while let Some((digit, curr)) = self.curr.take() {
            self.cell_stack.swap(digit as u8);
            match curr {
                Node::Parent(children) => {
                    let mut iter = make_node_stack_iter_mut(children.as_mut());
                    self.curr = iter.next();
                    // This branch is not 100% necessary, but I prefer
                    // pushing an actual digit instead of 0 and
                    // relying on the swap the further up to replace
                    // it with the correct value.
                    if let Some((digit, _)) = self.curr {
                        self.cell_stack.push(digit as u8)
                    }
                    self.stack.push(iter);
                }
                Node::Leaf(value) => {
                    self.curr = None;
                    return Some((
                        *self.cell_stack.cell().expect("corrupted cell-stack"),
                        value,
                    ));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{Cell, HexTreeMap};
    use byteorder::{LittleEndian as LE, ReadBytesExt};
    use h3_lorawan_regions::compact::US915 as COMPACT_US915_INDICES;
    use std::convert::TryFrom;

    #[test]
    fn test_kv_iter_derives_key_cells() {
        // Create a map where the key==value
        let hexmap = {
            let mut map = HexTreeMap::new();
            for cell in COMPACT_US915_INDICES
                .iter()
                .map(|&idx| Cell::try_from(idx).unwrap())
            {
                map.insert(cell, cell);
            }
            map
        };
        // Assert that the cell keys derived while iterating the tree,
        // and returned by `next()`, are the same as those we called
        // `insert` with.
        assert!(hexmap.iter().all(|(k, v)| k == *v));
    }

    #[test]
    fn test_kv_iter_mut_derives_key_cells() {
        // Create a map where the key==value
        let mut hexmap = {
            let mut map = HexTreeMap::new();
            for cell in COMPACT_US915_INDICES
                .iter()
                .map(|&idx| Cell::try_from(idx).unwrap())
            {
                map.insert(cell, cell);
            }
            map
        };
        // Assert that the cell keys derived while iterating the tree,
        // and returned by `next()`, are the same as those we called
        // `insert` with.
        assert!(hexmap.iter_mut().all(|(k, v)| k == *v));
    }

    #[test]
    fn test_kv_iter_mut() {
        let idx_bytes = include_bytes!("../assets/monaco.res12.h3idx");
        let rdr = &mut idx_bytes.as_slice();

        let cell_value_pairs = {
            let mut cell_value_pairs: Vec<(Cell, i32)> = Vec::new();
            let mut count = 0;
            while let Ok(idx) = rdr.read_u64::<LE>() {
                cell_value_pairs.push((Cell::try_from(idx).unwrap(), count));
                count += 1;
            }
            cell_value_pairs
        };

        let map = {
            let mut map = HexTreeMap::new();
            for (cell, value) in cell_value_pairs.iter() {
                map.insert(*cell, *value);
            }
            map
        };

        let map_plus_one = {
            let mut map = map;
            for (_, value) in map.iter_mut() {
                *value += 1;
            }
            map
        };

        assert!(cell_value_pairs
            .iter()
            .all(|(cell, value)| map_plus_one[cell] == value + 1));
    }
}
