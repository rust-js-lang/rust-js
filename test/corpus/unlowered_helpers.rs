// A type's own `io::Read` is never lowered (ADR 0425), and so neither is a
// private function only it calls, base64's `DecoderReader`'s
// `read_from_delegate`, which reads its delegate: no JS calls them.
use std::io::{self, Read};

struct Counting<R> {
    inner: R,
    count: usize,
}

impl<R: Read> Counting<R> {
    fn new(inner: R) -> Self {
        Counting { inner, count: 0 }
    }

    fn read_from_delegate(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.count += n;
        Ok(n)
    }
}

impl<R: Read> Read for Counting<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_from_delegate(buf)
    }
}

fn main() {
    let reader = Counting::new(&b"abc"[..]);
    println!("{} {}", reader.count, reader.inner.len());
}
