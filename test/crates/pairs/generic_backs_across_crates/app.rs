//! The app gives the library's generic code iterators it steps from their
//! back: a `Vec`'s, a range's, a map of a slice's, a filter's, a chain's, a
//! string's `chars()`, an enumeration's, each with a closure of its own that
//! prints as it runs, so an item stepped twice or out of order shows.
use dep::{back_digits, ends, last_two, mapped_back, nth_from_back, pairs_back};

#[derive(Debug, Default, PartialEq, PartialOrd)]
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

pub fn main() {
    let v = vec![1, 2, 3, 4, 5];
    println!("{:?}", last_two(v.iter()));
    println!("{:?}", last_two(v.iter().map(|x| {
        println!("map {x}");
        x * 10
    })));
    println!("{:?}", ends(0..7));
    println!("{:?}", ends("héllo".chars()));
    println!("{:?}", ends(v.iter().filter(|x| {
        println!("filter {x}");
        **x % 2 == 1
    })));
    println!("{}", back_digits([1u32, 2, 3].into_iter()));
    println!("{}", back_digits((1..4u32).chain(7..9)));
    println!("{:?}", nth_from_back(v.iter().copied(), 1));
    println!("{:?}", nth_from_back("abc".char_indices(), 2));
    println!("{:?}", mapped_back(v.iter().copied().map(|x| x as i32)));
    println!("{:?}", mapped_back(-3..4));
    println!("{:?}", pairs_back(["x", "y", "z"].iter()));
    println!("{:?}", pairs_back(v.iter().map(|x| x + 1)));

    // Of items that drop, which each step skipped drops as Rust does.
    let louds = || vec![Loud(1), Loud(2), Loud(3), Loud(4)];
    println!("{:?}", last_two(louds().into_iter()));
    println!("{:?}", ends(louds().into_iter()));
    println!("{:?}", nth_from_back(louds().into_iter(), 1));
    println!("{:?}", pairs_back(louds().into_iter()));
    println!("{:?}", last_two(louds().into_iter().map(|l| Loud(l.0 + 10))));
}
