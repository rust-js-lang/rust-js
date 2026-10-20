// An `Rc` given to two libraries by their type parameters, which clone and
// drop it as Rust does (ADR 0523).
use std::rc::Rc;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

pub fn main() {
    let rc = Rc::new(Loud(3));
    let (a, b) = dep::pair(Rc::clone(&rc));
    println!("{}", Rc::strong_count(&rc));
    println!("{}", dep::count_all(vec![a, b]));
    println!("{}", Rc::strong_count(&rc));
    let mut bag = mid::Bag { items: vec![Rc::clone(&rc)] };
    bag.dup_first();
    println!("{} {}", bag.items.len(), Rc::strong_count(&rc));
    drop(bag);
    println!("{}", Rc::strong_count(&rc));
    drop(rc);
    println!("end");
}
