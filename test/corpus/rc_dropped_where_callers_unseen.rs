//@ library-differs
// Generic code whose callers can't all be seen, a `pub` trait's method,
// which JS or a consumer may call with any type (#1009): its `Rc<T>` of a
// value that drops is counted, so its value is dropped by its last clone,
// though no count is read.
use std::fmt::Debug;
use std::rc::Rc;

pub trait Keep {
    fn keep<T: Debug>(&self, value: T);
}

struct Keeper;

impl Keep for Keeper {
    fn keep<T: Debug>(&self, value: T) {
        let first = Rc::new(value);
        let other = Rc::clone(&first);
        drop(first);
        println!("still {other:?}");
        drop(other);
        println!("kept");
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
    Keeper.keep(Loud(1));
}
