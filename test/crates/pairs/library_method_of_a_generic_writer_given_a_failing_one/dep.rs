//! A library's generic struct of a writer, whose methods write through it:
//! its consumer's writer may fail (#1392).
use std::io::{self, Write};

pub struct Sink<W: Write> {
    out: W,
    written: usize,
}

impl<W: Write> Sink<W> {
    pub fn new(out: W) -> Self {
        Sink { out, written: 0 }
    }

    pub fn put(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.out.write_all(bytes)?;
        self.written += bytes.len();
        Ok(())
    }

    pub fn put_all(&mut self, lines: &[&str]) -> usize {
        lines.iter().filter(|line| self.put(line.as_bytes()).is_ok()).count()
    }

    pub fn written(&self) -> usize {
        self.written
    }
}
