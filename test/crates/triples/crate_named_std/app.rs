//! A crate using one that's given a crate named `std` (#455): its own
//! `std` is std's (ADR 1052).
pub fn main() {
    println!("{}", mid::quadruple(3));
    println!("{}", mid::exited(4));
    println!("{:?}", mid::taken(5));
    println!("{} {}", mid::hidden(6), std::hint::black_box(6));
    println!("{} {}", mid::same(1, 1), mid::same(1, 2));
    let mut x = 7;
    println!("{} {x}", std::mem::take(&mut x));
}
