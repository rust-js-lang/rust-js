// A closure that passes on to a scope struct's method is that method's
// function only where nothing reads a closure's identity (ADR 1043,
// amended, #500, the user's decision). Here the crate compares two `Rc`s
// of closures of one method with `Rc::ptr_eq`, which Rust made apart, so
// each closure an `Rc` is made of stays an arrow of its own (ADR 0959),
// and the comparison stays `false`.
use std::cell::Cell;
use std::rc::Rc;

type Bump = dyn Fn(u32) -> u32;

struct Counter {
    count: Cell<u32>,
}

impl Counter {
    fn bump(&self, by: u32) -> u32 {
        self.count.set(self.count.get() + by);
        self.count.get()
    }
}

fn rcs() -> (bool, bool, u32) {
    let counter = Rc::new(Counter { count: Cell::new(0) });
    let first: Rc<Bump> = {
        let counter = Rc::clone(&counter);
        Rc::new(move |by| counter.bump(by))
    };
    let again = Rc::clone(&first);
    let second: Rc<Bump> = Rc::new(move |by| counter.bump(by));
    first(2);
    (Rc::ptr_eq(&first, &second), Rc::ptr_eq(&first, &again), second(3))
}

fn main() {
    println!("{:?}", rcs());
}
