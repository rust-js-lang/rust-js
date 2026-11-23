//@ library-differs
// A trait's provided method its type's impl writes itself is that impl's,
// called on a type parameter or a `dyn` too, as Rust's dispatch picks:
// `BufRead::read_line`, of a reader of the crate's that writes it; and std's
// default, of std's own readers, which write none (#2414).
use std::io::{BufRead, Read};

struct Lines(Vec<u8>, usize);

impl Read for Lines {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = (self.0.len() - self.1).min(buf.len());
        buf[..n].copy_from_slice(&self.0[self.1..self.1 + n]);
        self.1 += n;
        Ok(n)
    }
}

impl BufRead for Lines {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Ok(&self.0[self.1..])
    }
    fn consume(&mut self, amount: usize) {
        self.1 += amount;
    }
    fn read_line(&mut self, buf: &mut String) -> std::io::Result<usize> {
        println!("own read_line");
        buf.push_str("overridden\n");
        Ok(11)
    }
}

fn first_line<R: BufRead>(r: &mut R) -> String {
    let mut line = String::new();
    r.read_line(&mut line).unwrap();
    line
}

fn dyn_line(r: &mut dyn BufRead) -> String {
    let mut line = String::new();
    r.read_line(&mut line).unwrap();
    line
}

fn main() {
    let mut plain: &[u8] = b"p
q
";
    print!("{}", dyn_line(&mut plain));
    let mut lines = Lines(b"a\nb\n".to_vec(), 0);
    print!("{}", first_line(&mut lines));
    let mut line = String::new();
    lines.read_line(&mut line).unwrap();
    print!("{line}");
    print!("{}", dyn_line(&mut lines));
}
