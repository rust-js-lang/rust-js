//@ run-fail: RefCell already mutably borrowed
//@ library-differs
// An `Rc<RefCell<Circle>>` made an `Rc<RefCell<dyn Shape>>` shares its
// cell: a borrow of the `dyn` one held while the concrete one is borrowed
// is the same cell's, and the second finds it borrowed (ADRs 0397, 0863).

use std::cell::RefCell;
use std::rc::Rc;

trait Shape {
    fn grow(&mut self, by: f64);
}

struct Circle {
    r: f64,
}

impl Shape for Circle {
    fn grow(&mut self, by: f64) {
        self.r += by;
    }
}

fn radius(circle: &RefCell<Circle>) -> f64 {
    circle.borrow().r
}

fn main() {
    let circle = Rc::new(RefCell::new(Circle { r: 1.0 }));
    let shape: Rc<RefCell<dyn Shape>> = circle.clone();
    shape.borrow_mut().grow(1.0);
    println!("{}", radius(&circle));
    let mut held = shape.borrow_mut();
    held.grow(1.0);
    println!("{}", radius(&circle));
}
