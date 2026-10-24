// A generic function's `Rc` of its type parameter's value, which no
// expression of its caller's names, `Rc<Loud>`: what it's given may have a
// destructor, which the last clone runs, so it's counted (ADRs 0494, 0844).
use std::rc::Rc;

struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn shared_twice<T>(value: T) {
    let first = Rc::new(value);
    let second = Rc::clone(&first);
    drop(first);
    println!("one left");
    drop(second);
    println!("both gone");
}

fn main() {
    shared_twice(Loud(1));
    shared_twice(2u8);
}
