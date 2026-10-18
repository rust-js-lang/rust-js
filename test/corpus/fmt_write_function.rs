// `fmt::write(w, args)` is `w.write_fmt(args)`, of a `&mut dyn Write`: the
// crate's own writer, as anyhow's `ensure!` renders its sides, and a
// `String`; and a function of any writer, a `dyn fmt::Write` (ADR 0446).
use std::fmt::{self, Write};

struct Buf {
    text: String,
    cap: usize,
}

impl Write for Buf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.text.len() + s.len() > self.cap {
            return Err(fmt::Error);
        }
        self.text.push_str(s);
        Ok(())
    }
}

fn put(w: &mut dyn Write, n: u32) -> fmt::Result {
    write!(w, "<{n}>")
}

fn main() {
    let mut buf = Buf { text: String::new(), cap: 8 };
    let short = fmt::write(&mut buf, format_args!("{:?}", [1, 2]));
    println!("{} {:?}", short.is_ok(), buf.text);
    let long = fmt::write(&mut buf, format_args!("{}", "too long"));
    println!("{} {:?}", long.is_ok(), buf.text);
    let mut s = String::from(">");
    fmt::write(&mut s, format_args!("{}-{}", 3, "x")).unwrap();
    println!("{s}");
    put(&mut s, 7).unwrap();
    let mut wide = Buf { text: String::new(), cap: 99 };
    put(&mut wide, 8).unwrap();
    println!("{s} {}", wide.text);
}
