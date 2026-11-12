// Every operator of a `Wrapping` at every width, its `op=` by a `Wrapping`
// and by its number, as std's impls write them: `w += 1` reads its number,
// never `1[0]` (#1439); `/` and `%` are `wrapping_div` and `wrapping_rem`,
// `MIN / -1` its `MIN` and `MIN % -1` its `0`, not a panic (#1440); by zero
// each panics as std's does. `usize` and `isize` are a target's width, so
// they're left out.
use std::num::Wrapping;
use std::panic;

macro_rules! each_width {
    ($($t:ty),*) => {$(
        {
            let (lo, hi) = (Wrapping(<$t>::MIN), Wrapping(<$t>::MAX));
            let one = Wrapping(1 as $t);
            let two = Wrapping(2 as $t);
            println!("{} + {}", stringify!($t), hi + one);
            println!("{:?} {:?} {:?}", lo - one, hi * two, hi / two);
            println!("{} {} {}", hi % two, -lo, -one);
            println!("{} {} {} {}", !lo, hi & two, lo | one, hi ^ two);
            println!("{} {}", one << 9, hi >> 9);
            let mut w = hi;
            w += one;
            w += 1;
            w -= two;
            w -= 3;
            w *= two;
            w *= 5;
            println!("{w}");
            w /= two;
            w /= 3;
            w %= Wrapping(7 as $t);
            w %= 4;
            println!("{w}");
            w |= one;
            w |= 6;
            w &= hi;
            w &= 5;
            w ^= two;
            w ^= 9;
            w <<= 3;
            w >>= 1;
            println!("{w}");
            let zero = Wrapping(0 as $t);
            let by_zero = panic::catch_unwind(|| one / zero).is_err();
            let rem_zero = panic::catch_unwind(|| one % zero).is_err();
            let mut z = one;
            let assign_zero = panic::catch_unwind(move || { z /= 0; z }).is_err();
            println!("{by_zero} {rem_zero} {assign_zero}");
        }
    )*};
}

macro_rules! each_signed {
    ($($t:ty),*) => {$(
        {
            let lo = Wrapping(<$t>::MIN);
            let minus = Wrapping(-1 as $t);
            let mut w = lo;
            w /= minus;
            let mut r = lo;
            r %= -1;
            println!("{} {} {w} {r}", lo / minus, lo % minus);
        }
    )*};
}

/// Generic code's `/` and `%`, given a `Wrapping`: its dictionary's, std's.
fn divided<T: std::ops::Div<Output = T> + std::ops::Rem<Output = T> + Copy>(a: T, b: T) -> (T, T) {
    (a / b, a % b)
}

fn main() {
    each_width!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
    each_signed!(i8, i16, i32, i64, i128);
    println!("{:?}", divided(Wrapping(i8::MIN), Wrapping(-1)));
    println!("{:?}", divided(Wrapping(i64::MIN), Wrapping(-1)));
    println!("{:?}", divided(Wrapping(i64::MIN), Wrapping(7)));
}
