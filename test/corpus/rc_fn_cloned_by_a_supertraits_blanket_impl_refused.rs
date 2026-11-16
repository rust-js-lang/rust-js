//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32 {double}>` that may be shared before
// An `Rc` of a function item cloned before it's unsized to a `dyn Fn`, by
// generic code bound by a trait, `outer<T: Twin>`, whose supertrait `Dup` is
// a blanket impl that clones it, `impl<T: Clone> Dup for T`: the call's
// bound is `Rc<..>: Twin`, whose impl says nothing, and its supertrait's
// says `Clone`. Native Rust's `Rc::as_ptr`s of the two `Rc<dyn Fn>`s are
// one; read before the second is made, rust-js's would be two functions of
// their own, so it's refused where the crate reads a `dyn Fn`'s address (ADR
// 0959, #1053): one row of the clone routes' table (#1968, #1896).
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

trait Twin: Dup {}

impl<T: ?Sized> Twin for Rc<T> {}

fn outer<T: Twin>(t: &T) -> T {
    t.dup()
}

fn main() {
    let held = Rc::new(double);
    let copy = outer(&held);
    let first: Rc<dyn Fn(u32) -> u32> = held;
    let a = Rc::as_ptr(&first) as *const ();
    let second: Rc<dyn Fn(u32) -> u32> = copy;
    let b = Rc::as_ptr(&second) as *const ();
    println!("{} {}", a == b, first(1) + second(2));
}
