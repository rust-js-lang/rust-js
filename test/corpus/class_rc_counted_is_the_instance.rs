// A counted `Rc` of a `#[rust_js::class]` is the instance itself, its
// counts hidden beside it, so JS calls its methods on it as on any instance
// (#1228, the user's decision): a constructor that gives `Rc<Self>`, a
// method whose `self` is `&Rc<Self>`, its counts, its `Weak`s upgraded and
// failing once it's dropped, what it holds dropped at its last owner, and
// generic code given it, each as native Rust does; and an `Rc::new` of an
// instance a constructor gave as it is.
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
    n: u32,
    seen: Cell<u32>,
    held: Loud,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Chunk {
    fn new(n: u32) -> Rc<Self> {
        Rc::new(Chunk {
            n,
            seen: Cell::new(0),
            held: Loud(n),
        })
    }

    fn watch(self: &Rc<Self>) -> u32 {
        self.seen.set(self.seen.get() + 1);
        self.n + Rc::strong_count(self) as u32
    }

    fn weak(self: &Rc<Self>) -> Weak<Self> {
        Rc::downgrade(self)
    }

    fn total(&self) -> u32 {
        self.seen.get() + self.held.0
    }
}

#[cfg_attr(rust_js, rust_js::class)]
struct Plain {
    held: Loud,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Plain {
    fn new(n: u32) -> Self {
        Plain { held: Loud(n) }
    }

    fn n(&self) -> u32 {
        self.held.0
    }
}

fn owners<T>(rc: &Rc<T>) -> (usize, usize) {
    (Rc::strong_count(rc), Rc::weak_count(rc))
}

fn main() {
    let chunk = Chunk::new(5);
    let other = chunk.clone();
    println!("{} {}", chunk.watch(), other.watch());
    println!("{:?} {}", owners(&chunk), Rc::ptr_eq(&chunk, &other));
    let weak = chunk.weak();
    println!("{:?} {}", owners(&chunk), weak.upgrade().map_or(0, |c| c.watch()));
    drop(other);
    println!("{:?} {}", owners(&chunk), chunk.total());
    drop(chunk);
    println!("{} {}", weak.upgrade().is_none(), weak.strong_count());
    let plain = Rc::new(Plain::new(7));
    let again = Rc::clone(&plain);
    println!("{} {:?}", again.n(), owners(&plain));
    drop(plain);
    println!("{:?}", owners(&again));
}
