use crate::index::Index;

pub(crate) struct Digits {
    digits: u64,
    remaining: u8,
}

impl Digits {
    #[inline]
    pub(crate) fn new(idx: Index) -> Self {
        let res = idx.resolution();
        let mask = u128::MAX.wrapping_shl(64 - (3 * res as u32)) as u64;
        let digits: u64 = idx.0.wrapping_shl(19) & mask;
        Self {
            digits,
            remaining: res,
        }
    }
}

impl Iterator for Digits {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            None
        } else {
            let out = (self.digits & (0b111 << 61)) >> 61;
            self.digits <<= 3;
            debug_assert!(out < 7);
            self.remaining -= 1;
            Some(out as u8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digits() {
        let test_cases: &[(u64, &[u8])] = &[
            (577164439745200127, &[]),                    // res 0
            (585793956755800063, &[2, 0]),                // res 2
            (592638622797135871, &[6, 3, 2]),             // res 3
            (596251300178427903, &[3, 6, 6, 2]),          // res 4
            (599803672997658623, &[3, 4, 4, 1, 4]),       // res 5
            (604614882611953663, &[1, 4, 0, 4, 1, 0]),    // res 6
            (608557861265473535, &[2, 0, 2, 3, 2, 1, 1]), // res 7
        ];
        for (index, ref_digits) in test_cases {
            let idx = Index::from_raw(*index).unwrap();
            let digits = Digits::new(idx).collect::<Vec<u8>>();
            assert_eq!(&&digits, ref_digits);
        }
    }
}
