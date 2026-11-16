//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32 {double}>` that may be shared before
// An `Rc` of a function item cloned before it's unsized to a `dyn Fn`, by a
// blanket impl called on it, `held.dup()`, beside the same impl's `dyn` of
// it, `&held as &dyn Dup`. Native Rust's `Rc::as_ptr`s of the two `Rc<dyn
// Fn>`s are one; read before the second is made, rust-js's would be two
// functions of their own, so it's refused where the crate reads a `dyn Fn`'s
// address (ADR 0959, #1053): one row of the clone routes' table (#1968).
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

trait Dup {
    fn dup(&self) -> Self
    where
        Self: Sized;
    fn twin(&self) -> Box<dyn std::any::Any>;
}

impl<T: Clone + 'static> Dup for T {
    fn dup(&self) -> T {
        self.clone()
    }
    fn twin(&self) -> Box<dyn std::any::Any> {
        Box::new(())
    }
}

fn main() {
    let held = Rc::new(double);
    let shown: &dyn Dup = &held;
    let _ = shown.twin();
    let copy = held.dup();
    let first: Rc<dyn Fn(u32) -> u32> = held;
    let a = Rc::as_ptr(&first) as *const ();
    let second: Rc<dyn Fn(u32) -> u32> = copy;
    let b = Rc::as_ptr(&second) as *const ();
    println!("{} {}", a == b, first(1) + second(2));
}
