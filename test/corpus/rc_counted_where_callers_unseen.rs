// Generic code whose callers can't all be seen, a `pub` trait's methods,
// which JS or a consumer may call with any type (#1009): its `Rc<Slot<T>>`
// is counted where a `Slot<u32>`'s is, one representation for both, as
// each may be the other; and its `Rc<T>` may be any `Rc`, so beside one
// counted every `Rc` is, an `Other`'s given it too.
use std::fmt::Debug;
use std::rc::Rc;

struct Slot<T> {
    value: T,
}

pub trait Peek {
    fn peek<T: Copy>(&self, slot: &Rc<Slot<T>>) -> T;
    fn show<T: Debug>(&self, rc: &Rc<T>) -> String;
}

struct Peeker;

impl Peek for Peeker {
    fn peek<T: Copy>(&self, slot: &Rc<Slot<T>>) -> T {
        slot.value
    }

    fn show<T: Debug>(&self, rc: &Rc<T>) -> String {
        format!("{rc:?}")
    }
}

#[derive(Debug)]
struct Other {
    name: &'static str,
}

fn main() {
    let slot = Rc::new(Slot { value: 5u32 });
    let again = Rc::clone(&slot);
    println!("{} {}", Rc::strong_count(&slot), Peeker.peek(&again));
    let other = Rc::new(Other { name: "x" });
    println!("{} {}", Peeker.show(&other), other.name);
}
