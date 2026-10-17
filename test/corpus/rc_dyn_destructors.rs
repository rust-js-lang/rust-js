//@ library-refused: another crate may share
// An `Rc<dyn T>` of a value with a destructor is a counted `Rc` of the
// pair, `{ value: { value, impl }, strong, weak }`, made where the `Rc` is
// made (ADR 0320): its methods are the pair's, and the last of it drops
// what it points at through its dictionary's drop.
use std::rc::Rc;

trait Shape {
    fn area(&self) -> u32;
}

struct Square(u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

impl Drop for Square {
    fn drop(&mut self) {
        println!("drop square {}", self.0);
    }
}

fn made(side: u32) -> Rc<dyn Shape> {
    Rc::new(Square(side))
}

fn main() {
    let a: Rc<dyn Shape> = Rc::new(Square(3));
    let b = a.clone();
    println!("{} {}", b.area(), Rc::strong_count(&a));
    drop(a);
    println!("one left");
    drop(b);
    let shapes: Vec<Rc<dyn Shape>> = vec![made(1), made(2)];
    let first = Rc::clone(&shapes[0]);
    drop(shapes);
    println!("first {}", first.area());
    println!("end");
}
