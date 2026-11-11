//@ library-differs
// Generic code whose callers can't all be seen, a `pub` trait's method
// (#1009): its `Rc` of a closure that captures a value that drops is
// counted, so the value is dropped by the `Rc`'s last clone, though no
// count is read.
use std::fmt::Debug;
use std::rc::Rc;

pub trait Keep {
    fn keep_in_closure<T: Debug>(&self, value: T);
}

struct Keeper;

impl Keep for Keeper {
    fn keep_in_closure<T: Debug>(&self, value: T) {
        let first = Rc::new(move || println!("in {value:?}"));
        let other = Rc::clone(&first);
        drop(first);
        other();
        drop(other);
        println!("closed");
    }
}

#[derive(Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    Keeper.keep_in_closure(Loud(2));
}
