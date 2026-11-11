// `?` of a `Result` of another error in a function giving back an
// `io::Result<()>`, whose `Err` is thrown (ADR 0462), converts its error by
// the `From` `?` calls, the crate's `From<RandError> for io::Error`, as one
// giving back an `io::Result<u32>` does; it threw the `RandError` itself,
// which no caller caught as the `Err`, where Rust gives `Err` of the
// `io::Error` (rand_core 0.5's `From<Error> for io::Error`, found by #1403).
use std::error::Error;
use std::fmt;
use std::io;

#[derive(Debug)]
struct RandError(u32);

impl fmt::Display for RandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "rand {}", self.0)
    }
}

impl Error for RandError {}

impl From<RandError> for io::Error {
    fn from(error: RandError) -> Self {
        io::Error::other(error)
    }
}

fn fill() -> io::Result<()> {
    Err(RandError(4))?
}

fn fill_ok(n: u32) -> io::Result<u32> {
    if n > 1 {
        Err(RandError(n))?;
    }
    Ok(n)
}

fn main() {
    match fill() {
        Ok(()) => println!("ok"),
        Err(e) => println!("{e} {:?}", e.kind()),
    }
    println!("{:?}", fill_ok(1).map_err(|e| e.to_string()));
    println!("{:?}", fill_ok(5).map_err(|e| e.to_string()));
}
