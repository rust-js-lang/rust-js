// A writer of the crate's own, as anyhow's and writeable's are: `write!`
// into it gives its `write_str` the text, whole, and `write_char` a `char`.
// Its text is Rust's; how many `write_str` calls carry it isn't (ADR 0166).

use std::fmt::{self, Write};

#[derive(Default)]
struct Collector {
    text: String,
    bytes: usize,
}

impl Write for Collector {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.text.push_str(s);
        self.bytes += s.len();
        Ok(())
    }
}

struct Shouting(String);

impl Write for Shouting {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(&s.to_uppercase());
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.push('<');
        self.0.push(c);
        self.0.push('>');
        Ok(())
    }
}

struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

fn describe(out: &mut Collector, p: &Point) -> fmt::Result {
    writeln!(out, "at {p} [{:>6}]", "zoë")?;
    out.write_char('é')?;
    out.write_str("!")
}

fn main() {
    let mut c = Collector::default();
    write!(c, "{} and {:?}", 1, "two").unwrap();
    describe(&mut c, &Point { x: 3, y: -4 }).unwrap();
    println!("{:?} {}", c.text, c.bytes);

    let mut s = Shouting(String::new());
    write!(s, "hi {}", 5).unwrap();
    s.write_char('x').unwrap();
    let _ = writeln!(s);
    println!("{:?}", s.0);
}
