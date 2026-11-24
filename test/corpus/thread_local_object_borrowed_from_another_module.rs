// A thread-local whose borrows nothing counts is its module's variable, what
// its cell holds (ADR 0270), in every module that borrows it: another
// module's `with_borrow_mut` and `with_borrow` are given the variable as its
// own module's are, `Fiber.log.push(1)`, never counted; one of a value, a
// `String`, is set by its module and read by name in another. Where the crate
// has a generic impl for `RefMut<'_, T>`, a `RefCell` of an object is a
// `{ value }` (ADR 0863), which its module unboxes into its variable;
// another module's borrow, counted, read the `value` of the array and threw
// (#2531).
use std::cell::{RefCell, RefMut};

mod fiber {
    use std::cell::RefCell;

    thread_local! {
        pub static LOG: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
        pub static NAME: RefCell<String> = const { RefCell::new(String::new()) };
    }

    pub fn logged() -> usize {
        LOG.with_borrow(|log| log.len())
    }

    pub fn set_name(name: &str) {
        NAME.with_borrow_mut(|named| named.push_str(name));
    }
}

trait Observe {
    fn observe(&self) -> usize;
}

impl<T: ?Sized + Observe> Observe for RefMut<'_, T> {
    fn observe(&self) -> usize {
        (**self).observe()
    }
}

impl Observe for Vec<u32> {
    fn observe(&self) -> usize {
        self.len()
    }
}

fn main() {
    fiber::LOG.with_borrow_mut(|log| log.push(1));
    fiber::LOG.with_borrow_mut(|log| log.extend([2, 3]));
    fiber::set_name("App");
    println!("{:?} {}", fiber::LOG.with_borrow(|log| log.clone()), fiber::logged());
    println!("{}", fiber::NAME.with_borrow(|name| name.len()));
    let cell = RefCell::new(vec![4u32, 5]);
    println!("{}", cell.borrow_mut().observe());
}
