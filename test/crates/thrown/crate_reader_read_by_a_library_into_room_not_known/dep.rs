//! A library's generic code reading into a `String` it's lent, which it
//! can't prove fresh-empty (#1297).
use std::io::Read;

pub fn read_into<R: Read>(mut reader: R, s: &mut String) -> usize {
    reader.read_to_string(s).unwrap()
}
