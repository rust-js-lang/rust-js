// Generic code's `Rc::new` and `Rc::new_cyclic` of a type parameter a
// counted `#[rust_js::class]` may be, given a class's instance and a value
// of another type alike: its caller says which, `TInstance`, so the class's
// is the instance itself, its counts hidden beside it, and the other's a
// box, each counted, upgraded and dropped as native Rust does (#1423).
use std::cell::Cell;
use std::rc::{Rc, Weak};

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[cfg_attr(rust_js, rust_js::class)]
struct Chunk {
    n: Cell<u32>,
    held: Loud,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Chunk {
    fn new(n: u32) -> Self {
        Chunk {
            n: Cell::new(n),
            held: Loud(n),
        }
    }

    fn bump(self: &Rc<Self>) -> u32 {
        self.n.set(self.n.get() + 1);
        self.n.get() + Rc::strong_count(self) as u32
    }
}

fn share<T>(value: T) -> (Rc<T>, usize) {
    let shared = Rc::new(value);
    let again = Rc::clone(&shared);
    let owners = Rc::strong_count(&again);
    (shared, owners)
}

fn cyclic<T>(make: impl FnOnce(&Weak<T>) -> T) -> (Rc<T>, bool) {
    let none = Cell::new(false);
    let made = Rc::new_cyclic(|me| {
        none.set(me.upgrade().is_none());
        make(me)
    });
    (made, none.get())
}

fn main() {
    let (chunk, owners) = share(Chunk::new(5));
    println!("{owners} {}", chunk.bump());
    let (loud, owners) = share(Loud(7));
    println!("{owners} {}", loud.0);
    let weak = Rc::downgrade(&chunk);
    drop(chunk);
    println!("{}", weak.upgrade().is_none());
    let (made, none) = cyclic(|_| Chunk::new(9));
    println!("{none} {}", made.bump());
    let (other, none) = cyclic(|_| Loud(11));
    println!("{none} {}", other.0);
    drop(loud);
    println!("end");
}
