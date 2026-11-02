//! An operator impl only generic code reads the dictionary of, written only
//! where it's reached (ADR 0477, amended): its JS exports its dictionary,
//! which `CheckedMul`'s reads as its supertrait's, and no function of its
//! `mul`, as rust_decimal's `Decimal`'s.
use std::ops::Mul;

#[derive(Clone, Copy, Debug)]
pub struct Digits(pub u32);

impl Mul for Digits {
    type Output = Digits;

    fn mul(self, other: Digits) -> Digits {
        Digits(self.0 * other.0)
    }
}

pub trait CheckedMul: Sized + Mul<Self, Output = Self> {
    fn checked_mul(&self, v: &Self) -> Option<Self>;
}

impl CheckedMul for Digits {
    fn checked_mul(&self, v: &Digits) -> Option<Digits> {
        self.0.checked_mul(v.0).map(Digits)
    }
}

pub fn squared<T: CheckedMul + Copy>(t: T) -> Option<T> {
    t.checked_mul(&t)
}
