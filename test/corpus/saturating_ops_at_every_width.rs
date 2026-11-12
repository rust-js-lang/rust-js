//@ native: wasm32
// `std::num::Saturating`, a number in a `[x]` as a `Wrapping` is, each
// operator the method of its number std's impl calls: `+`, `-` and `*`
// saturating, a signed one's `/` `saturating_div`, `MIN / -1` its `MAX`,
// `-` `saturating_neg`; `%`, the bit operations and `!` its number's. Its
// `op=` by a `Saturating` and by its number, its own methods, `pow`
// saturating, `abs` `saturating_abs`, its `MIN` and `MAX`, `{}`, `{:?}`,
// hex, `==`, order, `Hash` and derived `Default` its number's, at each
// width, `usize` and
// `isize` as wasm32's. It was refused (#1434). And a `Wrapping` of a
// `usize` or an `isize`, as wasm32's (#1439, #1440).
use std::collections::HashSet;
use std::num::{Saturating, Wrapping};

/// Generic code's operators, given a `Saturating`: its dictionary's, std's.
fn summed<T: std::ops::Add<Output = T> + std::ops::Mul<Output = T> + Copy>(a: T, b: T) -> (T, T) {
    (a + b, a * b)
}

macro_rules! each_width {
    ($($t:ty),*) => {$(
        {
            let (lo, hi) = (Saturating::<$t>::MIN, Saturating::<$t>::MAX);
            let one = Saturating(1 as $t);
            let two = Saturating(2 as $t);
            println!("{} {} {}", stringify!($t), hi + one, lo - one);
            println!("{:?} {} {}", hi * two, hi / two, hi % two);
            println!("{} {} {} {}", !lo, hi & two, lo | one, hi ^ two);
            let mut s = hi;
            s += one;
            s += 1;
            s -= two;
            s -= 3;
            s *= two;
            s *= 5;
            println!("{s}");
            s /= two;
            s /= 3;
            s %= Saturating(7 as $t);
            s %= 4;
            println!("{s}");
            s |= one;
            s |= 6;
            s &= hi;
            s &= 5;
            s ^= two;
            s ^= 9;
            println!("{s} {:x}", s);
            println!("{} {} {}", two.pow(3), hi.pow(2), two.pow(0));
            println!("{} {} {}", two.count_ones() + 1, hi.count_zeros(), one.leading_zeros());
            println!("{:?} {:?}", Saturating::<$t>::default(), Wrapping::<$t>::default());
            println!("{} {}", two.rotate_left(3), two.reverse_bits());
            println!("{} {} {:?}", lo < hi, one == Saturating(1), one.max(two));
            let set: HashSet<Saturating<$t>> = [one, two, one].into_iter().collect();
            println!("{} {:?}", set.len(), summed(hi, one));
        }
    )*};
}

macro_rules! each_signed {
    ($($t:ty),*) => {$(
        {
            let (lo, hi) = (Saturating::<$t>::MIN, Saturating::<$t>::MAX);
            let minus = Saturating(-1 as $t);
            println!("{} {} {} {}", stringify!($t), lo / minus, -lo, -minus);
            let mut s = lo;
            s /= minus;
            let mut t = hi;
            t -= -1;
            println!("{s} {t} {} {}", lo.abs(), minus.abs());
            println!("{} {} {}", lo.signum(), lo.is_negative(), hi.is_positive());
            println!("{} {}", minus.pow(3), lo.pow(2));
        }
    )*};
}

macro_rules! each_wrapping {
    ($($t:ty),*) => {$(
        {
            let (lo, hi) = (Wrapping(<$t>::MIN), Wrapping(<$t>::MAX));
            let one = Wrapping(1 as $t);
            let mut w = hi;
            w += 1;
            w *= 3;
            w -= one;
            println!("{} {} {} {} {w}", stringify!($t), hi + one, lo - one, hi * Wrapping(2 as $t));
            println!("{} {} {} {:x}", hi / Wrapping(2 as $t), hi % Wrapping(7 as $t), -one, hi);
        }
    )*};
}

fn main() {
    each_width!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);
    each_signed!(i8, i16, i32, i64, i128, isize);
    each_wrapping!(usize, isize);
    println!("{}", Wrapping(isize::MIN) / Wrapping(-1));
    println!("{}", Saturating(4u8).is_power_of_two());
    println!("{:?}", std::cmp::Reverse::<u8>::default());
    println!("{:?}", Saturating(250u8) + Saturating(10));
}
