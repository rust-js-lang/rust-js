// An `io::Error` made of a value with a destructor only by
// `io::Error::other` drops it where Rust drops the `io::Error` (#1403, ADR
// 0462).
use std::error::Error;
use std::fmt;
use std::io;

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
        let e = io::Error::other(Loud(4));
        println!("{e}");
    }
    println!("end");
}
