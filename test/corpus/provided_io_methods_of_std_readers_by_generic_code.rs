//@ library-differs
// Generic code's `read_line`, `rewind` and `stream_position` of std's own
// readers, a slice, a `BufReader`, a `Cursor` and a `BufReader` of a
// `Chain`: std's defaults, which their dictionaries don't write (#2414).
use std::io::{BufRead, BufReader, Cursor, Read, Seek};

fn first_line<R: BufRead>(r: &mut R) -> String {
    let mut line = String::new();
    r.read_line(&mut line).unwrap();
    line
}

fn reset<S: Seek>(s: &mut S) -> u64 {
    s.rewind().unwrap();
    s.stream_position().unwrap()
}

fn main() {
    let mut slice: &[u8] = b"one\ntwo\n";
    print!("{}", first_line(&mut slice));
    let mut reader = BufReader::new(&b"three\nfour\n"[..]);
    print!("{}", first_line(&mut reader));
    let mut cursor = Cursor::new(b"five\nsix\n".to_vec());
    print!("{}", first_line(&mut cursor));
    println!("{}", reset(&mut cursor));
    let mut chained = (&b"x"[..]).chain(&b"y\nz"[..]);
    let mut buffered = BufReader::new(&mut chained);
    print!("{}", first_line(&mut buffered));
}
