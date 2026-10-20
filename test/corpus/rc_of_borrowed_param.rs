// An `Rc` of a reference to a type parameter's value drops nothing it
// points at: in a crate with a destructor, it's still the value it points
// at, not counted (ADR 0494).
use std::rc::Rc;

struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn shared<T>(value: &T) -> (Rc<&T>, Rc<&T>) {
    let one = Rc::new(value);
    (one.clone(), one)
}

fn main() {
    let loud = Loud(1);
    let (a, b) = shared(&loud);
    println!("{} {}", a.0, b.0);
    let n = 5;
    let (c, _) = shared(&n);
    println!("{c}");
}
