// A `Vec`'s capacity after std's byte sources read to their end into it
// (#1338): a slice's, a `Cursor`'s and a `VecDeque`'s `read_to_end` each
// reserve all they add first, `try_reserve(len)`, grown as std's growth
// does, known and, but a `VecDeque`'s (#1302), generic.
use std::collections::VecDeque;
use std::io::{Cursor, Read};

fn generic<R: Read>(mut r: R, mut v: Vec<u8>) -> (usize, usize) {
    r.read_to_end(&mut v).unwrap();
    (v.len(), v.capacity())
}

fn main() {
    let mut v = Vec::new();
    (&b"ab"[..]).read_to_end(&mut v).unwrap();
    println!("{} {}", v.len(), v.capacity());
    let mut v = vec![1u8, 2];
    (&[3u8; 9][..]).read_to_end(&mut v).unwrap();
    println!("{} {}", v.len(), v.capacity());
    let mut v = Vec::with_capacity(40);
    (&[4u8; 30][..]).read_to_end(&mut v).unwrap();
    println!("{} {}", v.len(), v.capacity());
    let mut v = Vec::new();
    (&b""[..]).read_to_end(&mut v).unwrap();
    println!("{} {}", v.len(), v.capacity());

    let mut v = Vec::new();
    Cursor::new(vec![1u8, 2, 3]).read_to_end(&mut v).unwrap();
    println!("{} {}", v.len(), v.capacity());
    let mut c = Cursor::new([5u8; 20]);
    c.set_position(15);
    let mut v = vec![0u8; 4];
    c.read_to_end(&mut v).unwrap();
    println!("{} {}", v.len(), v.capacity());

    let mut d: VecDeque<u8> = (1..=20).collect();
    let mut v = vec![0u8; 3];
    d.read_to_end(&mut v).unwrap();
    println!("{} {} {}", v.len(), v.capacity(), d.len());
    let mut d: VecDeque<u8> = VecDeque::from([1, 2]);
    let mut v = Vec::new();
    d.read_to_end(&mut v).unwrap();
    println!("{} {}", v.len(), v.capacity());

    println!("{:?}", generic(&b"abc"[..], Vec::new()));
    println!("{:?}", generic(Cursor::new([5u8; 40]), vec![9]));
    println!("{:?}", generic(&[7u8; 5][..], Vec::with_capacity(6)));
}
