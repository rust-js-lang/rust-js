//! A library's generic code reading what it's given, and a reader of its
//! own: a consumer's reader sees each read std's `read_to_end` makes, of the
//! capacity of what it reads into, the library's `Vec::with_capacity` or the
//! consumer's, and the fresh `String` the library proves at its own call
//! (#1297).
use std::io::{self, Read};

pub fn slurp<R: Read>(mut reader: R) -> io::Result<String> {
    let mut s = String::new();
    reader.read_to_string(&mut s)?;
    Ok(s)
}

pub fn into_room<R: Read>(mut reader: R, room: usize) -> io::Result<usize> {
    let mut v = Vec::with_capacity(room);
    reader.read_to_end(&mut v)
}

pub fn fill<R: Read>(mut reader: R, v: &mut Vec<u8>) -> io::Result<usize> {
    reader.read_to_end(v)
}

pub struct Letters(pub usize);

impl Read for Letters {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.0);
        print!(" letters {}->{n}", buf.len());
        buf[..n].fill(b'l');
        self.0 -= n;
        Ok(n)
    }
}
