//@ compile-fail: generic code has one JS for each of `T`'s instances, and a `std::option::Option<Point>` is the object it holds, which isn't at its start, where a `Point` isn't (#1299)
// A pointer made in generic code to a value of a type parameter, in a
// crate that reads addresses: one JS for every instance, but a `Point` is
// its own object, at its start, where an `Option<Point>` holds it past its
// tag (ADR 0051), so its address is the object's less that. Native Rust
// prints `true` twice. Refused, naming its open issue, till generic code
// asks each instance where its object sits (#1299).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

fn address<T>(value: &T) -> usize {
    value as *const T as usize
}

fn main() {
    let p = Point { x: 1, y: 2 };
    let o = Some(Point { x: 3, y: 4 });
    let (a, b) = (address(&p), address(&o));
    println!("{}", a == &p as *const Point as usize);
    println!("{}", b == &o as *const Option<Point> as usize);
}
