//@ library-differs
// A cell made a `dyn` is asked whether another of the crate's traits its
// value has clashes with it (ADR 0397), each trait's own arguments made of
// that value: a trait of a lifetime, `Spans<'a>`, given an erased lifetime
// for it, not the type, where that crashed rust-js (#951).
use std::cell::RefCell;
use std::rc::Rc;

trait Shape {
    fn area(&self) -> f64;
}

trait Spans<'a> {
    fn span(&self) -> &'a str;
}

struct Circle {
    r: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.0 * self.r * self.r
    }
}

impl Spans<'static> for Circle {
    fn span(&self) -> &'static str {
        "circle"
    }
}

fn main() {
    let circle = Rc::new(RefCell::new(Circle { r: 2.0 }));
    let shape: Rc<RefCell<dyn Shape>> = circle.clone();
    println!("{} {}", shape.borrow().area(), circle.borrow().span());
}
