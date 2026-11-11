//! A writer and a value of its own, given as `dyn`s.
use std::fmt;
use std::io::{self, Write};

struct Refusing(u8);

impl Write for Refusing {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.0 {
            0 => Err(io::Error::other("refused")),
            _ => Ok(bytes.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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
    println!("{:?}", dep::emit(&mut refusing).map_err(|e| e.to_string()));
    println!("{:?}", dep::emit(&mut taking).map_err(|e| e.to_string()));
    println!("{} {}", dep::emitted(&mut refusing), dep::emitted(&mut taking));
    println!("{}", dep::joined(&[Odd(0), Odd(1)]));
}
