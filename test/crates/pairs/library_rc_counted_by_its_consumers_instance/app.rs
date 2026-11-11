//! A consumer that changes a `Copy` value in place and gives it to the
//! library's `counted_copy` (#1332).
#[derive(Clone, Copy, Debug)]
struct Point {
    n: u32,
}

pub fn main() {
    let mut p = Point { n: 1 };
    p.n += 1;
    let (n, q) = dep::counted_copy(p);
    p.n += 10;
    println!("{n} {q:?} {p:?}");
}
