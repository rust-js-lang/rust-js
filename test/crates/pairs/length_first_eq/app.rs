//! Unequal `TrustedLen` sides given to the library's `equal`, whose items
//! print as they drop: none compared, `b`'s dropped, then `a`'s, where a
//! filter, which isn't `TrustedLen`, is compared item by item; a range and a
//! slice's copies, collected first, which drop nothing; a `Vec`'s iterator
//! and a `VecDeque` given whole, counted.
use std::collections::VecDeque;

#[derive(Debug)]
struct L(u32);

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl PartialEq for L {
    fn eq(&self, other: &L) -> bool {
        println!("compare {} {}", self.0, other.0);
        self.0 == other.0
    }
}

impl PartialEq<L> for u32 {
    fn eq(&self, other: &L) -> bool {
        println!("compare {} {}", self, other.0);
        *self == other.0
    }
}

pub fn main() {
    println!("{}", dep::equal(vec![L(1), L(2)], vec![L(1), L(2), L(3)]));
    println!("{}", dep::equal(vec![L(4), L(5)], [L(4), L(6)]));
    println!("{}", dep::equal(vec![L(7), L(8)].into_iter().filter(|_| true), vec![L(7)]));
    let v = [1u32, 2, 3];
    println!("{}", dep::equal(v.iter().copied().rev(), vec![L(3), L(2)]));
    println!("{}", dep::equal(0..2u32, vec![L(0), L(1)]));
    println!("{}", dep::equal(vec![L(9)].into_iter(), [L(9), L(10)]));
    println!("{}", dep::equal(VecDeque::from([L(11), L(12)]), vec![L(11)]));
}
