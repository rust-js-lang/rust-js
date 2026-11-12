//! A reader of the consumer's own read by its library into a `String` of
//! a capacity no JS string carries: std's reads of it would be of that
//! capacity, so it's refused as the program meets it (#1297, #1261).
use std::io::{self, Read};

struct Logged(usize);

impl Read for Logged {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.0);
        buf[..n].fill(b'g');
        self.0 -= n;
        Ok(n)
    }
}

pub fn main() {
    println!("{}", dep::read_into(&b"abc"[..], &mut String::with_capacity(8)));
    println!("{}", dep::read_into(Logged(3), &mut String::with_capacity(8)));
}
