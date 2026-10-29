// A struct whose last field is made a `dyn` in a `RefCell`, an `Rc` of
// it shared: the cell is the struct, given its last field's `impl`, as
// one behind a reference is, so a borrow's call reads it, and so is one
// lent, `&RefCell<Framed<dyn Shape>>` (#375).
use std::cell::RefCell;
use std::rc::Rc;

trait Shape {
    fn grow(&mut self, by: f64);
    fn area(&self) -> f64;
}

struct Circle {
    r: f64,
}

impl Shape for Circle {
    fn grow(&mut self, by: f64) {
        self.r += by;
    }

    fn area(&self) -> f64 {
        self.r * self.r * 3.0
    }
}

struct Square(f64);

impl Shape for Square {
    fn grow(&mut self, by: f64) {
        self.0 *= by;
    }

    fn area(&self) -> f64 {
        self.0 * self.0
    }
}

impl Drop for Square {
    fn drop(&mut self) {
        println!("dropped a square of {}", self.0);
    }
}

struct Framed<T: ?Sized> {
    label: u32,
    shape: T,
}

fn total(shapes: &[Rc<RefCell<Framed<dyn Shape>>>]) -> f64 {
    shapes.iter().map(|s| s.borrow().shape.area()).sum()
}

fn grown(framed: &RefCell<Framed<dyn Shape>>) -> f64 {
    framed.borrow_mut().shape.grow(0.5);
    framed.borrow().shape.area()
}

fn main() {
    let lent = RefCell::new(Framed {
        label: 2,
        shape: Circle { r: 1.5 },
    });
    println!("{} {}", grown(&lent), lent.borrow().shape.r);
    let circle = Rc::new(RefCell::new(Framed {
        label: 0,
        shape: Circle { r: 1.0 },
    }));
    let shape: Rc<RefCell<Framed<dyn Shape>>> = circle.clone();
    shape.borrow_mut().shape.grow(1.0);
    println!("{}", circle.borrow().shape.r + f64::from(circle.borrow().label));
    let square = Rc::new(RefCell::new(Framed {
        label: 1,
        shape: Square(2.0),
    }));
    let shapes: Vec<Rc<RefCell<Framed<dyn Shape>>>> = vec![shape, square.clone()];
    shapes[1].borrow_mut().shape.grow(3.0);
    shapes[1].borrow_mut().label += 1;
    println!("{} {}", total(&shapes), square.borrow().label);
    drop(square);
    drop(shapes);
    println!("{}", Rc::strong_count(&circle));
}
