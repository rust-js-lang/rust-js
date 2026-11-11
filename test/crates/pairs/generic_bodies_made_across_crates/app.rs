//! The app gives each a type that drops and clones aloud, so the library's
//! own copy of each is compiled here, and what each step skips or drops
//! prints where Rust drops it.
use dep::{chained_back, first_last, pair_of, peek, reversed, tail_pairs, without};
use std::cell::RefCell;

#[derive(Debug, PartialEq)]
struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Loud {
        println!("clone {}", self.0);
        Loud(self.0 + 100)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn louds() -> Vec<Loud> {
    vec![Loud(1), Loud(2), Loud(3), Loud(4)]
}

pub fn main() {
    println!("{:?}", reversed(louds()));
    println!("{:?}", tail_pairs(louds()));
    println!("{:?}", without(louds(), &Loud(3)));
    println!("{:?}", first_last(louds()));
    let cell = RefCell::new(louds());
    println!("{:?}", peek(&cell));
    println!("{:?}", pair_of(Loud(9)));
    println!("{:?}", chained_back(louds(), vec![Loud(5), Loud(6)]));
    println!("{:?}", reversed(vec![1u8, 2, 3]));
    println!("{:?}", first_last(vec!["a", "b"]));
}
