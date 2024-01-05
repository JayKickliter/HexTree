use crate::{
    disktree::{dptr::Dptr, varint},
    error::Result,
};
use byteorder::ReadBytesExt;
use std::{
    io::{Read, Seek},
    mem::size_of,
    ops::Range,
};

// Enough bytes to read node tag and 7 child dptrs.
const NODE_BUF_SZ: usize = size_of::<u8>() + 7 * Dptr::size() as usize;

pub(crate) enum Node {
    // value_begin..value_end
    Leaf(Range<usize>),
    // (H3 Cell digit, file position of child's node tag)
    Parent([Option<Dptr>; 7]),
}

impl Node {
    pub(crate) fn read<R>(rdr: &mut R) -> Result<Node>
    where
        R: Seek + Read,
    {
        let start_pos = rdr.stream_position()?;
        let mut buf = [0u8; NODE_BUF_SZ];
        let bytes_read = rdr.read(&mut buf)?;
        let buf_rdr = &mut &buf[..bytes_read];
        let node_tag = buf_rdr.read_u8()?;
        if 0 == node_tag & 0b1000_0000 {
            let (val_len, n_read) = varint::read(&mut &buf[..bytes_read])?;
            let begin = (start_pos + n_read) as usize;
            let end = begin + val_len as usize;
            Ok(Node::Leaf(begin..end))
        } else {
            let mut children: [Option<Dptr>; 7] = [None, None, None, None, None, None, None];
            for (_digit, child) in (0..7)
                .zip(children.iter_mut())
                .filter(|(digit, _)| node_tag & (1 << digit) != 0)
            {
                *child = Some(Dptr::read(buf_rdr)?);
            }
            Ok(Node::Parent(children))
        }
    }
}
