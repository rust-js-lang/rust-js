//! Each `Vec` its library grew, with std's capacity.
use dep::{Bag, fill, grown_by, reserved, shared, squares, squares_twice};
use std::cell::RefCell;
use std::rc::Rc;

pub fn main() {
    let a = squares(5);
    println!("{} {}", a.len(), a.capacity());
    let b = squares_twice(5);
    println!("{} {}", b.len(), b.capacity());
    let mut c = vec![9];
    fill(&mut c, 6);
    println!("{} {}", c.len(), c.capacity());
    let d = reserved();
    println!("{} {}", d.len(), d.capacity());
    let mut bag = Bag::new();
    for word in ["a", "b", "c", "d", "e"] {
        bag.add(word);
    }
    println!("{} {}", bag.items.len(), bag.items.capacity());
    let e = grown_by(|v| v.extend([1, 2, 3, 4]));
    println!("{} {}", e.len(), e.capacity());
    let rc = Rc::new(RefCell::new(vec![7]));
    shared(&rc);
    println!("{} {}", rc.borrow().len(), rc.borrow().capacity());
}
