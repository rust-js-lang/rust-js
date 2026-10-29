//! Its first argument a value with a destructor, its others `()`s: dropped
//! by the callback that's given it, and the fiber restored after.
use std::rc::Rc;

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn main() {
    let fiber = Rc::new(dep::Fiber { tag: 3 });
    dep::run_with_fiber_in_dev(
        Some(fiber),
        |a: Loud, (), (), (), ()| println!("{} {:?}", a.0, dep::tag_now()),
        Loud(1),
        (),
        (),
        (),
        (),
    );
    println!("after {:?}", dep::tag_now());
}
