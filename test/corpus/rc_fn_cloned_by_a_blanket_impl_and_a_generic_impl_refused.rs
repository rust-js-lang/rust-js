//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32 {double}>` that may be shared before
// An `Rc` of a function item cloned before it's unsized to a `dyn Fn`, by a
// trait's blanket impl, `impl<T: Clone> Dup for T`, called on it,
// `held.dup()`, and a generic struct's inherent method,
// `Holder(rc).again()`. Native Rust's `Rc::as_ptr`s of the two `Rc<dyn Fn>`s
// are one; read before the second is made, rust-js's would be two functions
// of their own, so it's refused where the crate reads a `dyn Fn`'s address
// (ADR 0959, #1053): one row of the clone routes' table (#1968).
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

trait Dup {
    fn dup(&self) -> Self;
}

impl<T: Clone> Dup for T {
    fn dup(&self) -> T {
        self.clone()
    }
}

struct Holder<T>(T);

impl<T: Clone> Holder<T> {
    fn again(&self) -> T {
        self.0.clone()
    }
}

fn main() {
    let held = Rc::new(double);
    let copy = held.dup();
    let other = Holder(Rc::new(double)).again();
    let first: Rc<dyn Fn(u32) -> u32> = held;
    let second: Rc<dyn Fn(u32) -> u32> = copy;
    let third: Rc<dyn Fn(u32) -> u32> = other;
    let a = Rc::as_ptr(&first) as *const ();
    let b = Rc::as_ptr(&second) as *const ();
    println!("{} {} {}", a == b, first(1) + second(2), third(3));
}
