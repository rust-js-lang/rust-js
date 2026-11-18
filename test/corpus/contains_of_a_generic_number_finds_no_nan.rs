//@ run-fail: RefCell already borrowed
// `contains` in generic code every caller gives a number: a float's `NaN`
// isn't equal to itself, so `contains` never finds it, as JS's `indexOf`
// doesn't, where `includes` would (#2054, ADR 0966). Of `u32`s and of
// `f64`s, and the merge of floats under one `RefCell` whose push's
// borrow meets the loop's, which panics as Rust's does.
use std::cell::RefCell;
use std::rc::Rc;

fn has<T: Copy + PartialEq>(items: &[T], x: T) -> bool {
    items.contains(&x)
}

fn merge<T: Copy + PartialEq>(values: &Rc<RefCell<Vec<T>>>, into: &Rc<RefCell<Vec<T>>>) {
    for v in values.borrow().iter() {
        if !into.borrow().contains(v) {
            into.borrow_mut().push(*v);
        }
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let _loud = Loud("main");
    println!("{} {}", has(&[1u32, 2, 3], 2), has(&[1u32, 2, 3], 4));
    println!(
        "{} {} {}",
        has(&[1.5, f64::NAN], 1.5),
        has(&[1.5, f64::NAN], f64::NAN),
        has(&[0.0], -0.0)
    );
    let f = Rc::new(RefCell::new(vec![1.5, f64::NAN]));
    let g = Rc::new(RefCell::new(Vec::new()));
    merge(&f, &g);
    println!("{}", g.borrow().len());
    let u = Rc::new(RefCell::new(vec![1u32, 2]));
    merge(&u, &u);
    println!("{:?}", u.borrow());
    merge(&f, &f);
}
