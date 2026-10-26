// Many items added at once, more than a JS call takes as its arguments:
// a `Vec<u8>`'s writes, a splice, a repeat and a heap's append each
// push them, not spread them (ADR 0919).
use std::collections::BinaryHeap;
use std::io::Write;

fn main() {
    let text = "ab".repeat(600_000);
    let mut bytes: Vec<u8> = Vec::new();
    write!(bytes, "{text}").unwrap();
    bytes.write_all(text.as_bytes()).unwrap();
    let writer: &mut dyn Write = &mut bytes;
    writer.write_all(text.as_bytes()).unwrap();
    println!("{} {:?}", bytes.len(), bytes.last());

    let mut v = vec![1u8, 2, 3];
    let removed: Vec<u8> = v.splice(1..2, std::iter::repeat_n(7, 1_100_000)).collect();
    println!("{} {removed:?} {:?} {:?}", v.len(), v.get(1), v.last());

    let repeated = [1u8, 2, 3].repeat(400_000).repeat(3);
    println!("{} {:?}", repeated.len(), repeated.last());

    let mut a: BinaryHeap<u32> = (0..1_100_000).collect();
    let mut b: BinaryHeap<u32> = (0..10).collect();
    a.append(&mut b);
    println!("{} {:?} {}", a.len(), a.peek(), b.len());
}
