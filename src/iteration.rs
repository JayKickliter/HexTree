use crate::{h3ron::H3Cell, node::Node};

type NodeStackIter<'a, V> = std::iter::Flatten<std::slice::Iter<'a, Option<Box<Node<V>>>>>;

pub(crate) struct Iter<'a, V> {
    stack: Vec<NodeStackIter<'a, V>>,
    #[allow(clippy::borrowed_box)]
    curr: Option<&'a Box<Node<V>>>,
}

impl<'a, V> Iter<'a, V> {
    pub(crate) fn new(base: &'a [Option<Box<Node<V>>>]) -> Self {
        let mut iter = base.iter().flatten();
        let curr = iter.next();
        let mut stack = Vec::with_capacity(16);
        stack.push(iter);
        Self { stack, curr }
    }
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = (&'a H3Cell, &'a V);

    fn next(&mut self) -> Option<(&'a H3Cell, &'a V)> {
        while self.curr.is_none() {
            if let Some(mut iter) = self.stack.pop() {
                if let Some(node) = iter.next() {
                    self.curr = Some(node);
                    self.stack.push(iter);
                }
            } else {
                break;
            }
        }
        while let Some(curr) = self.curr {
            match curr.as_ref() {
                Node::Parent(_, children) => {
                    let mut iter = children.iter().flatten();
                    self.curr = iter.next();
                    self.stack.push(iter);
                }
                Node::Leaf(cell, value) => {
                    self.curr = None;
                    return Some((cell, value));
                }
            }
        }
        None
    }
}

type NodeStackIterMut<'a, V> = std::iter::Flatten<std::slice::IterMut<'a, Option<Box<Node<V>>>>>;

pub(crate) struct IterMut<'a, V> {
    stack: Vec<NodeStackIterMut<'a, V>>,
    #[allow(clippy::borrowed_box)]
    curr: Option<&'a mut Box<Node<V>>>,
}

impl<'a, V> IterMut<'a, V> {
    pub(crate) fn new(base: &'a mut [Option<Box<Node<V>>>]) -> Self {
        let mut iter = base.iter_mut().flatten();
        let curr = iter.next();
        let mut stack = Vec::with_capacity(16);
        stack.push(iter);
        Self { stack, curr }
    }
}

impl<'a, V> Iterator for IterMut<'a, V> {
    type Item = (&'a H3Cell, &'a mut V);

    fn next(&mut self) -> Option<(&'a H3Cell, &'a mut V)> {
        while self.curr.is_none() {
            if let Some(mut iter) = self.stack.pop() {
                if let Some(node) = iter.next() {
                    self.curr = Some(node);
                    self.stack.push(iter);
                }
            } else {
                break;
            }
        }
        while let Some(curr) = self.curr.take() {
            match curr.as_mut() {
                Node::Parent(_, children) => {
                    let mut iter = children.iter_mut().flatten();
                    self.curr = iter.next();
                    self.stack.push(iter);
                }
                Node::Leaf(cell, value) => {
                    self.curr = None;
                    return Some((cell, value));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        h3ron::{FromH3Index, H3Cell},
        HexTreeMap,
    };
    use byteorder::{LittleEndian as LE, ReadBytesExt};

    #[test]
    fn test_kv_iter() {
        let idx_bytes = include_bytes!("../assets/monaco.res12.h3idx");
        let rdr = &mut idx_bytes.as_slice();

        let cell_value_pairs = {
            let mut cell_value_pairs: Vec<(H3Cell, i32)> = Vec::new();
            let mut count = 0;
            while let Ok(idx) = rdr.read_u64::<LE>() {
                cell_value_pairs.push((H3Cell::from_h3index(idx), count));
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

        let map_collected = {
            let mut map_collected: Vec<(H3Cell, i32)> = map.iter().map(|(c, v)| (*c, *v)).collect();
            map_collected.sort_by(|a, b| a.1.cmp(&b.1));
            map_collected
        };

        assert_eq!(cell_value_pairs, map_collected);
    }

    #[test]
    fn test_kv_iter_mut() {
        let idx_bytes = include_bytes!("../assets/monaco.res12.h3idx");
        let rdr = &mut idx_bytes.as_slice();

        let cell_value_pairs = {
            let mut cell_value_pairs: Vec<(H3Cell, i32)> = Vec::new();
            let mut count = 0;
            while let Ok(idx) = rdr.read_u64::<LE>() {
                cell_value_pairs.push((H3Cell::from_h3index(idx), count));
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
