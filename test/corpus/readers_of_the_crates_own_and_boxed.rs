//@ library-differs
// A reader of the crate's own that implements `BufRead` and `Seek` too: its
// `fill_buf` and `consume`, and std's provided `read_line`, `read_until` and
// `lines` over them; its `seek`, and std's provided `stream_position`,
// `seek_relative` and `rewind` over it. A `dyn Read`'s `read_to_string`, and
// a `Box<dyn Read>`'s calls, through what the `Box` holds, std's `impl Read
// for Box<R>`, generic code's too (#1489).
use std::io::{self, BufRead, Read, Seek, SeekFrom};

struct Text {
    data: Vec<u8>,
    pos: usize,
}

impl Read for Text {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

impl BufRead for Text {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        Ok(&self.data[self.pos..])
    }

    fn consume(&mut self, n: usize) {
        self.pos += n;
    }
}

impl Seek for Text {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let at = match to {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.pos as i64 + n,
            SeekFrom::End(n) => self.data.len() as i64 + n,
        };
        if at < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "before the start"));
        }
        self.pos = at as usize;
        Ok(self.pos as u64)
    }
}

fn text(s: &str) -> Text {
    Text {
        data: s.as_bytes().to_vec(),
        pos: 0,
    }
}

fn count_lines<R: BufRead>(r: R) -> usize {
    r.lines().count()
}

fn sum_bytes<R: Read + ?Sized>(r: &mut R) -> u32 {
    r.bytes().map(|b| b.unwrap() as u32).sum()
}

fn slurp(r: &mut dyn Read) -> io::Result<String> {
    let mut s = String::new();
    r.read_to_string(&mut s)?;
    Ok(s)
}

fn main() {
    let mut t = text("ab\ncd\nef");
    let mut line = String::new();
    println!("{:?} {line:?}", t.read_line(&mut line).map_err(|e| e.kind()));
    let mut until = Vec::new();
    println!("{:?} {until:?}", t.read_until(b'\n', &mut until).map_err(|e| e.kind()));
    println!("{:?}", t.stream_position().map_err(|e| e.kind()));
    println!("{:?}", t.seek_relative(-2).map_err(|e| e.kind()));
    println!("{:?}", t.stream_position().map_err(|e| e.kind()));
    println!("{:?}", t.seek_relative(-100).map_err(|e| (e.kind(), e.to_string())));
    t.rewind().unwrap();
    println!(
        "{:?} {}",
        t.seek(SeekFrom::End(-2)).map_err(|e| e.kind()),
        count_lines(text("x\ny\nz"))
    );

    let mut own = text("dyn text");
    println!("{:?}", slurp(&mut own).map_err(|e| e.kind()));
    let mut source: &[u8] = b"slice";
    println!("{:?}", slurp(&mut source).map_err(|e| e.kind()));

    let mut boxed: Box<dyn Read> = Box::new(&b"boxed bytes"[..]);
    let mut first = [0u8; 5];
    println!("{:?} {first:?}", boxed.read(&mut first).map_err(|e| e.kind()));
    let mut rest = Vec::new();
    println!("{:?} {rest:?}", boxed.read_to_end(&mut rest).map_err(|e| e.kind()));
    let mut boxed_own: Box<dyn Read> = Box::new(text("own boxed"));
    let mut s = String::new();
    println!("{:?} {s:?}", boxed_own.read_to_string(&mut s).map_err(|e| e.kind()));
    let mut twice: Box<Box<dyn Read>> = Box::new(Box::new(text("twice")));
    let mut t2 = String::new();
    println!("{:?} {t2:?}", twice.read_to_string(&mut t2).map_err(|e| e.kind()));
    let mut generic: Box<dyn Read> = Box::new(text("abc"));
    println!("{}", sum_bytes(&mut generic));
}
