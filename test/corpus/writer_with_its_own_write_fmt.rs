// A writer of the crate's own that defines its own `write_fmt`: a `write!`
// into it calls that function, given the `Arguments` as any function of the
// crate's is, the function that makes its text (#1565, #1695), directly and
// through generic code's `W: fmt::Write`.
use std::fmt::{self, Write};

struct Text(String);

impl Write for Text {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(s);
        Ok(())
    }
}

// Counts its `write_fmt` calls, and passes what it's given on, bracketed.
struct Counting {
    calls: u32,
    text: Text,
}

impl Write for Counting {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.text.write_str(s)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.calls += 1;
        self.text.write_str("<")?;
        self.text.write_fmt(args)?;
        self.text.write_str(">")
    }
}

fn put<W: Write>(w: &mut W, n: i32) -> fmt::Result {
    write!(w, "[{n:>3}]")
}

fn main() {
    let mut w = Counting {
        calls: 0,
        text: Text(String::new()),
    };
    let n = -5;
    write!(w, "a{}b", n).unwrap();
    writeln!(w, "c").unwrap();
    w.write_str("plain").unwrap();
    put(&mut w, 7).unwrap();
    println!("{} {:?}", w.calls, w.text.0);
}
