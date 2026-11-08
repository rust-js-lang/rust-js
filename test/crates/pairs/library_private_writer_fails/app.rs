//! Its helper's `Err` its fallback, and what it was given dropped.
use dep::{Hour, stamp};

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    println!("{}", stamp(Loud(1), &Hour(7)));
    println!("{}", stamp(Loud(2), &Hour(30)));
}
