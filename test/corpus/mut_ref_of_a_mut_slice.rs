// A `&mut` of a `&mut [T]`, a cursor over a buffer: a write of the whole
// through it re-points the slice, `*buf = &mut take(buf)[n..]`, and a write
// through the slice writes the buffer, each seen by the caller (#1282).
// std's `Write for &mut [u8]` advances its slice so, `Read for &[u8]` its
// shared one, and httparse fills a `&mut &mut [Header]` and shortens it.
use std::io::{Read, Write};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Header<'a> {
    name: &'a str,
    value: u32,
}

// httparse's shape: each header written in turn, the slice shortened to
// what's left, the caller's slice then what's left of its array.
fn parse<'h, 'b>(text: &'b str, headers: &mut &'h mut [Header<'b>]) -> usize {
    let mut count = 0;
    for line in text.lines() {
        let Some((name, value)) = line.split_once(": ") else { continue };
        let slot = std::mem::take(headers);
        let Some((first, rest)) = slot.split_first_mut() else { break };
        *first = Header { name, value: value.parse().unwrap_or(0) };
        *headers = rest;
        count += 1;
    }
    count
}

fn fill(buf: &mut &mut [u8], byte: u8, n: usize) {
    for i in 0..n {
        buf[i] = byte;
    }
    let taken = std::mem::take(buf);
    *buf = &mut taken[n..];
}

// Each once refused, now as Rust: the caller's slice re-pointed by a
// parameter, through a struct that holds the `&mut`, and by
// `split_off_mut` through a held one.
fn reset<'a>(s: &mut &'a mut [i32], other: &'a mut [i32]) {
    *s = other;
}

struct Holder<'a, 'b> {
    s: &'a mut &'b mut [i32],
}

fn held() -> (Vec<i32>, Vec<i32>) {
    let mut xs = vec![1, 2];
    let mut ys = vec![9];
    {
        let mut rest: &mut [i32] = &mut xs;
        let h = Holder { s: &mut rest };
        *h.s = &mut ys;
        rest[0] = 5;
    }
    (xs, ys)
}

fn split() -> Vec<i32> {
    let mut w = vec![1, 2, 3, 4, 5];
    let mut rest: &mut [i32] = &mut w;
    let r = &mut rest;
    if let Some(h) = r.split_off_mut(..2) {
        h[0] = 9;
    }
    rest[0] = -7;
    w
}

fn swap_ends(buf: &mut &mut [i32]) {
    let n = buf.len();
    if n > 1 {
        buf.swap(0, n - 1);
    }
    let taken = std::mem::take(buf);
    if let Some((_, rest)) = taken.split_last_mut() {
        *buf = rest;
    }
}

pub fn main() {
    // std's `Write for &mut [u8]`, which advances the slice.
    let mut storage = [0u8; 12];
    let mut cursor: &mut [u8] = &mut storage;
    cursor.write_all(b"head").unwrap();
    write!(cursor, "-{}-", 42).unwrap();
    let left = cursor.len();
    let written = cursor.write(b"overflowing!").unwrap();
    println!("{left} {written} {:?}", std::str::from_utf8(&storage));

    // `(&mut buf[..])` as the issue writes it.
    let mut buf = [b'.'; 8];
    (&mut buf[2..]).write_all(b"ab").unwrap();
    println!("{:?}", std::str::from_utf8(&buf));

    // std's `Read for &[u8]`, which advances its shared slice.
    let mut reader: &[u8] = b"one two three";
    let mut word = [0u8; 4];
    reader.read_exact(&mut word).unwrap();
    let mut rest = String::new();
    reader.read_to_string(&mut rest).unwrap();
    println!("{:?} {rest:?} {}", std::str::from_utf8(&word), reader.len());

    // A function of the crate's own, its caller's slice re-pointed.
    let mut data = [0u8; 6];
    let mut at: &mut [u8] = &mut data;
    fill(&mut at, 7, 2);
    fill(&mut at, 9, 3);
    println!("{} {data:?}", at.len());

    let mut numbers = [1, 2, 3, 4, 5];
    let mut window: &mut [i32] = &mut numbers;
    swap_ends(&mut window);
    swap_ends(&mut window);
    println!("{window:?}");
    println!("{numbers:?}");

    let mut a = [1, 2, 3];
    let mut b = [7, 8];
    let mut s: &mut [i32] = &mut a;
    reset(&mut s, &mut b);
    s[1] = 0;
    println!("{a:?} {b:?} {:?} {:?}", held(), split());

    // httparse's.
    let mut headers = [Header::default(); 4];
    let mut free: &mut [Header] = &mut headers;
    let n = parse("Host: 80\nAccept: 1\nnone\nAge: 7", &mut free);
    println!("{n} {} {:?}", free.len(), &headers[..n]);
}
