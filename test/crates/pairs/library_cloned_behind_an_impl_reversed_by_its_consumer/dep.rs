//! A clone of a deque's items a public function gives behind an `impl
//! DoubleEndedIterator`, which its consumer reverses where this library can't
//! see: made knowing its back where it's made, as std's `Cloned` is kept as
//! its meaning, not std's struct, and a deque's items have no form that steps
//! their clones from the back (#1024; #1979's `library-impl-backs-unmade`).
use std::collections::VecDeque;

#[derive(Debug)]
pub struct Noisy(pub u32);

impl Clone for Noisy {
    fn clone(&self) -> Noisy {
        println!("clone {}", self.0);
        Noisy(self.0)
    }
}

pub fn noisy(d: &VecDeque<Noisy>) -> impl DoubleEndedIterator<Item = Noisy> + '_ {
    d.iter().cloned()
}
