//! A reader of the consumer's own, given to the library's generic code, and
//! the library's reader read by the consumer: each sees std's reads (#1297).
use std::io::{self, Read};

struct Logged(usize);

impl Read for Logged {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.0);
        print!(" logged {}->{n}", buf.len());
        buf[..n].fill(b'g');
        self.0 -= n;
        Ok(n)
    }
}

pub fn main() {
    println!(" = {:?}", dep::slurp(Logged(40)).map(|s| s.len()).map_err(|e| e.kind()));
    println!(" = {:?}", dep::into_room(Logged(70), 40).map_err(|e| e.kind()));
    let mut v = Vec::with_capacity(20);
    println!(
        " = {:?} {}",
        dep::fill(Logged(50), &mut v).map_err(|e| e.kind()),
        v.len()
    );
    let mut s = String::new();
    let read = dep::Letters(35).read_to_string(&mut s);
    println!(" = {:?} {}", read.map_err(|e| e.kind()), s.len());
    let mut v = Vec::with_capacity(8);
    let read = dep::Letters(9).take(30).read_to_end(&mut v);
    println!(" = {:?} {}", read.map_err(|e| e.kind()), v.len());
}
