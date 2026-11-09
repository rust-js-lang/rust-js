//! Enums of a `repr` wider than 32 bits, whose discriminants are BigInts in
//! JS, their derives compiled by the consumer from this crate's MIR (#1131).
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
#[repr(i64)]
pub enum Wide {
    Low = -5,
    High(u8) = 1 << 40,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
#[repr(u64)]
pub enum Huge {
    Small = 3,
    Big = u64::MAX - 1,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
#[repr(i128)]
pub enum Vast {
    Neg = -(1 << 100),
    Pos(u16) = 7,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
#[repr(u128)]
pub enum Endless {
    First = 1,
    Last = u128::MAX,
}

pub fn digest<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub fn same_wide(a: &Wide, b: &Wide) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

pub fn same_vast(a: &Vast, b: &Vast) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

pub fn huge_number(h: Huge) -> u64 {
    h as u64
}
