//@ native: wasm32
// `sum()` and `product()` of `Wrapping`s and `Saturating`s, of each and of
// references to each, in place and in generic code: std's `iter.fold(
// Wrapping(0), |a, b| a + b)`, each step its own `+` or `*`, wrapping or
// saturating, from `[0]` or `[1]`, an empty one's that. std has a
// `Saturating`'s for its unsigned integers alone. `usize` and `isize` as
// wasm32's. It was refused (#1445).
use std::iter::{Product, Sum};
use std::num::{Saturating, Wrapping};

fn total<T: Sum + Copy>(items: &[T]) -> T {
    items.iter().copied().sum()
}

fn total_of_refs<'a, T: Sum<&'a T> + 'a>(items: &'a [T]) -> T {
    items.iter().sum()
}

fn multiplied<T: Product + Copy>(items: &[T]) -> T {
    items.iter().copied().product()
}

macro_rules! each_unsigned {
    ($($t:ty),*) => {$(
        {
            let w = [Wrapping(<$t>::MAX), Wrapping(2 as $t), Wrapping(3 as $t)];
            let s = [Saturating(<$t>::MAX), Saturating(2 as $t), Saturating(3 as $t)];
            let ws: Wrapping<$t> = w.iter().sum();
            let wp: Wrapping<$t> = w.into_iter().product();
            let ss: Saturating<$t> = s.iter().copied().sum();
            let sp: Saturating<$t> = s.iter().product();
            println!("{} {ws} {wp} {ss} {sp}", stringify!($t));
            println!("{} {} {}", total(&w), total_of_refs(&s), multiplied(&s));
            let none: [Saturating<$t>; 0] = [];
            println!("{} {}", none.iter().sum::<Saturating<$t>>(), none.iter().product::<Saturating<$t>>());
        }
    )*};
}

macro_rules! each_signed {
    ($($t:ty),*) => {$(
        {
            let w = [Wrapping(<$t>::MIN), Wrapping(-1 as $t), Wrapping(<$t>::MIN)];
            println!("{} {} {} {}", stringify!($t), w.iter().sum::<Wrapping<$t>>(), w.iter().product::<Wrapping<$t>>(), total(&w));
        }
    )*};
}

fn main() {
    each_unsigned!(u8, u16, u32, u64, u128, usize);
    each_signed!(i8, i16, i32, i64, i128, isize);
}
