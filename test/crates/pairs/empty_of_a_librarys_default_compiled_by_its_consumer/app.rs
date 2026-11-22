//! A consumer's impl of the library's trait, whose defaults it compiles,
//! each value of no size what std's constructor makes; and `Default` of
//! each, a field's and a type parameter's too (#1420).
use dep::Shrink;
use std::io::Read;

#[derive(Clone, Debug)]
struct Point;

impl Shrink for Point {}

#[derive(Default, Debug)]
struct Holder {
    items: std::iter::Empty<u8>,
    n: u8,
}

fn made<T: Default>() -> T {
    T::default()
}

pub fn main() {
    let p = Point;
    println!("{:?}", p.shrink().collect::<Vec<_>>());
    println!("{:?}", p.bytes().collect::<Vec<_>>());
    println!("{}", p.bytes().count());
    let mut s = String::new();
    println!("{} {s:?}", p.reader().read_to_string(&mut s).unwrap());

    let e: std::iter::Empty<u8> = Default::default();
    println!("{}", e.count());
    let h = Holder::default();
    println!("{h:?} {}", h.n);
    println!("{}", h.items.chain([1, 2]).count());
    let g: std::iter::Empty<i32> = made();
    println!("{:?}", g.collect::<Vec<_>>());
    let mut r: std::io::Empty = Default::default();
    println!("{} {s:?}", r.read_to_string(&mut s).unwrap());
}
