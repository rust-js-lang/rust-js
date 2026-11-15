// A `Copy` value a `Box` holds, written whole, `*b += v` or `*b = v`: the
// box's value is the object itself, so the write changes it in place, where
// a variable's write is the variable given another. A copy read out of it
// before, `Box::new(*c)` or `let d = *c`, is its own (ADR 0052, #967).
use std::num::Wrapping as W;

#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
}

fn main() {
    let mut c = Box::new(W(5u32));
    let d = Box::new(*c);
    *c += *d << 1;
    println!("{} {}", c.0, d.0);

    let mut p = Box::new(Point { x: 1 });
    let before = *p;
    *p = Point { x: 2 };
    println!("{before:?} {p:?}");
}
