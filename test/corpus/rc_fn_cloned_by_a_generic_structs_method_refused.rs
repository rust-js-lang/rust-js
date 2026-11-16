//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32 {double}>` that may be shared before
// An `Rc` of a function item cloned before it's unsized to a `dyn Fn`, by a
// generic struct's inherent method, `impl<T: Clone> Holder<T> { fn
// again(&self) -> T }`. Native Rust's `Rc::as_ptr`s of the two `Rc<dyn Fn>`s
// are one; read before the second is made, rust-js's would be two functions
// of their own, so it's refused where the crate reads a `dyn Fn`'s address
// (ADR 0959, #1053): one row of the clone routes' table (#1968).
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

struct Holder<T>(T);

impl<T: Clone> Holder<T> {
    fn again(&self) -> T {
        self.0.clone()
    }
}

fn main() {
    let h = Holder(Rc::new(double));
    let copy = h.again();
    let held = h.0;
    let first: Rc<dyn Fn(u32) -> u32> = held;
    let a = Rc::as_ptr(&first) as *const ();
    let second: Rc<dyn Fn(u32) -> u32> = copy;
    let b = Rc::as_ptr(&second) as *const ();
    println!("{} {}", a == b, first(1) + second(2));
}
