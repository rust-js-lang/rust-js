//! A writer of its own that takes three writes and refuses the rest.
use std::io::{self, Write};

struct Budget(u8);

impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0 == 0 {
            return Err(io::Error::other("spent"));
        }
        self.0 -= 1;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn main() {
    let mut sink = dep::Sink::new(Budget(3));
    println!("{:?}", sink.put(b"ab").map_err(|e| e.to_string()));
    println!("{}", sink.put_all(&["c", "de", "f"]));
    println!("{:?} {}", sink.put(b"g").map_err(|e| e.to_string()), sink.written());
}
