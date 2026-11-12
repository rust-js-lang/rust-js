// A `Take`'s `read_to_end` and `read_to_string`, and a `Chain`'s
// `read_to_string`, are std's provided methods, alloc's
// `default_read_to_end`: its 32-byte probe reads, `try_reserve(32)` where
// fewer than 32 bytes are spare, its reads of the spare room capped by a
// `max_read_size` that doubles, or is uncapped where a source's `read_buf`
// leaves its buffer uninitialized. Each source's: a `repeat`'s and a
// slice's leave it so, a `Cursor`'s too, a `BufReader`'s copies from its
// buffer. So the capacity of the `Vec` it grows is std's, at each probe and
// growth edge, into a new `Vec` and into one with room to spare (#1340).
// A `String`'s is read alike, its capacity none a JS string carries (#1261).
use std::io::{self, BufReader, Cursor, Read};

fn show(label: &str, n: usize, v: &[u8], cap: usize, read: io::Result<usize>) {
    println!("{label} {n}: read {:?} len {} cap {cap}", read.map_err(|e| e.kind()), v.len());
}

fn main() {
    let data: Vec<u8> = (0..9000u32).map(|i| b'a' + (i % 26) as u8).collect();
    let counts = [0, 1, 31, 32, 33, 50, 64, 100, 4095, 4096, 4097, 5000, 8192, 8193];
    for &n in &counts {
        let mut v = Vec::new();
        let read = io::repeat(1).take(n as u64).read_to_end(&mut v);
        show("repeat", n, &v, v.capacity(), read);

        let mut v = Vec::new();
        let read = (&data[..]).take(n as u64).read_to_end(&mut v);
        show("slice", n, &v, v.capacity(), read);

        let mut v = Vec::with_capacity(10);
        v.extend_from_slice(b"ab");
        let read = (&data[..]).take(n as u64).read_to_end(&mut v);
        show("slice into room", n, &v, v.capacity(), read);

        let mut v = Vec::new();
        let read = Cursor::new(&data[..]).take(n as u64).read_to_end(&mut v);
        show("cursor", n, &v, v.capacity(), read);

        let mut v = Vec::new();
        let read = BufReader::new(&data[..]).take(n as u64).read_to_end(&mut v);
        show("bufreader", n, &v, v.capacity(), read);

        let mut v = Vec::new();
        let read = (&data[..n / 2]).chain(&data[..n - n / 2]).take(n as u64).read_to_end(&mut v);
        show("chain", n, &v, v.capacity(), read);

        let mut s = String::new();
        let read = (&data[..]).take(n as u64).read_to_string(&mut s);
        println!("take string {n}: read {:?} len {}", read.map_err(|e| e.kind()), s.len());

        let mut s = String::new();
        let read = (&data[..n / 2]).chain(&data[..n - n / 2]).read_to_string(&mut s);
        println!("chain string {n}: read {:?} len {}", read.map_err(|e| e.kind()), s.len());
    }
    let mut v = Vec::new();
    let read = io::empty().take(5).read_to_end(&mut v);
    show("empty", 5, &v, v.capacity(), read);
}
