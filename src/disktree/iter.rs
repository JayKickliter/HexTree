use crate::{
    cell::CellStack,
    disktree::{dptr::Dp, dtseek::DtSeek, tree::HDR_SZ, varint},
    error::{Error, Result},
    Cell,
};
use byteorder::ReadBytesExt;
use std::io::Cursor;

pub(crate) struct Iter<'a> {
    cell_stack: CellStack,
    curr_node: Option<(u8, Dp)>,
    disktree_buf: &'a [u8],
    disktree_csr: Cursor<&'a [u8]>,
    node_stack: Vec<Vec<(u8, Dp)>>,
    recycle_bin: Vec<Vec<(u8, Dp)>>,
}

enum Node {
    // File position for the fist byte of value data.
    Leaf(Dp),
    // (H3 Cell digit, file position of child's node tag)
    Parent(Vec<(u8, Dp)>),
}

impl<'a> Iter<'a> {
    pub(crate) fn read_base_nodes(rdr: &mut Cursor<&[u8]>) -> Result<Vec<(u8, Dp)>> {
        let mut buf = Vec::with_capacity(122);
        rdr.seek(HDR_SZ.into())?;
        for digit in 0..122 {
            let dptr = Dp::read(rdr)?;
            if !dptr.is_null() {
                buf.push((digit, dptr));
            }
        }
        buf.reverse();
        Ok(buf)
    }

    // `pos` is a position in the file of this node's tag.
    fn read_node(&mut self, dptr: Dp) -> Result<Node> {
        let dptr = self.seek(dptr)?;
        let node_tag = self.disktree_csr.read_u8()?;
        if 0 == node_tag & 0b1000_0000 {
            Ok(Node::Leaf(dptr))
        } else {
            let mut children = self.node_buf();
            let n_children = (node_tag & 0b0111_1111).count_ones() as usize;
            let child_dptrs = Dp::read_n(&mut self.disktree_csr, n_children)?;
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
    fn node_buf(&mut self) -> Vec<(u8, Dp)> {
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
    fn recycle_node_buf(&mut self, buf: Vec<(u8, Dp)>) {
        debug_assert!(buf.is_empty());
        self.recycle_bin.push(buf);
    }

    // We've encountered an IO error. We're still going to return
    // `Some` with the contents of the user's deserializer, but let's
    // make sure we never yield another value by clearing stack.
    fn stop_yielding(&mut self) {
        self.node_stack.clear();
        self.curr_node = None;
    }

    pub(crate) fn new(disktree_buf: &'a [u8]) -> Result<Iter<'a>> {
        let mut disktree_csr = Cursor::new(disktree_buf);
        let mut cell_stack = CellStack::new();
        let mut node_stack = Vec::new();
        let recycle_bin = Vec::new();
        let mut base_nodes = Self::read_base_nodes(&mut disktree_csr)?;
        let curr_node = base_nodes.pop();
        node_stack.push(base_nodes);
        if let Some((digit, _)) = curr_node {
            cell_stack.push(digit);
        }
        Ok(Self {
            cell_stack,
            curr_node,
            disktree_buf,
            disktree_csr,
            recycle_bin,
            node_stack,
        })
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<(Cell, &'a [u8])>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.curr_node.is_none() {
            if let Some(mut dptrs) = self.node_stack.pop() {
                self.cell_stack.pop();
                if let Some((digit, dptr)) = dptrs.pop() {
                    self.cell_stack.push(digit);
                    self.curr_node = Some((digit, dptr));
                    self.node_stack.push(dptrs);
                } else {
                    self.recycle_node_buf(dptrs);
                }
            } else {
                break;
            }
        }
        while let Some((digit, dptr)) = self.curr_node {
            self.cell_stack.swap(digit);
            match self.read_node(dptr) {
                Err(e) => {
                    self.stop_yielding();
                    return Some(Err(e));
                }
                Ok(Node::Parent(mut children)) => {
                    if let Some((digit, dptr)) = children.pop() {
                        self.cell_stack.push(digit);
                        self.curr_node = Some((digit, dptr));
                        self.node_stack.push(children);
                    } else {
                        self.recycle_node_buf(children);
                    }
                }
                Ok(Node::Leaf(dptr)) => {
                    self.curr_node = None;
                    if let Err(e) = self.seek(dptr) {
                        self.stop_yielding();
                        return Some(Err(Error::from(e)));
                    }
                    match varint::read(&mut self.disktree_csr) {
                        Err(e) => {
                            self.stop_yielding();
                            return Some(Err(e));
                        }
                        Ok((val_len, _n_read)) => {
                            let pos = self.disktree_csr.position() as usize;
                            let val_buf = &self.disktree_buf[pos..][..val_len as usize];
                            return Some(Ok((
                                *self.cell_stack.cell().expect("corrupted cell-stack"),
                                val_buf,
                            )));
                        }
                    }
                }
            };
        }
        None
    }
}

impl<'a> DtSeek for Iter<'a> {
    fn pos(&mut self) -> std::io::Result<Dp> {
        self.disktree_csr.pos()
    }

    fn seek(&mut self, dp: Dp) -> std::io::Result<Dp> {
        self.disktree_csr.seek(dp)
    }

    fn fast_forward(&mut self) -> std::io::Result<Dp> {
        self.disktree_csr.fast_forward()
    }
}
