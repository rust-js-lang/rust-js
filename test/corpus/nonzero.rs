// `NonZero` integers: each its number, as chrono keeps a date's packed
// fields and deranged its bounds. `new` of `0` is `None`, and parsing or
// converting `0` an error, as Rust's.

use std::num::{NonZero, NonZeroI32, NonZeroU64, NonZeroU8};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Year(NonZeroI32);

fn year(n: i32) -> Option<Year> {
    NonZeroI32::new(n).map(Year)
}

fn main() {
    let a = NonZeroU8::new(7).unwrap();
    let b = NonZero::new(0u32);
    println!("{} {:?} {:?} {}", a, a, b, a.get() + 1);
    println!("{:?} {:?}", year(2024), year(0));
    let big = NonZeroU64::new(u64::MAX).unwrap();
    println!("{} {} {}", big, big.leading_zeros(), NonZeroU8::MIN.get());
    let c = a | 8;
    println!("{} {} {}", c, a < c, a == NonZeroU8::new(7).unwrap());
    println!("{:?} {:?}", "12".parse::<NonZeroU8>(), "0".parse::<NonZeroU8>());
    println!("{:?} {:?}", NonZeroU8::try_from(3u8).ok(), NonZeroU8::try_from(0u8));
    // SAFETY: not zero.
    let d = unsafe { NonZeroI32::new_unchecked(-5) };
    println!("{} {:?}", d.get().abs(), d.checked_mul(NonZeroI32::new(2).unwrap()));
    let mut v = vec![NonZeroU8::new(9).unwrap(), a, NonZeroU8::new(1).unwrap()];
    v.sort();
    println!("{:?} {}", v, std::mem::size_of::<Option<NonZeroU64>>());
    const SEVEN: NonZeroU8 = NonZeroU8::new(7).unwrap();
    for n in v {
        match n {
            SEVEN => println!("seven"),
            NonZeroU8::MIN => println!("one"),
            _ => println!("{}", n),
        }
    }
}
