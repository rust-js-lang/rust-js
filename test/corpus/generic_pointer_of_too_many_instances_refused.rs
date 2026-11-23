//@ compile-fail: generic code has one JS for each of `T`'s instances, and it has too many to check each holds its object where the others do (#2406)
// A pointer made in generic code to a value of a type parameter, in a
// crate that reads addresses: one JS for every instance, so each instance
// must hold its object where the others do (#1299). Given 65 types, more
// than the 64 the crate lists of one type parameter, they're too many to
// check, so it's refused rather than taken for each at its start. Native
// Rust prints `true` 65 times.
fn address<T>(value: &T) -> usize {
    value as *const T as usize
}

macro_rules! addresses {
    ($($n:literal)*) => {
        $(
            let array = [0u8; $n];
            println!("{}", address(&array) == &array as *const [u8; $n] as usize);
        )*
    };
}

fn main() {
    addresses!(1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64 65);
}
