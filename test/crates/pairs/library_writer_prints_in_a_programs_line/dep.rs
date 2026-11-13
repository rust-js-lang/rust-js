//! A library's writers that print as they format (#1608): a list whose
//! `entries` print as they're stepped, a set of items that print as
//! they're dropped, and a struct that shows both through a builder; and a
//! `print!` that leaves stdout's line unended.
use std::fmt;

pub struct Stepped(pub Vec<u32>);

impl fmt::Debug for Stepped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.0.iter().map(|n| {
                print!("<step {n}>");
                n
            }))
            .finish()
    }
}

pub struct Loud(pub u32);

impl Drop for Loud {
    fn drop(&mut self) {
        print!("<drop {}>", self.0);
    }
}

impl fmt::Debug for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}", self.0)
    }
}

pub struct Dropping(pub u32);

impl fmt::Debug for Dropping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries((0..self.0).map(Loud)).finish()
    }
}

pub struct Outer;

impl fmt::Debug for Outer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Outer")
            .field("inner", &Stepped(vec![1, 2]))
            .field("set", &Dropping(2))
            .finish()
    }
}

pub struct Noisy(pub u32);

impl fmt::Debug for Noisy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("<noisy {}>", self.0);
        write!(f, "N{}", self.0)
    }
}

pub struct Many(pub Vec<Noisy>);

impl fmt::Debug for Many {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "m{:?}", self.0)
    }
}

pub fn many() -> Many {
    Many(vec![Noisy(1), Noisy(2)])
}

pub fn begin(word: &str) {
    print!("{word} ");
}
