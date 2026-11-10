//! A library whose own code reads no `Vec`'s capacity, so its JS keeps
//! none: its consumer that reads one compiles its own of each function it
//! reaches, so what they grow has std's capacity (#1180's (5)).
use std::cell::RefCell;
use std::rc::Rc;

pub fn squares(n: u32) -> Vec<u32> {
    let mut v = Vec::new();
    for i in 0..n {
        v.push(i * i);
    }
    v
}

/// Each of the library's own it reaches is the consumer's too.
pub fn squares_twice(n: u32) -> Vec<u32> {
    let mut v = squares(n);
    v.extend(squares(n));
    v
}

pub fn fill(v: &mut Vec<u32>, n: u32) {
    for i in 0..n {
        v.push(i);
    }
}

pub fn reserved() -> Vec<u8> {
    let mut v = Vec::with_capacity(3);
    v.reserve(40);
    v.push(1);
    v
}

pub struct Bag {
    pub items: Vec<String>,
}

impl Bag {
    pub fn new() -> Self {
        Bag { items: Vec::new() }
    }

    pub fn add(&mut self, s: &str) {
        self.items.push(s.to_string());
    }
}

impl Default for Bag {
    fn default() -> Self {
        Self::new()
    }
}

/// What a callback it's given does, the consumer's, beside its own.
pub fn grown_by(f: impl Fn(&mut Vec<u32>)) -> Vec<u32> {
    let mut v = Vec::new();
    f(&mut v);
    v.push(1);
    v
}

pub fn shared(rc: &Rc<RefCell<Vec<u32>>>) {
    rc.borrow_mut().extend((0..9).filter(|n| n % 2 == 0));
}
