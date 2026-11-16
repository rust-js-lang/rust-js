//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32 {double}>` that may be shared before
// An `Rc` of a function item made a `dyn Dup`, whose generic impl, `impl<T:
// Clone> Dup for T`, clones it through the dictionary, before it's unsized
// to a `dyn Fn`: no call names the `Rc`'s type, but the `dyn` gives it to
// generic code all the same, so it may be shared before, and it's refused
// where the crate reads a `dyn Fn`'s address (#1053, #1896).
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

trait Dup {
    fn dup(&self) -> Box<dyn std::any::Any>;
}

impl<T: Clone + 'static> Dup for T {
    fn dup(&self) -> Box<dyn std::any::Any> {
        Box::new(self.clone())
    }
}

fn main() {
    let held = Rc::new(double);
    let shown: &dyn Dup = &held;
    let copy = shown.dup();
    let first: Rc<dyn Fn(u32) -> u32> = held;
    let a = Rc::as_ptr(&first) as *const ();
    println!("{} {}", copy.is::<Rc<fn(u32) -> u32>>(), a.is_null());
}
