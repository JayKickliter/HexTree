use crate::{
    compaction::Compactor,
    error::{Error, Result},
    hexdb::{dbseek::DbSeek, dptr::P, tree::HDR_MAGIC, varint},
    node::Node,
    HexTreeMap,
};
use byteorder::WriteBytesExt;
use std::io::Write;

impl<V, C> HexTreeMap<V, C>
where
    C: Compactor<V>,
{
    /// Encode self as a [HexDb](crate::hexdb::HexDb) to the provided writer.
    pub fn to_hexdb<W, F, E>(&self, wtr: W, f: F) -> Result
    where
        W: Write + std::io::Seek,
        F: Fn(&mut dyn Write, &V) -> std::result::Result<(), E>,
        E: std::error::Error + Sync + Send + 'static,
    {
        HexDbWriter::new(wtr).write(self, f)
    }
}

pub(crate) struct HexDbWriter<W> {
    scratch_pad: Vec<u8>,
    wtr: W,
}

impl<W> HexDbWriter<W> {
    pub fn new(wtr: W) -> Self {
        let scratch_pad = Vec::new();
        Self { wtr, scratch_pad }
    }
}

impl<W> HexDbWriter<W>
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

        let mut fixups: Vec<(P, &Node<V>)> = Vec::new();

        // Write base cells placeholder offsets.
        for base in hextree.nodes.iter() {
            match base.as_deref() {
                None => P::null().write(&mut self.wtr)?,
                Some(node) => {
                    fixups.push((self.pos()?, node));
                    P::null().write(&mut self.wtr)?
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

    fn write_node<V, F, E>(&mut self, node: &Node<V>, f: &mut F) -> Result<P>
    where
        F: FnMut(&mut dyn Write, &V) -> std::result::Result<(), E>,
        E: std::error::Error + Sync + Send + 'static,
    {
        let node_pos = self.fast_forward()?;
        let mut node_fixups: Vec<(P, &Node<V>)> = Vec::new();
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
                            P::null().write(&mut self.wtr)?;
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

impl<W> DbSeek for HexDbWriter<W>
where
    W: std::io::Seek,
{
    fn pos(&mut self) -> std::io::Result<P> {
        self.wtr.pos()
    }

    fn seek(&mut self, dp: P) -> std::io::Result<P> {
        DbSeek::seek(&mut self.wtr, dp)
    }

    fn fast_forward(&mut self) -> std::io::Result<P> {
        self.wtr.fast_forward()
    }
}
