// Generic code compares a type parameter by its dictionary wherever JS's own
// operators may not be Rust's (#1874): a caller gives it a struct, a
// `char`, whose `<` JS orders by UTF-16 units, or an `i64`, a `BigInt`; a
// generic caller gives on its own parameter, which a caller gives a
// `String`; the function is `pub`, which JS may call; or it asks
// `partial_cmp`, an `Option<Ordering>` no operator is.

use std::cmp::Ordering;

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
struct Version(u32, u32);

fn smaller<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if b < a { b } else { a }
}

fn equal<T: PartialEq>(a: T, b: T) -> bool {
    a == b
}

// Given `char`s alone.
fn before<T: PartialOrd>(a: T, b: T) -> bool {
    a < b
}

// Given `i64`s alone.
fn wide_below<T: PartialOrd>(a: T, b: T) -> bool {
    a < b
}

fn below<T: PartialOrd>(a: &T, b: &T) -> bool {
    a < b
}

fn given_on<T: PartialOrd>(a: &T, b: &T) -> bool {
    below(a, b)
}

fn order<T: PartialOrd>(a: T, b: T) -> (Option<Ordering>, bool) {
    let less = a < b;
    (a.partial_cmp(&b), less)
}

pub fn largest_of<T: PartialOrd + Copy>(a: T, b: T) -> T {
    if a < b { b } else { a }
}

fn main() {
    println!("{} {:?}", smaller(4, 2), smaller(Version(1, 9), Version(1, 2)));
    println!("{} {}", equal(0.0, -0.0), equal(Version(2, 0), Version(2, 0)));
    println!("{} {}", before('é', 'z'), before('\u{ffff}', '\u{10000}'));
    println!("{} {}", wide_below(i64::MAX, -1), wide_below(i64::MIN, 0));
    println!("{} {}", given_on(&1, &2), given_on(&String::from("b"), &String::from("a")));
    println!("{:?} {:?}", order(1, 2), order(f64::NAN, 1.0));
    println!("{} {}", largest_of(3, 8), largest_of(-0.5, -1.5));
}
