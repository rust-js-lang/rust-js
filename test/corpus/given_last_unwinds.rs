// What a variable is given last, `x = { let x = Loud(k); check(k); x }`,
// given it where it's made only where nothing between can leave before it
// is: of a value with a destructor, a panic between unwinds dropping the
// old one, and what's made, as Rust does, not the new one alone. A `Cell`
// read through a reference set again before the read, `node.v.set({ node
// = b; .. })`, or given to what keeps it, `hold(&node.v)` or a method of
// its own, `node.v.keep()`, is of the node it was made of (ADR 0849).
use std::cell::Cell;
use std::panic;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn check(k: u32) {
    if k == 3 {
        panic!("three");
    }
}

fn replaced(keys: &[u32]) -> u32 {
    let mut x = Loud(0);
    for &k in keys {
        x = {
            let x = Loud(k);
            check(k);
            x
        };
    }
    x.0
}

// A closure's variable it captures, set by what it makes, read after the
// closure unwinds: as it was.
fn caught(k: u32) -> u32 {
    let mut x = 1;
    let unwound = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        x = {
            let x = k * 10;
            check(k);
            x
        };
    }));
    println!("{}", unwound.is_err());
    x
}

struct Node {
    v: Cell<u32>,
}

fn node(v: u32) -> &'static Node {
    Box::leak(Box::new(Node { v: Cell::new(v) }))
}

fn hold(cell: &'static Cell<u32>) -> &'static Cell<u32> {
    cell
}

trait Keep {
    fn keep(&'static self) -> &'static Self;
}

impl Keep for Cell<u32> {
    fn keep(&'static self) -> &'static Self {
        self
    }
}

fn reassigned(a: &'static Node, b: &'static Node) -> (u32, u32, u32) {
    let mut node = a;
    node.v.set({
        node = b;
        node.v.get() + 1
    });
    let held = hold(&node.v);
    node = a;
    held.set(held.get() + 10);
    let kept = node.v.keep();
    node = b;
    kept.set(kept.get() + 1000);
    (a.v.get(), b.v.get(), node.v.get())
}

pub fn main() {
    let unwound = panic::catch_unwind(|| replaced(&[1, 3, 4]));
    println!("{}", unwound.is_err());
    println!("{}", replaced(&[1, 2]));
    println!("{:?}", reassigned(node(1), node(2)));
    println!("{} {}", caught(3), caught(2));
}
