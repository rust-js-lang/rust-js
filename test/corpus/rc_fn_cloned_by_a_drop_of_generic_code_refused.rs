//@ compile-fail: a `dyn Fn` made of a `std::rc::Rc<fn(u32) -> u32 {double}>` that may be shared before
// An `Rc` of a function item cloned before it's unsized to a `dyn Fn` by a
// `Drop` impl of generic code, `impl<T: Clone> Drop for Keep<'_, T>`, of a
// struct a literal makes, `Keep { value: &held, .. }`: no call names `Keep`,
// so no obligation says what its `T` is, and the drop glue that runs it is
// no caller the crate sees. The clone of its bare `T` is the `Drop` impl's,
// whose callers aren't seen (`Cloned::owners_of`, `seen_whole`), so it
// counts. Native Rust's `Rc::as_ptr`s of the two `Rc<dyn Fn>`s are one;
// rust-js's would be two functions of their own, so it's refused where the
// crate reads a `dyn Fn`'s address (ADR 0959, #1053): one row of the clone
// routes' table (#1968, #1896).
use std::cell::RefCell;
use std::rc::Rc;

fn double(x: u32) -> u32 {
    x * 2
}

struct Keep<'a, T: Clone> {
    value: &'a T,
    out: &'a RefCell<Option<T>>,
}

impl<T: Clone> Drop for Keep<'_, T> {
    fn drop(&mut self) {
        *self.out.borrow_mut() = Some(self.value.clone());
    }
}

fn main() {
    let held = Rc::new(double);
    let out = RefCell::new(None);
    drop(Keep {
        value: &held,
        out: &out,
    });
    let copy = out.borrow_mut().take().unwrap();
    let first: Rc<dyn Fn(u32) -> u32> = held;
    let a = Rc::as_ptr(&first) as *const ();
    let second: Rc<dyn Fn(u32) -> u32> = copy;
    let b = Rc::as_ptr(&second) as *const ();
    println!("{} {}", a == b, first(1) + second(2));
}
