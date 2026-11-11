//! A library's generic code that makes an `Rc` of a type parameter its own
//! counted class may be, `Rc::new` and `Rc::new_cyclic`, and its consumer,
//! which gives it the library's class and a value that drops: the consumer
//! compiles its own copy (ADR 0914), which takes `TInstance` as the
//! library's does, of the library's class it counts too (#1423).
use std::cell::Cell;
use std::rc::{Rc, Weak};

pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct Chunk {
    pub n: Cell<u32>,
    pub held: Loud,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Chunk {
    pub fn new(n: u32) -> Self {
        Chunk {
            n: Cell::new(n),
            held: Loud(n),
        }
    }

    pub fn bump(self: &Rc<Self>) -> u32 {
        self.n.set(self.n.get() + 1);
        self.n.get() + Rc::strong_count(self) as u32
    }
}

pub fn share<T>(value: T) -> (Rc<T>, usize) {
    let shared = Rc::new(value);
    let again = Rc::clone(&shared);
    let owners = Rc::strong_count(&again);
    (shared, owners)
}

pub fn cyclic<T>(make: impl FnOnce(&Weak<T>) -> T) -> (Rc<T>, bool) {
    let none = Cell::new(false);
    let made = Rc::new_cyclic(|me| {
        none.set(me.upgrade().is_none());
        make(me)
    });
    (made, none.get())
}
