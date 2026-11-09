//! What a consumer may compare the identity of, made by a library that
//! compares none itself (#1052): two `Rc::new`s are two allocations, and two
//! boxes of closures two addresses, as Rust promises across crates.
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// Two `Rc<dyn Fn>`s of one closure, cloned: two allocations.
pub fn cloned(count: &Rc<RefCell<u32>>) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>) {
    let count = Rc::clone(count);
    let read = move || *count.borrow();
    let first: Rc<dyn Fn() -> u32> = Rc::new(read.clone());
    let second: Rc<dyn Fn() -> u32> = Rc::new(read);
    (first, second)
}

/// # Safety
/// Its target stays put while it's moved.
pub unsafe trait StableDeref: Deref {}

unsafe impl<T: ?Sized> StableDeref for Rc<T> {}

pub fn total<D: StableDeref<Target = Vec<u32>>>(items: D) -> u32 {
    items.iter().sum()
}

pub struct Node {
    pub compute: Box<dyn Fn(Option<Rc<u32>>) -> Rc<u32>>,
}

/// A box of a closure that converts what it's given and passes it on to the
/// function it captured: each its own allocation.
pub fn computed(getter: impl Fn(Option<&u32>) -> u32 + 'static) -> Node {
    Node {
        compute: Box::new(move |previous: Option<Rc<u32>>| Rc::new(getter(previous.as_deref()))),
    }
}

/// The same, made through another function of the library's, which its
/// consumer compiles too. Of a `fn` pointer, which each closure holds, so
/// each box has an address of its own: of `double` itself, the closure
/// would be of no size, whose boxes' addresses Rust promises nothing of.
pub fn doubled() -> Node {
    let getter: fn(Option<&u32>) -> u32 = double;
    computed(getter)
}

pub fn double(previous: Option<&u32>) -> u32 {
    previous.map_or(1, |n| n * 2)
}

pub type Bump = dyn Fn(u32) -> u32;

struct Counter {
    count: Cell<u32>,
}

impl Counter {
    fn bump(&self, by: u32) -> u32 {
        self.count.set(self.count.get() + by);
        self.count.get()
    }
}

/// Two closures that pass on to a scope struct's method, each its own box
/// (ADR 1043): the method's function, where the library compares none.
pub fn bumps() -> (Box<Bump>, Box<Bump>) {
    let counter = Rc::new(Counter { count: Cell::new(0) });
    let first: Box<Bump> = {
        let counter = Rc::clone(&counter);
        Box::new(move |by| counter.bump(by))
    };
    let second: Box<Bump> = Box::new(move |by| counter.bump(by));
    (first, second)
}

/// What makes two `Rc<dyn Fn>`s of one closure, through its impl.
pub trait Pair {
    fn pair(&self) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>);
}

pub struct Twice(pub u32);

impl Pair for Twice {
    fn pair(&self) -> (Rc<dyn Fn() -> u32>, Rc<dyn Fn() -> u32>) {
        let n = self.0;
        let read = move || n;
        (Rc::new(read), Rc::new(read))
    }
}

fn ping() {
    println!("ping");
}

/// Two closures that ignore what they're given and call one function, each
/// its own `Rc`: that function, where the library compares none.
pub fn pings() -> (Rc<dyn Fn(bool)>, Rc<dyn Fn(bool)>) {
    (Rc::new(|_| ping()), Rc::new(|_| ping()))
}
