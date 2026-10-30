//@ library-differs
// A `RefCell<dyn T>`, in an `Rc` or a `Box`, is its cell with its `impl`:
// the pair a `dyn` is, its guard too (ADR 0397). An `Rc` of the cell made
// before it's a `dyn` is the same cell: what's changed through one is seen
// through the other. One made a `dyn` of a second trait has both's methods.
use std::cell::RefCell;
use std::rc::Rc;

trait Shape {
    fn area(&self) -> f64;
    fn grow(&mut self, by: f64);
    fn name(&self) -> String;
}

#[derive(PartialEq)]
struct Circle {
    r: f64,
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        3.0 * self.r * self.r
    }
    fn grow(&mut self, by: f64) {
        self.r += by;
    }
    fn name(&self) -> String {
        format!("circle {}", self.r)
    }
}

trait Labeled {
    fn label(&self) -> String;
}

impl Labeled for Circle {
    fn label(&self) -> String {
        format!("r={}", self.r)
    }
}

struct Square {
    side: f64,
}

impl Shape for Square {
    fn area(&self) -> f64 {
        self.side * self.side
    }
    fn grow(&mut self, by: f64) {
        self.side += by;
    }
    fn name(&self) -> String {
        format!("square {}", self.side)
    }
}

fn total(shapes: &[Rc<RefCell<dyn Shape>>]) -> f64 {
    shapes.iter().map(|s| s.borrow().area()).sum()
}

fn grow_all(shapes: &[Rc<RefCell<dyn Shape>>], by: f64) {
    for s in shapes {
        s.borrow_mut().grow(by);
    }
}

fn main() {
    let circle = Rc::new(RefCell::new(Circle { r: 1.0 }));
    let shapes: Vec<Rc<RefCell<dyn Shape>>> = vec![circle.clone(), Rc::new(RefCell::new(Square { side: 2.0 }))];
    println!("{}", total(&shapes));
    grow_all(&shapes, 1.0);
    // Through the concrete `Rc`, the same cell.
    circle.borrow_mut().r = 10.0;
    for s in &shapes {
        println!("{} {}", s.borrow().name(), s.borrow().area());
    }
    *circle.borrow_mut() = Circle { r: 0.5 };
    println!("{}", shapes[0].borrow().name());
    // The same cell, a `dyn` of another trait too.
    let labeled: Rc<RefCell<dyn Labeled>> = circle.clone();
    shapes[0].borrow_mut().grow(1.0);
    println!("{} {}", labeled.borrow().label(), shapes[0].borrow().area());
    // Its `impl` isn't one of its fields: it's equal to a cell of its value.
    println!("{}", *circle == RefCell::new(Circle { r: 1.5 }));
    let boxed: Box<RefCell<dyn Shape>> = Box::new(RefCell::new(Square { side: 1.0 }));
    boxed.borrow_mut().grow(2.0);
    println!("{}", boxed.borrow().name());
    let again = shapes[1].try_borrow_mut();
    println!("{}", again.is_ok());
}
