// A writer of bytes of the crate's, `impl io::Write`, given where a `W:
// Write` goes: its dictionary's `write_all` and `write_fmt` are std's, its
// `write` till all's written, as arrayvec's `ArrayVec<u8, CAP>`'s are (ADR
// 0392); a write to one of known type, its dictionary's too.
use std::io::{self, Write};

struct Buf<const CAP: usize> {
    bytes: Vec<u8>,
}

impl<const CAP: usize> Write for Buf<CAP> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let len = (CAP - self.bytes.len()).min(data.len());
        self.bytes.extend_from_slice(&data[..len]);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// With a destructor, as an `ArrayVec`'s is: a write doesn't drop it.
impl<const CAP: usize> Drop for Buf<CAP> {
    fn drop(&mut self) {
        println!("dropped {}", self.bytes.len());
    }
}

fn render<W: Write>(out: &mut W, n: u32) -> io::Result<()> {
    write!(out, "n={n};")?;
    out.write_all(b"ok")
}

fn main() {
    let mut b: Buf<16> = Buf { bytes: Vec::new() };
    render(&mut b, 7).unwrap();
    let n = b.write(b"!!").ok();
    write!(b, "{}", 1).unwrap_or(());
    b.write_all(b"?").unwrap_or(());
    println!("{} {:?}", String::from_utf8(b.bytes.clone()).unwrap(), n);
}
