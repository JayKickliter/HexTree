use crate::error::{Error, Result};
use byteorder::{BigEndian as BE, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

// 134_217_727
// 2^27 - 1
#[allow(dead_code)]
const MAX_VARINT_VAL: u32 = 0x7FF_FFFF;

pub(crate) fn write<W: Write>(mut wtr: W, value: u32) -> Result<u64> {
    if value < 0x40 {
        // 01xx_xxxx
        wtr.write_u8((value | 0x40) as u8)?;
        Ok(1)
    } else if value < 0x2000 {
        // 001x_xxxx xxxx_xxxx
        wtr.write_u16::<BE>((value | 0x2000) as u16)?;
        Ok(2)
    } else if value < 0x10_0000 {
        // 0001_xxxx xxxx_xxxx xxxx_xxxx
        let value = value | 0x10_0000;
        wtr.write_u8((value >> 16) as u8)?;
        wtr.write_u16::<BE>((value & 0xffff) as u16)?;
        Ok(3)
    } else if value < 0x800_0000 {
        // 0000_1xxx xxxx_xxxx xxxx_xxxx xxxx_xxxx
        wtr.write_u32::<BE>(value | 0x800_0000)?;
        Ok(4)
    } else {
        Err(Error::Varint(value))
    }
}

pub(crate) fn read<R: Read>(mut rdr: R) -> Result<(u32, u64)> {
    let a = rdr.read_u8()?;
    match a.leading_zeros() {
        1 => {
            // 01xx_xxxx
            let val = (a & 0x3F) as u32;
            Ok((val, 1))
        }
        2 => {
            // 001x_xxxx xxxx_xxxx
            let a = (a & 0x1F) as u32;
            let b = rdr.read_u8()? as u32;
            let val = a << 8 | b;
            Ok((val, 2))
        }
        3 => {
            // 0001_xxxx xxxx_xxxx
            let a = (a & 0x0F) as u32;
            let b = rdr.read_u16::<BE>()? as u32;
            let val = a << 16 | b;
            Ok((val, 3))
        }
        4 => {
            // 0000_1xxx xxxx_xxxx xxxx_xxxx
            let a = (a & 0x07) as u32;
            let b = rdr.read_u8()? as u32;
            let c = rdr.read_u16::<BE>()? as u32;
            let val = a << 24 | b << 16 | c;
            Ok((val, 4))
        }
        _ => Err(Error::Varint(a as u32)),
    }
}

#[cfg(test)]
mod tests {
    use super::{read, write, MAX_VARINT_VAL};

    #[test]
    fn test_varint() {
        let mut buf = Vec::new();
        for val in 0..=MAX_VARINT_VAL {
            write(&mut buf, val).unwrap();
            assert!(buf[0].leading_zeros() > 0);
            let (r_val, _n) = read(&mut &buf[..]).unwrap();
            assert_eq!(val, r_val);
            buf.clear();
        }
        for val in MAX_VARINT_VAL + 1..=u32::MAX {
            assert!(write(&mut buf, val).is_err());
        }
    }
}
