// A trait of classes: each method, an impl's or the trait's default, is
// each class's prototype's, a default compiled for each class with `Self`
// known, one function shared where the copies print alike (rd283).
use std::cell::{Cell, RefCell};

trait Counter {
    fn counter(&self) -> &Cell<u32>;
    fn label(&self) -> String;

    fn step(&self, by: u32) -> u32 {
        self.counter().set(self.counter().get() + by);
        self.counter().get()
    }

    fn describe(&self) -> String {
        format!("{} at {}", self.label(), self.counter().get())
    }
}

#[cfg_attr(rust_js, rust_js::class)]
struct Clicks {
    count: Cell<u32>,
    log: RefCell<Vec<String>>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Clicks {
    fn new() -> Self {
        Clicks {
            count: Cell::new(0),
            log: RefCell::new(Vec::new()),
        }
    }
}

impl Counter for Clicks {
    fn counter(&self) -> &Cell<u32> {
        &self.count
    }

    fn label(&self) -> String {
        "clicks".to_string()
    }

    fn step(&self, by: u32) -> u32 {
        self.log.borrow_mut().push(format!("+{by}"));
        self.count.set(self.count.get() + by * 2);
        self.count.get()
    }
}

#[cfg_attr(rust_js, rust_js::class)]
struct Keys {
    count: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Keys {
    fn new(start: u32) -> Self {
        Keys { count: Cell::new(start) }
    }
}

impl Counter for Keys {
    fn counter(&self) -> &Cell<u32> {
        &self.count
    }

    fn label(&self) -> String {
        format!("keys from {}", 1)
    }
}

fn main() {
    let clicks = Clicks::new();
    let keys = Keys::new(5);
    println!("{} {}", clicks.step(1), clicks.step(2));
    println!("{} {}", keys.step(1), keys.step(3));
    println!("{}", clicks.describe());
    println!("{}", keys.describe());
    println!("{:?}", clicks.log.borrow());
}
