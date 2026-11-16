//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32 {double}>` that may be shared before
// An `Rc` of a function item given to generic code that clones its type
// parameter, `dup<T: Clone>(&held)`, before it's unsized: the clone is the
// same allocation, so the two `Rc<dyn Fn>`s made of them are one address to
// Rust. Where the crate reads a `dyn Fn`'s address, each is a function of
// its own where it's unsized, which would tell them apart, so it's refused
// (the user's decision, #1053), as a clone of it written out is; a generic
// clone counts where the crate gives generic code what holds it (#1896).
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

fn dup<T: Clone>(t: &T) -> T {
    t.clone()
}

fn main() {
    let held = Rc::new(double);
    let copy = dup(&held);
    let first: Rc<dyn Fn(u32) -> u32> = held;
    let second: Rc<dyn Fn(u32) -> u32> = copy;
    let a = Rc::as_ptr(&first) as *const ();
    let b = Rc::as_ptr(&second) as *const ();
    println!("{} {}", a == b, first(1) + second(2));
}
