//! The library's clones reversed here, each cloned as it's stepped.
use dep::{Noisy, noisy};
use std::collections::VecDeque;

pub fn main() {
    let d: VecDeque<Noisy> = (1..4).map(Noisy).collect();
    let back: Vec<Noisy> = noisy(&d).rev().collect();
    println!("{back:?}");
    let mut both = noisy(&d);
    println!("{:?} {:?}", both.next_back(), both.next());
}
