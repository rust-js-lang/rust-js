// A box's content read, then written through it, `*n += 1`, before what
// it was read for runs: the read keeps the value it read, as a variable's
// does, where it was moved past the write and read the new one (#980).
// And a box of a reference written again, `*node = b`: a place reached
// through what it held is the one it was, as through a reference
// variable written again.
use std::cell::Cell;

struct Node {
    v: Cell<u32>,
}

struct Counter {
    n: u32,
}

fn bumped(a: &Node, b: &Node) -> (u32, u32) {
    let mut node = Box::new(a);
    (*node).v.set({
        *node = b;
        (*node).v.get() + 1
    });
    (a.v.get(), b.v.get())
}

fn moved_on(a: &mut Counter, b: &mut Counter) -> u32 {
    let mut at = Box::new(a);
    let n = &mut (**at).n;
    *at = b;
    *n += 1;
    (**at).n += 10;
    *n
}

fn pair(a: i32, b: i32) -> (i32, i32) {
    (a, b)
}

struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let mut n = Box::new(5);
    println!("{:?}", pair(*n, {
        *n += 1;
        *n
    }));
    let mut p = Box::new(Point { x: 1, y: 2 });
    let before = p.x;
    p.x = 10;
    println!("{before} {} {}", p.x, p.y);
    let mut s = Box::new(String::from("a"));
    let len = s.len();
    s.push('b');
    println!("{len} {s}");
    let mut total = Box::new(0);
    for i in 1..=3 {
        let was = *total;
        *total += i;
        println!("{was} -> {}", *total);
    }
    let (a, b) = (Node { v: Cell::new(1) }, Node { v: Cell::new(10) });
    println!("{:?}", bumped(&a, &b));
    let (mut c, mut d) = (Counter { n: 1 }, Counter { n: 100 });
    println!("{}", moved_on(&mut c, &mut d));
    println!("{} {}", c.n, d.n);
}
