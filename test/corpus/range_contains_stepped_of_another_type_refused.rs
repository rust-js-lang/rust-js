//@ compile-fail: may have been stepped
// `contains` of another type's item in an `a..=b` of integers that may have
// been stepped is refused: JS's steps leave its start past its end, which
// the item's `PartialOrd` would be given, where Rust's keeps it and an
// `exhausted` flag (#1158).
use std::cmp::Ordering;

#[derive(PartialEq, PartialOrd, Debug)]
struct Meters(f64);

impl PartialEq<Meters> for u8 {
    fn eq(&self, other: &Meters) -> bool {
        f64::from(*self) == other.0
    }
}

impl PartialOrd<Meters> for u8 {
    fn partial_cmp(&self, other: &Meters) -> Option<Ordering> {
        println!("compare {self}");
        f64::from(*self).partial_cmp(&other.0)
    }
}

impl PartialEq<u8> for Meters {
    fn eq(&self, other: &u8) -> bool {
        self.0 == f64::from(*other)
    }
}

impl PartialOrd<u8> for Meters {
    fn partial_cmp(&self, other: &u8) -> Option<Ordering> {
        self.0.partial_cmp(&f64::from(*other))
    }
}

fn main() {
    let mut r = 1u8..=3;
    for _ in r.by_ref() {}
    println!("{}", r.contains(&Meters(3.0)));
}
