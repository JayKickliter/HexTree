use crate::{
    digits::Digits,
    disktree::{dptr::Dptr, iter::Iter, node::Node, ReadVal},
    error::{Error, Result},
    Cell,
};
use byteorder::ReadBytesExt;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub(crate) const HDR_SZ: u64 = 1;

/// An on-disk hextree map.
pub struct DiskTree<R>(R);

impl DiskTree<File> {
    /// Opens a `DiskTree` at the specified path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Self::from_reader(file)
    }
}

impl<R> DiskTree<R> {
    /// Conumes `self` and returns the backing store.
    pub fn into_inner(self) -> R {
        self.0
    }
}

impl<R: Read + Seek> DiskTree<R> {
    /// Opens a `DiskTree` with a provided reader.
    pub fn from_reader(mut rdr: R) -> Result<Self> {
        rdr.seek(SeekFrom::Start(0))?;
        let version = {
            // We use 0xFE as a version offset since it is much less
            // likely to randomly appear than 0;
            0xFE - rdr.read_u8()?
        };
        match version {
            0 => Ok(Self(rdr)),
            unsupported_version => Err(Error::Version(unsupported_version)),
        }
    }

    /// Returns a reader pre-seeked to the value for cell, if present.
    pub fn seek_to_cell(&mut self, cell: Cell) -> Result<Option<(Cell, &mut R)>> {
        let base_cell_pos = Self::base_cell_dptr(cell);
        self.seek_to_pos(base_cell_pos)?;
        let node_dptr = Dptr::read(&mut self.0)?;
        if node_dptr.is_null() {
            return Ok(None);
        }
        let digits = Digits::new(cell);
        if let Some((cell, dptr)) = self._get(0, node_dptr, cell, digits)? {
            self.seek_to_pos(dptr.into())?;
            Ok(Some((cell, &mut self.0)))
        } else {
            Ok(None)
        }
    }

    /// Returns `true` if the tree fully contains `cell`.
    pub fn contains(&mut self, cell: Cell) -> Result<bool> {
        let base_cell_pos = Self::base_cell_dptr(cell);
        self.seek_to_pos(base_cell_pos)?;
        let node_dptr = Dptr::read(&mut self.0)?;
        if node_dptr.is_null() {
            return Ok(false);
        }
        let digits = Digits::new(cell);
        self._get(0, node_dptr, cell, digits)
            .map(|opt| opt.is_some())
    }

    /// Returns an iterator visiting all cell-value pairs in arbitrary
    /// order.
    ///
    /// However, insteading of returning the concrete value, the
    /// iterator retuns a reader pre-seeked to the node's value.
    pub fn iter<'a, F>(
        &'a mut self,
        f: F,
    ) -> Result<impl Iterator<Item = <F as ReadVal<R>>::T> + 'a>
    where
        F: ReadVal<R> + 'a,
    {
        Iter::new(&mut self.0, f)
    }

    /// Leaf:   | 0_u8 | bincode bytes |
    /// Parent: | 1_u8 | Dptr | Dptr | Dptr | Dptr | Dptr | Dptr | Dptr |
    fn _get(
        &mut self,
        res: u8,
        node_dptr: Dptr,
        cell: Cell,
        mut digits: Digits,
    ) -> Result<Option<(Cell, Dptr)>> {
        self.seek_to_pos(node_dptr)?;
        let node = Node::read(&mut self.0)?;
        match (digits.next(), node) {
            (None, Node::Leaf(dptr)) => Ok(Some((cell, dptr))),
            (Some(_), Node::Leaf(dptr)) => Ok(Some((
                cell.to_parent(res).expect("invalid condition"),
                dptr,
            ))),
            (Some(digit), Node::Parent(children)) => match children[digit as usize] {
                None => Ok(None),
                Some(dptr) => self._get(res + 1, dptr, cell, digits),
            },
            // No digits left, but `self` isn't full, so this cell
            // can't fully contain the target.
            (None, _) => Ok(None),
        }
    }

    fn seek_to_pos(&mut self, dptr: Dptr) -> Result {
        self.0.seek(SeekFrom::Start(u64::from(dptr)))?;
        Ok(())
    }

    /// Returns the DPtr to a base (res0) cell dptr.
    fn base_cell_dptr(cell: Cell) -> Dptr {
        Dptr::from(HDR_SZ + Dptr::size() * (cell.base() as u64))
    }
}
