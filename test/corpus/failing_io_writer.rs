// An `io::Write` of the crate's whose `write` fails, an `Err` of its
// `io::Result<usize>`: std's `write_all` of it gives that `Err`, and so
// does a function that gives on what `write_all` gave, where it fails and
// `Ok` where it doesn't (#1072).
use std::io::{self, Write};

struct Full(usize);

impl Write for Full {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.0 {
            return Err(io::Error::other("full"));
        }
        self.0 -= bytes.len();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn put(w: &mut Full, bytes: &[u8]) -> io::Result<()> {
    w.write_all(bytes)
}

fn main() {
    let mut full = Full(4);
    println!("{} {}", put(&mut full, b"abc").is_ok(), full.0);
    println!("{} {}", put(&mut full, b"abc").is_err(), full.0);
    match full.write_all(b"ab") {
        Ok(()) => println!("written"),
        Err(e) => println!("refused: {e}"),
    }
}
