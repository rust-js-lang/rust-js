// A reader's `bytes()` in generic code, stepped by `next()`: std's compiled
// `Bytes::next` is given the `Bytes` it reads, as it is, never a JS
// iterator made of it, which has no `inner` (#2567). Held, stepped by a
// `for`, taken, and of the crate's own reader with a `Drop`.
use std::io::{self, Read};

struct Loud {
    data: Vec<u8>,
    at: usize,
}

impl Read for Loud {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.data.len() - self.at);
        buf[..n].copy_from_slice(&self.data[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop Loud at {}", self.at);
    }
}

fn first<R: Read>(r: R) -> Option<u8> {
    r.bytes().next().map(|b| b.unwrap())
}

fn held<R: Read>(r: R) -> Vec<u8> {
    let mut b = r.bytes();
    let x = b.next().unwrap().unwrap();
    let y = b.next().unwrap().unwrap();
    vec![x, y]
}

fn summed<R: Read>(r: R) -> u32 {
    let mut n = 0;
    for b in r.bytes() {
        n += b.unwrap() as u32;
    }
    n
}

fn taken<R: Read>(r: R) -> Vec<u8> {
    r.bytes().take(2).map(|b| b.unwrap()).collect()
}

fn main() {
    println!("{:?}", first(&b"xy"[..]));
    println!("{:?}", first(&b""[..]));
    println!("{:?}", held(&b"xyz"[..]));
    println!("{}", summed(&b"ab"[..]));
    println!("{:?}", taken(&b"abc"[..]));
    println!("{:?}", first(io::Cursor::new(vec![7u8, 8])));
    let loud = Loud {
        data: vec![1, 2, 3],
        at: 0,
    };
    println!("{:?}", held(loud));
    let loud = Loud {
        data: vec![4, 5],
        at: 0,
    };
    println!("{}", summed(loud));
}
