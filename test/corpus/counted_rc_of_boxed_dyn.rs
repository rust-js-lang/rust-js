// A `Box<dyn T>` made a counted `Rc<dyn T>` or `Arc<dyn T>`, by `Rc::from`,
// `into()` or `Arc::from`: the box's value and `impl` with the counts, as
// one unsized from `Rc::new` is, so its calls, clones and drop read them
// (#399).
use std::rc::Rc;
use std::sync::Arc;

trait Shape {
    fn area(&self) -> u32;
}

struct Square(u32);

impl Shape for Square {
    fn area(&self) -> u32 {
        self.0 * self.0
    }
}

struct Loud(u32);

impl Shape for Loud {
    fn area(&self) -> u32 {
        self.0
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn boxed(n: u32) -> Box<dyn Shape> {
    Box::new(Square(n))
}

fn main() {
    let r: Rc<dyn Shape> = Rc::from(boxed(3));
    println!("{} {}", r.area(), Rc::strong_count(&r));
    let into: Rc<dyn Shape> = boxed(4).into();
    let again = Rc::clone(&into);
    println!("{} {}", again.area(), Rc::strong_count(&into));
    let shared: Arc<dyn Shape> = Arc::from(boxed(5));
    println!("{} {}", shared.area(), Arc::strong_count(&shared));
    let loud: Box<dyn Shape> = Box::new(Loud(7));
    let loud: Rc<dyn Shape> = Rc::from(loud);
    let other = loud.clone();
    println!("{} {}", other.area(), Rc::strong_count(&loud));
    drop(loud);
    println!("one left");
    drop(other);
    println!("done");
}
