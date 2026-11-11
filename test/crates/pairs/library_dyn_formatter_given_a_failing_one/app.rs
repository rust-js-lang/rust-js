//! A value of its own whose `Display` fails past a bound, given as a `dyn`.
use std::fmt;

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
    println!("{} {}", dep::shown(&Odd(2)), dep::shown(&Odd(1)));
}
