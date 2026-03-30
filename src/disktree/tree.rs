use crate::{
    digits::Digits,
    disktree::{dptr::Dp, iter::Iter, node::Node},
    error::Result,
    Cell, Error,
};
use byteorder::ReadBytesExt;
use memmap::MmapOptions;
use std::{
    fs::File,
    io::{Cursor, Read, Seek, SeekFrom},
    marker::Send,
    path::Path,
};

pub(crate) const HDR_MAGIC: &[u8] = b"hextree\0";
pub(crate) const HDR_SZ: usize = HDR_MAGIC.len() + 1;

/// A memory-mapped, on-disk HexTreeMap.
///
/// This structure provides read-only access to a HexTreeMap that has
/// been serialized to disk.
pub struct DiskTreeMap(pub(crate) Box<dyn AsRef<[u8]> + Send + Sync + 'static>);

impl DiskTreeMap {
    /// Opens a `DiskTree` at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::memmap(&file)
    }

    /// Memory maps the provided disktree-containing file.
    pub fn memmap(file: &File) -> Result<Self> {
        #[allow(unsafe_code)]
        let mm = unsafe { MmapOptions::new().map(file)? };
        Self::with_buf(mm)
    }

    /// Opens a `DiskTree` with a provided buffer.
    pub fn with_buf<B>(buf: B) -> Result<Self>
    where
        B: AsRef<[u8]> + Send + Sync + 'static,
    {
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
            0 => Ok(Self(Box::new(csr.into_inner()))),
            unsupported_version => Err(Error::Version(unsupported_version)),
        }
    }

    /// Returns `(Cell, &[u8])`, if present.
    pub fn get(&self, cell: Cell) -> Result<Option<(Cell, &[u8])>> {
        if let Some((cell, _, Node::Leaf(range))) = self.get_raw(cell)? {
            let val_bytes = &(*self.0).as_ref()[range];
            Ok(Some((cell, val_bytes)))
        } else {
            Ok(None)
        }
    }

    /// Returns `(Cell, Node)`, if present.
    pub(crate) fn get_raw(&self, cell: Cell) -> Result<Option<(Cell, Dp, Node)>> {
        let base_cell_pos = Self::base_cell_dptr(cell);
        let mut csr = Cursor::new((*self.0).as_ref());
        csr.seek(SeekFrom::Start(base_cell_pos.into()))?;
        let node_dptr = Dp::read(&mut csr)?;
        if node_dptr.is_null() {
            return Ok(None);
        }
        let digits = Digits::new(cell);
        Self::_get_raw(&mut csr, 0, node_dptr, cell, digits)
    }

    fn _get_raw(
        csr: &mut Cursor<&[u8]>,
        res: u8,
        node_dptr: Dp,
        cell: Cell,
        mut digits: Digits,
    ) -> Result<Option<(Cell, Dp, Node)>> {
        csr.seek(SeekFrom::Start(node_dptr.into()))?;
        let node = Node::read(csr)?;
        match (digits.next(), &node) {
            (None, _) => Ok(Some((cell, node_dptr, node))),
            (Some(_), Node::Leaf(_)) => Ok(Some((
                cell.to_parent(res).expect("invalid condition"),
                node_dptr,
                node,
            ))),
            (Some(digit), Node::Parent(children)) => match children[digit as usize] {
                None => Ok(None),
                Some(dptr) => Self::_get_raw(csr, res + 1, dptr, cell, digits),
            },
        }
    }

    /// Returns `true` if the tree fully contains `cell`.
    pub fn contains(&self, cell: Cell) -> Result<bool> {
        self.get(cell).map(|opt| opt.is_some())
    }

    /// Returns an iterator visiting all `(Cell, &[u8])` pairs in
    /// arbitrary order.
    pub fn iter(&self) -> Result<impl Iterator<Item = Result<(Cell, &[u8])>>> {
        Iter::new((*self.0).as_ref())
    }

    /// Returns an iterator visiting the specified `cell` or its descendants.
    pub fn descendants(&self, cell: Cell) -> Result<impl Iterator<Item = Result<(Cell, &[u8])>>> {
        let iter = match self.get_raw(cell)? {
            None => crate::disktree::iter::Iter::empty((*self.0).as_ref()),
            Some((cell, dp, node)) => {
                crate::disktree::iter::Iter::descendants((*self.0).as_ref(), cell, dp, node)?
            }
        };
        Ok(iter)
    }

    /// Returns the DPtr to a base (res0) cell dptr.
    fn base_cell_dptr(cell: Cell) -> Dp {
        Dp::from(HDR_SZ + Dp::size() * cell.base() as usize)
    }
}
