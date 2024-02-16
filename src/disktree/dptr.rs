use crate::Result;
use std::{
    convert::TryFrom,
    io::{Read, Write},
    mem::size_of,
    ops::Add,
};

/// A 'disk' pointer.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct Dp(u64);

impl Dp {
    #[allow(clippy::cast_possible_truncation)]
    const MAX: u64 = 2_u64.pow(Self::DISK_REPR_SZ as u32 * 8) - 1;
    const DISK_REPR_SZ: usize = 5;
    const NULL: u64 = 0;

    pub(crate) const fn is_null(self) -> bool {
        self.0 == Self::NULL
    }

    pub(crate) const fn null() -> Dp {
        Dp(Self::NULL)
    }

    pub(crate) const fn size() -> usize {
        Self::DISK_REPR_SZ
    }

    /// Read 5 bytes from disk and parses them as little-endian `u64`.
    pub(crate) fn read<R>(src: &mut R) -> Result<Self>
    where
        R: Read,
    {
        let mut buf = [0u8; size_of::<u64>()];
        src.read_exact(&mut buf[..Self::DISK_REPR_SZ])?;
        let dptr = u64::from_le_bytes(buf);
        Ok(dptr.into())
    }

    /// Read 5 * `n` bytes from disk, for up to n=7, and parses them as
    /// little-endian `u64`s.
    pub(crate) fn read_n<R>(src: &mut R, n: usize) -> Result<Vec<Dp>>
    where
        R: Read,
    {
        debug_assert!(n <= 7);
        let mut buf = [0; Self::DISK_REPR_SZ * 7];
        src.read_exact(&mut buf[..(Self::DISK_REPR_SZ * n)])?;
        Ok(buf[..(Self::DISK_REPR_SZ * n)]
            .chunks(Self::DISK_REPR_SZ)
            .map(|chunk| {
                let mut buf = [0u8; size_of::<u64>()];
                buf[..Self::DISK_REPR_SZ].copy_from_slice(chunk);
                u64::from_le_bytes(buf)
            })
            .map(Dp::from)
            .collect())
    }

    /// Writes the 5 lower bytes of a `u64` to disk.
    pub(crate) fn write<W>(self, dst: &mut W) -> Result
    where
        W: Write,
    {
        let buf = self.0.to_le_bytes();
        Ok(dst.write_all(&buf[..Self::DISK_REPR_SZ])?)
    }
}

impl Add<usize> for Dp {
    type Output = Dp;

    fn add(self, rhs: usize) -> Dp {
        Dp::from(self.0 + rhs as u64)
    }
}

impl Add<u64> for Dp {
    type Output = Dp;

    fn add(self, rhs: u64) -> Dp {
        Dp::from(self.0 + rhs)
    }
}

impl Add<u32> for Dp {
    type Output = Dp;

    fn add(self, rhs: u32) -> Dp {
        Dp::from(self.0 + rhs as u64)
    }
}

impl From<Dp> for u64 {
    fn from(Dp(raw): Dp) -> u64 {
        raw
    }
}

impl From<u64> for Dp {
    fn from(raw: u64) -> Dp {
        assert!(raw <= Self::MAX);
        Dp(raw)
    }
}

impl From<usize> for Dp {
    fn from(raw: usize) -> Dp {
        Dp::from(raw as u64)
    }
}

impl From<Dp> for usize {
    fn from(Dp(raw): Dp) -> usize {
        usize::try_from(raw).unwrap()
    }
}
