// A type's own `io::Seek` is never lowered (ADR 0425), and so neither is a
// private function only it calls, as base64's `DecoderReader`'s
// `read_from_delegate` was of its `io::Read`, lowered since (#1297): no JS
// calls them.
use std::io::{self, Seek, SeekFrom};

struct Counting<S> {
    inner: S,
    count: usize,
}

impl<S: Seek> Counting<S> {
    fn new(inner: S) -> Self {
        Counting { inner, count: 0 }
    }

    fn seek_in_delegate(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let at = self.inner.seek(pos)?;
        self.count += 1;
        Ok(at)
    }
}

impl<S: Seek> Seek for Counting<S> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek_in_delegate(pos)
    }
}

fn main() {
    let seeker = Counting::new(io::Cursor::new(b"abc"));
    println!("{} {}", seeker.count, seeker.inner.get_ref().len());
}
