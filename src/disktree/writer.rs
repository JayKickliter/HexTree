use crate::{
    compaction::Compactor,
    disktree::dptr::Dptr,
    error::{Error, Result},
    node::Node,
    HexTreeMap,
};
use byteorder::WriteBytesExt;
use std::io::{Seek, SeekFrom, Write};

impl<V, C: Compactor<V>> HexTreeMap<V, C> {
    /// Write self to disk.
    pub fn to_disktree<W, F, E>(&self, wtr: W, f: F) -> Result
    where
        W: Write + Seek,
        F: Fn(&mut W, &V) -> std::result::Result<(), E>,
        E: std::error::Error + Sync + Send + 'static,
    {
        DiskTreeWriter(wtr).write(self, f)
    }
}

pub(crate) struct DiskTreeWriter<W>(W);

impl<W: Write + Seek> DiskTreeWriter<W> {
    pub fn write<V, C, F, E>(&mut self, hextree: &HexTreeMap<V, C>, mut f: F) -> Result
    where
        F: Fn(&mut W, &V) -> std::result::Result<(), E>,
        E: std::error::Error + Sync + Send + 'static,
    {
        // Write version field
        const VERSION: u8 = 0;
        self.0.write_u8(0xFE - VERSION)?;
        // Write base cells placeholder offsets.
        let mut fixups: Vec<(Dptr, &Node<V>)> = Vec::new();

        // Empty:  | DPTR_DEFAULT |
        // Node:   | Dptr         |
        for base in hextree.nodes.iter() {
            match base.as_deref() {
                None => Dptr::null().write(&mut self.0)?,
                Some(node) => {
                    fixups.push((self.pos()?, node));
                    Dptr::null().write(&mut self.0)?
                }
            }
        }

        for (fixee_dptr, node) in fixups {
            let node_dptr = self.write_node(node, &mut f)?;
            self.seek_to(fixee_dptr)?;
            node_dptr.write(&mut self.0)?;
        }

        Ok(())
    }

    /// Leaf:   | 0_u8 | bincode bytes |
    /// Parent: | 1_u8 | Dptr | Dptr | Dptr | Dptr | Dptr | Dptr | Dptr |
    fn write_node<V, F, E>(&mut self, node: &Node<V>, f: &mut F) -> Result<Dptr>
    where
        F: Fn(&mut W, &V) -> std::result::Result<(), E>,
        E: std::error::Error + Sync + Send + 'static,
    {
        let node_pos: Dptr = self.0.seek(SeekFrom::End(0))?.into();
        let mut node_fixups: Vec<(Dptr, &Node<V>)> = Vec::new();
        match node {
            Node::Leaf(val) => {
                self.0.write_u8(0)?;
                // bincode::serialize_into(&mut self.0, val)?;
                f(&mut self.0, val).map_err(|e| Error::Writer(Box::new(e)))?
            }
            Node::Parent(children) => {
                let tag_pos = self.pos()?;
                self.0.write_u8(0b1000_0000)?;
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
                            Dptr::null().write(&mut self.0)?;
                        }
                    };
                }
                self.seek_to(tag_pos)?;
                // Make the top bit 1 as a sentinel.
                tag = (tag >> 1) | 0b1000_0000;
                // println!("{tag_pos:010x}: write tag {tag:08b}");
                self.0.write_u8(tag)?;
            }
        };

        for (fixee_dptr, node) in node_fixups {
            let node_dptr = self.write_node(node, f)?;
            self.seek_to(fixee_dptr)?;
            node_dptr.write(&mut self.0)?;
        }

        Ok(node_pos)
    }

    fn pos(&mut self) -> Result<Dptr> {
        Ok(Dptr::from(self.0.stream_position()?))
    }

    fn seek_to(&mut self, dptr: Dptr) -> Result<Dptr> {
        Ok(Dptr::from(self.0.seek(SeekFrom::Start(u64::from(dptr)))?))
    }
}
