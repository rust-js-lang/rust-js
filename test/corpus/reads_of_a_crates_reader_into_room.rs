// A reader of the crate's own sees each read std's `read_to_end` makes, of
// the spare capacity of what it reads into: a `Vec::with_capacity`'s, and a
// `String::new()`'s, none. The crate never asks a capacity, but its reader
// shows it, so rust-js keeps it, and the reads are std's (#1297).
use std::io::{self, Read};

struct Logged(usize);

impl Read for Logged {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.0);
        print!(" {}->{n}", buf.len());
        buf[..n].fill(b'r');
        self.0 -= n;
        Ok(n)
    }
}

fn into_room<R: Read>(mut reader: R, room: usize) -> usize {
    let mut v = Vec::with_capacity(room);
    reader.read_to_end(&mut v).unwrap()
}

fn main() {
    for (room, left) in [(0, 5), (10, 4), (64, 100), (100, 64), (1000, 3000)] {
        print!("{room} {left}:");
        let mut v = Vec::with_capacity(room);
        let read = Logged(left).read_to_end(&mut v).unwrap();
        println!(" = {read}");
    }
    print!("generic:");
    println!(" = {}", into_room(Logged(70), 40));
    print!("string:");
    let mut s = String::new();
    let read = Logged(40).read_to_string(&mut s).unwrap();
    println!(" = {read} {}", s.len());
}
