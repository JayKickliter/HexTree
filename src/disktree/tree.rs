use crate::{
    digits::Digits,
    disktree::{dptr::Dptr, iter::Iter, node::Node},
    error::Result,
    Cell, Error,
};
use byteorder::ReadBytesExt;
use memmap::{Mmap, MmapOptions};
use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    path::Path,
};

pub(crate) const HDR_MAGIC: &[u8] = b"hextree\0";
pub(crate) const HDR_SZ: u64 = HDR_MAGIC.len() as u64 + 1;

/// An on-disk hextree map.
pub struct DiskTree<B>(B);

impl DiskTree<Mmap> {
    /// Opens a `DiskTree` at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::memmap(file)
    }

    /// Memory maps the provided disktree-containing.
    pub fn memmap(file: File) -> Result<Self> {
        #[allow(unsafe_code)]
        let mm = unsafe { MmapOptions::new().map(&file)? };
        Self::with_buf(mm)
    }
}

impl<B: AsRef<[u8]>> DiskTree<B> {
    /// Opens a `DiskTree` with a provided buffer.
    pub fn with_buf(buf: B) -> Result<Self> {
        let mut csr = Cursor::new(buf);
        let magic = {
            let mut buf = [0_u8; HDR_MAGIC.len()];
            csr.read_exact(&mut buf)?;
            buf
        };
        if magic != HDR_MAGIC {
            return Err(Error::NotDisktree);
        }

        let version = {
            // We use 0xFE as a version offset since it is much less
            // likely to randomly appear than 0;
            0xFE - csr.read_u8()?
        };
        match version {
            0 => Ok(Self(csr.into_inner())),
            unsupported_version => Err(Error::Version(unsupported_version)),
        }
    }

    /// Returns `(Cell, &[u8])`, if present.
    pub fn get(&self, cell: Cell) -> Result<Option<(Cell, &[u8])>> {
        let base_cell_pos = Self::base_cell_dptr(cell);
        let mut csr = Cursor::new(self.0.as_ref());
        csr.seek(SeekFrom::Start(base_cell_pos.into()))?;
        let node_dptr = Dptr::read(&mut csr)?;
        if node_dptr.is_null() {
            return Ok(None);
        }
        let digits = Digits::new(cell);
        if let Some((cell, dptr)) = Self::_get(&mut csr, 0, node_dptr, cell, digits)? {
            csr.seek(SeekFrom::Start(dptr.into()))?;
            let val_len = leb128::read::unsigned(&mut csr).unwrap() as usize;
            let val_start = csr.position() as usize;
            let val_bytes = &self.0.as_ref()[val_start..][..val_len];
            Ok(Some((cell, val_bytes)))
        } else {
            Ok(None)
        }
    }

    /// Returns `true` if the tree fully contains `cell`.
    pub fn contains(&self, cell: Cell) -> Result<bool> {
        self.get(cell).map(|opt| opt.is_some())
    }

    /// Returns an iterator visiting all `(Cell, &[u8])` pairs in
    /// arbitrary order.
    pub fn iter(&self) -> Result<impl Iterator<Item = Result<(Cell, &[u8])>>> {
        Iter::new(self.0.as_ref())
    }

    fn _get(
        csr: &mut Cursor<&[u8]>,
        res: u8,
        node_dptr: Dptr,
        cell: Cell,
        mut digits: Digits,
    ) -> Result<Option<(Cell, Dptr)>> {
        csr.seek(SeekFrom::Start(node_dptr.into()))?;
        let node = Node::read(csr)?;
        match (digits.next(), node) {
            (None, Node::Leaf(dptr)) => Ok(Some((cell, dptr))),
            (Some(_), Node::Leaf(dptr)) => Ok(Some((
                cell.to_parent(res).expect("invalid condition"),
                dptr,
            ))),
            (Some(digit), Node::Parent(children)) => match children[digit as usize] {
                None => Ok(None),
                Some(dptr) => Self::_get(csr, res + 1, dptr, cell, digits),
            },
            // No digits left, but `self` isn't full, so this cell
            // can't fully contain the target.
            (None, _) => Ok(None),
        }
    }

    /// Returns the DPtr to a base (res0) cell dptr.
    fn base_cell_dptr(cell: Cell) -> Dptr {
        Dptr::from(HDR_SZ + Dptr::size() * (cell.base() as u64))
    }
}