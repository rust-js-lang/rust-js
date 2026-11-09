//! An `io::Write` whose `write` fails once it's full (#1072).
use std::io;

pub struct Full(pub usize);

impl io::Write for Full {
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
