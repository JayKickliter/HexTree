use crate::{
    compaction::Compactor,
    disktree::{dptr::Dp, dtseek::DtSeek, tree::HDR_MAGIC, varint, DiskTreeMap},
    error::{Error, Result},
    node::Node,
    HexTreeMap,
};
use byteorder::WriteBytesExt;
use std::io::{Cursor, Write};

impl<V, C> HexTreeMap<V, C>
where
    C: Compactor<V>,
{
    /// Write self to disk.
    pub fn to_disktree<W, F, E>(&self, wtr: W, f: F) -> Result
    where
        W: Write + std::io::Seek,
        F: Fn(&mut dyn Write, &V) -> std::result::Result<(), E>,
        E: std::error::Error + Sync + Send + 'static,
    {
        DiskTreeWriter::new(wtr).write(self, f)
    }
}

pub(crate) struct DiskTreeWriter<W> {
    scratch_pad: Vec<u8>,
    wtr: W,
}

impl<W> DiskTreeWriter<W> {
    pub fn new(wtr: W) -> Self {
        let scratch_pad = Vec::new();
        Self { wtr, scratch_pad }
    }
}

impl<W> DiskTreeWriter<W>
where
    W: Write + std::io::Seek,
{
    pub fn write<V, C, F, E>(&mut self, hextree: &HexTreeMap<V, C>, mut f: F) -> Result
    where
        F: Fn(&mut dyn Write, &V) -> std::result::Result<(), E>,
        E: std::error::Error + Sync + Send + 'static,
    {
        // Write magic string
        self.wtr.write_all(HDR_MAGIC)?;
        // Write version field
        const VERSION: u8 = 0;
        self.wtr.write_u8(0xFE - VERSION)?;

        let mut fixups: Vec<(Dp, &Node<V>)> = Vec::new();

        // Write base cells placeholder offsets.
        for base in hextree.nodes.iter() {
            match base.as_deref() {
                None => Dp::null().write(&mut self.wtr)?,
                Some(node) => {
                    fixups.push((self.pos()?, node));
                    Dp::null().write(&mut self.wtr)?
                }
            }
        }

        for (fixee_dptr, node) in fixups {
            let node_dptr = self.write_node(node, &mut f)?;
            self.seek(fixee_dptr)?;
            node_dptr.write(&mut self.wtr)?;
        }

        Ok(())
    }

    pub fn merge<F, E>(&mut self, subtrees: &[DiskTreeMap], mut f: F) -> Result
    where
        F: FnMut(&mut dyn Write, &&[u8]) -> std::result::Result<(), E>,
        E: std::error::Error + Sync + Send + 'static,
    {
        {
            // Write magic string
            self.wtr.write_all(HDR_MAGIC)?;
            // Write version field
            const VERSION: u8 = 0;
            self.wtr.write_u8(0xFE - VERSION)?;
        }

        {
            // We currently need to read the incoming disktrees to memory
            // before writing them out. This creates a lifetime problem
            // where we have to do res0 fixups immediately after writing a
            // res0 node. Therefore, to ensure offsets are correct, we pad
            // out the file to make sure we don't write over
            // not-yet-written res0 base offsets.
            let pos = self.pos()?;
            self.wtr.write_all(&vec![0; 122 * Dp::size()])?;
            self.seek(pos)?;
        }

        let root_disk_trees = {
            let mut root_disk_trees: Box<[Option<&DiskTreeMap>]> = (0..122).map(|_| None).collect();
            for disktree in subtrees {
                let tree_roots = crate::disktree::iter::Iter::read_base_nodes(&mut Cursor::new(
                    (*disktree.0).as_ref(),
                ))?;
                if !tree_roots.is_empty() {
                    assert!(root_disk_trees[tree_roots[0].0 as usize].is_none());
                    root_disk_trees[tree_roots[0].0 as usize] = Some(disktree);
                }
            }
            root_disk_trees
        };

        for (idx, maybe_disktree) in root_disk_trees.iter().enumerate() {
            match maybe_disktree {
                None => Dp::null().write(&mut self.wtr)?,
                Some(disktree) => {
                    let mut fixups: Vec<(Dp, &Node<&[u8]>)> = Vec::new();
                    let mut hextree: HexTreeMap<&[u8]> = HexTreeMap::new();
                    for res in disktree.iter()? {
                        let (cell, val) = res?;
                        hextree.insert(cell, val);
                    }
                    if let Some(node) = hextree
                        .nodes
                        .get(idx)
                        .expect("we already determined this node should exist")
                        .as_deref()
                    {
                        fixups.push((self.pos()?, node));
                        Dp::null().write(&mut self.wtr)?
                    }
                    for (fixee_dptr, node) in fixups {
                        let node_dptr = self.write_node(node, &mut f)?;
                        self.seek(fixee_dptr)?;
                        node_dptr.write(&mut self.wtr)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn write_node<V, F, E>(&mut self, node: &Node<V>, f: &mut F) -> Result<Dp>
    where
        F: FnMut(&mut dyn Write, &V) -> std::result::Result<(), E>,
        E: std::error::Error + Sync + Send + 'static,
    {
        let node_pos = self.fast_forward()?;
        let mut node_fixups: Vec<(Dp, &Node<V>)> = Vec::new();
        match node {
            Node::Leaf(val) => {
                self.scratch_pad.clear();
                f(&mut self.scratch_pad, val).map_err(|e| Error::Writer(Box::new(e)))?;
                let val_len = self.scratch_pad.len() as u64;
                varint::write(&mut self.wtr, val_len as u32)?;
                self.wtr.write_all(&self.scratch_pad)?;
            }
            Node::Parent(children) => {
                let tag_pos = self.pos()?;
                // Write a dummy value so children have accurate
                // stream position information.
                self.wtr.write_u8(0b1000_0000)?;
                let mut tag = 0;
                for child in children.iter() {
                    match child.as_deref() {
                        None => {
                            // "insert" a 0 into the tag denoting that
                            // this node is empty.
                            tag >>= 1;
                        }
                        Some(node) => {
                            // "insert" a 1 into the tag denoting that
                            // this node is empty.
                            tag = (tag >> 1) | 0b1000_0000;
                            node_fixups.push((self.pos()?, node));
                            Dp::null().write(&mut self.wtr)?;
                        }
                    };
                }
                self.seek(tag_pos)?;
                // Make the top bit 1 as a sentinel.
                tag = (tag >> 1) | 0b1000_0000;
                self.wtr.write_u8(tag)?;
            }
        };

        for (fixee_dptr, node) in node_fixups {
            let node_dptr = self.write_node(node, f)?;
            self.seek(fixee_dptr)?;
            node_dptr.write(&mut self.wtr)?;
        }

        Ok(node_pos)
    }
}

impl<W> DtSeek for DiskTreeWriter<W>
where
    W: std::io::Seek,
{
    fn pos(&mut self) -> std::io::Result<Dp> {
        self.wtr.pos()
    }

    fn seek(&mut self, dp: Dp) -> std::io::Result<Dp> {
        DtSeek::seek(&mut self.wtr, dp)
    }

    fn fast_forward(&mut self) -> std::io::Result<Dp> {
        self.wtr.fast_forward()
    }
}
