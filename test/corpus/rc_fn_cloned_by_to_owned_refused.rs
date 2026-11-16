//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32 {double}>` that may be shared before
// An `Rc` of a function item cloned before it's unsized to a `dyn Fn`, by
// `held.to_owned()`: the call's bound is `Rc<..>: ToOwned`, and std's
// `impl<T: Clone> ToOwned for T` says `Clone`. Native Rust's `Rc::as_ptr`s
// of the two `Rc<dyn Fn>`s are one; read before the second is made,
// rust-js's would be two functions of their own, so it's refused where the
// crate reads a `dyn Fn`'s address (ADR 0959, #1053): one row of the clone
// routes' table (#1968).
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

fn main() {
    let held = Rc::new(double);
    let copy = held.to_owned();
    let first: Rc<dyn Fn(u32) -> u32> = held;
    let a = Rc::as_ptr(&first) as *const ();
    let second: Rc<dyn Fn(u32) -> u32> = copy;
    let b = Rc::as_ptr(&second) as *const ();
    println!("{} {}", a == b, first(1) + second(2));
}
