//@ library-differs
// Discriminants of a `repr` wider than 32 bits, BigInts in JS: a `u128`'s
// past `i128::MAX` ordered, cast, hashed and shown as `u128`'s, not wrapped
// negative, `Last < First` and `-1` where Rust's is `340282366920938463463374607431768211455`;
// and a `Discriminant` of one shown as its number (#1131). Their derives
// compiled through a library are `test/crates/pairs/wide_discriminants`.
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
#[repr(u128)]
enum Endless {
    First = 1,
    Last = u128::MAX,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, Hash)]
#[repr(i64)]
enum Wide {
    Low = -5,
    High(u8) = 1 << 40,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
#[repr(u64)]
enum Huge {
    Small = 3,
    Big = u64::MAX - 1,
}

fn digest<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn main() {
    println!("{} {}", Endless::First < Endless::Last, Endless::First.cmp(&Endless::Last) as i8);
    println!("{} {}", Huge::Small < Huge::Big, Wide::Low < Wide::High(0));
    let pick = |n: u8| if n > 1 { Endless::Last } else { Endless::First };
    println!("{} {}", pick(2) as u128, pick(0) as u128);
    println!("{:?} {:?}", std::mem::discriminant(&pick(2)), std::mem::discriminant(&Wide::High(1)));
    let hugest = |n: u8| if n > 1 { Huge::Big } else { Huge::Small };
    println!("{} {}", hugest(2) as u64, hugest(2) as i64);
    let mut endless = vec![Endless::Last, Endless::First];
    endless.sort();
    let mut wide = vec![Wide::High(2), Wide::Low, Wide::High(1)];
    wide.sort();
    println!("{endless:?} {wide:?}");
    println!("{} {} {}", digest(&Endless::Last), digest(&Wide::Low), digest(&Wide::High(7)));
}
