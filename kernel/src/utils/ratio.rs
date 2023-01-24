use core::fmt;

/// Convert `numerator / denominator` to `mult / (1 << shift)` to avoid `u128` division.
pub struct Ratio {
    numerator: u32,
    denominator: u32,
    mult: u32,
    shift: u32,
}

impl Ratio {
    pub const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 0,
            mult: 0,
            shift: 0,
        }
    }

    pub fn new(numerator: u32, denominator: u32) -> Self {
        assert!(!(denominator == 0 && numerator != 0));
        // numerator / denominator == (numerator * (1 << shift) / denominator) / (1 << shift)
        let mut shift = 31;
        let mut mult = 0;
        while shift > 0 {
            mult = (((numerator as u64) << shift) + denominator as u64 / 2) / denominator as u64;
            if mult <= u32::MAX as u64 {
                break;
            }
            shift -= 1;
        }
        Self {
            numerator,
            denominator,
            mult: mult as u32,
            shift,
        }
    }

    pub fn inverse(&self) -> Self {
        Self::new(self.denominator, self.numerator)
    }

    pub const fn mul(&self, value: u64) -> u64 {
        ((value as u128 * self.mult as u128) >> self.shift) as u64
    }
}

impl fmt::Debug for Ratio {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Ratio({}/{} ~= {}/{})",
            self.numerator,
            self.denominator,
            self.mult,
            1u32 << self.shift
        )
    }
}
