//@ library-differs
// A `dyn Error` made of a value with a destructor only by std's `From<E>
// for Box<dyn Error + Send + Sync>`, called by its path, drops it where
// Rust drops the box (#1403, ADR 0462).
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct Loud(u32);

impl fmt::Display for Loud {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "loud {}", self.0)
    }
}

impl Error for Loud {}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

fn main() {
    {
        let e = <Box<dyn Error + Send + Sync>>::from(Loud(3));
        println!("{e}");
    }
    println!("end");
}
