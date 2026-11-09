//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32>` that may be shared before
// An `Rc` of a `fn` pointer and the one its `Weak` upgrades to are one
// allocation, so two `Rc<dyn Fn>`s made of them are one address to Rust.
// Where the crate reads a `dyn Fn`'s address, each would be a function of
// its own where it's unsized, apart, so it's refused (the user's decision,
// #1053).
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

fn main() {
    let held: Rc<fn(u32) -> u32> = Rc::new(double);
    let weak = Rc::downgrade(&held);
    let first: Rc<dyn Fn(u32) -> u32> = held;
    let second: Rc<dyn Fn(u32) -> u32> = weak.upgrade().expect("held");
    let same = Rc::as_ptr(&first) as *const () == Rc::as_ptr(&second) as *const ();
    println!("{same} {}", first(1) + second(2));
}
