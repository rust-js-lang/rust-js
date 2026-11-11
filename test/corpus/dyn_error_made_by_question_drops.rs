//@ library-differs
// A `dyn Error` made of a value with a destructor only by `?`'s `From<E>
// for Box<dyn Error>` drops it where Rust drops the box: the crate makes
// one so, std's conversion its `?` calls (#1403, ADR 0462).
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

fn failing() -> Result<u32, Box<dyn Error>> {
    Err(Loud(1))?
}

fn main() {
    match failing() {
        Ok(n) => println!("ok {n}"),
        Err(e) => println!("err {e}"),
    }
    println!("end");
}
