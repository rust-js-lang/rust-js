// A closure that passes on to a scope struct's method is that method's
// function only where nothing reads a closure's identity (ADR 1043,
// amended, #500, the user's decision). Here the crate compares two boxed
// closures of one method by their addresses, `ptr::addr_eq`, which Rust
// made apart, so each stays an arrow of its own, and the comparison stays
// `false`.
use std::cell::Cell;
use std::rc::Rc;

type Step = dyn Fn(u32) -> u32;

struct Steps {
    count: Cell<u32>,
}

impl Steps {
    fn step(&self, by: u32) -> u32 {
        self.count.set(self.count.get() + by);
        self.count.get()
    }
}

fn boxes() -> (bool, bool, u32) {
    let steps = Rc::new(Steps { count: Cell::new(10) });
    let first: Box<Step> = {
        let steps = Rc::clone(&steps);
        Box::new(move |by| steps.step(by))
    };
    let second: Box<Step> = Box::new(move |by| steps.step(by));
    first(1);
    (
        std::ptr::addr_eq(&*first, &*second),
        std::ptr::addr_eq(&*first, &*first),
        second(1),
    )
}

fn main() {
    println!("{:?}", boxes());
}
