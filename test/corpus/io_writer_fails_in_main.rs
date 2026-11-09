// An `io::Write` of the crate's whose `write` gives on the `Err` of a
// method of its own, where nothing of the crate's returns a write's result:
// `main`'s own `write_all` of it gives that `Err` (#1072).
use std::io::{self, Write};

struct Full(usize);

impl Full {
    fn take(&mut self, n: usize) -> io::Result<usize> {
        if n > self.0 {
            return Err(io::Error::other("full"));
        }
        self.0 -= n;
        Ok(n)
    }
}

impl Write for Full {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.take(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn main() {
    let mut full = Full(4);
    println!("{}", full.write_all(b"abc").is_ok());
    match full.write_all(b"ab") {
        Ok(()) => println!("written"),
        Err(e) => println!("refused: {e}"),
    }
}
