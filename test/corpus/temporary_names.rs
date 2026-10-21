// A temporary kept to be read more than once, a pattern's that takes it
// apart, is named as a Rust developer's tooling names it, rust-analyzer's
// suggestion (ADR 0569): after the method the source calls,
// `firstKeyValue` and `lastKeyValue`, `borrowMut` and `borrow`, and `get`
// of a reference `println!` takes to its result; after the program's type
// it holds, `point` of an `Option<Point>`; and `value` where both are
// taken, `find` read and `point` another's.

use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

fn find(points: &[Point], x: i32) -> Option<Point> {
    points.iter().copied().find(|p| p.x == x)
}

fn main() {
    let ages = BTreeMap::from([("ann", 31), ("bob", 27), ("cy", 40)]);
    if let Some((name, age)) = ages.first_key_value() {
        println!("first {name} {age}");
    }
    if let Some((name, age)) = ages.last_key_value() {
        println!("last {name} {age}");
    }
    // What `println!` shows, a reference it takes to a call's result: the
    // call's name too.
    println!("{:?} {:?}", ages.get("bob"), ages.len().checked_sub(4));
    let points = [Point { x: 1, y: 2 }, Point { x: 3, y: 4 }];
    if let Some(Point { x, y }) = find(&points, 3) {
        println!("at {x} {y}");
    }
    if let Some(Point { x, y }) = find(&points, 1) {
        println!("at {x} {y}");
    }
    let count = RefCell::new(0);
    for step in [1, 2, 3] {
        *count.borrow_mut() += step;
    }
    println!("{}", count.borrow());
}
