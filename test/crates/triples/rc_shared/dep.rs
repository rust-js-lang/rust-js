// The crate an `Rc` is made in: its counts, its `Weak`s, and a last owner
// dropped here (ADR 0523).
use std::rc::{Rc, Weak};

pub struct Noisy(pub &'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

pub fn tag(text: &str) -> Rc<String> {
    Rc::new(text.to_string())
}

pub fn make(name: &'static str) -> Rc<Noisy> {
    Rc::new(Noisy(name))
}

pub fn count(rc: &Rc<Noisy>) -> usize {
    Rc::strong_count(rc)
}

pub fn finish(rc: Rc<Noisy>) {
    println!("dep drops {}", rc.0);
}

pub fn watch(rc: &Rc<Noisy>) -> Weak<Noisy> {
    Rc::downgrade(rc)
}

pub fn alive(weak: &Weak<Noisy>) -> bool {
    weak.upgrade().is_some()
}
