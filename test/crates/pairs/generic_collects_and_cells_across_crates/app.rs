//! The app gives the library's generic code its own types: numbers,
//! strings, structs with `Display` of their own that pad, cells of them,
//! and arrays it then changes, each against what Rust prints.
use dep::{at_least, doubled, len_of, padded, push_twice, read, three};
use std::cell::RefCell;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
struct Score(u16);

impl std::ops::Add for Score {
    type Output = Score;
    fn add(self, other: Score) -> Score {
        Score(self.0 + other.0)
    }
}

impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(&format!("s{}", self.0))
    }
}

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
    println!("{:?}", doubled(vec![1u8, 2, 3]));
    println!("{:?}", doubled(vec![Score(4), Score(5)]));
    println!("{:?}", at_least(vec![3, 9, 1, 7], 5));
    println!("{:?}", at_least(vec!["pear".to_string(), "apple".into(), "fig".into()], "b".to_string()));

    let cell = RefCell::new(vec![Score(1)]);
    println!("{:?} {}", read(&cell), len_of(&cell));
    println!("{} {:?}", push_twice(&cell, Score(2)), cell.borrow());
    let text = RefCell::new(String::from("hi"));
    println!("{}", read(&text));

    let mut a = three(Score(3));
    a[1] = Score(30);
    let mut b = three(2.5f32);
    b[0] = -1.0;
    println!("{a:?} {b:?}");

    println!("{}", padded(42));
    println!("{}", padded("ab"));
    println!("{}", padded(Score(7)));

    // Of a type that drops and clones aloud.
    println!("{:?}", at_least(vec![Loud(3), Loud(9), Loud(1), Loud(7)], Loud(5)));
    let cell = RefCell::new(Loud(8));
    println!("{:?}", read(&cell));
    let louds = RefCell::new(vec![Loud(1)]);
    println!("{} {}", push_twice(&louds, Loud(2)), len_of(&louds));
}
