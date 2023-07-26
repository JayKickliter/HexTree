use crate::{
    cell::CellStack,
    disktree::{dptr::Dptr, tree::HDR_SZ, ReadVal},
    error::Result,
};
use byteorder::ReadBytesExt;
use std::io::{Read, Seek, SeekFrom};

pub(crate) struct Iter<'a, R, F> {
    cell_stack: CellStack,
    curr: Option<(u8, Dptr)>,
    rdr: &'a mut R,
    recycle_bin: Vec<Vec<(u8, Dptr)>>,
    stack: Vec<Vec<(u8, Dptr)>>,
    f: F,
}

enum Node {
    // File position for the fist byte of value data.
    Leaf(Dptr),
    // (H3 Cell digit, file position of child's node tag)
    Parent(Vec<(u8, Dptr)>),
}

impl<'a, R, F> Iter<'a, R, F>
where
    R: Seek + Read,
{
    fn seek_to(&mut self, dptr: Dptr) -> Result<Dptr> {
        Ok(Dptr::from(self.rdr.seek(SeekFrom::Start(u64::from(dptr)))?))
    }

    fn read_base_nodes(rdr: &mut R) -> Result<Vec<(u8, Dptr)>> {
        let mut buf = Vec::with_capacity(122);
        rdr.seek(SeekFrom::Start(HDR_SZ))?;
        for digit in 0..122 {
            let dptr = Dptr::read(rdr)?;
            if !dptr.is_null() {
                buf.push((digit, dptr));
            }
        }
        buf.reverse();
        Ok(buf)
    }

    // `pos` is a position in the file of this node's tag.
    fn read_node(&mut self, dptr: Dptr) -> Result<Node> {
        let dptr = self.seek_to(dptr)?;
        let node_tag = self.rdr.read_u8()?;
        let base_pos = Dptr::from(u64::from(dptr) + std::mem::size_of_val(&node_tag) as u64);
        debug_assert_eq!(base_pos, Dptr::from(self.rdr.stream_position().unwrap()));
        assert!(node_tag == 0 || node_tag > 0b1000_0000);
        if node_tag == 0 {
            Ok(Node::Leaf(base_pos))
        } else {
            let mut children = self.node_buf();
            let n_children = (node_tag & 0b0111_1111).count_ones() as usize;
            let child_dptrs = Dptr::read_n(&mut self.rdr, n_children)?;
            children.extend(
                (0..7)
                    .rev()
                    .filter(|digit| node_tag & (1 << digit) != 0)
                    .zip(child_dptrs.into_iter().rev()),
            );
            Ok(Node::Parent(children))
        }
    }

    /// Returns a recycled node buffer if available, otherwise
    /// allocates a new one.
    ///
    /// See [`Iter::recycle_node_buf`].
    fn node_buf(&mut self) -> Vec<(u8, Dptr)> {
        let buf = self
            .recycle_bin
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(7));
        debug_assert!(buf.is_empty());
        buf
    }

    /// Accepts a used, empty, node buffer for later reuse.
    ///
    /// See  [`Iter::node_buf`].
    fn recycle_node_buf(&mut self, buf: Vec<(u8, Dptr)>) {
        debug_assert!(buf.is_empty());
        self.recycle_bin.push(buf);
    }

    // We've encountered an IO error. We're still going to return
    // `Some` with the contents of the user's deserializer, but let's
    // make sure we never yeild another value by clearing stack.
    fn stop_yeilding(&mut self) {
        self.stack.clear();
        self.curr = None;
    }

    pub(crate) fn new(rdr: &'a mut R, f: F) -> Result<Iter<'a, R, F>> {
        let mut cell_stack = CellStack::new();
        let mut stack = Vec::new();
        let recycle_bin = Vec::new();
        let mut base_nodes = Self::read_base_nodes(rdr)?;
        let curr = base_nodes.pop();
        stack.push(base_nodes);
        if let Some((digit, _)) = curr {
            cell_stack.push(digit);
        }
        Ok(Self {
            cell_stack,
            curr,
            rdr,
            recycle_bin,
            stack,
            f,
        })
    }
}

impl<'a, R, F> Iterator for Iter<'a, R, F>
where
    R: Read + Seek,
    F: ReadVal<R>,
{
    type Item = <F as ReadVal<R>>::T;

    fn next(&mut self) -> Option<Self::Item> {
        while self.curr.is_none() {
            if let Some(mut dptrs) = self.stack.pop() {
                self.cell_stack.pop();
                if let Some((digit, dptr)) = dptrs.pop() {
                    self.cell_stack.push(digit);
                    self.curr = Some((digit, dptr));
                    self.stack.push(dptrs);
                } else {
                    self.recycle_node_buf(dptrs);
                }
            } else {
                break;
            }
        }
        while let Some((digit, dptr)) = self.curr {
            self.cell_stack.swap(digit);
            match self.read_node(dptr) {
                Err(e) => {
                    self.stop_yeilding();
                    return Some(self.f.read(Err(e)));
                }
                Ok(Node::Parent(mut children)) => {
                    if let Some((digit, dptr)) = children.pop() {
                        self.cell_stack.push(digit);
                        self.curr = Some((digit, dptr));
                        self.stack.push(children);
                    } else {
                        self.recycle_node_buf(children);
                    }
                }
                Ok(Node::Leaf(dptr)) => {
                    self.curr = None;
                    if let Err(e) = self.seek_to(dptr) {
                        self.stop_yeilding();
                        return Some(self.f.read(Err(e)));
                    }
                    return Some(self.f.read(Ok((
                        *self.cell_stack.cell().expect("corrupted cell-stack"),
                        self.rdr,
                    ))));
                }
            };
        }
        None
    }
}
