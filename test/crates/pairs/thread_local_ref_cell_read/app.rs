use std::rc::Rc;

use dep::{CURRENT, Fiber};

fn child() -> Option<Rc<Fiber>> {
    CURRENT.with_borrow(|current| current.clone())
}

pub fn main() {
    println!("{:?}", child().map(|fiber| fiber.tag));
    dep::set_current(Some(Rc::new(Fiber { tag: 5 })));
    let child = child();
    println!("{:?} {:?}", child.as_ref().map(|fiber| fiber.tag), dep::current_tag());
    dep::set_current(None);
    println!("{:?} {}", dep::current_tag(), child.is_some());
}
