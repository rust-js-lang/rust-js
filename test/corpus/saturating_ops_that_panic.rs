// A `Saturating`'s `/` and `%` by zero panic, as std's `saturating_div`
// and `%` do, and a signed one's `MIN % -1`, std's `%` of its number, where
// its `MIN / -1` is its `MAX`; each `op=` too (#1434).
use std::num::Saturating;
use std::panic;

macro_rules! each_width {
    ($($t:ty),*) => {$(
        {
            let one = Saturating(1 as $t);
            let zero = Saturating(0 as $t);
            let by_zero = panic::catch_unwind(|| one / zero).is_err();
            let rem_zero = panic::catch_unwind(|| one % zero).is_err();
            let mut s = one;
            let assign_zero = panic::catch_unwind(move || {
                s %= 0;
                s
            })
            .is_err();
            println!("{} {by_zero} {rem_zero} {assign_zero}", stringify!($t));
        }
    )*};
}

macro_rules! each_signed {
    ($($t:ty),*) => {$(
        {
            let lo = Saturating(<$t>::MIN);
            let minus = Saturating(-1 as $t);
            let rem = panic::catch_unwind(|| lo % minus).is_err();
            println!("{} {} {rem}", stringify!($t), lo / minus);
        }
    )*};
}

fn main() {
    each_width!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);
    each_signed!(i8, i16, i32, i64, i128);
}
