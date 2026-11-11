//! A writer of its own that refuses text, and a value whose `Display`
//! fails past a bound: each error the library's result.
use std::fmt::{self, Write};

struct Refusing(u8);

impl Write for Refusing {
    fn write_str(&mut self, _: &str) -> fmt::Result {
        match self.0 {
            0 => Err(fmt::Error),
            _ => Ok(()),
        }
    }
}

struct Odd(u8);

impl fmt::Display for Odd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 > 1 {
            return Err(fmt::Error);
        }
        write!(f, "odd{}", self.0)
    }
}

pub fn main() {
    let mut refusing = Refusing(0);
    let mut taking = Refusing(1);
    println!("{:?} {:?}", dep::render(&mut refusing, 3), dep::render(&mut taking, 3));
    println!("{} {}", dep::rendered(&mut refusing, 3), dep::rendered(&mut taking, 3));
    let mut text = String::new();
    println!("{:?} {text:?}", dep::show_into(&mut text, &Odd(2)));
    println!("{:?} {text:?}", dep::show_into(&mut text, &Odd(1)));
    println!("{} {}", dep::shown(&Odd(2)), dep::shown(&Odd(0)));
}
