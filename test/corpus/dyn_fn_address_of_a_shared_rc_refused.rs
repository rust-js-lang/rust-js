//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32>` that may be shared before
// An `Rc` of a `fn` pointer cloned before it's unsized is one allocation,
// so two `Rc<dyn Fn>`s made of its clones are one address to Rust. Where
// the crate reads a `dyn Fn`'s address, each `Rc` made a `dyn Fn` of a
// `fn` pointer is a function of its own where it's unsized, which would
// make the two apart, and it can't be one where `Rc::new` is given it,
// which a `fn` pointer read out of it compares by. So it's refused (the
// user's decision, #1053).
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

fn main() {
    let held: Rc<fn(u32) -> u32> = Rc::new(double);
    let first: Rc<dyn Fn(u32) -> u32> = held.clone();
    let second: Rc<dyn Fn(u32) -> u32> = held;
    println!("{} {}", std::ptr::addr_eq(&*first, &*second), first(1) + second(2));
}
