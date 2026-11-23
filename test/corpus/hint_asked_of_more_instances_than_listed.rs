// Generic code that asks the hint of what it's given, `hint_of`, given 65
// iterators of distinct types, one closure each: more instances than the
// crate lists (`MOST_INSTANCES`, 64), each type its callers give made
// knowing its hint all the same, never none, where JS's own `.map` of an
// array's items knows no hint (#2406).

fn hint_of<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

macro_rules! hints {
    ($v:ident, $($n:literal)*) => {
        $(println!("{} {:?}", $n, hint_of($v.iter().map(|x| x + $n)));)*
    };
}

fn main() {
    let v = vec![1u32, 2, 3, 4, 5];
    hints!(v, 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64);
}
